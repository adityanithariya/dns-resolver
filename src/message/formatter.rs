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
