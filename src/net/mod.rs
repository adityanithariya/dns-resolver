use crate::message::Message;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

#[derive(Debug)]
pub enum QueryError {
    Io(std::io::Error),
    Decode(crate::message::DecodeError),
    IdMismatch,
    Timeout,
}

impl From<std::io::Error> for QueryError {
    fn from(e: std::io::Error) -> Self {
        QueryError::Io(e)
    }
}

/// Abstraction over "send this query to this server, get a response back".
/// Lets the resolution logic in resolver.rs be tested with a scripted mock,
/// independent of whatever the real network happens to do.
pub trait Transport {
    fn query(&self, server: SocketAddr, msg: &Message) -> Result<Message, QueryError>;
}

pub struct UdpTransport {
    pub timeout: Duration,
    pub attempts: u32,
}

impl Default for UdpTransport {
    fn default() -> Self {
        UdpTransport {
            timeout: Duration::from_secs(3),
            attempts: 2,
        }
    }
}

impl Transport for UdpTransport {
    fn query(&self, server: SocketAddr, msg: &Message) -> Result<Message, QueryError> {
        query_udp(server, msg, self.timeout, self.attempts)
    }
}

/// Send `msg` to `server` over UDP and wait for a matching response.
/// Retries up to `attempts` times on timeout, each with its own fresh socket
/// bound to an ephemeral port (source port randomization is cheap insurance
/// against off-path spoofing, so we get it for free by not reusing a socket).
pub fn query_udp(
    server: SocketAddr,
    msg: &Message,
    timeout: Duration,
    attempts: u32,
) -> Result<Message, QueryError> {
    let request_bytes = msg.encode();
    let expected_id = msg.header.id;

    let mut last_err = QueryError::Timeout;

    for _ in 0..attempts {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(timeout))?;
        socket.connect(server)?;

        if let Err(e) = socket.send(&request_bytes) {
            last_err = QueryError::Io(e);
            continue;
        }

        let mut buf = [0u8; 4096]; // room for EDNS0-sized responses, not just the classic 512
        match socket.recv(&mut buf) {
            Ok(n) => match Message::decode(&buf[..n]) {
                Ok(response) => {
                    if response.header.id != expected_id {
                        // Stale or spoofed reply for a different query; ignore and retry.
                        last_err = QueryError::IdMismatch;
                        continue;
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_err = QueryError::Decode(e);
                    continue;
                }
            },
            Err(e) => {
                last_err = QueryError::Io(e);
                continue;
            }
        }
    }

    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Message, QType};
    use std::net::SocketAddr;
    use std::time::Duration;

    #[test]
    fn test_query_udp() {
        let id = 0x1234;
        let query = Message::new_query(id, "example.com", QType::A);

        let root_server: SocketAddr = "198.41.0.4:53".parse().unwrap(); // a.root-servers.net
        println!("Querying root server {} for example.com A...", root_server);

        match query_udp(root_server, &query, Duration::from_secs(3), 2) {
            Ok(resp) => {
                println!(
                    "Got response, id={:#06x}, rcode={:?}",
                    resp.header.id, resp.header.rcode
                );
                println!(
                    "ancount={} nscount={} arcount={}",
                    resp.header.ancount, resp.header.nscount, resp.header.arcount
                );
                for a in &resp.answers {
                    println!("ANS:  {} {:?} {:?}", a.name.to_string(), a.rtype, a.rdata);
                }
                for a in &resp.authorities {
                    println!("AUTH: {} {:?} {:?}", a.name.to_string(), a.rtype, a.rdata);
                }
                for a in &resp.additionals {
                    println!("ADD:  {} {:?} {:?}", a.name.to_string(), a.rtype, a.rdata);
                }
            }
            Err(e) => println!("Query failed: {:?}", e),
        }
    }
}
