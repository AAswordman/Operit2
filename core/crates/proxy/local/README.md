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

## Headless UI Resource Checks

`cargo test --manifest-path core/Cargo.toml -p operit-proxy-local --test shared_concurrency_test ui_resource_tests -- --include-ignored --nocapture`

The opt-in latency probe loads the embedded character-card package into fresh temporary
storage and reports one cold read plus 20 warm samples. The contention regression gates
real archive extraction and session resource reads, then verifies that generated proxy
engine calls still complete. It also checks context release during loading, stale opens
after engine replacement, script contents, and live-session identity. These measurements
cover Rust and the local proxy; they do not measure Flutter scheduling or frame rendering.

The IPC/public-API regression uses the real sidebar API and memory-preferences channel.
It counts main-entry file reads across repeated and concurrent IPC, verifies exact script
bytes against direct archive reads, checks registry reload invalidation, and gates cold
I/O to verify manager availability and rejection after engine release. Its timing output
separates main-script preparation from the complete business request.
