use crate::message::{Message, Name, QType, RData, ResourceRecord};
use crate::net::{QueryError, Transport};
use crate::root_hints::root_server_addrs;
use std::net::SocketAddr;

#[derive(Debug)]
pub enum ResolveError {
    NxDomain,
    NoData,
    ServFail,
    TooManyHops,
    NoUsableServers,
    Transport(QueryError),
}

const MAX_HOPS: u32 = 20;

enum IterResult {
    Answer(Vec<ResourceRecord>),
    Cname(String),
}

fn names_equal_ci(a: &Name, b: &Name) -> bool {
    a.to_string().eq_ignore_ascii_case(&b.to_string())
}

/// Public entry point: resolve `qname`/`qtype` from the root down, following
/// CNAMEs as needed. Returns the final matching records for `qtype`.
pub fn resolve<T: Transport>(
    transport: &T,
    qname: &str,
    qtype: QType,
) -> Result<Vec<ResourceRecord>, ResolveError> {
    let mut current_name = qname.to_string();
    let mut hops = 0u32;
    let mut cname_chases = 0u32;

    loop {
        match resolve_iterative(transport, &current_name, qtype, &mut hops)? {
            IterResult::Answer(records) => return Ok(records),
            IterResult::Cname(next_name) => {
                cname_chases += 1;
                if cname_chases > 10 {
                    return Err(ResolveError::TooManyHops);
                }
                current_name = next_name;
                println!("cname: {}", current_name);
            }
        }
    }
}

/// One full iterative walk (root -> ... -> authoritative) for a single name,
/// without following CNAMEs itself (the caller does that). `hops` is shared
/// across the whole resolution, including nested glue lookups, so a
/// pathological zone can't make us loop forever.
fn resolve_iterative<T: Transport>(
    transport: &T,
    name: &str,
    qtype: QType,
    hops: &mut u32,
) -> Result<IterResult, ResolveError> {
    let mut servers: Vec<SocketAddr> = root_server_addrs();

    loop {
        *hops += 1;
        if *hops > MAX_HOPS {
            return Err(ResolveError::TooManyHops);
        }

        let resp = query_first_available(transport, &servers, name, qtype)?;

        match resp.header.rcode {
            crate::message::Rcode::NxDomain => return Err(ResolveError::NxDomain),
            crate::message::Rcode::NoError => {}
            _ => return Err(ResolveError::ServFail),
        }

        if !resp.answers.is_empty() {
            let matched: Vec<ResourceRecord> = resp
                .answers
                .iter()
                .filter(|r| r.rtype == qtype)
                .cloned()
                .collect();
            if !matched.is_empty() {
                return Ok(IterResult::Answer(matched));
            }
            if let Some(cname_rr) = resp.answers.iter().find(|r| r.rtype == QType::CNAME) {
                if let RData::CNAME(target) = &cname_rr.rdata {
                    return Ok(IterResult::Cname(target.to_string()));
                }
            }
            return Err(ResolveError::NoData);
        }

        // Empty answer section: this should be a referral (NS in authority).
        if resp.authorities.is_empty() {
            return Err(ResolveError::NoData);
        }

        let ns_names: Vec<Name> = resp
            .authorities
            .iter()
            .filter_map(|r| match &r.rdata {
                RData::NS(n) => Some(n.clone()),
                _ => None,
            })
            .collect();

        println!("ns: {:?}", ns_names.iter().map(|ns| ns.to_string()).collect::<Vec<_>>());

        if ns_names.is_empty() {
            return Err(ResolveError::ServFail);
        }

        let mut next_servers = glued_addrs(&ns_names, &resp.additionals);

        if next_servers.is_empty() {
            // No glue for any NS name (e.g. out-of-bailiwick). Resolve one
            // nameserver's A record via its own nested iterative walk,
            // sharing the same hop budget so this can't be used to loop.
            for ns_name in &ns_names {
                *hops += 1;
                if *hops > MAX_HOPS {
                    return Err(ResolveError::TooManyHops);
                }
                if let Ok(IterResult::Answer(records)) =
                    resolve_iterative(transport, &ns_name.to_string(), QType::A, hops)
                {
                    for r in records {
                        if let RData::A(ip) = r.rdata {
                            next_servers.push(SocketAddr::new(ip.into(), 53));
                        }
                    }
                }
                if !next_servers.is_empty() {
                    break;
                }
            }
        }

        if next_servers.is_empty() {
            return Err(ResolveError::ServFail);
        }

        servers = next_servers;
    }
}

fn glued_addrs(ns_names: &[Name], additionals: &[ResourceRecord]) -> Vec<SocketAddr> {
    let mut out = Vec::new();
    for ns_name in ns_names {
        for add in additionals {
            if names_equal_ci(&add.name, ns_name) {
                if let RData::A(ip) = &add.rdata {
                    out.push(SocketAddr::new((*ip).into(), 53));
                }
            }
        }
    }
    out
}

fn query_first_available<T: Transport>(
    transport: &T,
    servers: &[SocketAddr],
    name: &str,
    qtype: QType,
) -> Result<Message, ResolveError> {
    let mut last_err = ResolveError::NoUsableServers;
    for server in servers {
        let id: u16 = rand::random();
        let query = Message::new_query(id, name, qtype);
        match transport.query(*server, &query) {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                last_err = ResolveError::Transport(e);
                continue;
            }
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Header, Opcode, QClass, Question, Rcode};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::net::Ipv4Addr;

    /// A scripted transport: maps (server, qname, qtype) -> a canned response,
    /// so we can simulate a full root -> TLD -> authoritative referral chain
    /// without touching a real network.
    struct MockTransport {
        script: RefCell<HashMap<(SocketAddr, String, u16), Message>>,
    }

    impl MockTransport {
        fn new() -> Self {
            MockTransport {
                script: RefCell::new(HashMap::new()),
            }
        }

        fn program(&self, server: SocketAddr, qname: &str, qtype: QType, response: Message) {
            self.script
                .borrow_mut()
                .insert((server, qname.to_string(), qtype.to_u16()), response);
        }
    }

    impl Transport for MockTransport {
        fn query(&self, server: SocketAddr, msg: &Message) -> Result<Message, QueryError> {
            let q = &msg.questions[0];
            let key = (server, q.name.to_string(), q.qtype.to_u16());
            match self.script.borrow().get(&key) {
                Some(resp) => {
                    let mut resp = resp.clone();
                    resp.header.id = msg.header.id; // real servers echo the query id
                    Ok(resp)
                }
                None => Err(QueryError::Timeout),
            }
        }
    }

    fn referral(ns_name: &str, ns_ip: Ipv4Addr) -> Message {
        Message {
            header: Header {
                id: 0,
                qr: true,
                opcode: Opcode::Query,
                aa: false,
                tc: false,
                rd: false,
                ra: false,
                rcode: Rcode::NoError,
                qdcount: 1,
                ancount: 0,
                nscount: 1,
                arcount: 1,
            },
            questions: vec![Question {
                name: Name::from_str("example.com"),
                qtype: QType::A,
                qclass: QClass::IN,
            }],
            answers: vec![],
            authorities: vec![ResourceRecord {
                name: Name::from_str("com"),
                rtype: QType::NS,
                rclass: QClass::IN,
                ttl: 172800,
                rdata: RData::NS(Name::from_str(ns_name)),
            }],
            additionals: vec![ResourceRecord {
                name: Name::from_str(ns_name),
                rtype: QType::A,
                rclass: QClass::IN,
                ttl: 172800,
                rdata: RData::A(ns_ip),
            }],
        }
    }

    fn final_answer(name: &str, ip: Ipv4Addr) -> Message {
        Message {
            header: Header {
                id: 0,
                qr: true,
                opcode: Opcode::Query,
                aa: true,
                tc: false,
                rd: false,
                ra: false,
                rcode: Rcode::NoError,
                qdcount: 1,
                ancount: 1,
                nscount: 0,
                arcount: 0,
            },
            questions: vec![Question {
                name: Name::from_str(name),
                qtype: QType::A,
                qclass: QClass::IN,
            }],
            answers: vec![ResourceRecord {
                name: Name::from_str(name),
                rtype: QType::A,
                rclass: QClass::IN,
                ttl: 300,
                rdata: RData::A(ip),
            }],
            authorities: vec![],
            additionals: vec![],
        }
    }

    #[test]
    fn walks_root_to_tld_to_authoritative() {
        let transport = MockTransport::new();

        let tld_server: SocketAddr = "192.5.6.30:53".parse().unwrap(); // a.gtld-servers.net
        let auth_server: SocketAddr = "93.184.216.34:53".parse().unwrap(); // pretend authoritative NS

        // Every root hint returns the same referral to the .com TLD servers.
        for root in root_server_addrs() {
            transport.program(
                root,
                "example.com",
                QType::A,
                referral("a.gtld-servers.net", "192.5.6.30".parse().unwrap()),
            );
        }

        // The TLD server refers down to the (pretend) authoritative server.
        transport.program(
            tld_server,
            "example.com",
            QType::A,
            referral("ns1.example.com", "93.184.216.34".parse().unwrap()),
        );

        // The authoritative server gives the final answer.
        transport.program(
            auth_server,
            "example.com",
            QType::A,
            final_answer("example.com", "93.184.216.34".parse().unwrap()),
        );

        let result = resolve(&transport, "example.com", QType::A).unwrap();
        assert_eq!(result.len(), 1);
        match result[0].rdata {
            RData::A(ip) => assert_eq!(ip, "93.184.216.34".parse::<Ipv4Addr>().unwrap()),
            _ => panic!("expected A record"),
        }
    }

    #[test]
    fn nxdomain_propagates_as_error() {
        let transport = MockTransport::new();
        let mut nx = final_answer("nope.invalid", "0.0.0.0".parse().unwrap());
        nx.header.rcode = Rcode::NxDomain;
        nx.answers.clear();
        nx.header.ancount = 0;

        for root in root_server_addrs() {
            transport.program(root, "nope.invalid", QType::A, nx.clone());
        }

        let result = resolve(&transport, "nope.invalid", QType::A);
        assert!(matches!(result, Err(ResolveError::NxDomain)));
    }
}
