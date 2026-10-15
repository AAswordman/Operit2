# operit-link

`operit-link` defines the Core values, requests, responses, events and streams
used by local clients and authenticated node communication. It does not expose
a network server or own pairing, credentials, Space authorization or sockets.

## Protocol

The standard wire contract has exactly three operation families:

- `CoreLinkRequest::Call` / `CoreLinkResponse::Call`
- `CoreLinkRequest::Watch` / `CoreLinkResponse::Watch`
- `CoreLinkRequest::Push` / `CoreLinkResponse::Push`

Call executes one request/response operation. Watch includes snapshot, open,
event delivery and close. Push includes open, ordered input values and close.
Their lifecycle messages do not add operation families.

`CoreLinkSession<C: CoreLinkClient>` dispatches these requests and owns one
admitted connection's watches and pushes. Authentication and authorization
must be established by its caller before dispatch.

`CoreLinkClient` and `CoreLinkSharedClient` both expose `call`, `watchSnapshot`,
`watch` and `openPush`. Shared clients open caller-owned input sessions without
requiring mutable ownership or cloning the client.

Every message is encoded with MessagePack through `encodeLink` and
`decodeLink`. There is no codec negotiation, JSON transport, CBOR transport,
or platform-specific Link envelope.

`CoreValue` maps directly to MessagePack primitives and preserves native binary
values as MessagePack `bin` data. Runtime conversion uses `toCoreValue` and
`fromCoreValue`; it never normalizes values through `serde_json::Value`.

## Carriers

The local Flutter and WASM bridges carry MessagePack values to generated local
dispatch. This crate does not implement an HTTP or WebSocket application server.

Node transport belongs to `node/peer-link` (`operit-peer-link`). HTTP,
WebSocket, TCP, serial and Bluetooth carry `PeerMessage::Request(CoreLinkRequest)`
or `PeerMessage::Response(CoreLinkResponse)`. Its untagged serialization preserves
the standard Link envelope. All carriers share MessagePack framing; serial adds
its OPS1/CRC byte framing.

Node runtime owns pairing, session encryption, connection reuse and correlation.
Pairing uses standard Calls. An encrypted session carries an encoded standard
Link message as binary arguments of a `$peer.session/data` Call; runtime decrypts
it before the authorized `CoreLinkSession` executes its original operation.

For routed calls, watches and push opens, node runtime puts
`RoutedCoreRequest<T>` in the standard request's arguments. The routed payload
preserves the original request and arguments. `PeerRouter` receives the decoded
route and delegates to Core or Edge execution. Watch lifecycle messages use
`CoreRequestId`; Push uses `pushId` and ordered `sequence` values.

## Benchmarks

`browser_surface_codec_bench.rs` compares the final MessagePack representation
against historical JSON/base64 and CBOR baselines. Those baseline codecs are
benchmark-only and are not exported by the product protocol.

`protocol_codec_bench.rs` measures the final Link codec for small calls and
native binary browser frame payloads.

## Stable targets

Calls, watches, pushes, events and embedded stream descriptors carry a string
`target`, not an object-pool index. Generated Proxy and SDK targets are
`core/<schema_key>`; Space route targets are `space/<target_type>/<method>`.
These names are public contracts: adding or reordering declarations cannot alter
existing addresses. Renaming a schema or exported route is a breaking API change.
MessagePack remains the only encoding; binary payloads remain native `bin` values.

The stream resource manager remains internal. Its protocol target is
`$core.stream.open`; the annotation wrapper target is `$core.internal`.
Neither uses a numeric sentinel. Request, watch and push IDs identify operations
or streams, not API endpoints. Current peer authentication uses the node runtime's
`PAIRING_SERVICE_VERSION = 1`. Targets use their declared string addresses.

See [the architecture document](../../../../docs/link-access-architecture.md)
for the runtime boundaries and remaining cleanup items.
