mod message;
mod net;
mod resolver;
mod root_hints;

use message::QType;
use net::UdpTransport;

fn main() {
    let transport = UdpTransport::default();

    let args: Vec<String> = std::env::args().collect();
    let name = args.get(1).map(|s| s.as_str()).unwrap_or("billitx.adityanithariya.com");

    println!("Resolving {} (A) iteratively from root hints...", name);
    match resolver::resolve(&transport, name, QType::A) {
        Ok(records) => {
            for r in records {
                println!("{} -> {:?}", name, r.rdata);
            }
        }
        Err(e) => println!("Resolution failed: {:?}", e),
    }
}
