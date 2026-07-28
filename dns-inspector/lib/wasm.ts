"use client";

import type { Message } from "./types";

// This points at the output of `wasm-pack build --target web`, copied into
// this project — see README.md for exactly how to produce it. It's a plain
// ES module plus a .wasm file, so it only ever loads in the browser.
type DnsWasmModule = {
  version: () => string;
  encode_query: (id: number, name: string, qtype: number) => Uint8Array;
  decode_message: (bytes: Uint8Array) => Message;
};

let modulePromise: Promise<DnsWasmModule> | null = null;

/** Loads and initializes the wasm module exactly once, then reuses it. */
export function getDnsWasm(): Promise<DnsWasmModule> {
  if (!modulePromise) {
    modulePromise = (async () => {
      const wasm = (await import(
        /* webpackIgnore: false */ "dns-wasm"
      )) as unknown as DnsWasmModule;
      return wasm;
    })();
  }
  return modulePromise;
}

export async function encodeQuery(
  id: number,
  name: string,
  qtype: number
): Promise<Uint8Array> {
  const wasm = await getDnsWasm();
  return wasm.encode_query(id, name, qtype);
}

export async function decodeMessage(bytes: Uint8Array): Promise<Message> {
  const wasm = await getDnsWasm();
  return wasm.decode_message(bytes);
}
