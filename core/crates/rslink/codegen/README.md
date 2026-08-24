# operit-rslink-codegen

Shared code-generation model utilities for Rust-to-Link-to-Rust bridges.

## Scope

- Defines the language-neutral IR used by proxy and route generators.
- Resolves Rust types into stable string forms for generated Link payloads.
- Provides shared generic parsing and naming helpers.

## Boundaries

- Does not scan core-app registered roots.
- Does not generate Rust, Dart, or other language output.
- Does not contain runtime Link conversion logic; runtime conversion lives in `operit-rslink-runtime`.
