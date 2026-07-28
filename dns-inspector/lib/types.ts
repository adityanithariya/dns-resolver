// These mirror exactly what serde produces for the Rust types in
// dns_core::message (Header, Question, ResourceRecord, RData, ...) once they
// cross the wasm boundary via serde_wasm_bindgen. Keep this file in lockstep
// with the Rust side — it is not generated automatically.

/** A domain name decodes to its ordered list of labels, e.g. ["www","example","com"]. */
export type DnsName = string[];

/** Unit-variant enums serialize as their variant name; `Other(u8)` as { Other: n }. */
export type Opcode = "Query" | "IQuery" | "Status" | { Other: number };
export type Rcode =
  | "NoError"
  | "FormErr"
  | "ServFail"
  | "NxDomain"
  | "NotImp"
  | "Refused"
  | { Other: number };

export type QType =
  | "A"
  | "NS"
  | "CNAME"
  | "SOA"
  | "PTR"
  | "MX"
  | "TXT"
  | "AAAA"
  | "ANY"
  | "OPT"
  | { Other: number };

export type QClass = "IN" | { Other: number };

export interface Header {
  id: number;
  qr: boolean;
  opcode: Opcode;
  aa: boolean;
  tc: boolean;
  rd: boolean;
  ra: boolean;
  rcode: Rcode;
  qdcount: number;
  ancount: number;
  nscount: number;
  arcount: number;
}

export interface Question {
  name: DnsName;
  qtype: QType;
  qclass: QClass;
}

export interface SOARecord {
  mname: DnsName;
  rname: DnsName;
  serial: number;
  refresh: number;
  retry: number;
  expire: number;
  minimum: number;
}

export interface EdnsOption {
  code: number;
  data: number[];
}

export interface OptRecord {
  udp_payload_size: number;
  extended_rcode: number;
  version: number;
  flags: number;
  options: EdnsOption[];
}

/** RData is an externally-tagged Rust enum: { "<Variant>": <payload> }. */
export type RData =
  | { A: string }
  | { AAAA: string }
  | { NS: DnsName }
  | { CNAME: DnsName }
  | { SOA: SOARecord }
  | { TXT: string[] }
  | { OPT: OptRecord }
  | { Raw: number[] };

export interface ResourceRecord {
  name: DnsName;
  rtype: QType;
  rclass: QClass;
  ttl: number;
  rdata: RData;
}

export interface Message {
  header: Header;
  questions: Question[];
  answers: ResourceRecord[];
  authorities: ResourceRecord[];
  additionals: ResourceRecord[];
}

/** Query types exposed in the UI, mapped to the u16 values QType::to_u16 emits. */
export const QUERY_TYPES: { label: string; value: number }[] = [
  { label: "A", value: 1 },
  { label: "NS", value: 2 },
  { label: "CNAME", value: 5 },
  { label: "SOA", value: 6 },
  // { label: "PTR", value: 12 },
  // { label: "MX", value: 15 },
  { label: "TXT", value: 16 },
  { label: "AAAA", value: 28 },
  { label: "ANY", value: 255 },
];

export function nameToString(name: DnsName): string {
  return name.length === 0 ? "." : name.join(".");
}

export function variantTag(v: { Other: number } | string): string {
  return typeof v === "string" ? v : `Other(${v.Other})`;
}
