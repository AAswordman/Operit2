# OpenHarmony Host

`operit-host-ohos-native` owns the Rust host implementations selected by
`target_env = "ohos"`.

The initial host registers these concrete capabilities:

```text
POSIX application file access
native HTTP
runtime storage
SQLite storage
PTY sessions
managed language runtimes through OpenHarmony-visible executables
Flutter-owned browser interactions
Sherpa ONNX local inference
ArkTS AVPlayer generated TTS playback controls
```

OpenHarmony application storage roots must be passed to
`operit_flutter_bridge_create_with_storage_roots`. The generic bridge creator
does not choose application paths.

OHOS local inference loads the checksum-verified Sherpa ONNX engine selected by
the local model registry from its installed runtime directory. TTS providers
generate runtime storage audio through Core, and one ArkTS `AVPlayer` owns
playback, pause, resume, stop, and state queries. The API 18 SDK does not expose
a public system text-to-speech synthesis API, so the OHOS host does not
advertise `SYSTEM_TTS`.
