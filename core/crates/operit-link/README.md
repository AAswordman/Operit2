# operit-link

`operit-link` defines the only application-to-Core protocol used by local and
remote Operit clients.

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

Physical carriers transport the same encoded messages:

- HTTP request and response bodies use `application/msgpack`.
- WebSocket messages use binary frames.
- Flutter MethodChannel uses `Uint8List`.
- WebAssembly uses `Uint8Array`.
- HTTP watch streams use repeated four-byte big-endian lengths followed by one
  MessagePack `LinkWatchChannelEvent` frame.

Watch channels multiplex logical subscriptions by `subscriptionId`. Events for
each subscription remain ordered, and a `Completed` event terminates that
subscription.

Push streams use `pushId` and a monotonically increasing `sequence`. A carrier
must preserve item order within a push stream. WebSocket carriers keep push
input separate from HTTP calls and from watch event responses, so a large watch
payload cannot occupy the request/response path used by interaction input.

Link protocol version 3 adds push streams. The HTTP carrier exposes
`/link/push/open`, `/link/push/item`, and `/link/push/close`; the WebSocket
carrier uses tagged `PushOpen`, `PushItem`, and `PushClose` binary messages.

## Benchmarks

`browser_surface_codec_bench.rs` compares the final MessagePack representation
against historical JSON/base64 and CBOR baselines. Those baseline codecs are
benchmark-only and are not exported by the product protocol.

`protocol_codec_bench.rs` measures the final Link codec for small calls and
native binary browser frame payloads.
