use crate::cache::Cache;
use crate::message::{QType, ResourceRecord};
use crate::net::Transport;
use crate::resolver::{self, ResolveError};
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

/// A Clone-able, simplified error for broadcasting a resolution outcome to
/// waiting followers. ResolveError itself can't be Clone (it wraps
/// std::io::Error inside QueryError), so this collapses transport-level
/// detail into one variant — followers only need "it failed", the leader
/// already logged/handled the specific cause.
#[derive(Debug, Clone)]
pub enum SharedResolveError {
    NxDomain,
    NoData,
    ServFail,
    TooManyHops,
    NoUsableServers,
    Transport,
}

impl From<ResolveError> for SharedResolveError {
    fn from(e: ResolveError) -> Self {
        match e {
            ResolveError::NxDomain => SharedResolveError::NxDomain,
            ResolveError::NoData => SharedResolveError::NoData,
            ResolveError::ServFail => SharedResolveError::ServFail,
            ResolveError::TooManyHops => SharedResolveError::TooManyHops,
            ResolveError::NoUsableServers => SharedResolveError::NoUsableServers,
            ResolveError::Transport(_) => SharedResolveError::Transport,
        }
    }
}

pub type SharedResult = Result<Vec<ResourceRecord>, SharedResolveError>;

/// One in-flight resolution that other callers can wait on.
struct Waiter {
    result: Mutex<Option<SharedResult>>,
    cvar: Condvar,
}

/// Deduplicates concurrent identical in-flight resolutions (the "singleflight"
/// pattern). If two callers ask for the same (name, qtype) while a
/// resolution is already underway, the second caller blocks on the first's
/// result instead of independently re-walking root -> TLD -> authoritative.
/// This matters most exactly when the cache *can't* help: the first request
/// for a name under load, before anything is cached, is precisely when a
/// naive server would fire off N redundant walks of the same hierarchy.
pub struct SingleFlight {
    inflight: Mutex<HashMap<(String, u16), Arc<Waiter>>>,
}

impl SingleFlight {
    pub fn new() -> Self {
        SingleFlight {
            inflight: Mutex::new(HashMap::new()),
        }
    }

    pub fn resolve<T: Transport>(
        &self,
        transport: &T,
        cache: &Cache,
        name: &str,
        qtype: QType,
    ) -> SharedResult {
        let key = (name.trim_end_matches('.').to_lowercase(), qtype.to_u16());

        // Whoever finds no existing entry becomes the leader and does the
        // real work; everyone else finds the leader's Waiter and blocks on it.
        let (is_leader, waiter) = {
            let mut map = self.inflight.lock().unwrap();
            if let Some(existing) = map.get(&key) {
                (false, Arc::clone(existing))
            } else {
                let w = Arc::new(Waiter {
                    result: Mutex::new(None),
                    cvar: Condvar::new(),
                });
                map.insert(key.clone(), Arc::clone(&w));
                (true, w)
            }
        };

        if is_leader {
            let outcome: SharedResult =
                resolver::resolve(transport, cache, name, qtype).map_err(SharedResolveError::from);

            {
                let mut slot = waiter.result.lock().unwrap();
                *slot = Some(outcome.clone());
            }
            waiter.cvar.notify_all();

            // Remove only after publishing the result, so a straggler that
            // locks the map between publish and removal still finds (and
            // joins) this entry rather than starting a redundant lookup.
            self.inflight.lock().unwrap().remove(&key);

            outcome
        } else {
            let mut slot = waiter.result.lock().unwrap();
            while slot.is_none() {
                slot = waiter.cvar.wait(slot).unwrap();
            }
            slot.clone().unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Header, Message, record_types::Name, Opcode, QClass, RData, Rcode};
    use crate::net::QueryError;
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    /// Answers every query the same way after a deliberate delay, and counts
    /// how many times it was actually invoked -- lets us prove deduplication
    /// happened rather than just that the answer was eventually correct.
    struct SlowCountingTransport {
        calls: AtomicUsize,
        delay: Duration,
    }

    impl Transport for SlowCountingTransport {
        fn query(&self, _server: SocketAddr, msg: &Message) -> Result<Message, QueryError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            thread::sleep(self.delay);
            Ok(Message {
                header: Header {
                    id: msg.header.id,
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
                questions: msg.questions.clone(),
                answers: vec![ResourceRecord {
                    name: Name::from_str("slow.example"),
                    rtype: QType::A,
                    rclass: QClass::IN,
                    ttl: 300,
                    rdata: RData::A(Ipv4Addr::new(1, 2, 3, 4)),
                }],
                authorities: vec![],
                additionals: vec![],
            })
        }
    }

    #[test]
    fn concurrent_identical_lookups_dedupe_to_one_network_call() {
        let transport = Arc::new(SlowCountingTransport {
            calls: AtomicUsize::new(0),
            delay: Duration::from_millis(200),
        });
        let cache = Arc::new(Cache::new());
        let sf = Arc::new(SingleFlight::new());
        let barrier = Arc::new(Barrier::new(4));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let transport = Arc::clone(&transport);
                let cache = Arc::clone(&cache);
                let sf = Arc::clone(&sf);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait(); // line all 4 callers up to hit resolve() at roughly the same instant
                    sf.resolve(transport.as_ref(), cache.as_ref(), "slow.example", QType::A)
                })
            })
            .collect();

        let results: Vec<SharedResult> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        for r in &results {
            assert!(
                r.is_ok(),
                "expected every caller to get a successful result"
            );
        }
        assert_eq!(
            transport.calls.load(Ordering::SeqCst),
            1,
            "expected only one real network call despite 4 concurrent identical lookups"
        );
    }

    #[test]
    fn distinct_names_are_not_deduped_against_each_other() {
        let transport = Arc::new(SlowCountingTransport {
            calls: AtomicUsize::new(0),
            delay: Duration::from_millis(50),
        });
        let cache = Arc::new(Cache::new());
        let sf = Arc::new(SingleFlight::new());

        let names = ["a.example", "b.example"];
        let handles: Vec<_> = names
            .iter()
            .map(|name| {
                let transport = Arc::clone(&transport);
                let cache = Arc::clone(&cache);
                let sf = Arc::clone(&sf);
                let name = name.to_string();
                thread::spawn(move || {
                    sf.resolve(transport.as_ref(), cache.as_ref(), &name, QType::A)
                })
            })
            .collect();

        for h in handles {
            assert!(h.join().unwrap().is_ok());
        }
        assert_eq!(
            transport.calls.load(Ordering::SeqCst),
            2,
            "two distinct names should each get their own real lookup"
        );
    }
}
