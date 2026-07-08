# operit-link

`operit-link` defines the transport-neutral protocol used to call and observe
Operit2 core objects. It describes call, watch, event, stream, and error shapes
without owning app access state.

## Responsibilities

- Model object paths with `CoreObjectPath`.
- Model one-shot method calls with `CoreCallRequest` and `CoreCallResponse`.
- Model observable values and event streams with `CoreWatchRequest`,
  `CoreEvent`, and `CoreEventStream`.
- Define `CoreLinkClient`, the async interface implemented by local and remote
  core clients.
- Provide optional Axum HTTP/WebSocket dispatch helpers for native builds.

## Key Files

- `src/protocol.rs`: protocol data structures, object paths, request IDs,
  events, watch metadata, and link errors.
- `src/client.rs`: `CoreLinkClient` trait.
- `src/http.rs`: HTTP and WebSocket dispatcher for `CoreLinkClient`
  implementations on native targets.
- `src/lib.rs`: crate exports and target-specific module wiring.

## Boundary

This crate carries core call/watch/event semantics only. Pairing, signatures,
sessions, device trust, and server lifecycle are owned by the app access layer
that embeds or exposes the link client.
