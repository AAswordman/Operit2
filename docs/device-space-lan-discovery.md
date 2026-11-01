# Device Space LAN Discovery

## Contract

- Discovery queries `_operit-link._tcp.local.`; it does not enumerate IP addresses.
- A device advertises only while a configured network listener is running and
  `discoveryEnabled` is enabled. The default listeners are HTTP and WebSocket on
  an OS-assigned port, bound to `0.0.0.0`.
- TXT metadata must contain `nodeId`, `displayName`, and `transports`. Incomplete
  advertisements are rejected with diagnostics, without inventing protocol data.
- The add-device dialog performs a five-second active scan. Refresh restarts the
  shared query while retaining subscription handles. It does not just read an
  old snapshot or create a competing browser.
- Local and already-paired devices are intentionally excluded from add-device
  results, including paired devices that are currently offline. Their resolved
  endpoints remain available to the connection layer.
- The availability worker subscribes to the same Host browser even while the
  add-device dialog is closed. An offline paired network device reconnects using
  ranked discovered addresses, with its saved address as a fallback. Each
  alternative gets a bounded connection and authentication attempt.
- A discovered `nodeId` is only a dial hint. After a session handshake verifies
  the expected node and proves possession of the existing pairing secret, the
  runtime persists the successful address **and transport** in the outbound
  pairing record. This includes changes to an OS-assigned listener port and
  does not require a new pairing code. Failed authentication leaves the saved
  endpoint unchanged; concurrent unpairing or credential replacement cannot be
  undone by an in-flight connection. An established channel is reused until it
  fails; discovery does not open an additional channel for an online peer.

## Host Resources

Windows, Linux, Android, and OHOS use the shared native mDNS provider. The daemon
and its browser workers are owned by the Host. Android acquires its Wi-Fi
multicast lock before the daemon opens sockets and releases it after shutdown.
Ethernet-only Android hardware has no Wi-Fi multicast filter to acquire.

iOS and macOS use the system Bonjour DNS-SD daemon, not application-owned UDP
multicast sockets. Browse, SRV/TXT resolution, address watching, registration,
and cancellation stay in the Apple Host. IPv4 and IPv6 address watchers retain
the interface on which a service was discovered. The Flutter app declares
`_operit-link._tcp` and a local-network permission description on both platforms.
The macOS CLI installs the same Apple discovery Host.

## Diagnostics

The `ServiceDiscovery` log records the selected native browse interfaces,
resolved addresses and ports, daemon errors, advertisements, and scan totals.
Apple permission denial is reported explicitly instead of becoming an empty
successful scan. Native daemon failures also reach the discovery caller.

`RuntimePeerService` records raw advertisement totals and filtering totals for
local, paired, invalid, and addable devices. This distinguishes missing network
packets from advertisements that the add-device contract intentionally hides.

Both endpoints must permit local multicast DNS traffic (UDP 5353 to
`224.0.0.251` / `ff02::fb`) and inbound connections to the advertised listener
port. AP/client isolation, multicast filtering, and firewall policy are network
configuration issues and are not bypassed by another discovery protocol.

## Verification

Run the shared provider tests from `hosts/common/operit-host-native-common`:

```powershell
cargo test --lib ServiceDiscovery::tests
cargo test --test apple_discovery_contract
cargo test --lib ServiceDiscovery::tests::liveLanAdvertisementResolvesToListeningPort -- --ignored
```

The opt-in live test starts independent publisher and scanner daemons, resolves
the advertised service, and connects to its real TCP listener using the returned
non-loopback IPv4 address and port. It requires a multicast-enabled LAN. It is a
single-machine network smoke test, not evidence of a two-device pairing.

Run the session authentication and random-port reconnection regressions from `core`:

```sh
cargo test -p operit-node-runtime --no-default-features --features peer-runtime --lib HostRuntimePeerService
```

These tests use real loopback TCP listeners with deterministic discovery events.
They cover automatic reconnection after a listener changes port, endpoint and
transport persistence, failed credential proofs, address fallback, and pairing
removal during an in-flight update.

Run UI checks from `apps/flutter/app`:

```powershell
fvm flutter test --no-pub test/device_space_actions_test.dart test/device_space_panel_test.dart
```

For device acceptance, run two updated devices on the same non-isolated LAN,
confirm both network listeners are enabled, grant Apple's Local Network access,
refresh the add-device dialog, and complete code-confirmed pairing. Verify both
the discovered identity and the advertised endpoint in the logs. Physical
Android and Apple permission/lifecycle behavior requires on-device validation.

## QR connections

The add-device dialog offers **Scan code** and **Show connection code** separately
from the discovery scan. Camera capture, importing a PNG/JPEG/WebP image, and
pasting connection information all use the runtime's same versioned decoder.
Camera capture is available on Android, iOS, macOS, and web; other hosts can
import an image or paste the code. A detected device is shown for confirmation
before starting any network connection.

A v2 connection code carries the node ID, display name, live network addresses,
and a random invitation secret valid for five minutes. The scanner shows one
**Confirm join** action before opening a connection. Confirmation authenticates
through the existing SPAKE2 pairing exchange and submits the Space join request;
neither device shows a manual pairing-code dialog for this path. When the scanned
device currently holds both admission and approval capabilities, it approves the
request using the existing durable review transaction. Other gateways retain the
normal assigned-reviewer flow. Consent is bound to the displayed Space and the
current pairing, so an old invitation cannot authorize admission to another Space.
A focused direct-link dialog still accepts only the selected device's node ID and
ends after connection when the device is already a member.

Addresses come from active listener sockets and the Host's current network
interfaces. Wildcard binds, port zero, loopback, and unscoped IPv6 link-local
addresses are never published. The Host enumerates interfaces without starting
an mDNS browser or advertisement, so QR connections work with discovery off.
A code does not bypass a firewall or create connectivity between isolated
networks. Refresh the code after the network or listening port changes.

The CLI renders a black-on-white terminal QR with a four-module quiet zone:

```sh
operit2 cli link qr
operit2 cli link qr http --no-discovery
```

The first form reuses an existing daemon/TUI owner when one is running. Otherwise
it starts an HTTP listener and keeps it alive until Ctrl-C. Listener options
apply only when creating this foreground listener. `--json` emits the connection
information without terminal escape sequences. In an interactive
`operit2 cli link session http --no-discovery`, use `qr` to show the same code,
`pair <connection-code>` to complete QR pairing, and the existing `space`
commands to submit or inspect joins. Refresh an expired code with `qr`.
