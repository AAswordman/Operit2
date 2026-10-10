# operit-node-edge

Lightweight device-side Edge Core node.

The crate owns typed Edge Services and adapts them to the internal
`CoreLinkSharedClient` boundary. It consumes `operit-host-api::HostManager` and
does not represent the full `CoreNode` defined by the Space architecture. It
does not own `OperitApplication`, providers, ToolPkg, chat orchestration,
persistence, identity, Access sessions, Space sync, or Proxy implementations.

## Native plugins

Firmware registers small `EdgePlugin` implementations at startup with
`EdgeNode::withPlugin`. Each declares a stable id and action names. Authenticated
PeerLink callers use the `edge.plugins` target: `list` returns manifests;
`invoke` accepts `pluginId`, `action`, and `args`. Core callers can use
`EdgeProxy::plugins()` for the same operations.

Only firmware-registered actions run on the device. ToolPkg JavaScript and
unadapted plugin UI remain on Core and are not transferred as executable code.

Transport carriers are composition concerns. A future ESP32 Wi-Fi, BLE, or
serial carrier should feed the node through a transport adapter while keeping
Link request and event types out of the device app layer.

## Streaming microphone input

An optional `AudioCaptureHost` installed in `HostManager` supplies microphone
inputs. Core plugins use `Tools.Edge.listAudioInputs`, `openAudioInput`,
`readAudioInput`, and `closeAudioInput`; Edge uploads bounded binary PCM through
its existing admitted TCP/serial Link Push connection. Hardware capture drivers
must report overruns and release recording resources on Drop.

See [the audio contract and plugin adapter](../../../../docs/edge-streaming-audio.md)
for lifecycle, format and UART bandwidth requirements.
