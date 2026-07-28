/* @ts-self-types="./dns_wasm.d.ts" */
import * as wasm from "./dns_wasm_bg.wasm";
import { __wbg_set_wasm } from "./dns_wasm_bg.js";

__wbg_set_wasm(wasm);
wasm.__wbindgen_start();
export {
    decode_message, encode_query, format_message, version
} from "./dns_wasm_bg.js";
