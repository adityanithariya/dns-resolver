"use client";

import type { Message, Question, ResourceRecord, RData, QType, QClass } from "@/lib/types";
import { nameToString, variantTag } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

function SectionHeading({ id, title, count }: { id: string; title: string; count?: number }) {
  return (
    <h3
      id={id}
      className="mb-2.5 scroll-mt-6 text-[13px] font-medium tracking-[0.08em] text-muted-foreground"
    >
      {title.toUpperCase()}
      {count !== undefined && <span className="text-muted-foreground/60"> ({count})</span>}
    </h3>
  );
}

function RDataView({ rdata }: { rdata: RData }) {
  if ("A" in rdata) return <span>{rdata.A}</span>;
  if ("AAAA" in rdata) return <span>{rdata.AAAA}</span>;
  if ("NS" in rdata) return <span>{nameToString(rdata.NS)}</span>;
  if ("CNAME" in rdata) return <span>{nameToString(rdata.CNAME)}</span>;
  if ("TXT" in rdata)
    return (
      <span>
        {rdata.TXT.map((s, i) => (
          <span key={i}>
            {i > 0 && " "}
            &ldquo;{s}&rdquo;
          </span>
        ))}
      </span>
    );
  if ("SOA" in rdata) {
    const soa = rdata.SOA;
    return (
      <span className="text-muted-foreground">
        mname={nameToString(soa.mname)} rname={nameToString(soa.rname)} serial={soa.serial} refresh=
        {soa.refresh} retry={soa.retry} expire={soa.expire} minimum={soa.minimum}
      </span>
    );
  }
  if ("OPT" in rdata) {
    const opt = rdata.OPT;
    return (
      <span className="text-muted-foreground">
        udp_payload={opt.udp_payload_size} ext_rcode={opt.extended_rcode} version={opt.version} flags=0x
        {opt.flags.toString(16)} options=[{opt.options.map((o) => o.code).join(", ")}]
      </span>
    );
  }
  if ("Raw" in rdata) return <span className="text-muted-foreground">{rdata.Raw.length} raw bytes</span>;
  return <span className="text-muted-foreground">unknown</span>;
}

function typeLabel(t: QType | QClass): string {
  return variantTag(t as { Other: number } | string);
}

function QuestionRow({ q }: { q: Question }) {
  return (
    <TableRow>
      <TableCell>{nameToString(q.name)}</TableCell>
      <TableCell>{typeLabel(q.qtype)}</TableCell>
      <TableCell>{typeLabel(q.qclass)}</TableCell>
    </TableRow>
  );
}

function RecordRow({ r }: { r: ResourceRecord }) {
  return (
    <TableRow>
      <TableCell>{nameToString(r.name)}</TableCell>
      <TableCell>{typeLabel(r.rtype)}</TableCell>
      <TableCell>{typeLabel(r.rclass)}</TableCell>
      <TableCell className="text-accent">{r.ttl}</TableCell>
      <TableCell>
        <RDataView rdata={r.rdata} />
      </TableCell>
    </TableRow>
  );
}

function RecordTable({
  headers,
  emptyLabel,
  children,
  isEmpty,
}: {
  headers: string[];
  emptyLabel: string;
  isEmpty: boolean;
  children: React.ReactNode;
}) {
  if (isEmpty) {
    return <p className="text-[13px] text-muted-foreground/60">{emptyLabel}</p>;
  }
  return (
    <Table>
      <TableHeader>
        <TableRow className="hover:bg-transparent">
          {headers.map((h) => (
            <TableHead key={h}>{h}</TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>{children}</TableBody>
    </Table>
  );
}

export default function MessageInspector({ message }: { message: Message }) {
  const h = message.header;
  const rcodeOk = typeLabel(h.rcode) === "NoError";

  return (
    <div className="flex flex-col gap-7">
      <section>
        <SectionHeading id="section-header" title="Header" />
        <Card>
          <CardContent className="flex flex-wrap gap-2 pt-4">
            <Badge>ID 0x{h.id.toString(16).padStart(4, "0")}</Badge>
            <Badge variant={h.qr ? "on" : "off"}>{h.qr ? "RESPONSE" : "QUERY"}</Badge>
            <Badge>{typeLabel(h.opcode)}</Badge>
            <Badge variant={h.aa ? "on" : "off"}>AA</Badge>
            <Badge variant={h.tc ? "warn" : "off"}>TC</Badge>
            <Badge variant={h.rd ? "on" : "off"}>RD</Badge>
            <Badge variant={h.ra ? "on" : "off"}>RA</Badge>
            <Badge variant={rcodeOk ? "on" : "warn"}>{typeLabel(h.rcode)}</Badge>
            <Badge>QD {h.qdcount}</Badge>
            <Badge>AN {h.ancount}</Badge>
            <Badge>NS {h.nscount}</Badge>
            <Badge>AR {h.arcount}</Badge>
          </CardContent>
        </Card>
      </section>

      <section>
        <SectionHeading id="section-question" title="Question" count={message.questions.length} />
        <RecordTable headers={["Name", "Type", "Class"]} emptyLabel="No question records." isEmpty={false}>
          {message.questions.map((q, i) => (
            <QuestionRow key={i} q={q} />
          ))}
        </RecordTable>
      </section>

      <section>
        <SectionHeading id="section-answer" title="Answer" count={message.answers.length} />
        <RecordTable
          headers={["Name", "Type", "Class", "TTL", "Data"]}
          emptyLabel="No answer records."
          isEmpty={message.answers.length === 0}
        >
          {message.answers.map((r, i) => (
            <RecordRow key={i} r={r} />
          ))}
        </RecordTable>
      </section>

      <section>
        <SectionHeading id="section-authority" title="Authority" count={message.authorities.length} />
        <RecordTable
          headers={["Name", "Type", "Class", "TTL", "Data"]}
          emptyLabel="No authority records."
          isEmpty={message.authorities.length === 0}
        >
          {message.authorities.map((r, i) => (
            <RecordRow key={i} r={r} />
          ))}
        </RecordTable>
      </section>

      <section>
        <SectionHeading id="section-additional" title="Additional" count={message.additionals.length} />
        <RecordTable
          headers={["Name", "Type", "Class", "TTL", "Data"]}
          emptyLabel="No additional records."
          isEmpty={message.additionals.length === 0}
        >
          {message.additionals.map((r, i) => (
            <RecordRow key={i} r={r} />
          ))}
        </RecordTable>
      </section>
    </div>
  );
}
