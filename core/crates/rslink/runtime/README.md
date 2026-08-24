# operit-rslink-runtime

Runtime protocol conversion for Rust-to-Link-to-Rust calls.

## Scope

- Encodes Rust values into `operit-link::CoreValue`.
- Decodes Link request arguments and response values back into Rust types.
- Bridges Flow, StateFlow, CoreStream, and reverse-stream payloads through Link events.
- Provides route helper functions used by annotation-generated route bridges.

## Boundaries

- Does not scan source code.
- Does not generate Rust or Dart files.
- Does not know about concrete proxy objects, route catalogs, or Flutter clients.
