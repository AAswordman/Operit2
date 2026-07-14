# OpenHarmony Flutter Adaptation Checklist

This checklist tracks the Flutter-side work needed to make the HAP build and
runtime usable. The validation command for each resolved item is:

```powershell
.\.venv\Scripts\python.exe tools\build_scripts\build_flutter_ohos.py --enforce-lockfile
```

## Current Native OHOS Coverage

- [x] `record` via local `apps/flutter/thirdparty/record_ohos`.
- [x] `webview_all` via `webview_all_ohos` in the generated OHOS plugin graph.

## P0 Startup And Storage

- [x] `ClientLogger_io.dart`: add explicit OHOS support in the startup logging
      path.
- [x] `path_provider`: add `path_provider_ohos` from OpenHarmony-SIG as a
      locked git dependency at `35fb467533e174411a117b2a030c15d2a3a9687c`.
- [x] Re-run `fvm flutter pub get` from `apps/flutter/app` and verify
      `.flutter-plugins-dependencies` contains `path_provider_ohos` under
      `plugins.ohos`.
- [x] Run the HAP build script and fix source-owned compile errors.

## P1 File And Link Interactions

- [ ] `url_launcher`: add `url_launcher_ohos` from OpenHarmony-SIG
      `flutter_packages/packages/url_launcher/url_launcher_ohos`.
- [ ] `file_selector`: add `file_selector_ohos` from OpenHarmony-SIG
      `flutter_packages/packages/file_selector/file_selector_ohos`.
- [ ] `file_picker`: test direct YAML package `file_picker_ohos: ^1.0.1`.
- [ ] Re-run plugin graph generation and verify OHOS plugin entries for
      `url_launcher`, `file_selector`, and `file_picker`.
- [ ] Run the HAP build script and vendor the plugin source locally when edits
      are required.

## P2 Media And Document Preview

- [ ] `video_player`: test OpenHarmony-SIG
      `flutter_packages/packages/video_player/video_player_ohos`.
- [ ] `printing`: test direct YAML package `printing_ohos: ^1.0.1`.
- [x] TTS playback: generated speech audio now uses the runtime owner's
      `TtsPlaybackHost` and the OHOS owner `AVPlayer`; Flutter TTS code no longer
      depends on `audioplayers`.
- [ ] `audioplayers`: still used by Markdown audio and workspace media preview;
      adapt those general media surfaces independently from TTS.
- [ ] Run the HAP build script after each package group.

## P3 Desktop-Specific UI Dependencies

- [ ] `desktop_multi_window`: verify HAP compile with existing OHOS platform
      gate in `OperitWindowPlatform.dart`.
- [ ] `desktop_drop`: verify HAP compile for chat input widgets.
- [ ] `dynamic_color`: verify runtime initialization and theme usage on OHOS.

## Package Source Policy

- [ ] Use direct YAML or locked git dependencies for packages that compile and
      register cleanly.
- [ ] Move packages into `apps/flutter/thirdparty/` only when the HAP build exposes
      source-level fixes that must be maintained by this repo.
- [ ] Keep each vendored package documented with upstream origin, revision, and
      local patch notes.
