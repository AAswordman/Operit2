# ToolPkg WASM Demo

Minimal ToolPkg example for prime-number logic:

- `src/main.ts` is the ToolPkg authoring entry. Keep public export names here.
- `src/wasm/core.ts` is the typed TypeScript facade used by `src/main.ts`.
- `src/wasm/core.as.ts` contains the AssemblyScript algorithm compiled to WASM.
- `manifest.json` declares the `wasm_modules` entry consumed by the host.
- `build/main.js`, `modules/core.wasm`, and `dist/toolpkg_wasm_demo.toolpkg` are generated locally.

Author code with normal TypeScript imports:

```ts
import { nthPrime } from "./wasm/core";

export async function nth_prime(params: { index: number }) {
  return { prime: await nthPrime(params.index) };
}
```

Build the JS entry, WASM module, and ToolPkg archive:

```bash
npm install
npm run pack:toolpkg
```
