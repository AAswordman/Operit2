# Real USearch WASM exercise

`main.js` exercises the real bundled HNSW implementation through `ToolPkg.wasm`:
batch insertion, nearest neighbors, deletion, snapshot restore, and exact u64
labels above JavaScript's safe integer limit. It reports query latency together
with recall@10 against an exact JavaScript L2 scan on fixed-seed random data.

The source, pinned revision, build script and license notices are owned by the
memory plugin in `../../buildin/character_cards/vendor/usearch/`.

`corepack pnpm run pack:toolpkg` builds that pinned module before packaging this
example. It uses the repository Python environment and installed Emscripten
toolchain, and reuses the owner build when its inputs and output are unchanged.
The normal plugin synchronizer invokes the same command for this example.

Run the embedded native V8 integration test from the repository root:

```sh
cargo test --manifest-path core/Cargo.toml -p operit-js-bridge \
  wasm_usearch_indexes_searches_deletes_and_restores_real_vectors -- --nocapture
```

For the larger fixture, set `OPERIT_USEARCH_COUNT=10000`,
`OPERIT_USEARCH_DIMENSIONS=1024`, and `OPERIT_USEARCH_QUERIES=50` before that command.
The default is 2,000 vectors, 128 dimensions, and 20 queries. These are synthetic
ANN measurements, not end-to-end HTTP embedding or SQLite retrieval timings.
