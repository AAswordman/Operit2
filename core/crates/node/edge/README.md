# operit-node-edge

Lightweight device-side Edge Core node.

The crate owns typed Edge Services and adapts them to the internal
`CoreLinkSharedClient` boundary. It consumes `operit-host-api::HostManager` and
does not represent the full `CoreNode` defined by the Space architecture. It
does not own `OperitApplication`, providers, ToolPkg, chat orchestration,
persistence, identity, Access sessions, Space sync, or Proxy implementations.

## Pairing startup

Firmware, simulator and other Edge apps use `peer::createPeerService` to select
code-only pairing in the shared runtime. The receiver never asks for an Edge
token, including on remote network connections. Confirm the six-digit code on
the receiving device; pairing alone does not grant Space membership or business
permissions. Outbound pairing to a Core uses the same code-confirmed handshake.
The shared listener preference schema no longer includes a token field.

## Native application endpoints

Device applications attach their own Call dispatch with `withCallHandler` and
Stream / ReverseStream endpoints with `withStreamHandlers`. These are native
application callbacks on the existing shared Link client. A firmware application
owns its device behavior, resource configuration, display input and optional
local processing.

`EdgePeerRouter` carries Call, Watch and Push through the shared authenticated
runtime. ToolPkg code runs on capable hosts and uses the generic device transport;
its configuration editor is optional once configuration has been saved to hardware.
The common node boundary does not contain an audio-specific runtime.

## Application configuration and replication

The shared Space control channel exchanges bounded device preferences independently
of `storage.provide`. Every exchange checks current membership, revocation, Space
and device scope. Full operation clocks, business logs and blob endpoints still
require storage capability at both endpoints. Native apps choose configuration
keys and own their local session state; Host only supplies storage and transport.

The ESP app provides `edge.preferences` for configuration editors and native
direct chat over Host HTTP. None of this requires a ToolPkg runtime on Edge.
