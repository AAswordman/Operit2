# Proxy

Proxy crates own the core-app local proxy surface and its code generators.

This domain is separate from node route generation. Proxy generation is root-registered scanning for local core app calls; route generation is annotation-driven scanning for Rust-internal route calls.

## Crates

- `scan`: root-registered source scanner and shared proxy IR builder.
- `rust-codegen`: Rust emitter for the proxy IR.
- `dart-codegen`: Dart emitter for the proxy IR.
- `bridge`: shared proxy bridge contracts.
- `local`: local proxy runtime, generated dispatch inclusion, object registry, and stream pool.

## Boundary

Proxy crates use the shared link value protocol for local calls. They should not become owners of router, space runtime, access server, or peer-link lifecycle.

Generated local proxy dispatch must not query route runtime state or decide whether a call/watch crosses nodes. Rust-internal route wrappers own that decision.
