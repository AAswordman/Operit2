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
managed Python/Node/uv/pnpm runtimes through QEMU-vroot Alpine
offline Tesseract OCR and Poppler PDF rasterization
Flutter-owned browser interactions
Sherpa ONNX local inference
ArkTS AVPlayer generated TTS playback controls
```

OpenHarmony application storage roots must be passed to
`operit_flutter_bridge_create_with_storage_roots_and_system_language`. The generic bridge creator
does not choose application paths.

OHOS local inference loads the checksum-verified Sherpa ONNX engine selected by
the local model registry from its installed runtime directory. TTS providers
generate runtime storage audio through Core, and one ArkTS `AVPlayer` owns
playback, pause, resume, stop, and state queries. The API 18 SDK does not expose
a public system text-to-speech synthesis API, so the OHOS host does not
advertise `SYSTEM_TTS`.

The terminal installs bundled packages from the checksum-verified offline Alpine
repository. An update runs apk against the existing rootfs; user-installed
packages and files are retained. Guest DNS is supplied by the active OHOS owner
network. Archive hashes and package payloads are checked during installation,
with the ready result shared by terminal instances for the current process.

General Flutter audio widgets use the owner audioplayers protocol adapter.
PDF preview uses the existing printing raster channel and the same terminal
host, without platform conditions in shared Dart. Offline OCR callbacks are
fixed by the OHOS host assembly rather than an unavailable native OCR method.
