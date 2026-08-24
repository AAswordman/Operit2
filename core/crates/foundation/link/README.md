# operit-link

`operit-link` defines the Core value, request, response, event and stream types
used by the local bridge and by Space peer frames. It does not expose a network
server or application-to-Core remote entrypoint.

## Protocol

The protocol consists of these semantic messages:

- `CoreCallRequest` / `CoreCallResponse`
- `CoreWatchRequest`
- `CoreEvent` / `CoreEventStream`
- `CorePushRequest` / `CorePushItem`

`call` is a client-to-Core request/response operation. `watch` is a
Core-to-client stream. `push` is the directional counterpart of `watch`: the
client opens a logical input stream, sends ordered argument values, and closes
the stream without creating one request/response operation per item.

Every message is encoded with MessagePack through `encodeLink` and
`decodeLink`. There is no codec negotiation, JSON transport, CBOR transport,
or platform-specific Link envelope.

`CoreValue` maps directly to MessagePack primitives and preserves native binary
values as MessagePack `bin` data. Runtime conversion uses `toCoreValue` and
`fromCoreValue`; it never normalizes values through `serde_json::Value`.

## Carriers

The local Flutter and WASM bridges carry MessagePack values directly to the
generated local dispatch surface. There is no generic HTTP or WebSocket
application-to-Core dispatcher in this crate.

Space transport belongs to `operit-access-runtime`. It places the standard Link
request types inside authenticated `PeerFrame` messages exchanged by
`CoreNodeRouter` instances. Watch subscriptions use `subscriptionId`; Push
streams use `pushId` and a monotonically increasing `sequence`. Each id remains
ordered for the lifetime of its PeerLink.

## Benchmarks

`browser_surface_codec_bench.rs` compares the final MessagePack representation
against historical JSON/base64 and CBOR baselines. Those baseline codecs are
benchmark-only and are not exported by the product protocol.

`protocol_codec_bench.rs` measures the final Link codec for small calls and
native binary browser frame payloads.
