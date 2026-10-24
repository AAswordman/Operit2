# operit-board-esp32

ESP32-2432S028 implementations of Host API capabilities, used by
[`apps/esp32`](../../../apps/esp32/README.md). The firmware application assembles
identity, shared pairing/Space services, device behavior and UI. See
[Edge responsibilities](../../../docs/edge-architecture.md).

The board crate provides GPIO, ILI9341 RGB565 display and touch adaptation,
robot-face support, and ESP-IDF TCP/serial Host adapters. Host code owns the
platform sockets, byte I/O and physical display; it does not own pairing trust,
Space policy, Link semantics or application navigation.

The panel uses rotation 90 and a physical 320×240 landscape display. The current
firmware flushes two RGB565 rows at a time (1280 bytes). The optional 75 KiB
RGB332 diagnostic mirror is disabled by the firmware to preserve heap capacity.

[`app/`](app/README.md) is a separate minimal GPIO example, not the production
firmware or a device-space acceptance test.
