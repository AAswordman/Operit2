# operit-proxy-scan

Core-app proxy scanner for registered roots.

## Scope

- Scans the concrete core-app source roots used by `operit-proxy-local`.
- Produces `CoreProxyScanOutput` with objects, methods, serializable models, and error models.
- Enforces host platform boundary checks during proxy generation.

## Boundaries

- Does not generate Rust proxy files.
- Does not generate Dart proxy files.
- Does not define the shared IR; shared IR lives in `operit-rslink-codegen`.

## Owned futures

The scanner recognizes synchronous methods returning
`impl Future<Output = T> + 'static` from Rust syntax and classifies `T` as the
wire result. `returns_owned_future` records the capture contract separately from
`is_async`, which describes the asynchronous client call. Borrowed futures are
not classified as owned operations. The contract needs no platform branch or
method-name registration.
