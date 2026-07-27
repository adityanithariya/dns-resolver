mod cache;
mod message;
mod net;
mod resolver;
mod root_hints;
mod server;
mod singleflight;
mod workerpool;

use cache::Cache;
use message::QType;
use net::UdpTransport;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("serve") {
        let bind_addr = args.get(2).map(|s| s.as_str()).unwrap_or("127.0.0.1:5300");
        if let Err(e) = server::run(bind_addr) {
            eprintln!("server error: {}", e);
        }
        return;
    }

    let transport = UdpTransport::default();
    let cache = Cache::new();
    let name = args.get(1).map(|s| s.as_str()).unwrap_or("example.com");

    println!("Resolving {} (A) iteratively from root hints...", name);
    match resolver::resolve(&transport, &cache, name, QType::A) {
        Ok(records) => {
            for r in records {
                println!("{} -> {:?}", name, r.rdata);
            }
        }
        Err(e) => println!("Resolution failed: {:?}", e),
    }
}
