# operit-node-edge

Lightweight device-side Edge Core node.

The crate owns typed Edge Services and adapts them to the internal
`CoreLinkSharedClient` boundary. It consumes `operit-host-api::HostManager` and
does not represent the full `CoreNode` defined by the Space architecture. It
does not own `OperitApplication`, providers, ToolPkg, chat orchestration,
persistence, identity, Access sessions, Space sync, or Proxy implementations.

Transport carriers are composition concerns. A future ESP32 Wi-Fi, BLE, or
serial carrier should feed the node through a transport adapter while keeping
Link request and event types out of the device app layer.
