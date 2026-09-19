# operit-host-api

`operit-host-api` defines the stable host boundary used by Operit2 core.
Platform crates implement these traits, while runtime crates consume them
through trait objects and data models.

## Responsibilities

- Define `HostError`, `HostResult`, and host-facing data structures.
- Provide `HostManager`, the host capability manager used by runtime startup
  and service wiring.
- Describe platform environments through `HostEnvironmentDescriptor`.
- Declare file system, HTTP, web visit, browser automation, terminal, managed
  runtime process, runtime storage, SQLite, audio, Bluetooth, TTS, system
  operation, and Plugin SDK IPC traits.
- Define `PluginSdkIpcHost` for connection, framed send/receive, close, session,
  and stream lifetime of third-party Plugin SDK clients. Hosts carry bytes only;
  Link call/watch/push semantics stay in `operit-link`.
- Define `HttpHost` batch download requests with bounded concurrency,
  aggregate progress events, native target paths, and cancellation controls.
- Keep platform code outside `operit-runtime` by routing host capabilities
  through explicit Rust traits.
- Define `SerialPortHost` and `SerialPortConnection` for opening platform
  serial devices/accessories and exchanging ordered bytes. Link framing and
  pairing remain in Core; device access and capability errors belong to Hosts.

## Key Files

- `src/lib.rs`: host capability traits, request and response data models,
  `HostManager`, and environment descriptors.
- `src/HostManager.rs`: active host registration and access point for runtime
  host capabilities.
- `src/PluginSdkIpc.rs`: Plugin SDK IPC host contract for third-party process
  admission.
- `src/TimeUtils.rs`: cross-target millisecond clock helpers for native and
  WebAssembly builds.
- `src/SerialPort.rs`: asynchronous serial byte-stream contract, including
  cancellation and connection shutdown semantics.

## Capability Model

Environment descriptors advertise capability tags such as `fs.read`,
`fs.write`, `web.visit`, `runtime.process`, `runtime.storage`,
`runtime.sqlite`, `tts.synthesis`, `tts.playback`, and `system.settings`.
These tags are descriptive metadata for prompts, UI surfaces, and tool
registration; actual enforcement lives in the runtime permission and host
implementation layers.

## Design Notes

The crate intentionally contains no platform implementation. Native and web
hosts provide concrete behavior, and `operit-runtime` remains coupled only to
this contract.
