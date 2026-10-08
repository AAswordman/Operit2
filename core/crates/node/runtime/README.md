# operit-node-runtime

The shared node runtime owns routing, pairing, authenticated sessions, Space
membership and peer connection lifecycle. Device roles must reuse these
implementations rather than introduce an Edge-specific pairing protocol or
an independent authorization state machine.

## Communication ownership

- `RuntimePeerService` and `NodeServices`: shared service contract and injection.
- `HostRuntimePeerService`: production pairing, listening and connection owner.
- `PeerStateStore`: versioned local credentials and listener configuration.
- `peer/crypto.rs`: handshake proofs and authenticated encryption.
- `peer/dispatch.rs`: authenticated Link sessions, delegated to the shared router.
- `peer/space_channel.rs`: separately authorized Space return channels.
- `peer/availability.rs`: authenticated availability and revocation handling.

Transport I/O belongs to `operit-peer-link` and platform Host implementations.
Accepting a socket is not authentication; pairing is not Space admission and
does not implicitly grant the reverse pairing direction.

## Local execution boundary

`CoreNodeLocalRuntime` accepts Space call/watch execution through the existing
`CoreLinkSharedClient` contract, not a concrete chat runtime. Full Core supplies
`SpaceRuntime`, which implements that contract and retains the existing chat
execution and embedded-stream ownership. Router authorization remains unchanged.

This boundary does not make unsupported business methods succeed, and does not
add Space Push support. Local execution services sit behind routing and access
checks; they are not anonymous network endpoints.

## Space merge contract

An approved join merges the applicant's complete current Space, including members
reachable only through existing peers. The request freezes the source membership,
real device profiles and directed link advertisements. The reviewer approves that
exact set with one `AdmitSpace` command; source administrators do not inherit
administrator rights in the target Space. Approval displays the source device
names together.

Direct peer projection exchange carries profiles, topology and target control
operations before publishing member records. A source peer follows a cross-Space
migration only when an existing source member presents an accepted target
`AdmitSpace` covering its source membership. B-C-D-(E,F)-G converges hop-by-hop
without creating direct B-D/B-G pairings. Independent pairings alone do not merge
Spaces. Topology rendering uses one membership snapshot for devices and endpoints.

Group approval records use `runtime/link_access/space_merge_*.preferences.json`.
Single-device `space_join_*` records remain untouched and are not interpreted as
consent to a group merge; pending requests from the previous protocol must be
resubmitted. All participants must run the updated protocol.

## Features and generated code

- No default features: shared node contracts; **not** a production peer runtime.
- `route-catalog`: shared route IDs, binding metadata and permission metadata.
  It does not link the application, tools, providers or persistence stores.
- `peer-state`: existing identity/credential persistence via the lightweight
  `operit-store/node-state` feature, without chat DTOs or database backends.
- `peer-runtime`: the shared production pairing, authentication, encrypted sessions
  and listener, without the full application/tool runtime.
- `full` (default): the current complete node implementation and business adapter.
  It includes `route-catalog`.

`build.rs` scans the same route annotations once and emits:

- `generated_route_catalog.rs`: wire addresses and routing/permission metadata.
- `generated_space_dispatch.rs` (full only): concrete ChatServiceCore execution.

Metadata-only builds still need the annotation sources in this repository at
build time. They do not compile those sources as an application dependency.
There is no separately maintained Edge route or permission table.

## Edge integration

`PeerRouter` is the shared authenticated execution boundary. Full Core adapts
`CoreNodeRouter`; Edge uses `operit-node-edge::PeerRouter::EdgePeerRouter`, which
forwards local Target call/watch requests to the existing `EdgeNode` services.
Edge does not copy pairing, authorization, credential formats or wire sessions.
Standalone Edge rejects Space routes and Push instead of pretending to support
them. A paired session may provide an explicitly negotiated, same-Space return
route to a full Core; this does not install Core business services on Edge.

ESP32 installs the shared `HostRuntimePeerService` after Wi-Fi readiness, using
board-specific RuntimeStorageHost (NVS) and ServiceDiscoveryHost (mDNS)
adapters. Identity and paired credentials persist in NVS; the listener uses the
same versioned PeerStateStore and standard `_operit-link._tcp` advertisement.
The current ESP32 adapter advertises but does not implement active discovery
or discovery subscriptions. NVS runtime records have a 7000-byte budget.

Build and software tests do not replace flashing, hardware resource checks,
actual discovery or confirmation-code pairing on the connected device.

## Verification

From the repository root:

```powershell
cargo check --manifest-path core/Cargo.toml -p operit-node-runtime --no-default-features --features route-catalog
cargo test --manifest-path core/Cargo.toml -p operit-node-runtime --no-default-features --features route-catalog --lib
cargo test --manifest-path core/Cargo.toml -p operit-node-runtime --lib -- --test-threads=1
cargo check --manifest-path core/Cargo.toml -p operit-proxy-local
```

## Session and resource boundaries

- `PeerLink::exclusiveEndpointKey` identifies exclusive I/O resources;
  connection setup locks are per endpoint, never global across network transports.
  `PeerConnection::requiresSessionReuse` selects reusable session handling.
- Session return is an optional authenticated capability (`$peer.space-channel`,
  `capabilities` / `session-return`). Legacy peers without it retain ordinary
  directional calls. Every return operation still checks the original pairing
  and current Space admission; it creates neither reverse pairing nor a role.
- The embedding app owns the `PeerRouter`. Peer services and idle dispatchers
  hold weak references; releasing the router returns `PEER_ROUTER_CLOSED`.
- `newWithLimits` accepts `PeerRuntimeLimits`; the existing `new` keeps normal
  defaults. Firmware and simulator select `constrained()` explicitly. Shared
  session limits and probe concurrency do not depend on an OS name.
- `nodeHasCapability` reports policy, not installed services. Business replication
  requires both `storage.provide` authorization and an available Core persistence
  service. An admin/storage grant cannot install that service on a lightweight
  endpoint. Blob streams recheck eligibility for every chunk and final commit.
- Control-only exchange is additive: only `METHOD_NOT_FOUND` falls back to the
  legacy sync path. Denials, malformed payloads and network failures propagate.

## Repeated Space admission and recovery

Node-local projection cleanup uses `CoreNodeStateStore::delete` so deletion of
profile/topology/member files also clears their shared Preferences snapshot.
A same-Core rejoin after an Edge-local exit must recreate identical profiles,
not skip the durable write because an earlier profile remains cached.

An approval claim and its persisted admission are not a pending offer. Applicant
cancellation before claim is terminal; a cancellation after an authoritative
approval cannot revoke membership and must report the actual recovery state.
If a partial approval has published membership but a profile file is missing,
retry can restore only missing current-member profiles from that exact durable
review, after matching its authorized admission, request and assignment version.
No new admission, role or business-storage synchronization is introduced.

The rejoin regressions reopen both Core and Edge storage hosts. A failed Pending
cancellation survives restart and is retried by normal refresh; cancellation
racing a completed approval reconciles to Joined instead of fabricating Cancelled.
Both paths preserve pairing and keep the non-storage Edge out of business replicas.

Targeted software regressions (real TCP/pairing and the ESP32 snapshot codec,
without touching hardware):

```powershell
cargo test --manifest-path core/Cargo.toml -p operit-node-edge --lib rejoin
cargo test --manifest-path core/Cargo.toml -p operit-node-edge --lib uncommitted_claim
cargo test --manifest-path core/Cargo.toml -p operit-store --lib node_local_
```
