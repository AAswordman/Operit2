# Headless sensor product

This independent product demonstrates the manufacturer SDK without a display,
GPIO service, scene renderer or full Core. It exposes `manufacturer.sensor/read`
and `manufacturer.agent/run` through the authenticated device service router.
The desktop sensor returns 23.5 C; replace `Temperature::call` with a real HAL.
The offline backend demonstrates the complete model -> tool -> observation ->
reply loop. A product can replace that backend with `http_agent::HttpBackend`
by enabling the SDK's optional `http-agent` feature.

```sh
cargo run --manifest-path edge/boards/headless_sensor/Cargo.toml -- --once
cargo run --manifest-path edge/boards/headless_sensor/Cargo.toml
```

`--once` runs the local agent without creating a network listener. Normal startup
persists identity and Space state in `.sensor-state` (override with
`OPERIT_SENSOR_STATE`) and prints its real TCP address. Console commands:

- `agent`: read the local sensor through the agent tool loop.
- `status`: print Space ID, paired peers and locally displayed pairing codes.
- `pair <address>`: initiate the SDK pairing protocol with a Core.
- `confirm <pairing-id> <six-digit-code>`: confirm the code displayed by Core.
- `join <core-node-id>`: request admission to Core's device space.
- `refresh <request-id>`: reconcile the approval and persist admission.
- `quit`: stop the listeners and connections.

Core can also initiate pairing using the printed address; `status` displays the
local confirmation code. The product keeps operating locally before admission
and after Core goes offline. Space membership and service authorization use the
same runtime as the ESP32 product. The full graphical acceptance suite lives in
`../esp32/editor`; SDK and agent boundaries live in `../../sdk/tests`.
