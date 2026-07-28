# DNS Packet Inspector

A Next.js page for the `dns_core` resolver: build a query, send it to your DoH
server, and inspect the full decoded `Message` (not `format_message`'s string
output) — header flags, question, answer/authority/additional records, all
`RData` variants.

## UI stack

Built on [shadcn/ui](https://ui.shadcn.com) (`Button`, `Input`, `Label`,
`Select`, `Badge`, `Card`, `Table` in `components/ui/`) on top of Tailwind,
forced into dark mode via the `dark` class on `<html>` in `app/layout.tsx`.
The palette lives as HSL CSS variables in `app/globals.css` — edit those to
retheme instead of touching the components themselves. `npm install` pulls in
Tailwind and the Radix primitives these components depend on
(`@radix-ui/react-select`, `@radix-ui/react-label`, `@radix-ui/react-slot`,
`class-variance-authority`, `clsx`, `tailwind-merge`, `lucide-react`).

## 1. Build the wasm crate

From the crate that has the `#[wasm_bindgen]` functions (`version`,
`encode_query`, `decode_message`):

```bash
wasm-pack build --target web --out-dir pkg --out-name dns_wasm
```

`--target web` matters: this app loads the module directly in the browser
with `await wasm.default()`, which is that target's init call.

## 2. Vendor the output into this app

Copy the generated `pkg/` directory here so the import in `lib/wasm.ts`
resolves:

```bash
mkdir -p wasm
cp -r /path/to/your/crate/pkg wasm/pkg
```

You should end up with `wasm/pkg/dns_wasm.js` and `wasm/pkg/dns_wasm_bg.wasm`.
If you name the crate or the `--out-name` differently, update the import path
in `lib/wasm.ts` to match.

## 3. Point at your DoH server

```bash
cp .env.local.example .env.local
# edit NEXT_PUBLIC_CLIENT_URL if it's not on 127.0.0.1:8443
```

This assumes the server exposes `POST /dns-query`, accepting and returning
raw DNS wire-format bytes (`application/dns-message`) — the same shape the
`doh post` CLI mode in `main.rs` talks to.

## 4. Run it

```bash
npm install
npm run dev
```

## How a query flows through the app

1. `QueryForm` collects a name and a query type.
2. `lib/wasm.ts` calls `encode_query(id, name, qtype)` in the browser to get
   raw wire-format bytes — no server-side encoding involved.
3. `lib/api.ts` POSTs those bytes to `NEXT_PUBLIC_CLIENT_URL/dns-query` via
   axios and gets raw bytes back.
4. `decode_message(bytes)` turns the response into the same `Message` shape
   `Message::decode` produces on the Rust side (via `serde_wasm_bindgen`).
5. `PacketStrip` and `MessageInspector` render that struct directly.

`lib/types.ts` mirrors the serde JSON shape of `Header`, `Question`,
`ResourceRecord`, and the `RData` enum by hand — if you add a new `RData`
variant or field on the Rust side, update it there too.
