# operit-node-runtime

`operit-node-runtime` is the current aggregate node runtime.

It contains `CoreNodeRouter`, local route runtime capability wiring, space runtime support, remote link service hooks, discovery hooks, space persistence sync, and generated route catalog support.

## Boundary

- Owns route target selection and route dispatch.
- Receives peer transport capability from access.
- Receives local execution capability from runtime/application-facing handles.
- Does not own pairing, session authentication, static web control, or local proxy code generation.

## Migration Note

This crate is intentionally an aggregate during the directory migration. Its router, local runtime, space runtime, and space sync pieces should become narrower node crates after the application tree owns construction and shutdown.
