# Operit ESP32 app

This app is the ESP32 Edge Core target using ESP-IDF. The app shell wires the
board Host, Edge Node, and typed Edge Proxy; Link request and event types remain
inside the Core and proxy crates.

This is a device capability node, not a full Operit `CoreNode`. It does not own
`OperitApplication`, chat or agent state, persistence, identity, Space sync, or
Access pairing. A phone, computer, or server remains the full Core runtime owner.

The current Edge Service exposes one device-owned object at object id `1`:

- `setDigitalOutput({ pin: u8, level: bool })`
- `getDigitalOutput({ pin: u8 })`
- watch `digitalOutputState({ pin: u8 })`

The LED implementation is provided through `operit-host-api::DeviceIoHost`,
implemented by `hosts/boards/esp32`. `operit-node-edge` owns the typed device service,
and `operit-proxy-edge` owns the typed proxy surface.

The directory responsibilities are:

```text
hosts/boards/esp32/app  composition root and device app behavior
hosts/boards/esp32      ESP-IDF implementations of Host API capabilities
core/node/edge   lightweight Edge Node and typed Edge Services
core/proxy/edge  typed Edge Proxy facade over the internal Link client
```

The full Core runtime remains the owner on a phone, computer, or server. The
ESP32 process owns the device Edge Node and later exposes its services through a
transport carrier; the carrier is separate from both the Host implementation and
the app behavior.

## Toolchain

Install the ESP Rust toolchain and ESP-IDF environment with `espup`, then run
the application from this directory:

```text
cargo run
```

The checked-in target is `xtensa-esp32-espidf`, with the LED connected to GPIO2.
