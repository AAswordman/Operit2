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
