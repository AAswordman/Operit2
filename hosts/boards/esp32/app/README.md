# ESP32 typed GPIO example

This minimal example composes `Esp32GpioHost`, `EdgeNode` and `EdgeProxy` and
locally toggles GPIO2. Run `cargo run` from this directory with the ESP Rust /
ESP-IDF toolchain installed.

The example does not assemble network pairing, Space admission, a display or
chat. Use [`apps/esp32`](../../../../apps/esp32/README.md) for the current device
firmware and [`tools/esp32-editor`](../../../../tools/esp32-editor/README.md) for
its build, simulation and device-debug workflow. The shared architecture is
specified in [Edge responsibilities](../../../../docs/edge-architecture.md).
