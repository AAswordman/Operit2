# operit-proxy-local

Concrete core-app proxy runtime.

## Scope

- Exposes `LocalCoreProxy`, an in-process `CoreLinkClient` backed by `OperitApplication`.
- Includes generated Rust dispatch emitted by `operit-proxy-rust-codegen`.
- Wires scanned core-app services into Link calls, watches, and push streams.
- Orchestrates build-time scanning and language emitters in `build.rs`.

## Build Pipeline

1. `operit-proxy-scan` scans the registered core-app roots.
2. `operit-proxy-rust-codegen` writes Rust dispatch and schema artifacts.
3. `operit-proxy-dart-codegen` writes Flutter Dart proxy artifacts.

## Boundaries

- Does not own the shared IR or type parser; those live in `operit-rslink-codegen`.
- Does not own runtime value conversion; conversion lives in `operit-rslink-runtime`.
- Does not own generic shared-client bridge wrappers; bridge wrappers live in `operit-proxy-bridge`.
