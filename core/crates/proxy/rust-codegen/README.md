# operit-proxy-rust-codegen

Rust emitter for the core-app proxy scan model.

## Scope

- Consumes `CoreProxyScanOutput` data produced by `operit-proxy-scan`.
- Generates `generated_core_dispatch.rs` for `operit-proxy-local`.
- Generates the JSON proxy schema consumed by external clients.

## Boundaries

- Does not scan source roots.
- Does not generate Dart files.
- Does not implement runtime protocol conversion; runtime conversion lives in `operit-rslink-runtime`.
