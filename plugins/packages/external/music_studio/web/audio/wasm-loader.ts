import { WASM_BASE64, WASM_SHA256 } from './generated/wasm';
export { WASM_SHA256 };
let promise: Promise<WebAssembly.Module> | undefined;
export function wasmModule(): Promise<WebAssembly.Module> {
  return promise??=WebAssembly.compile(Uint8Array.from(atob(WASM_BASE64),c=>c.charCodeAt(0)));
}
