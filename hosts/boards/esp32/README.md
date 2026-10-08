# operit-board-esp32

Board profile for ESP32-2432S028 hardware.

This crate owns the board-specific Host API implementations used by
`apps/esp32`. The implemented surfaces are:

- `DeviceIoHost` for the onboard RGB status LEDs
- `RobotFaceHost` for the ILI9341 face display

The crate does not own Operit Core startup, Access pairing, Link packets,
Wi-Fi transport, or the firmware HTTP home page. Those belong to `apps/esp32`.

Display rotation `90` is used for the physical 320x240 landscape panel.

The physical panel receives RGB565 pixels. The board still exposes an optional
RGB332 diagnostic mirror for hosts that explicitly enable it, but the firmware
releases that 75 KiB buffer because its current UI uses the physical panel only.
The self-drawn UI uses a synchronous two-row RGB565 strip buffer (1280 bytes), without a widget-library runtime.
