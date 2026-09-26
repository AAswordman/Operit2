# PR #135 boundary corrections

## Integration

- Working branch: `fix/pr-135-boundaries`.
- PR head: `5c89fb52`; merged onto `fa2935d8` by `e13950c9`.
- Release version remains `2.0.0-preview.8`; Flutter build remains `2.0.0+10`.
- The original PR commits are preserved. Unrelated workflow, storage logging,
  dependency and Space-join changes were not silently removed or represented as
  independently validated fixes.

## Ownership

- `ServiceDiscoveryHost` exposes resolved service records and owned subscriptions.
  Native host code owns mDNS daemons, per-service browsers, snapshots and listener
  registration. Core and Edge discovery consume this contract; node-runtime no
  longer depends directly on mdns-sd.
- Native platform host managers install the provider. An unregistered capability
  returns an explicit error. No browser mDNS implementation or alternate discovery
  protocol is introduced.
- Discovery subscription handles remove callbacks on drop. Space synchronization
  owns its handle and callbacks hold only a weak reference to runtime state.
- Flutter can request both scans concurrently. It has no Windows multicast
  scheduling policy and uses localized, board-independent Edge pairing text.
- CLI resolves the persisted pairing kind before calling either completion API.
  Edge commands reject Core-only transport options before pairing side effects.
- Persisted Edge transactions are the sole client-side pairing state source.
  Completion opens a fresh carrier; the start carrier is explicitly closed.
  Cleanup errors are propagated instead of discarded.
- ESP-IDF socket adaptation is in the ESP32 host board crate, not firmware UI or
  service assembly. The existing LinkChannel protocol contract is unchanged.

## Checks and coverage

The following are checks, not application builds or device acceptance tests:

- Cargo check for CLI; access-runtime, edge-transport and node-runtime test targets;
  native-common library/test targets; native scheduler test targets.
- FVM Dart analyze for DeviceSpaceDiscoveryPanel and RemotePairingBridge.
- Local proxy generator execution and FVM localization generation.
- Rust syntax parsing, Cargo metadata resolution and Git whitespace checks.

Added regression test source covers explicit pairing-kind resolution, missing and
ambiguous records, pairing completion on a fresh carrier, discovery record
conversion, zero-duration scans, and listener release. Checking test targets does
not execute these tests.

Cargo lockfiles were regenerated for manifests affected by the moved dependencies.
Some standalone locks were stale before this change, so Cargo also resolved their
existing dependencies. A metadata check is not a cross-platform compilation test.

## Remaining validation and scope

- No ESP32 flashing, real mDNS LAN scan, serial-port reconnect, or physical pairing
  acceptance was performed. Fresh-carrier behavior needs real TCP and UART checks.
- No Android, Apple, OHOS, Linux, ESP-IDF or browser target compilation was run.
- Existing platform-gated Edge transport and Space synchronization code outside
  the discovery migration remains; this is not a claim that the entire runtime
  is free of historical platform conditionals.
- Pending-pairing cancellation/expiry and multi-process transaction locking still
  need a separate lifecycle design. This change does not claim to implement them.
- The original device-side clearPairings placeholder and the original PR's other
  unrelated changes remain subject to separate review.
