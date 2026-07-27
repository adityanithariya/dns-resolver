use std::collections::HashMap;

use super::*;
use crate::message::DResult;

// ---------- Name handling ----------

/// A domain name as an ordered list of labels, e.g. "example.com" -> ["example", "com"]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(pub Vec<String>);

impl Name {
    pub fn from_str(s: &str) -> Self {
        if s.is_empty() || s == "." {
            return Name(vec![]);
        }
        Name(
            s.trim_end_matches('.')
                .split('.')
                .map(|s| s.to_string())
                .collect(),
        )
    }

    pub fn to_string(&self) -> String {
        if self.0.is_empty() {
            return ".".to_string();
        }
        self.0.join(".")
    }

    /// Encode with compression: reuse the longest existing suffix match found in `name_offsets`.
    pub fn encode(&self, out: &mut Vec<u8>, name_offsets: &mut HashMap<Vec<String>, u16>) {
        // Walk label-suffixes from full name down to nothing, looking for a match to point to.
        for start in 0..=self.0.len() {
            let suffix = &self.0[start..];
            if start > 0 {
                if let Some(&offset) = name_offsets.get(suffix) {
                    // Emit the labels before this suffix literally, then a pointer.
                    for label in &self.0[..start] {
                        write_label(out, label);
                    }
                    let pointer = 0xC000u16 | offset;
                    out.extend_from_slice(&pointer.to_be_bytes());
                    return;
                }
            }
        }

        // No match found anywhere: record offsets for every suffix (as long as they fit in 14 bits)
        // and emit the name in full, terminated by a zero length octet.
        let mut offset_cursor = out.len();
        for (i, label) in self.0.iter().enumerate() {
            if offset_cursor <= 0x3FFF {
                name_offsets.insert(self.0[i..].to_vec(), offset_cursor as u16);
            }
            write_label(out, label);
            offset_cursor = out.len();
        }
        out.push(0);
    }

    pub fn decode(buf: &[u8], pos: &mut usize) -> DResult<Self> {
        let mut labels = Vec::new();
        let mut cursor = *pos;
        let mut jumped = false;
        let mut post_pointer_pos = 0usize;
        let mut hops = 0;

        loop {
            if cursor >= buf.len() {
                return Err(DecodeError::UnexpectedEof);
            }
            let len = buf[cursor];

            if len == 0 {
                cursor += 1;
                break;
            } else if len & 0xC0 == 0xC0 {
                // Pointer: 2 bytes, top two bits are 11, remaining 14 bits are the offset.
                if cursor + 1 >= buf.len() {
                    return Err(DecodeError::UnexpectedEof);
                }
                let b2 = buf[cursor + 1];
                let offset = (((len & 0x3F) as usize) << 8) | (b2 as usize);

                if !jumped {
                    post_pointer_pos = cursor + 2;
                    jumped = true;
                }

                hops += 1;
                if hops > 128 {
                    return Err(DecodeError::TooManyPointerHops);
                }
                if offset >= buf.len() {
                    return Err(DecodeError::BadPointerOffset);
                }
                cursor = offset;
            } else if len & 0xC0 == 0 {
                // Regular label, up to 63 bytes.
                let len = len as usize;
                let start = cursor + 1;
                let end = start + len;
                if end > buf.len() {
                    return Err(DecodeError::UnexpectedEof);
                }
                let label = String::from_utf8_lossy(&buf[start..end]).to_string();
                labels.push(label);
                cursor = end;
                if labels.len() > 128 {
                    return Err(DecodeError::NameTooLong);
                }
            } else {
                return Err(DecodeError::LabelTooLong);
            }
        }

        *pos = if jumped { post_pointer_pos } else { cursor };
        Ok(Name(labels))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SOARecord {
    pub mname: Name,
    pub rname: Name,
    pub serial: u32,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum: u32,
}

impl SOARecord {
    pub fn encode(&self, out: &mut Vec<u8>, name_offsets: &mut HashMap<Vec<String>, u16>) {
        self.mname.encode(out, name_offsets);
        self.rname.encode(out, name_offsets);
        out.extend_from_slice(&self.serial.to_be_bytes());
        out.extend_from_slice(&self.refresh.to_be_bytes());
        out.extend_from_slice(&self.retry.to_be_bytes());
        out.extend_from_slice(&self.expire.to_be_bytes());
        out.extend_from_slice(&self.minimum.to_be_bytes());
    }

    pub fn decode(buf: &[u8], p: &mut usize) -> DResult<Self> {
        let mname = Name::decode(buf, p)?;
        let rname = Name::decode(buf, p)?;
        let serial = read_u32(buf, p)?;
        let refresh = read_u32(buf, p)?;
        let retry = read_u32(buf, p)?;
        let expire = read_u32(buf, p)?;
        let minimum = read_u32(buf, p)?;
        Ok(SOARecord {
            mname,
            rname,
            serial,
            refresh,
            retry,
            expire,
            minimum,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TxtRecord(pub Vec<String>);

impl TxtRecord {
    pub fn encode(&self, out: &mut Vec<u8>) {
        for s in &self.0 {
            let bytes = s.as_bytes();

            // A single TXT string cannot exceed 255 bytes.
            assert!(bytes.len() <= 255);

            out.push(bytes.len() as u8);
            out.extend_from_slice(bytes);
        }
    }

    pub fn decode(buf: &[u8], p: &mut usize, rdlength: usize) -> DResult<Self> {
        let end = *p + rdlength;

        if end > buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }

        let mut strings = Vec::new();

        while *p < end {
            let len = read_u8(buf, p)? as usize;

            if *p + len > end {
                return Err(DecodeError::UnexpectedEof);
            }

            let s = String::from_utf8_lossy(&buf[*p..*p + len]).to_string();
            *p += len;

            strings.push(s);
        }

        Ok(TxtRecord(strings))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdnsOption {
    pub code: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OptRecord {
    pub udp_payload_size: u16,
    pub extended_rcode: u8,
    pub version: u8,
    pub flags: u16,
    pub options: Vec<EdnsOption>,
}

impl OptRecord {
    /// Encodes only the EDNS option list (RDATA).
    pub fn encode(&self, out: &mut Vec<u8>) {
        for option in &self.options {
            out.extend_from_slice(&option.code.to_be_bytes());
            out.extend_from_slice(&(option.data.len() as u16).to_be_bytes());
            out.extend_from_slice(&option.data);
        }
    }

    /// Decodes an OPT RR from its CLASS, TTL and RDATA.
    pub fn decode(
        class: u16,
        ttl: u32,
        buf: &[u8],
        p: &mut usize,
        rdlength: usize,
    ) -> DResult<Self> {
        let end = *p + rdlength;

        if end > buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }

        let mut options = Vec::new();

        while *p < end {
            let code = read_u16(buf, p)?;
            let len = read_u16(buf, p)? as usize;

            if *p + len > end {
                return Err(DecodeError::UnexpectedEof);
            }

            let data = buf[*p..*p + len].to_vec();
            *p += len;

            options.push(EdnsOption { code, data });
        }

        Ok(OptRecord {
            udp_payload_size: class,
            extended_rcode: (ttl >> 24) as u8,
            version: (ttl >> 16) as u8,
            flags: ttl as u16,
            options,
        })
    }

    /// Packs the extended RCODE, version and flags into the OPT TTL field.
    pub fn ttl(&self) -> u32 {
        ((self.extended_rcode as u32) << 24) | ((self.version as u32) << 16) | (self.flags as u32)
    }
}
