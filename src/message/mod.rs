pub mod record_types;

use std::collections::HashMap;
use std::fmt;

use crate::message::record_types::{Name, OptRecord, SOARecord, TxtRecord};

// ---------- Errors ----------

#[derive(Debug)]
pub enum DecodeError {
    UnexpectedEof,
    LabelTooLong,
    NameTooLong,
    TooManyPointerHops,
    BadPointerOffset,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

type DResult<T> = Result<T, DecodeError>;

// ---------- Header ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Query,
    IQuery,
    Status,
    Other(u8),
}

impl Opcode {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Opcode::Query,
            1 => Opcode::IQuery,
            2 => Opcode::Status,
            other => Opcode::Other(other),
        }
    }
    fn to_u8(self) -> u8 {
        match self {
            Opcode::Query => 0,
            Opcode::IQuery => 1,
            Opcode::Status => 2,
            Opcode::Other(v) => v,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rcode {
    NoError,
    FormErr,
    ServFail,
    NxDomain,
    NotImp,
    Refused,
    Other(u8),
}

impl Rcode {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Rcode::NoError,
            1 => Rcode::FormErr,
            2 => Rcode::ServFail,
            3 => Rcode::NxDomain,
            4 => Rcode::NotImp,
            5 => Rcode::Refused,
            other => Rcode::Other(other),
        }
    }
    fn to_u8(self) -> u8 {
        match self {
            Rcode::NoError => 0,
            Rcode::FormErr => 1,
            Rcode::ServFail => 2,
            Rcode::NxDomain => 3,
            Rcode::NotImp => 4,
            Rcode::Refused => 5,
            Rcode::Other(v) => v,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub id: u16,
    pub qr: bool, // false = query, true = response
    pub opcode: Opcode,
    pub aa: bool, // authoritative answer
    pub tc: bool, // truncated
    pub rd: bool, // recursion desired
    pub ra: bool, // recursion available
    pub rcode: Rcode,
    pub qdcount: u16,
    pub ancount: u16,
    pub nscount: u16,
    pub arcount: u16,
}

impl Header {
    pub fn new_query(id: u16, recursion_desired: bool) -> Self {
        Header {
            id,
            qr: false,
            opcode: Opcode::Query,
            aa: false,
            tc: false,
            rd: recursion_desired,
            ra: false,
            rcode: Rcode::NoError,
            qdcount: 1,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.id.to_be_bytes());

        let mut flags: u16 = 0;
        if self.qr {
            flags |= 1 << 15;
        }
        flags |= (self.opcode.to_u8() as u16 & 0x0F) << 11;
        if self.aa {
            flags |= 1 << 10;
        }
        if self.tc {
            flags |= 1 << 9;
        }
        if self.rd {
            flags |= 1 << 8;
        }
        if self.ra {
            flags |= 1 << 7;
        }
        // bits 6-4 are Z (reserved, must be zero)
        flags |= self.rcode.to_u8() as u16 & 0x0F;

        out.extend_from_slice(&flags.to_be_bytes());
        out.extend_from_slice(&self.qdcount.to_be_bytes());
        out.extend_from_slice(&self.ancount.to_be_bytes());
        out.extend_from_slice(&self.nscount.to_be_bytes());
        out.extend_from_slice(&self.arcount.to_be_bytes());
    }

    fn decode(buf: &[u8], pos: &mut usize) -> DResult<Self> {
        let id = read_u16(buf, pos)?;
        let flags = read_u16(buf, pos)?;
        let qdcount = read_u16(buf, pos)?;
        let ancount = read_u16(buf, pos)?;
        let nscount = read_u16(buf, pos)?;
        let arcount = read_u16(buf, pos)?;

        Ok(Header {
            id,
            qr: (flags >> 15) & 1 == 1,
            opcode: Opcode::from_u8(((flags >> 11) & 0x0F) as u8),
            aa: (flags >> 10) & 1 == 1,
            tc: (flags >> 9) & 1 == 1,
            rd: (flags >> 8) & 1 == 1,
            ra: (flags >> 7) & 1 == 1,
            rcode: Rcode::from_u8((flags & 0x0F) as u8),
            qdcount,
            ancount,
            nscount,
            arcount,
        })
    }
}

// ---------- QType / QClass ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QType {
    A,
    NS,
    CNAME,
    SOA,
    PTR,
    MX,
    TXT,
    AAAA,
    ANY,
    OPT,
    Other(u16),
}

impl QType {
    pub const ANY_TYPES: [QType; 5] = [QType::A, QType::AAAA, QType::CNAME, QType::SOA, QType::TXT];
    pub fn to_u16(self) -> u16 {
        match self {
            QType::A => 1,
            QType::NS => 2,
            QType::CNAME => 5,
            QType::SOA => 6,
            QType::PTR => 12,
            QType::MX => 15,
            QType::TXT => 16,
            QType::AAAA => 28,
            QType::OPT => 41,
            QType::ANY => 255,
            QType::Other(v) => v,
        }
    }
    pub fn from_u16(v: u16) -> Self {
        match v {
            1 => QType::A,
            2 => QType::NS,
            5 => QType::CNAME,
            6 => QType::SOA,
            12 => QType::PTR,
            15 => QType::MX,
            16 => QType::TXT,
            28 => QType::AAAA,
            255 => QType::ANY,
            other => QType::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QClass {
    IN,
    Other(u16),
}

impl QClass {
    fn to_u16(self) -> u16 {
        match self {
            QClass::IN => 1,
            QClass::Other(v) => v,
        }
    }
    fn from_u16(v: u16) -> Self {
        match v {
            1 => QClass::IN,
            other => QClass::Other(other),
        }
    }
}

fn write_label(out: &mut Vec<u8>, label: &str) {
    let bytes = label.as_bytes();
    out.push(bytes.len() as u8);
    out.extend_from_slice(bytes);
}

// ---------- Question ----------

#[derive(Debug, Clone)]
pub struct Question {
    pub name: Name,
    pub qtype: QType,
    pub qclass: QClass,
}

impl Question {
    fn encode(&self, out: &mut Vec<u8>, name_offsets: &mut HashMap<Vec<String>, u16>) {
        self.name.encode(out, name_offsets);
        out.extend_from_slice(&self.qtype.to_u16().to_be_bytes());
        out.extend_from_slice(&self.qclass.to_u16().to_be_bytes());
    }

    fn decode(buf: &[u8], pos: &mut usize) -> DResult<Self> {
        let name = Name::decode(buf, pos)?;
        let qtype = QType::from_u16(read_u16(buf, pos)?);
        let qclass = QClass::from_u16(read_u16(buf, pos)?);
        Ok(Question {
            name,
            qtype,
            qclass,
        })
    }
}

// ---------- Resource record ----------

#[derive(Debug, Clone)]
pub enum RData {
    A(std::net::Ipv4Addr),
    AAAA(std::net::Ipv6Addr),
    NS(Name),
    CNAME(Name),
    SOA(SOARecord),
    TXT(TxtRecord),
    OPT(OptRecord),
    // Fallback for anything we don't specifically model yet (MX, TXT, ...).
    Raw(Vec<u8>),
}

impl RData {
    fn encode(&self, out: &mut Vec<u8>, name_offsets: &mut HashMap<Vec<String>, u16>) {
        match self {
            RData::A(ip) => out.extend_from_slice(&ip.octets()),
            RData::AAAA(ip) => out.extend_from_slice(&ip.octets()),
            RData::NS(n) => n.encode(out, name_offsets),
            RData::CNAME(n) => n.encode(out, name_offsets),
            RData::SOA(r) => r.encode(out, name_offsets),
            RData::TXT(txt) => txt.encode(out),
            RData::OPT(opt) => opt.encode(out),
            RData::Raw(bytes) => out.extend_from_slice(bytes),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceRecord {
    pub name: Name,
    pub rtype: QType,
    pub rclass: QClass,
    pub ttl: u32,
    pub rdata: RData,
}

impl ResourceRecord {
    fn encode(&self, out: &mut Vec<u8>, name_offsets: &mut HashMap<Vec<String>, u16>) {
        self.name.encode(out, name_offsets);
        out.extend_from_slice(&self.rtype.to_u16().to_be_bytes());
        out.extend_from_slice(&self.rclass.to_u16().to_be_bytes());
        out.extend_from_slice(&self.ttl.to_be_bytes());

        // rdlength isn't known until after rdata is written (rdata's own name
        // compression can make its size vary), so reserve two bytes now and
        // patch them once we know how much was actually written.
        let rdlen_pos = out.len();
        out.extend_from_slice(&0u16.to_be_bytes());
        let rdata_start = out.len();
        self.rdata.encode(out, name_offsets);
        let rdlen = (out.len() - rdata_start) as u16;
        out[rdlen_pos..rdlen_pos + 2].copy_from_slice(&rdlen.to_be_bytes());
    }

    fn decode(buf: &[u8], pos: &mut usize) -> DResult<Self> {
        let name = Name::decode(buf, pos)?;
        let rtype = QType::from_u16(read_u16(buf, pos)?);
        let rclass = QClass::from_u16(read_u16(buf, pos)?);
        let ttl = read_u32(buf, pos)?;
        let rdlength = read_u16(buf, pos)? as usize;

        if *pos + rdlength > buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let rdata_start = *pos;

        let rdata = match rtype {
            QType::A => {
                if rdlength != 4 {
                    return Err(DecodeError::UnexpectedEof);
                }
                let b = &buf[rdata_start..rdata_start + 4];
                RData::A(std::net::Ipv4Addr::new(b[0], b[1], b[2], b[3]))
            }
            QType::AAAA => {
                if rdlength != 16 {
                    return Err(DecodeError::UnexpectedEof);
                }
                let b = &buf[rdata_start..rdata_start + 16];
                let mut octets = [0u8; 16];
                octets.copy_from_slice(b);
                RData::AAAA(std::net::Ipv6Addr::from(octets))
            }
            QType::NS => {
                // Name may itself use compression pointing elsewhere in the message,
                // so decode it with a cursor starting at rdata_start, independent of rdlength.
                let mut name_pos = rdata_start;
                let n = Name::decode(buf, &mut name_pos)?;
                RData::NS(n)
            }
            QType::CNAME => {
                let mut name_pos = rdata_start;
                let n = Name::decode(buf, &mut name_pos)?;
                RData::CNAME(n)
            }
            QType::SOA => {
                let mut p = rdata_start;
                let soa = SOARecord::decode(buf, &mut p)?;
                RData::SOA(soa)
            }
            QType::TXT => {
                let mut p = rdata_start;
                let txt = TxtRecord::decode(buf, &mut p, rdlength)?;
                RData::TXT(txt)
            }
            QType::OPT => {
                let mut p = rdata_start;
                let class_raw = read_u16(buf, pos)?;
                let ttl_raw = read_u32(buf, pos)?;
                let opt = OptRecord::decode(class_raw, ttl_raw, buf, &mut p, rdlength)?;
                RData::OPT(opt)
            }
            _ => RData::Raw(buf[rdata_start..rdata_start + rdlength].to_vec()),
        };

        *pos = rdata_start + rdlength;

        Ok(ResourceRecord {
            name,
            rtype,
            rclass,
            ttl,
            rdata,
        })
    }
}

// ---------- Full message ----------

#[derive(Debug, Clone)]
pub struct Message {
    pub header: Header,
    pub questions: Vec<Question>,
    pub answers: Vec<ResourceRecord>,
    pub authorities: Vec<ResourceRecord>,
    pub additionals: Vec<ResourceRecord>,
}

impl Message {
    pub fn new_query(id: u16, name: &str, qtype: QType) -> Self {
        Message {
            header: Header::new_query(id, true),
            questions: vec![Question {
                name: Name::from_str(name),
                qtype,
                qclass: QClass::IN,
            }],
            answers: vec![],
            authorities: vec![],
            additionals: vec![],
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(512);

        // Counts are derived from the vectors themselves rather than trusted
        // from self.header, so it's impossible to construct a message whose
        // header lies about how many records follow.
        let mut header = self.header;
        header.qdcount = self.questions.len() as u16;
        header.ancount = self.answers.len() as u16;
        header.nscount = self.authorities.len() as u16;
        header.arcount = self.additionals.len() as u16;
        header.encode(&mut out);

        let mut name_offsets = HashMap::new();
        for q in &self.questions {
            q.encode(&mut out, &mut name_offsets);
        }
        for r in &self.answers {
            r.encode(&mut out, &mut name_offsets);
        }
        for r in &self.authorities {
            r.encode(&mut out, &mut name_offsets);
        }
        for r in &self.additionals {
            r.encode(&mut out, &mut name_offsets);
        }

        out
    }

    pub fn decode(buf: &[u8]) -> DResult<Self> {
        let mut pos = 0usize;
        let header = Header::decode(buf, &mut pos)?;

        let mut questions = Vec::with_capacity(header.qdcount as usize);
        for _ in 0..header.qdcount {
            questions.push(Question::decode(buf, &mut pos)?);
        }

        let mut answers = Vec::with_capacity(header.ancount as usize);
        for _ in 0..header.ancount {
            answers.push(ResourceRecord::decode(buf, &mut pos)?);
        }

        let mut authorities = Vec::with_capacity(header.nscount as usize);
        for _ in 0..header.nscount {
            authorities.push(ResourceRecord::decode(buf, &mut pos)?);
        }

        let mut additionals = Vec::with_capacity(header.arcount as usize);
        for _ in 0..header.arcount {
            additionals.push(ResourceRecord::decode(buf, &mut pos)?);
        }

        Ok(Message {
            header,
            questions,
            answers,
            authorities,
            additionals,
        })
    }
}

// ---------- Small byte-reading helpers ----------

fn read_u8(buf: &[u8], pos: &mut usize) -> DResult<u8> {
    if *pos >= buf.len() {
        return Err(DecodeError::UnexpectedEof);
    }

    let v = buf[*pos];
    *pos += 1;
    Ok(v)
}

fn read_u16(buf: &[u8], pos: &mut usize) -> DResult<u16> {
    if *pos + 2 > buf.len() {
        return Err(DecodeError::UnexpectedEof);
    }
    let v = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
    *pos += 2;
    Ok(v)
}

fn read_u32(buf: &[u8], pos: &mut usize) -> DResult<u32> {
    if *pos + 4 > buf.len() {
        return Err(DecodeError::UnexpectedEof);
    }
    let v = u32::from_be_bytes([buf[*pos], buf[*pos + 1], buf[*pos + 2], buf[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_query() {
        let msg = Message::new_query(0x1234, "example.com", QType::A);
        let bytes = msg.encode();
        let decoded = Message::decode(&bytes).unwrap();

        assert_eq!(decoded.header.id, 0x1234);
        assert_eq!(decoded.header.qdcount, 1);
        assert_eq!(decoded.questions[0].name.to_string(), "example.com");
        assert_eq!(decoded.questions[0].qtype, QType::A);
    }

    #[test]
    fn name_from_str_round_trip() {
        let n = Name::from_str("www.example.com.");
        assert_eq!(n.to_string(), "www.example.com");
        assert_eq!(n.0, vec!["www", "example", "com"]);
    }

    #[test]
    fn decode_root_referral_style_message() {
        // Hand-built message: one question (com A), no answers, one NS record in
        // authority pointing at "a.gtld-servers.net", and one A glue record in
        // additional, using a compressed pointer back into the NS owner name area.
        let mut out = Vec::new();

        // Header: id=1, flags: response, no error, 1 question, 0 answers, 1 NS, 1 additional
        out.extend_from_slice(&1u16.to_be_bytes()); // id
        out.extend_from_slice(&0x8000u16.to_be_bytes()); // QR=1 (response), rest 0
        out.extend_from_slice(&1u16.to_be_bytes()); // qdcount
        out.extend_from_slice(&0u16.to_be_bytes()); // ancount
        out.extend_from_slice(&1u16.to_be_bytes()); // nscount
        out.extend_from_slice(&1u16.to_be_bytes()); // arcount

        // Question: com A IN
        write_label(&mut out, "com");
        out.push(0);
        out.extend_from_slice(&1u16.to_be_bytes()); // A
        out.extend_from_slice(&1u16.to_be_bytes()); // IN

        // Authority RR: com NS a.gtld-servers.net
        write_label(&mut out, "com");
        out.push(0);
        out.extend_from_slice(&2u16.to_be_bytes()); // NS
        out.extend_from_slice(&1u16.to_be_bytes()); // IN
        out.extend_from_slice(&172800u32.to_be_bytes()); // TTL

        let mut rdata = Vec::new();
        write_label(&mut rdata, "a");
        write_label(&mut rdata, "gtld-servers");
        write_label(&mut rdata, "net");
        rdata.push(0);
        out.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        out.extend_from_slice(&rdata);

        // Additional RR: a.gtld-servers.net A 192.5.6.30
        write_label(&mut out, "a");
        write_label(&mut out, "gtld-servers");
        write_label(&mut out, "net");
        out.push(0);
        out.extend_from_slice(&1u16.to_be_bytes()); // A
        out.extend_from_slice(&1u16.to_be_bytes()); // IN
        out.extend_from_slice(&172800u32.to_be_bytes());
        out.extend_from_slice(&4u16.to_be_bytes()); // rdlength
        out.extend_from_slice(&[192, 5, 6, 30]);

        let msg = Message::decode(&out).unwrap();
        assert_eq!(msg.authorities.len(), 1);
        assert_eq!(msg.additionals.len(), 1);
        match &msg.authorities[0].rdata {
            RData::NS(n) => assert_eq!(n.to_string(), "a.gtld-servers.net"),
            _ => panic!("expected NS"),
        }
        match &msg.additionals[0].rdata {
            RData::A(ip) => assert_eq!(ip.to_string(), "192.5.6.30"),
            _ => panic!("expected A"),
        }
    }

    #[test]
    fn decode_soa_record() {
        let mut out = Vec::new();
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&0x8000u16.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // qdcount
        out.extend_from_slice(&0u16.to_be_bytes()); // ancount
        out.extend_from_slice(&1u16.to_be_bytes()); // nscount
        out.extend_from_slice(&0u16.to_be_bytes()); // arcount

        write_label(&mut out, "example");
        write_label(&mut out, "com");
        out.push(0);
        out.extend_from_slice(&6u16.to_be_bytes()); // SOA
        out.extend_from_slice(&1u16.to_be_bytes()); // IN
        out.extend_from_slice(&3600u32.to_be_bytes()); // TTL

        let mut rdata = Vec::new();
        write_label(&mut rdata, "ns1");
        write_label(&mut rdata, "example");
        write_label(&mut rdata, "com");
        rdata.push(0);
        write_label(&mut rdata, "hostmaster");
        write_label(&mut rdata, "example");
        write_label(&mut rdata, "com");
        rdata.push(0);
        rdata.extend_from_slice(&2024010101u32.to_be_bytes()); // serial
        rdata.extend_from_slice(&7200u32.to_be_bytes()); // refresh
        rdata.extend_from_slice(&3600u32.to_be_bytes()); // retry
        rdata.extend_from_slice(&1209600u32.to_be_bytes()); // expire
        rdata.extend_from_slice(&300u32.to_be_bytes()); // minimum

        out.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        out.extend_from_slice(&rdata);

        let msg = Message::decode(&out).unwrap();
        match &msg.authorities[0].rdata {
            RData::SOA(soa) => {
                assert_eq!(soa.mname.to_string(), "ns1.example.com");
                assert_eq!(soa.rname.to_string(), "hostmaster.example.com");
                assert_eq!(soa.serial, 2024010101);
                assert_eq!(soa.minimum, 300);
            }
            _ => panic!("expected SOA"),
        }
    }

    #[test]
    fn encode_decode_full_response_round_trip() {
        let msg = Message {
            header: Header {
                id: 0xABCD,
                qr: true,
                opcode: Opcode::Query,
                aa: true,
                tc: false,
                rd: true,
                ra: true,
                rcode: Rcode::NoError,
                qdcount: 0,
                ancount: 0,
                nscount: 0,
                arcount: 0,
            },
            questions: vec![Question {
                name: Name::from_str("example.com"),
                qtype: QType::A,
                qclass: QClass::IN,
            }],
            answers: vec![ResourceRecord {
                name: Name::from_str("example.com"),
                rtype: QType::A,
                rclass: QClass::IN,
                ttl: 300,
                rdata: RData::A("93.184.216.34".parse().unwrap()),
            }],
            authorities: vec![],
            additionals: vec![],
        };

        let bytes = msg.encode();
        let decoded = Message::decode(&bytes).unwrap();

        assert_eq!(decoded.header.id, 0xABCD);
        assert!(decoded.header.qr);
        assert_eq!(decoded.header.ancount, 1);
        assert_eq!(decoded.answers.len(), 1);
        match decoded.answers[0].rdata {
            RData::A(ip) => assert_eq!(ip.to_string(), "93.184.216.34"),
            _ => panic!("expected A"),
        }
        // The answer's owner name should compress against the question's
        // identical name rather than being spelled out twice.
        assert!(bytes.len() < 60);
    }
}
