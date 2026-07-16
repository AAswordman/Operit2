# Building Operit2

This file describes how to build, run, and package Operit2 from the repository root.

Rust build warnings are currently expected. Treat command exit status as the signal for build success.

## Repository Layout

```text
apps/cli                  Rust CLI/TUI entry
apps/flutter/app          Flutter app entry
core/crates               Shared Rust runtime crates
hosts                     Native host implementations
tools/release             Release build and publish scripts
docs/release-versioning.md Version, tag, channel, and asset naming rules
```

## Requirements

Common tools:

```text
Rust stable toolchain with rustup
Python virtual environment at .venv
Git
```

Windows desktop CLI builds:

```text
Visual Studio 2022 C++ build tools
MSVC x64 and ARM64 components
LLVM clang at C:\Program Files\LLVM\bin\clang.exe
```

Flutter app builds:

```text
Flutter SDK
apps/flutter/app/android/local.properties with flutter.sdk
tools/release/secrets/android-signing.properties for Android release signing
```

GitHub publishing:

```text
tools/release/secrets/github.env
```

Required keys:

```text
GITHUB_TOKEN
GITHUB_API_URL
```

## Rust Targets

Windows CLI release builds use:

```powershell
rustup target add x86_64-pc-windows-msvc
rustup target add aarch64-pc-windows-msvc
```

Fedora WSL Linux CLI release builds use these Rust targets:

```bash
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-gnu
sudo dnf install -y musl-gcc musl-devel musl-libc-static gcc-aarch64-linux-gnu sysroot-aarch64-fc43-glibc
```

The aarch64 Linux CLI release requires an `aarch64-linux-gnu-gcc` cross C compiler and
the Fedora `/usr/aarch64-redhat-linux/sys-root/fc43` sysroot. The release script injects
the sysroot's `limits.h` while compiling C dependencies because Fedora's cross compiler
is built without target headers.

## Local CLI Checks

Run from the repository root.

```powershell
cargo check --manifest-path apps/cli/Cargo.toml
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- cli version
```

Run the TUI:

```powershell
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- tui
```

Run update checks:

```powershell
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- cli update target
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- cli update check 0.0.0-preview.0
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- tui --update-current-version 0.0.0-preview.0
```

## Release Script

Interactive release helper:

```powershell
.\.venv\Scripts\python.exe tools\release\release_interactive.py
```

Direct release script:

```powershell
.\.venv\Scripts\python.exe tools\release\release.py
```

Scopes:

```powershell
.\.venv\Scripts\python.exe tools\release\release.py --scope cli
.\.venv\Scripts\python.exe tools\release\release.py --scope app
.\.venv\Scripts\python.exe tools\release\release.py --scope full
.\.venv\Scripts\python.exe tools\release\release.py --scope none --build-only --no-wsl
```

Build only:

```powershell
.\.venv\Scripts\python.exe tools\release\release.py --scope cli --build-only
```

Successful App and full builds advance the Flutter build number in
`apps/flutter/app/pubspec.yaml` after their assets are packaged. CLI-only builds do not
change it.

CLI architecture selection:

```powershell
.\.venv\Scripts\python.exe tools\release\release.py --scope cli --build-only --cli-arches host
.\.venv\Scripts\python.exe tools\release\release.py --scope cli --build-only --cli-arches all
```

On Windows, `--cli-arches all` builds:

```text
operit2-cli-windows-x86_64.zip
operit2-cli-windows-aarch64.zip
operit2-cli-linux-x86_64.tar.gz
operit2-cli-linux-aarch64.tar.gz
```

Apple release assets are built in GitHub Actions for normal release work. Use
the macOS workflow for macOS App and CLI archives, and the iOS workflow for the
unsigned iOS package.

```powershell
gh workflow run "macOS Flutter Build" -f products=all -f build_web_assets=false
gh workflow run "iOS Flutter Build" -f build_web_assets=false
```

Collaborators can still build the current host locally with one Python command.
Run it from the repository root. On Windows, use the project virtual environment.
On macOS and Linux, use the active Python 3 environment.

```powershell
.\.venv\Scripts\python.exe tools\build_scripts\build_local.py --products all --cli-arches host
```

```bash
python3 tools/build_scripts/build_local.py --products all --cli-arches host
python3 tools/build_scripts/build_local.py --products app --include-ios
```

Local Apple builds require Xcode, Rust, FVM, Node/npm, and Python 3 on the macOS
host. Apple outputs include:

```text
operit2-app-macos-aarch64.zip
operit2-app-ios-arm64.zip
operit2-cli-macos-x86_64.tar.gz
operit2-cli-macos-aarch64.tar.gz
```

The output directory is:

```text
tools/release/dist
```

Each CLI archive contains:

```text
operit2 or operit2.exe
install.sh or install.bat
uninstall.sh or uninstall.bat
README.txt
```

Skip WSL Linux packaging:

```powershell
.\.venv\Scripts\python.exe tools\release\release.py --scope cli --build-only --cli-arches all --no-wsl
```

## Publishing

Publish selected scope to GitHub Release:

```powershell
.\.venv\Scripts\python.exe tools\release\release.py --scope cli
```

Publish as draft:

```powershell
.\.venv\Scripts\python.exe tools\release\release.py --scope cli --draft
```

The release script reads versions from:

```text
apps/cli/Cargo.toml
core/crates/operit-runtime/Cargo.toml
apps/flutter/app/pubspec.yaml
```

Version, tag, build number, and updater asset rules are defined in:

```text
docs/release-versioning.md
```

## Flutter App

The release script builds app packages through Flutter. Local checks can be run from:

```powershell
cd apps\flutter\app
fvm install --skip-pub-get
fvm dart pub get --enforce-lockfile
```

Windows app release build:

```powershell
fvm flutter build windows --release --no-pub --build-name 2.0.0 --build-number 1
```

Android release build requires signing values in:

```text
tools/release/secrets/android-signing.properties
```

Keep the Android release keystore and properties outside git and back them up:

```text
tools/release/secrets/operit2-release.keystore
tools/release/secrets/android-signing.properties
```

## OpenHarmony Flutter App

OpenHarmony builds require the OpenHarmony Flutter SDK maintained at:

```text
https://gitcode.com/openharmony-sig/flutter_flutter.git
```

The Flutter app is pinned to the OpenHarmony `oh-3.41.9-dev` branch in
`apps/flutter/app/.fvmrc`. Keep this SDK selected for OpenHarmony development;
the standard Flutter SDK is a different toolchain and does not provide the
OpenHarmony target.

The user environment must provide:

- FVM `4.1.2` through the Pub cache `bin` directory on `PATH`.
- `dart` through a Flutter or Dart SDK `bin` directory on `PATH` so the FVM
  launcher can start.
- `OHOS_SDK_HOME` pointing to the OpenHarmony SDK root.
- The selected OpenHarmony SDK `toolchains` directory on `PATH` for `hdc`.

Restart the terminal or IDE after changing user environment variables so new
processes inherit them.

The Windows user environment used for local OpenHarmony builds should keep
these values stable:

```text
HOS_SDK_HOME=%USERPROFILE%\harmony-tools\harmonyos-sdk
OHOS_SDK_HOME=%USERPROFILE%\harmony-tools\harmonyos-sdk
DEVECO_SDK_HOME=%USERPROFILE%\harmony-tools\harmonyos-sdk
```

Add these directories to the user `PATH`:

```text
%USERPROFILE%\harmony-tools\commandline-tools-2.0.0.2\command-line-tools\bin
%USERPROFILE%\harmony-tools\ohcommandline-tools-2.0.0.2\oh-command-line-tools\bin
%LOCALAPPDATA%\Pub\Cache\bin
```

The project-local OpenHarmony properties file should point at the same SDK and
the FVM-selected Flutter SDK:

```text
apps/flutter/app/ohos/local.properties
hwsdk.dir=<user-home>\\harmony-tools\\harmonyos-sdk
flutter.sdk=<repo-root>\\apps\\flutter\\app\\.fvm\\flutter_sdk
```

The SDK root must contain a numeric API directory with all build components.
The currently validated layout is:

```text
%USERPROFILE%\harmony-tools\harmonyos-sdk\18\ets
%USERPROFILE%\harmony-tools\harmonyos-sdk\18\js
%USERPROFILE%\harmony-tools\harmonyos-sdk\18\native
%USERPROFILE%\harmony-tools\harmonyos-sdk\18\previewer
%USERPROFILE%\harmony-tools\harmonyos-sdk\18\toolchains
```

Do not point this project at `%USERPROFILE%\harmony-tools\openharmony-sdk\9`;
that older SDK does not match the current OpenHarmony API 18 project files.

The FVM-selected SDK must expose the `ohos` platform and the `hap` build
command. Verify the selected SDK and DevEco environment from the Flutter app
directory:

```powershell
cd apps\flutter\app
fvm install --skip-pub-get
fvm flutter doctor -v
fvm flutter config
```

`fvm flutter doctor -v` must report both Flutter and OpenHarmony toolchains.

Generate the OpenHarmony application module once with the selected SDK:

```powershell
cd apps\flutter\app
fvm flutter create --platforms ohos .
```

Build the shared Web Access bundle from the repository root:

```powershell
.\.venv\Scripts\python.exe tools\build_scripts\build_flutter_web_access.py
```

The remote access bundle is written to `apps/web_access/build/bundle` and
synchronized into the native Flutter assets at:

```text
apps/flutter/app/assets/web_access
```

The bundle includes a generated `web_access_version.json`; native launchers use
that version to reuse an already materialized Web Access directory.

Build the OpenHarmony HAP with the repository script. The script invokes the
same FVM-selected Flutter SDK and the OpenHarmony native toolchain:

The Rust toolchain uses the native OpenHarmony target:

```powershell
rustup target add aarch64-unknown-linux-ohos
```

Build the HAP through the repository build script from the repository root:

```powershell
.\.venv\Scripts\python.exe tools\build_scripts\build_flutter_ohos.py --enforce-lockfile
```

The build script stages the Rust bridge, copies the OpenHarmony Flutter engine
HAR files, patches the embedding HAR for the local API 18 SDK surface, clears
the unpacked `ohpm` embedding package, runs `flutter build hap`, and signs the
unsigned HAP with the project OpenHarmony release signing material in:

```text
tools/release/secrets/ohos-signing/ohos-signing.properties
```

Keep these OpenHarmony release signing files outside git and back them up:

```text
tools/release/secrets/ohos-signing/operit-ohos-release-app.p12
tools/release/secrets/ohos-signing/operit-ohos-release-app.cer
tools/release/secrets/ohos-signing/operit-ohos-release-profile.p12
tools/release/secrets/ohos-signing/operit-ohos-release-profile.cer
tools/release/secrets/ohos-signing/ohos-signing.properties
```

The generated per-build profile files are written under:

```text
apps/flutter/app/ohos/signing
```

That directory is ignored by git and should be treated as local build output.

The signed build output is copied to:

```text
tools/release/dist/operit2-app-ohos-arm64.hap
```

If the Flutter command reports `Can't load Kernel binary`, clear stale FVM or
Flutter snapshot state for this app, run `fvm install --skip-pub-get` from
`apps/flutter/app`, and run the OpenHarmony build script again from the
repository root.

## Useful Cleanup

Remove release output:

```powershell
Remove-Item -Recurse -Force tools\release\dist, tools\release\work
```

Remove Rust build output for the CLI:

```powershell
Remove-Item -Recurse -Force apps\cli\target
```

Use cleanup commands only for build artifacts.
