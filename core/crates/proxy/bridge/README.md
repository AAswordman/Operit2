# operit-proxy-bridge

Core-app communication bridge components.

## Scope

- Adapts local application dispatch targets into `CoreLinkSharedClient`.
- Owns communication-facing wrappers that are not specific to proxy object scanning.
- Keeps Link client plumbing outside the concrete proxy runtime.

## Boundaries

- Does not scan proxy roots.
- Does not generate Rust or Dart code.
- Does not convert Rust values to Link payloads; conversion lives in `operit-rslink-runtime`.
