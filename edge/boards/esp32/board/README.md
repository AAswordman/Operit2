# operit-board-esp32

ESP32-2432S028 implementations of Host API capabilities, used by
[`edge/boards/esp32`](../README.md). The firmware application assembles
identity, shared pairing/Space services, device behavior and UI. See
[Edge responsibilities](../../../docs/architecture.md).

The board crate provides GPIO, ILI9341 RGB565 display and touch adaptation,
robot-face support, ESP-IDF TCP/serial Host adapters, and native I2S/DAC audio I/O.
Host code owns the platform sockets, byte I/O and physical display; it does not own pairing trust,
Space policy, Link semantics or application navigation.

Audio support here owns INMP441 I2S capture, DMA, the bounded acquisition task
and queue, microphone signal conversion, DAC output conversion, and hardware
resource cleanup. These are board primitives used by the firmware, not a
registered general-purpose `AudioPlaybackHost` music service. WAV decoding,
Core speech packets, recording/playback sessions and UI belong to
`edge/boards/esp32/src/{wav,audio,device_speech}.rs`.

The panel uses rotation 90 and a physical 320×240 landscape display. The current
firmware flushes two RGB565 rows at a time (1280 bytes). The optional 75 KiB
RGB332 diagnostic mirror is disabled by the firmware to preserve heap capacity.

[`app/`](../examples/minimal/README.md) is a separate minimal GPIO example, not the production
firmware or a device-space acceptance test.
