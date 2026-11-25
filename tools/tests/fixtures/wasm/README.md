# Embedded JavaScript/WASM fixture

`memory.wasm` is the small executable test fixture for `memory.c`, used by native
and browser JavaScript engine tests. Keep it in version control: the native test
crate includes these bytes at compile time, including when only the package
parsing gate is selected.

Regenerate it with LLVM's WebAssembly target and linker:

```sh
clang --target=wasm32 -O2 -nostdlib -Wl,--no-entry -Wl,--export-all tools/tests/fixtures/wasm/memory.c -o tools/tests/fixtures/wasm/memory.wasm
```
