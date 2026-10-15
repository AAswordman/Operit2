# Theme Studio Flutter Preview

Standalone Flutter Web preview entry (`tools/theme/preview`) for Operit2 Theme Studio. It depends on `operit_ui`, not `operit2`, and loads only the trusted Rust/WASM contract worker and Flutter bootstrap—with no Operit runtime bridge, native-assets hook, v86, Core, or model services.

## Overview

The preview renders the shared `ThemeClientScene` (`client-shell-v7`), including the default workspace sidebar (unbound conversations and named workspaces), history row and group chrome, top bar, user/AI message bubbles, 9-slice bubble and input surfaces, and both Agent and Classic input styles (with custom action icon assets and sizes).

In embedded Studio mode, incoming drafts and binary assets must pass the preview's own same-origin Rust/WASM worker (`web/theme_contract_worker.js`) before Flutter decodes images or registers fonts.

## MessagePort Protocol (`lib/protocol/theme_studio_message_port.dart`)

- Accepts only an exact `studioOrigin`, `nonce`, `protocolVersion: 1`, and `window.parent` as the message source, bound to the compiled `OPERIT_THEME_RENDERER_RELEASE_ID` and `OPERIT_THEME_FIXTURE_SOURCE_SHA256`.
- Supports live revision rendering (`load.v1` → `loaded.v1` → post-frame `rendered.v1`), normal-chat / empty-scene switching (`sceneMode.v1`, defaulting to normal conversation in Studio), interactive 6-target component inspection (`inspectMode.v1` / `selection.v1`), and `RepaintBoundary` PNG frame capture (`capture.v1` / `captured.v1`) used by Studio for `.toolpkg` package export.

## Building & Testing

Build an immutable preview release from the repository root using `tools/theme/build_preview.py`:

```powershell
..venvScriptspython.exe tools/theme/build_preview.py --renderer-release-id development-local-20 --flutter-sdk <flutter-sdk-root> --wasm-bindgen <wasm-bindgen-cli-path>
```

Check the shared UI package from `apps/flutter/app` with the project's FVM SDK:

```powershell
fvm dart analyze ../packages/operit_ui
```

See [`docs/theme-studio-protocol-v1.md`](../../../docs/theme-studio-protocol-v1.md) and [`tools/theme/studio/README.md`](../studio/README.md).
