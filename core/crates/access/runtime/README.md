# operit-access-runtime

`operit-access-runtime` is the current aggregate access runtime.

It contains the pairing, authentication, session, peer-link carrier, remote link server, discovery, and web-control pieces that used to live under the old access crate path.

## Boundary

- Owns access control, peer connection state, and transport entry points.
- Passes authenticated peer transport into `operit-node-runtime`.
- Does not own route selection, space route execution, or local core proxy dispatch.

## Migration Note

This crate is intentionally an aggregate during the directory migration. Its code should be split into narrower access crates after the application tree owns construction and shutdown.
