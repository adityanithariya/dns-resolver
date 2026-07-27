use crate::message::{QType, Rcode, ResourceRecord, record_types::Name};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub enum CacheHit {
    Positive(Vec<ResourceRecord>),
    Negative(Rcode),
}

struct PositiveEntry {
    records: Vec<ResourceRecord>,
    expires_at: Instant,
}

struct NegativeEntry {
    rcode: Rcode,
    expires_at: Instant,
}

struct ZoneEntry {
    servers: Vec<SocketAddr>,
    expires_at: Instant,
}

/// Two layers, mirroring what a real resolver keeps:
/// - `answers`/`negative`: the final RRset (or NXDOMAIN/NoData) for a name+type.
/// - `zones`: which servers are authoritative for a zone we've already been
///   referred to, so a later query under that zone doesn't have to restart at
///   the root. This is what makes a warmed-up resolver fast in practice — most
///   of the win isn't caching final answers, it's skipping already-known
///   referral hops.
pub struct Cache {
    answers: RwLock<HashMap<(String, u16), PositiveEntry>>,
    negative: RwLock<HashMap<(String, u16), NegativeEntry>>,
    zones: RwLock<HashMap<String, ZoneEntry>>,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            answers: RwLock::new(HashMap::new()),
            negative: RwLock::new(HashMap::new()),
            zones: RwLock::new(HashMap::new()),
        }
    }

    fn key(name: &str, qtype: QType) -> (String, u16) {
        (name.trim_end_matches('.').to_lowercase(), qtype.to_u16())
    }

    pub fn get_answer(&self, name: &str, qtype: QType) -> Option<CacheHit> {
        let key = Self::key(name, qtype);
        let now = Instant::now();

        if let Some(entry) = self.answers.read().unwrap().get(&key) {
            if entry.expires_at > now {
                return Some(CacheHit::Positive(entry.records.clone()));
            }
        }
        if let Some(entry) = self.negative.read().unwrap().get(&key) {
            if entry.expires_at > now {
                return Some(CacheHit::Negative(entry.rcode));
            }
        }
        None
    }

    pub fn put_answer(&self, name: &str, qtype: QType, records: &[ResourceRecord]) {
        if records.is_empty() {
            return;
        }
        // RFC 1035: TTL for a cached RRset is the minimum TTL across its members.
        let ttl = records.iter().map(|r| r.ttl).min().unwrap_or(0);
        let entry = PositiveEntry {
            records: records.to_vec(),
            expires_at: Instant::now() + Duration::from_secs(ttl as u64),
        };
        self.answers
            .write()
            .unwrap()
            .insert(Self::key(name, qtype), entry);
    }

    pub fn put_negative(&self, name: &str, qtype: QType, rcode: Rcode, ttl_secs: u32) {
        let entry = NegativeEntry {
            rcode,
            expires_at: Instant::now() + Duration::from_secs(ttl_secs as u64),
        };
        self.negative
            .write()
            .unwrap()
            .insert(Self::key(name, qtype), entry);
    }

    pub fn put_zone(&self, zone_name: &str, servers: Vec<SocketAddr>, ttl_secs: u32) {
        if servers.is_empty() {
            return;
        }
        let entry = ZoneEntry {
            servers,
            expires_at: Instant::now() + Duration::from_secs(ttl_secs as u64),
        };
        self.zones
            .write()
            .unwrap()
            .insert(zone_name.trim_end_matches('.').to_lowercase(), entry);
    }

    /// Find the most specific cached zone that is a suffix of `qname`'s labels,
    /// e.g. for "www.example.com" prefer a cached "example.com" zone over a
    /// cached "com" zone if both are present. Returns that zone's known servers
    /// so resolution can start there instead of at the root.
    pub fn best_zone_servers(&self, qname: &Name) -> Option<Vec<SocketAddr>> {
        let qlabels: Vec<String> = qname.0.iter().map(|l| l.to_lowercase()).collect();
        let now = Instant::now();

        let zones = self.zones.read().unwrap();
        let mut best: Option<(usize, Vec<SocketAddr>)> = None;

        for (zone_name, entry) in zones.iter() {
            if entry.expires_at <= now {
                continue;
            }
            let zlabels: Vec<&str> = zone_name.split('.').filter(|s| !s.is_empty()).collect();
            if zlabels.len() > qlabels.len() {
                continue;
            }
            let is_suffix = zlabels
                .iter()
                .rev()
                .zip(qlabels.iter().rev())
                .all(|(a, b)| a.eq_ignore_ascii_case(b));

            if is_suffix {
                let better = best.as_ref().map_or(true, |(len, _)| zlabels.len() > *len);
                if better {
                    best = Some((zlabels.len(), entry.servers.clone()));
                }
            }
        }

        best.map(|(_, servers)| servers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{QClass, RData};
    use std::net::Ipv4Addr;
    use std::thread::sleep;

    fn a_record(name: &str, ip: Ipv4Addr, ttl: u32) -> ResourceRecord {
        ResourceRecord {
            name: Name::from_str(name),
            rtype: QType::A,
            rclass: QClass::IN,
            ttl,
            rdata: RData::A(ip),
        }
    }

    #[test]
    fn positive_hit_then_expiry() {
        let cache = Cache::new();
        let rrs = vec![a_record("example.com", "1.2.3.4".parse().unwrap(), 1)];
        cache.put_answer("example.com", QType::A, &rrs);

        match cache.get_answer("example.com", QType::A) {
            Some(CacheHit::Positive(records)) => assert_eq!(records.len(), 1),
            _ => panic!("expected a positive cache hit"),
        }

        sleep(Duration::from_millis(1100));
        assert!(cache.get_answer("example.com", QType::A).is_none());
    }

    #[test]
    fn negative_hit_records_rcode() {
        let cache = Cache::new();
        cache.put_negative("nope.invalid", QType::A, Rcode::NxDomain, 300);
        match cache.get_answer("nope.invalid", QType::A) {
            Some(CacheHit::Negative(Rcode::NxDomain)) => {}
            _ => panic!("expected a negative cache hit"),
        }
    }

    #[test]
    fn most_specific_zone_wins() {
        let cache = Cache::new();
        let com_servers = vec!["192.5.6.30:53".parse().unwrap()];
        let example_servers = vec!["93.184.216.34:53".parse().unwrap()];

        cache.put_zone("com", com_servers.clone(), 3600);
        cache.put_zone("example.com", example_servers.clone(), 3600);

        let result = cache
            .best_zone_servers(&Name::from_str("www.example.com"))
            .unwrap();
        assert_eq!(result, example_servers);

        let result2 = cache.best_zone_servers(&Name::from_str("other.org"));
        assert!(result2.is_none());

        let result3 = cache
            .best_zone_servers(&Name::from_str("another.com"))
            .unwrap();
        assert_eq!(result3, com_servers);
    }
}
