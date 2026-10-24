# operit-node-edge

Device-side Edge Services and their Link adapter. The device's place in a Space
is defined in [Edge responsibilities](../../../../docs/edge-architecture.md).

This crate owns typed device services, native application endpoint hooks and
`EdgePeerRouter`. It does not assemble `OperitApplication`, an Agent runtime,
providers, ToolPkg engines or a full business replica. Device applications wire
shared node identity, pairing, Space control and storage services at their
composition root; those services are not independently reimplemented here.

`peer::createPeerService` uses the shared code-confirmed pairing and encrypted
session engine. Pairing and Space admission remain separate. `EdgePeerRouter`
accepts local Target routes and the defined shared Space control calls; it is
not a full Core Binding router or a relay implementation.

Applications register native calls with `withCallHandler` and optional output
and input endpoints with `withStreamHandlers`. These use Call, Watch / Stream
and Push / ReverseStream. Missing endpoints return a standard method error.
Host API traits provide hardware, I/O and storage; the device app owns behavior.

An Edge can persist its own configuration and resources without
`storage.provide`. Bounded device preferences are exchanged on the shared Space
control channel with membership and revocation checks. Full business replicas
require storage capability and actual Core storage services at both endpoints.

The current [ESP32 application](../../../../apps/esp32/README.md) supports native
device services, bounded independent HTTP chat and optional remote Core chat.
Configuration ToolPkg runs on a capable host and need not remain installed or
running after the device configuration has been saved.
