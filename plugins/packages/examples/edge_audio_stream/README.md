# Edge streaming STT adapter

`src/stream.ts` is a reusable adapter library for a ToolPkg running on Core.
Import `transcribeEdge` into your plugin and implement `StreamingStt` using your
local recognizer or cloud provider. The transport keeps using the existing
admitted TCP/serial PeerLink and its Link Push messages.

```ts
const { inputs } = await Tools.Edge.listAudioInputs(nodeId);
if (!inputs.length) throw new Error("No microphone input");
const transcript = await transcribeEdge(nodeId, {
  inputId: inputs[0].inputId,
  format: { encoding: "pcm_s16le", sampleRateHz: 16000, channels: 1 },
  maxDurationMs: 20000,
}, recognizer);
```

Your `recognizer.start(format)` opens one STT session; `write(pcm)` accepts an
ordered raw PCM block; `finish()` signals input EOF and returns the transcript;
`abort(error)` releases a failed recognizer. Await writes so the provider's
backpressure reaches the bounded audio queue. Providers expecting WAV or another
codec need their own format adapter. No WAV header is supplied by this API.

The JavaScript boundary returns Base64; `decodePcm` makes a Uint8Array without
requiring browser `atob` or Node `Buffer`. Link carries `CoreValue::Bytes`, so
Base64 never goes over the Edge transport.

Choose a duration shorter than the plugin invocation timeout. Always close in
`finally`; do not keep a recording loop in the six-second `on_edge_event` hook.

ESP firmware must install an `AudioCaptureHost` for its actual microphone.
Default 115200 baud cannot carry 16 kHz PCM: set matching
`HostManager.withPeerSerialBaudRate(...)` values on the two existing UART
endpoints. The ingress/upload tests use a simulated microphone; no particular
ESP microphone or I2S pin mapping is assumed.

Details: [Edge audio API](../../../../docs/edge-streaming-audio.md).
