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
  results, including paired devices that are currently offline.

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

A connection code contains only the node ID, display name, and public network
listener addresses. It does not include a pairing code, credential, or Space
admission. The initiator must still enter the receiver's six-digit pairing code
and follow the existing Space join approval flow. A focused direct-link dialog
accepts only the selected device's node ID.

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
`pair-code <connection-code>` to initiate pairing, and the existing pairing and
`space` commands to confirm connections and manage join approval.
