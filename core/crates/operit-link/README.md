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
- `src/codec.rs`: optional binary codec helpers for CBOR payloads.
- `src/http.rs`: HTTP and WebSocket dispatcher for `CoreLinkClient`
  implementations on native targets.
- `src/lib.rs`: crate exports and target-specific module wiring.
- `benches/`: protocol and dispatcher microbenchmarks for Link-only costs.
- `benches_data/`: stable payload fixtures used by benchmarks and external
  performance scenarios.
- `tests/`: protocol roundtrip and dispatcher behavior tests.

## Boundary

This crate carries core call/watch/event semantics only. Pairing, signatures,
sessions, device trust, and server lifecycle are owned by the app access layer
that embeds or exposes the link client.

## Dispatcher Client Modes

`CoreLinkHttpDispatcher::new` accepts the mutable `CoreLinkClient` interface and
serializes access to that client instance.

`CoreLinkHttpDispatcher::newShared` accepts `CoreLinkSharedClient`, whose
methods take `&self`. Use it for clients that are internally concurrency-safe
and can process call or watch requests without dispatcher-level serialization.

## Codec Modes

The JSON endpoints remain the stable default transport. Native callers that want
smaller binary bodies can use explicit binary dispatcher methods:

- `callCbor`
- `watchSnapshotCbor`
- `callMessagePack`
- `watchSnapshotMessagePack`

These methods use the same envelope and response types as JSON, encoded through
the exported codec helpers.

## Performance Tests

`operit-link` performance tests measure only protocol and carrier costs inside
this crate:

- `protocol_codec_bench.rs`: JSON encode/decode cost for calls, responses, and
  events.
- `dispatcher_call_bench.rs`: native HTTP dispatcher call cost around a
  deterministic `CoreLinkClient`.
- `watch_channel_bench.rs`: watch channel open and queued event dispatch cost.
- `link_stress_bench.rs`: sequential and concurrent dispatcher call burst cost
  for mutable and shared client modes.

Run microbenchmarks from `core`:

```powershell
cargo bench -p operit-link --bench protocol_codec_bench
cargo bench -p operit-link --bench dispatcher_call_bench
cargo bench -p operit-link --bench watch_channel_bench
cargo bench -p operit-link --bench link_stress_bench
```

End-to-end scenarios live in `tools/link-perf` because CLI, host, session,
signature, and server lifecycle behavior belongs to the embedding app access
layer.
