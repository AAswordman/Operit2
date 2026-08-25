# Node

Node crates own core route resolution and node runtime behavior.

This domain is responsible for annotation-driven route metadata, binding resolution, local or peer target selection, space runtime execution, and node-side synchronization orchestration. Access supplies transport; node decides where route traffic goes.

## Crates

- `route-macros`: procedural macros for annotated Rust route functions.
- `runtime`: current aggregate node runtime containing router, local runtime, space runtime, remote link service, discovery, and space sync code.

## Target Split

The target layout separates this aggregate into `contracts`, `router`, `local-runtime`, `space-runtime`, and `space-sync` crates.
