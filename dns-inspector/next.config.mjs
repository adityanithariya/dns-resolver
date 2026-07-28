/** @type {import('next').NextConfig} */
const nextConfig = {
  // Next.js 16+ defaults to Turbopack and warns if a `webpack` key exists
  // with no matching `turbopack` key. Turbopack handles async WebAssembly
  // imports (wasm-bindgen's --target web output) natively, so it needs no
  // extra experiments here — this empty object just opts in explicitly and
  // silences that warning.
  turbopack: {},

  // Only used if you explicitly run `next dev --webpack` / `next build --webpack`.
  webpack: (config, { isServer }) => {
    // wasm-bindgen's --target web output ships an ES module + a .wasm file.
    // asyncWebAssembly lets webpack pull the .wasm in as an async chunk
    // instead of trying to parse it as JS.
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      layers: true,
    };

    // The wasm module is only ever loaded from the browser (it calls into
    // encode_query/decode_message on user interaction), so keep it out of
    // the server bundle entirely.
    if (isServer) {
      config.output.webassemblyModuleFilename = "chunks/[modulehash].wasm";
    }

    return config;
  },
};

export default nextConfig;
