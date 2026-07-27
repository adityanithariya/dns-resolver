use crate::cache::Cache;
use crate::message::{Header, Message, Opcode, Rcode};
use crate::net::UdpTransport;
use crate::singleflight::{SharedResolveError, SingleFlight};
use crate::workerpool::{SubmitError, WorkerPool};

use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;

/// Runs a stub-resolver-facing DNS server: accepts queries over UDP, resolves
/// them iteratively (through the shared cache and singleflight dedup), and
/// replies. One thread per in-flight request is a deliberately simple
/// concurrency model for a mini project -- a production server would bound
/// this with a worker pool (or move to async) to avoid unbounded thread
/// growth under a flood of distinct concurrent names, since singleflight only
/// collapses *identical* in-flight lookups, not the total request volume.
pub fn run(bind_addr: &str) -> std::io::Result<()> {
    let socket = UdpSocket::bind(bind_addr)?;
    println!("dns_resolver server listening on {}", bind_addr);

    let cache = Arc::new(Cache::new());
    let singleflight = Arc::new(SingleFlight::new());
    let transport = UdpTransport::default();

    let mut buf = [0u8; 4096];
    let transport = Arc::new(transport);
    let workers = std::thread::available_parallelism()?.get() * 4;
    let pool = WorkerPool::new(workers, 1024);
    loop {
        let (n, src) = socket.recv_from(&mut buf)?;
        let data = buf[..n].to_vec();

        let socket_for_reply = socket.try_clone()?;
        let cache = Arc::clone(&cache);
        let singleflight = Arc::clone(&singleflight);
        let transport_clone = Arc::clone(&transport);
        let data_clone = data.clone();

        match pool.try_submit(Box::new(move || {
            handle_query(
                &socket_for_reply,
                src,
                &data_clone,
                &transport_clone,
                &cache,
                &singleflight,
            );
        })) {
            Ok(_) => {}
            Err(SubmitError::QueueFull) | Err(SubmitError::PoolShutDown) => {
                send_servfail(&socket, src, &data)
            }
        }
    }
}

fn handle_query(
    socket: &UdpSocket,
    src: SocketAddr,
    data: &[u8],
    transport: &UdpTransport,
    cache: &Cache,
    singleflight: &SingleFlight,
) {
    let request = match Message::decode(data) {
        Ok(m) => m,
        Err(_) => return, // malformed request; nothing sensible to reply with
    };

    let Some(question) = request.questions.first() else {
        return;
    };
    let name = question.name.to_string();
    let qtype = question.qtype;

    let outcome = singleflight.resolve(transport, cache, &name, qtype);

    let mut response = Message {
        header: Header {
            id: request.header.id,
            qr: true,
            opcode: Opcode::Query,
            aa: false,
            tc: false,
            rd: request.header.rd,
            ra: true,
            rcode: Rcode::NoError,
            qdcount: 0,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        },
        questions: request.questions.clone(),
        answers: vec![],
        authorities: vec![],
        additionals: vec![],
    };

    match outcome {
        Ok(records) => response.answers = records,
        Err(SharedResolveError::NxDomain) => response.header.rcode = Rcode::NxDomain,
        Err(SharedResolveError::NoData) => {} // NOERROR with an empty answer section
        Err(_) => response.header.rcode = Rcode::ServFail,
    }

    let bytes = response.encode();
    let _ = socket.send_to(&bytes, src);
}

fn send_servfail(socket: &UdpSocket, src: SocketAddr, data: &[u8]) {
    // Attempt to extract the request ID so the client can correlate this reply.
    // If decoding fails, fall back to ID 0.
    let request = match Message::decode(data) {
        Ok(m) => m,
        Err(_) => return, // malformed request; nothing sensible to reply with
    };

    let response = Message {
        header: Header {
            id: request.header.id,
            qr: true, // Mark as Response
            opcode: Opcode::Query,
            aa: false,
            tc: false,
            rd: request.header.rd,
            ra: true,
            rcode: Rcode::ServFail, // Indicates server capacity / processing failure
            qdcount: request.questions.len() as u16,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        },
        questions: request.questions,
        answers: vec![],
        authorities: vec![],
        additionals: vec![],
    };

    let bytes = response.encode();
    let _ = socket.send_to(&bytes, src);
}
