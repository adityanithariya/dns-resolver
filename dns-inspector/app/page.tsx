"use client";

import { useEffect, useState } from "react";
import QueryForm, { QueryStatus } from "@/components/QueryForm";
import PacketStrip from "@/components/PacketStrip";
import MessageInspector from "@/components/MessageInspector";
import { encodeQuery, decodeMessage } from "@/lib/wasm";
import { postDnsQuery, health } from "@/lib/api";
import type { Message } from "@/lib/types";
import Footer from "@/components/Footer";

const DOH_SERVER_URL =
  process.env.NEXT_PUBLIC_CLIENT_URL ?? "https://doh.adityanithariya.com";

export default function Home() {
  const [name, setName] = useState("");
  const [qtype, setQtype] = useState<number>(1);
  const [status, setStatus] = useState<QueryStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<Message | null>(null);
  const [queriedName, setQueriedName] = useState<string | null>(null);

  async function runQuery() {
    let q = name.trim();
    if (name.startsWith("http://") || name.startsWith("https://")) {
      const url = new URL(name);
      setName(url.hostname);
      q = url.hostname;
    }

    setError(null);
    setMessage(null);
    setQueriedName(q);

    try {
      setStatus("encoding");
      const id = Math.floor(Math.random() * 0xffff);
      const queryBytes = await encodeQuery(id, q, qtype);

      setStatus("sending");
      const responseBytes = await postDnsQuery(queryBytes);

      setStatus("decoding");
      const decoded = await decodeMessage(responseBytes);

      setMessage(decoded);
      setStatus("idle");
    } catch (err) {
      console.error(err);
      setError(err instanceof Error ? err.message : "Something went wrong while resolving that query.");
      setStatus("error");
    }
  }

  useEffect(() => {
    try {
      health();
    } catch (error) {
      console.error("Failed to check health:", error);
    }
  }, []);

  return (
    <main className="min-h-screen mx-auto max-w-3xl px-6 py-12 pb-24">
      <header className="mb-8">
        <div className="mb-2 flex items-baseline gap-2.5">
          <span className="text-[13px] text-primary">▲</span>
          <h1 className="text-2xl">Recursive DNS Resolver</h1>
        </div>
        <p className="max-w-xl text-sm text-muted-foreground">
          Encodes a query with the Rust resolver core compiled to WASM, sends it over DNS-over-HTTPS, and
          decodes the raw wire-format response back into its full <code>Message</code> struct.
        </p>
        <p className="mt-2 text-xs text-muted-foreground/60">
          Resolver: <span className="text-primary">{DOH_SERVER_URL.replace(/^https?:\/\//, "")}</span>
        </p>
      </header>

      <QueryForm
        qtype={qtype}
        setQtype={setQtype}
        name={name}
        setName={setName}
        onSubmit={runQuery}
        status={status}
        errorMessage={error}
      />

      <div className="mt-9">
        {message ? (
          <>
            <PacketStrip message={message} />
            <MessageInspector message={message} />
          </>
        ) : (
          <p className="mt-5 text-[13px] text-muted-foreground/60">
            {status === "idle" ? "Send a query above to inspect the response." : `Resolving ${queriedName}…`}
          </p>
        )}
      </div>

      <Footer />
    </main>
  );
}
