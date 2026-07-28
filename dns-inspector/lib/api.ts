import axios from "axios";

// Set NEXT_PUBLIC_CLIENT_URL in .env.local to wherever the DoH endpoint is
// running (e.g. the `doh` server matching main.rs's `serve` command).
const CLIENT_URL =
  process.env.NEXT_PUBLIC_CLIENT_URL ?? "http://127.0.0.1:8443";

const client = axios.create({
  baseURL: CLIENT_URL,
  responseType: "arraybuffer",
  headers: {
    "Content-Type": "application/dns-message",
    Accept: "application/dns-message",
  },
});

/**
 * POSTs a wire-format DNS query (already encoded client-side by wasm) to the
 * DoH endpoint and returns the raw wire-format response bytes, undecoded.
 */
export async function postDnsQuery(queryBytes: Uint8Array): Promise<Uint8Array> {
  const res = await client.post("/dns-query", queryBytes);
  return new Uint8Array(res.data as ArrayBuffer);
}

export async function health(): Promise<boolean> {
  const { status } = await client.get("/health");
  return status === 200;
}
