mod cache;
mod message;
mod net;
mod resolver;
mod root_hints;
mod server;
mod singleflight;
mod tls;
mod workerpool;
use std::io::Read;

use cache::Cache;
use message::{Message, QType};
use net::UdpTransport;

use crate::message::formatter;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("serve") {
        let host = args.get(2).map(String::as_str).unwrap_or("127.0.0.1");
        let port = args.get(3).map(String::as_str).unwrap_or("5300");

        if let Err(e) = server::run(host, port) {
            eprintln!("server error: {e}");
        }
        return;
    }

    if args.get(1).map(String::as_str) == Some("doh") {
        let method = args
            .get(2)
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| "post".to_string());

        let url = args
            .get(3)
            .map(String::as_str)
            .unwrap_or("http://127.0.0.1:8443/dns-query");

        let name = args.get(4).map(String::as_str).unwrap_or("example.com");

        let qtype = match args.get(5).map(|s| s.to_uppercase()) {
            Some(t) if t == "AAAA" => QType::AAAA,
            Some(t) if t == "NS" => QType::NS,
            Some(t) if t == "MX" => QType::MX,
            Some(t) if t == "TXT" => QType::TXT,
            Some(t) if t == "ANY" => QType::ANY,
            _ => QType::A,
        };

        let query = Message::new_query(0x1234, name, qtype);
        let body = query.encode();

        let mut response = match method.as_str() {
            "post" => ureq::post(url)
                .header("Content-Type", "application/dns-message")
                .header("Accept", "application/dns-message")
                .send(body)
                .expect("failed to contact DoH server"),

            "get" => {
                use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

                let dns = URL_SAFE_NO_PAD.encode(body);
                let url = format!("{url}?dns={dns}");

                ureq::get(&url)
                    .header("Accept", "application/dns-message")
                    .call()
                    .expect("failed to contact DoH server")
            }

            _ => {
                eprintln!("method must be either 'get' or 'post'");
                return;
            }
        };

        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .read_to_end(&mut bytes)
            .unwrap();

        let response = Message::decode(&bytes).expect("invalid DNS response");
        formatter::print_message(&response);

        return;
    }

    let transport = UdpTransport::default();
    let cache = Cache::new();
    let name = args.get(1).map(String::as_str).unwrap_or("example.com");

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
