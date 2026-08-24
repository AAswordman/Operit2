# operit-proxy-dart-codegen

Dart emitter for the core-app proxy scan model.

## Scope

- Consumes shared proxy IR from `operit-rslink-codegen`.
- Generates Flutter model and client files under the app generated proxy directory.
- Mirrors Link request, response, watch, and reverse-stream shapes for Dart callers.

## Boundaries

- Does not scan Rust source roots.
- Does not generate Rust proxy files.
- Does not implement the Rust runtime protocol; runtime conversion lives in `operit-rslink-runtime`.
