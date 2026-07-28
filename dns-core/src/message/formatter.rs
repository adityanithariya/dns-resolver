use super::*;

pub fn print_message(msg: &Message) {
    println!(";; HEADER");
    println!("  ID:      {}", msg.header.id);
    println!("  QR:      {}", msg.header.qr);
    println!("  OPCODE:  {:?}", msg.header.opcode);
    println!("  AA:      {}", msg.header.aa);
    println!("  TC:      {}", msg.header.tc);
    println!("  RD:      {}", msg.header.rd);
    println!("  RA:      {}", msg.header.ra);
    println!("  RCODE:   {:?}", msg.header.rcode);

    println!();
    println!(";; COUNTS");
    println!("  Questions:   {}", msg.questions.len());
    println!("  Answers:     {}", msg.answers.len());
    println!("  Authority:   {}", msg.authorities.len());
    println!("  Additional:  {}", msg.additionals.len());

    if !msg.questions.is_empty() {
        println!();
        println!(";; QUESTION SECTION");
        for q in &msg.questions {
            println!(";{}\t{:?}\t{:?}", q.name.to_string(), q.qclass, q.qtype);
        }
    }

    if !msg.answers.is_empty() {
        println!();
        println!(";; ANSWER SECTION");
        for rr in &msg.answers {
            print_rr(rr);
        }
    }

    if !msg.authorities.is_empty() {
        println!();
        println!(";; AUTHORITY SECTION");
        for rr in &msg.authorities {
            print_rr(rr);
        }
    }

    if !msg.additionals.is_empty() {
        println!();
        println!(";; ADDITIONAL SECTION");
        for rr in &msg.additionals {
            print_rr(rr);
        }
    }
}

fn print_rr(rr: &ResourceRecord) {
    println!(
        "{}\t{}\t{:?}\t{:?}\t{}",
        rr.name.to_string(),
        rr.ttl,
        rr.rclass,
        rr.rtype,
        format_rdata(&rr.rdata)
    );
}

fn format_rdata(rdata: &RData) -> String {
    match rdata {
        RData::A(ip) => ip.to_string(),
        RData::AAAA(ip) => ip.to_string(),
        RData::NS(name) => name.to_string(),
        RData::CNAME(name) => name.to_string(),
        RData::TXT(txt) => txt.0.join(" "),
        RData::SOA(soa) => format!(
            "{} {} {} {} {} {} {}",
            soa.mname.to_string(),
            soa.rname.to_string(),
            soa.serial,
            soa.refresh,
            soa.retry,
            soa.expire,
            soa.minimum
        ),
        RData::OPT(opt) => format!(
            "UDP={} EXT_RCODE={} VERSION={} DO={} OPTIONS={}",
            opt.udp_payload_size,
            opt.extended_rcode,
            opt.version,
            opt.flags,
            opt.options.len()
        ),
        RData::Raw(bytes) => format!("{:02x?}", bytes),
    }
}

pub fn format_message(msg: &Message) -> String {
    let mut output = String::new();

    output.push_str(";; HEADER\n");
    output.push_str(&format!("  ID:      {}\n", msg.header.id));
    output.push_str(&format!("  QR:      {}\n", msg.header.qr));
    output.push_str(&format!("  OPCODE:  {:?}\n", msg.header.opcode));
    output.push_str(&format!("  AA:      {}\n", msg.header.aa));
    output.push_str(&format!("  TC:      {}\n", msg.header.tc));
    output.push_str(&format!("  RD:      {}\n", msg.header.rd));
    output.push_str(&format!("  RA:      {}\n", msg.header.ra));
    output.push_str(&format!("  RCODE:   {:?}\n", msg.header.rcode));

    output.push_str("\n;; COUNTS\n");
    output.push_str(&format!("  Questions:   {}\n", msg.questions.len()));
    output.push_str(&format!("  Answers:     {}\n", msg.answers.len()));
    output.push_str(&format!("  Authority:   {}\n", msg.authorities.len()));
    output.push_str(&format!("  Additional:  {}\n", msg.additionals.len()));

    if !msg.questions.is_empty() {
        output.push_str("\n;; QUESTION SECTION\n");
        for q in &msg.questions {
            output.push_str(&format!(
                ";{}\t{:?}\t{:?}\n",
                q.name.to_string(),
                q.qclass,
                q.qtype
            ));
        }
    }

    if !msg.answers.is_empty() {
        output.push_str("\n;; ANSWER SECTION\n");
        for rr in &msg.answers {
            output.push_str(&format_rr(rr));
        }
    }

    if !msg.authorities.is_empty() {
        output.push_str("\n;; AUTHORITY SECTION\n");
        for rr in &msg.authorities {
            output.push_str(&format_rr(rr));
        }
    }

    if !msg.additionals.is_empty() {
        output.push_str("\n;; ADDITIONAL SECTION\n");
        for rr in &msg.additionals {
            output.push_str(&format_rr(rr));
        }
    }

    output
}

fn format_rr(rr: &ResourceRecord) -> String {
    format!(
        "{}\t{}\t{:?}\t{:?}\t{}\n",
        rr.name.to_string(),
        rr.ttl,
        rr.rclass,
        rr.rtype,
        format_rdata(&rr.rdata)
    )
}
