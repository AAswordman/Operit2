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

Fedora WSL Linux CLI release builds use static musl targets:

```bash
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-musl
sudo dnf install -y musl-gcc musl-devel musl-libc-static
```

The aarch64 Linux CLI release also requires an `aarch64-linux-musl-gcc` cross C compiler on PATH.

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

The signed build output is copied to:

```text
tools/release/dist/operit2-app-ohos-arm64.hap
```

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
