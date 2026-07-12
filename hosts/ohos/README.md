# OpenHarmony Host

`operit-host-ohos-native` owns the Rust host implementations selected by
`target_os = "ohos"`.

The initial host registers these concrete capabilities:

```text
POSIX application file access
native HTTP
runtime storage
SQLite storage
PTY sessions
Flutter-owned browser interactions
```

OpenHarmony application storage roots must be passed to
`operit_flutter_bridge_create_with_storage_roots`. The generic bridge creator
does not choose application paths.

System operations, secure secret storage, audio, Bluetooth, TTS, and managed
language runtimes require dedicated ArkTS or N-API implementations before they
are registered in `HostManager`.

