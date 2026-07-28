use rustls::{ServerConnection, StreamOwned};

use crate::cache::Cache;
use crate::message::{self, Header, Message, Opcode, RData, Rcode};
use crate::net::UdpTransport;
use crate::singleflight::{SharedResolveError, SingleFlight};
use crate::tls;
use crate::workerpool::{SubmitError, WorkerPool};
use std::io::{Read, Write};

use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::sync::Arc;
use std::thread;

pub struct ServerContext {
    pub cache: Arc<Cache>,
    pub singleflight: Arc<SingleFlight>,
    pub transport: Arc<UdpTransport>,
    pub pool: Arc<WorkerPool>,
}

impl ServerContext {
    pub fn new(workers: usize) -> ServerContext {
        ServerContext {
            cache: Arc::new(Cache::new()),
            singleflight: Arc::new(SingleFlight::new()),
            transport: Arc::new(UdpTransport::default()),
            pool: Arc::new(WorkerPool::new(workers, 1024)),
        }
    }
}

pub fn run(host: &str, port: &str) -> std::io::Result<()> {
    let addr = format!("{}:{}", host, port);
    let udp = UdpSocket::bind(addr.clone())?;
    let tcp = TcpListener::bind(addr)?;
    let dot = TcpListener::bind(format!("{}:{}", host, "8100"))?;

    println!("dns_resolver server listening on {}", host);

    let mut buf = [0u8; 4096];

    let workers = std::thread::available_parallelism()?.get() * 4;
    let ctx = Arc::new(ServerContext::new(workers));

    {
        let ctx = Arc::clone(&ctx);
        thread::spawn(move || run_tcp_listener(tcp, &ctx));
    }
    {
        let ctx = Arc::clone(&ctx);
        thread::spawn(move || run_tls_listener(dot, &ctx));
    }

    loop {
        let (n, src) = udp.recv_from(&mut buf)?;
        let data = buf[..n].to_vec();

        let socket_for_reply = udp.try_clone()?;
        let cache = Arc::clone(&ctx.cache);
        let singleflight = Arc::clone(&ctx.singleflight);
        let transport_clone = Arc::clone(&ctx.transport);
        let data_clone = data.clone();

        match ctx.pool.try_submit(Box::new(move || {
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
                send_servfail(&udp, src, &data)
            }
        }
    }
}

fn run_tcp_listener(tcp: TcpListener, ctx: &ServerContext) {
    for stream in tcp.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let pool = Arc::clone(&ctx.pool);
        let transport = Arc::clone(&ctx.transport);
        let cache = Arc::clone(&ctx.cache);
        let singleflight = Arc::clone(&ctx.singleflight);

        let _ = pool.try_submit(Box::new(move || {
            handle_tcp(stream, &transport, &cache, &singleflight);
        }));
    }
}

fn run_tls_listener(tcp: TcpListener, ctx: &ServerContext) {
    for stream in tcp.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let pool = Arc::clone(&ctx.pool);
        let transport = Arc::clone(&ctx.transport);
        let cache = Arc::clone(&ctx.cache);
        let singleflight = Arc::clone(&ctx.singleflight);

        if let Ok(config) = tls::get_tls_config() {
            let config = Arc::new(config);
            {
                if let Ok(conn) = ServerConnection::new(Arc::clone(&config)) {
                    let tls_stream = StreamOwned::new(conn, stream);
                    let _ = pool.try_submit(Box::new(move || {
                        handle_tcp(tls_stream, &transport, &cache, &singleflight);
                    }));
                }
            }
        }
    }
}

fn resolve_message(
    request: &Message,
    transport: &UdpTransport,
    cache: &Cache,
    singleflight: &SingleFlight,
    is_udp: bool,
) -> Result<Vec<u8>, SharedResolveError> {
    let Some(question) = request.questions.first() else {
        return Err(SharedResolveError::NoData);
    };

    let name = question.name.to_string();
    let qtype = question.qtype;

    let outcome = singleflight.resolve(transport, cache, &name, qtype);
    let mut response = get_default_res(&request);

    match outcome {
        Ok(records) => response.answers = records,
        Err(SharedResolveError::NxDomain) => response.header.rcode = Rcode::NxDomain,
        Err(SharedResolveError::NoData) => {} // NOERROR with an empty answer section
        Err(_) => response.header.rcode = Rcode::ServFail,
    }

    let bytes = response.encode();

    if !is_udp {
        return Ok(bytes);
    }

    let opt = request
        .additionals
        .iter()
        .find(|rr| rr.rtype == message::QType::OPT);

    let udp_limit: usize = match opt {
        Some(opt) => match opt.rdata.clone() {
            RData::OPT(opt) => opt.udp_payload_size as usize,
            _ => 512,
        },
        None => 512,
    };

    loop {
        let bytes = response.encode();

        if bytes.len() <= udp_limit {
            return Ok(bytes);
        }

        response.header.tc = true;

        if !response.additionals.is_empty() {
            response.additionals.pop();
            continue;
        }

        if !response.authorities.is_empty() {
            response.authorities.pop();
            continue;
        }

        if !response.answers.is_empty() {
            response.answers.pop();
            continue;
        }

        break;
    }

    Ok(bytes)
}

fn get_default_res(request: &Message) -> Message {
    Message {
        header: Header {
            id: request.header.id,
            qr: true,
            opcode: Opcode::Query,
            aa: false,
            tc: request.header.tc,
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

    let outcome = resolve_message(&request, transport, cache, singleflight, true);
    match outcome {
        Ok(bytes) => {
            let _ = socket.send_to(&bytes, src);
        }
        Err(_) => {}
    }
}

fn handle_tcp<S>(
    mut stream: S,
    transport: &UdpTransport,
    cache: &Cache,
    singleflight: &SingleFlight,
) where
    S: Read + Write,
{
    let mut len_buf = [0; 2];

    if stream.read_exact(&mut len_buf).is_err() {
        return;
    }

    let len = u16::from_be_bytes(len_buf) as usize;

    let mut packet = vec![0; len];

    if stream.read_exact(&mut packet).is_err() {
        return;
    }

    let request = Message::decode(&packet).unwrap();

    let outcome = resolve_message(&request, transport, cache, singleflight, false);

    match outcome {
        Ok(bytes) => {
            let len = bytes.len() as u16;

            stream.write_all(&len.to_be_bytes()).ok();
            stream.write_all(&bytes).ok();
        }
        Err(_) => {}
    }
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
