# OpenHarmony Flutter Adaptation Checklist

The current owner targets OpenHarmony API 18 and the FVM-pinned Flutter SDK.
Use `tools/build_scripts/build_flutter_ohos.py --enforce-lockfile` to build and
sign the HAP. Flutter commands run through FVM from `apps/flutter/app`.

## Implemented integration

- File selection, streamed document input and URI saves: local `file_selector_ohos`.
- Image selection: local `image_picker_ohos`.
- Paths, links and video: locked OpenHarmony-SIG platform plugins.
- Recording: local `record_ohos`.
- WebView: local `webview_all_ohos`, including scripts, zoom and local resources.
- Package metadata: owner channel backed by installed bundle information.
- General audio: `OperitAudioPlayers.ets` implements the existing audioplayers
  channels with one AVPlayer per player, local/URL/byte sources, events, seeking,
  volume, pause/resume and resource cleanup.
- TTS and music: owner AVPlayer controls wait for initialized/prepared states.
- PDF preview: `OperitPdfPreview.ets` implements printing's raster protocol with
  offline Poppler in the existing terminal runtime. Page ordering and scale are
  retained; temporary PDF/PNG files are removed after each job. Printing and
  sharing buttons remain disabled in the existing preview widget.
- OCR: OHOS Host uses packaged offline Tesseract with English, simplified and
  traditional Chinese, Japanese and Korean data. It advertises OCR only through
  this concrete implementation.
- Background work: `operit_keep_alive` uses the owner task-keeping API.

## Terminal and managed runtimes

- Native shell and QEMU-vroot Alpine PTYs retain the shared stream/resize/close
  and guest-directory contracts.
- Python, Node, npm, uv, pnpm, curl, Git and SSH are packaged for offline install.
- The image contains the Alpine base and an indexed offline APK repository.
  First launch installs packages through apk; upgrades update bundled packages
  in place and preserve user files and user package database entries.
- Installer staging is removed on completion or error. Installed images do not
  retain a duplicate APK repository.
- Raw assets copy in bounded chunks and only when the packaged checksum changes.
  The Rust host verifies the archive for installation, rather than hashing it
  before every command.
- The owner publishes active-network DNS servers for guest package managers and
  runtimes. The guest resolver points at that owner file.
- POSIX serial uses `serial2` and Tokio readiness, avoiding the old nix/libc ABI
  mismatch on OHOS. Windows keeps its existing overlapped serial implementation.

## Validation and device limits

- Host API tests, Windows/Linux serial tests and shared terminal contracts pass.
- `tools/ohos-runtime/test_rootfs_linux.sh` executes ARM64 packages through QEMU:
  offline installation, upgrade preservation, runtime startup, PDF page order and
  scale, and OCR are covered. Run as root in Linux with qemu-user installed.
- HAP compilation and signing are checked separately from device behavior.
- No OHOS device is connected. Installation, HNP execution policy, interactive
  PTY input/resize, AVPlayer output, recording, Bluetooth, location, background
  protection and native WebView behavior still require device validation.
- API 18 has no public system speech synthesis interface; Core-generated speech
  uses the implemented audio playback path. Arbitrary stopping of other apps
  requires privileges outside a normal application sandbox.
- PDF raster preview is implemented; the app does not expose physical printing
  on this owner. Dynamic color, desktop windows and desktop drag/drop follow
  their existing platform capability boundaries and are not claimed as OHOS
  native functionality.

## Maintained sources

Native plugin fixes live in `apps/flutter/thirdparty`, never in the Pub cache or
generated `.flutter_ohos_plugins`. Staged dependencies match the Flutter-generated
plugin registrant, including its integration-test plugin reference. Obsolete
Flutter embedding patches have been removed; remaining patches match the pinned
SDK sources.
