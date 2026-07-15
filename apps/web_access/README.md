# Operit2 Web Access

This directory owns the Web Access frontend boundary.

The deployment server must send these headers for every application asset so
threaded Sherpa ONNX WebAssembly can run local STT and TTS:

```text
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Resource-Policy: same-origin
```

The Operit CLI Web Access server emits these headers directly.

- `web/` contains the tracked Flutter Web shell files for Web Access.
- `build/bundle/` contains the generated bundle consumed by the Flutter app and CLI packages.
- `tools/build_scripts/build_flutter_web_access.py` ensures the Flutter web entry link exists and writes the shared bundle.
- `web_access_version.json` is generated from bundle content; its integer version increases when the bundle hash changes.
- The current Dart entrypoint still lives in `apps/flutter/app`; consumers depend on the generated bundle here, not on Flutter's internal `build/web` directory.
- `apps/flutter/app/web` is a symlink to `apps/web_access/web`.
- `apps/flutter/app/build/web` is a symlink to `apps/web_access/build/bundle` during Flutter Web builds.
