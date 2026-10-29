# Memory plugin's USearch dependency

This directory belongs to `character_cards`. It is not part of the external
Plugin SDK in `plugins/sdk`, whose clients use IPC/Link to connect to Operit.

The bundled module in `../../resources/usearch.wasm` is built from USearch commit
`a985f77553d2d58d5ef1516eafaae81480bb3c30` plus this directory's `index.cpp`.
USearch uses its HNSW index with f32 vectors, squared L2, connectivity 16,
construction expansion 128 and search expansion 512. The memory plugin stores
unit vectors, so cosine similarity is `1 - squaredL2 / 2`. Filter callbacks stay
inside WASM. Selected lexical and graph candidates can be reranked through
`index_score_keys`, without fetching or scoring every source vector.

Rebuild from the repository root with the installed Emscripten toolchain:

```sh
python3 plugins/packages/buildin/character_cards/vendor/usearch/build.py
# Or supply an existing checkout, whose revision is verified:
python3 plugins/packages/buildin/character_cards/vendor/usearch/build.py --source /path/to/USearch
```

The script downloads the pinned source to a temporary build cache, disables
NumKong and OpenMP, enables WASM SIMD, and also refreshes the exercise module in
`plugins/packages/examples/toolpkg_usearch_demo/modules/`. The script uses the
repository's `.ci-tools/emsdk` compiler unless `EMXX` or an `em++` on PATH is set.
The checked-in binary was built with Emscripten 4.0.14. USearch and Emscripten
license notices are included here and shipped in the plugin archive.

The plugin uses `ToolPkg.wasm.open` for a persistent instance and typed byte
transfers. No Node native addon or external SDK client participates.
