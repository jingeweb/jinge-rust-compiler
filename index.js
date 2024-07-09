import { fileURLToPath } from "url";
import path from "path";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export const SWCDebugPlugin = path.resolve(
  __dirname,
  "./target/wasm32-wasi/debug/jinge_swc_compiler.wasm"
);
export const SWCPlugin = path.resolve(
  __dirname,
  "./target/wasm32-wasi/release/jinge_swc_compiler.wasm"
);
