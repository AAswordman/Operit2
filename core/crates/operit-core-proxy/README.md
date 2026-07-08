# operit-core-proxy

`operit-core-proxy` generates and hosts typed proxy glue for Operit2 core
objects. It scans selected public surfaces across the split core crates during
the build, emits Rust dispatch code, and emits Dart proxy models and clients
for the Flutter application.

## Responsibilities

- Scan runtime, store, tools, providers, and model surfaces selected for proxy
  exposure.
- Generate Rust dispatch functions for local `operit-link` requests.
- Generate Rust client/proxy schema code.
- Generate Dart proxy models and clients consumed by the Flutter app.
- Expose `LocalCoreProxy`, a `CoreLinkClient` backed by an in-process
  `OperitApplication`.

## Key Files

- `src/lib.rs`: `LocalCoreProxy` and generated dispatch integration.
- `build.rs`: build-time scan and generation pipeline.
- `build_model.rs`: intermediate object, method, and type models.
- `build_scanner.rs`: Rust source scanner for exposed core methods.
- `build_type_resolver.rs`: type resolution across scanned sources.
- `build_rust_*`: Rust schema, proxy, and dispatch generators.
- `build_dart_codegen.rs`: Dart model and client generator.

## Generated Flow

Objects and methods are scanned from selected modules in `operit-runtime`,
`operit-store`, `operit-tools`, and `operit-providers`; supported types are
resolved across the configured crate roots, including `operit-model` and
public re-exports. Generated artifacts are then written for Rust dispatch and
Dart client consumption. Build script `cargo:rerun-if-changed` directives keep
this generation tied to the scanned source files and generator modules.

## Boundary

This crate projects selected core capabilities into the link protocol. It does
not own runtime behavior, provider behavior, tool execution, storage, pairing,
sessions, signatures, or platform host implementations.
