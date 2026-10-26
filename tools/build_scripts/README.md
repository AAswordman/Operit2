# Operit2 build scripts

Local builds and GitHub Actions use the same Python entry points. Scripts are
named by their product, platform, or operation; there are no separate
`_action.py` / `_local.py` platform wrappers.

Build outputs and transient logs belong under `tools/release/work` and
`tools/release/dist`. Private credentials belong under `tools/release/secrets`.
These ignored directories are not used to store reusable build scripts.

Release package filenames use
`operit2-<product>-<platform>-<architecture>-<release-version>.<extension>`.
`release_artifacts.py` validates the shared App runtime and CLI version and keeps
prerelease identifiers and compound extensions intact. App packages for macOS
use `universal` and `.zip`; macOS CLI packages use their target architecture and
`.tar.gz`. Build and verification scripts must not write separate `.sha256`
files or `SHA256SUMS` lists for release packages. Verification results may
include hashes in the existing build manifest.

## Entry points

- `build_flutter_<platform>.py`: Flutter builds for Android, iOS, Linux, macOS,
  OpenHarmony, and Windows.
- `build_cli_<platform>.py`: CLI builds for Linux, macOS, and Windows.
- `build_cli_current.py`: CLI build for the current host platform.
- `build_local.py`: existing current-host App/CLI/ESP32 dispatcher.
- `build_apple_release.py`: Apple release asset dispatcher.
- `build_esp32.py`: ESP32 firmware builder.
- `check_build_environment.py`: shared local/CI environment checker.
- `common.py`, `cli_common.py`, and `appstore_connect_common.py`: shared helpers.

## Generated plugin assets

Every App/CLI build embeds the ToolPkg assets generated from `plugins/packages/`
into `core/crates/runtime/application/assets/plugins/{buildin,external}`.
`prepare_plugin_assets()` in `common.py` runs
`plugins/tools/sync_plugin_packages.py --source runtime --no-hot-reload`; the
CLI entry points call it before `cargo build`, and Flutter builds trigger it
from their native hooks. `verify_cli_plugin_assets()` rejects a packaged CLI
whose binary omits any generated asset name. Do not commit or reuse stale
asset directories across machines.

## OpenHarmony native libraries

The HAP build uses the API 18 native SDK configured in the OHOS project's
`local.properties` and a current `ld.lld` on `PATH`. V8's Chromium compiler emits
ELF CREL relocations, which the SDK's bundled LLD 15 cannot read. The final Rust
link uses the selected LLD with the SDK sysroot and compiler builtins, and rejects
undefined symbols.

Provide a V8 archive and its matching generated Rust bindings for
the selected target (`aarch64-unknown-linux-ohos` or
`x86_64-unknown-linux-ohos`) through `RUSTY_V8_ARCHIVE` and
`RUSTY_V8_SRC_BINDING_PATH`. Set `RUSTY_V8_ARCHIVE_SHA256` to the archive's SHA-256.
Both artifacts must come from the locked V8 version, built against the OHOS
sysroot; Android and GNU/Linux artifacts have a different ABI.

Local native source caches can be supplied through `OPERIT_LLAMA_SOURCE`,
`OPERIT_MNN_SOURCE`, and `OPERIT_KLEIDIAI_SOURCE` (KleidiAI 1.16.0 for MNN 3.6.1).
Use CMake-style forward slashes in these paths. The OHOS engine uses the SDK's
C++ headers, aligns their namespace with the SDK's `__n1` ABI, and uses
`sched_setaffinity` for inference thread affinity. Its static C++ runtime stays
private behind the three exported inference C functions. Packaging includes both `liboperit_flutter_bridge.so` and
`liboperit_local_llm.so`.

The entry module enables `compressNativeLibs` so the HAP stores shared libraries
with compression. Before signing, terminal packaging keeps `qemu-harmonix-aarch64`,
its transitive `DT_NEEDED` libraries and the HNP metadata in both declared packages.
The dependency check excludes the OS-provided `libc.so` and rejects missing or
bundled libraries whose ELF architecture differs from the selected target. Full-system emulators, x86 guest programs and libraries
unreferenced by QEMU-vroot are excluded from the HAP. The offline Alpine tool
repository remains bundled for terminal, OCR and PDF capabilities.

Select the build architecture from the repository root:

```powershell
# ARM64 devices
.venv\Scripts\python.exe tools/build_scripts/build_flutter_ohos.py --target-platform ohos-arm64 --enforce-lockfile

# x86_64 emulator
.venv\Scripts\python.exe tools/build_scripts/build_flutter_ohos.py --target-platform ohos-x64 --enforce-lockfile
```

ARM64 remains the default target. Each target selects its Rust triple, Flutter
engine HAR, CMake ABI, packaged native libraries, HNP directory and output filename.
The Hvigor plugin reads Flutter's `TARGET_PLATFORM` parameter and sets native ABI
filters in memory. Both engine dependencies stay declared in the manifests;
switching targets does not rewrite architecture settings or remove cached libraries.
Before signing, the build checks that all required native libraries are present and
that every packaged ELF matches the selected ABI. The same selection works with
`fvm flutter build hap --target-platform ohos-x64` from `apps/flutter/app` after
staging the required Rust libraries, HARs and terminal packages.

The x86_64 host HNPs still
run the shared ARM64 Alpine guest through `qemu-harmonix-aarch64`. Supply both
HNP files under `apps/flutter/app/ohos/entry/hnp/x86_64/` for emulator builds.

## Apple release helpers

```bash
# Build Flutter artifacts
python3 tools/build_scripts/build_flutter_ios.py --build-name 2.0.0 --build-number 15
python3 tools/build_scripts/build_flutter_macos.py --build-name 2.0.0 --build-number 15

# Sign the macOS app and create a .pkg
python3 tools/build_scripts/sign_macos.py

# Validate or upload an .ipa/.pkg
python3 tools/build_scripts/validate_appstore.py path/to/App.ipa
python3 tools/build_scripts/upload_appstore.py path/to/App.ipa
```

The App Store Connect scripts read the ignored file
`tools/release/secrets/appstoreconnect/appstoreconnect.env`. No credentials are
stored in tracked build scripts.

## iOS V8 runtime

The iOS Rust bridge runs `prepare_ios_v8.py` before compiling Rust. It builds the
V8 version pinned in the bridge's `Cargo.lock` from source and supplies the
resulting static library and generated Rust bindings together. The embedded
startup snapshot comes from that same build. This avoids the incompatible
snapshot in the previously used precompiled iOS archive.

The iOS build uses compile-time jitless mode with Sparkplug, Maglev, TurboFan and
WebAssembly disabled. It also disables the cppgc caged heap, avoiding its large
aligned virtual-address reservation during startup without requiring Extended
Virtual Addressing entitlements. Intl remains enabled. Temporal is disabled
because the published source crate omits its Rust vendor dependencies; current
runtime plugins do not use Temporal. These settings belong to the iOS build
layer; the common JavaScript scheduler has no Apple-specific runtime branches.

On macOS, install the source-build prerequisites with
`brew install llvm@23 lld@23`. The helper combines Homebrew's LLVM/libclang with
Xcode's iOS compiler runtimes in a local toolchain overlay. It downloads missing
ICU data from the exact revision in V8's DEPS. The first build for each iOS
architecture takes substantially longer; subsequent builds reuse
`tools/release/work/ios-v8/<v8-version>/<rust-target>/`. Library, bindings and GN
arguments are checked against a configuration manifest and SHA-256 hashes before
reuse. A preparation failure stops the build instead of downloading the old
archive. CI caches the complete artifact set and uses the same helper.

For a manual build, after installing the Rust target:

```bash
python3 tools/build_scripts/prepare_ios_v8.py \
  --target aarch64-apple-ios --env-file tools/release/work/ios-v8/device.env
```

Debug uses a regular executable with dead-code stripping and disables exporting
all global symbols, so unused WebAssembly wrappers are removed. Swift-referenced
Rust bridge entry points remain linked. The iSH runtime also namespaces its
kernel printf/scanf functions so host V8 calls keep using Darwin libc. See
`tools/ios-runtime/ish/README.md` for that native regression. Build/cache checks:

```bash
python3 -m unittest discover -s tools/tests -p test_ios_v8_build.py -v
python3 -m unittest discover -s tools/tests -p test_apple_plugin_build.py -v
```

## Android V8 runtime

rusty_v8 publishes no prebuilt static libraries for Android targets, so the
crate's default GitHub download fails with a 404. The Android Gradle build
therefore runs `prepare_android_v8.py` before compiling Rust. Like the iOS
helper, it builds the V8 version pinned in the bridge's `Cargo.lock` from
source and writes `RUSTY_V8_ARCHIVE`, `RUSTY_V8_ARCHIVE_SHA256` and
`RUSTY_V8_SRC_BINDING_PATH` into an env file; the Cargo compile tasks apply it.
Library, bindings and GN arguments are verified against a configuration
manifest and SHA-256 hashes before reuse under
`tools/release/work/android-v8/<v8-version>/<rust-target>/`.

Unlike iOS, Android keeps the JIT with WebAssembly enabled. The sandbox,
pointer compression and cppgc caged heap stay disabled to keep both mobile
builds on the same memory layout, and Temporal stays disabled because the
published source crate omits its Rust vendor dependencies. The NDK API level
defaults to 24 (matching the app's minSdkVersion) and can be raised with
`OPERIT_ANDROID_V8_API_LEVEL`; the value is part of the cache identity.

An NDK r23+ installation is required and detected from `OPERIT_ANDROID_V8_NDK`,
`ANDROID_NDK_HOME`, `$ANDROID_HOME/ndk` or `~/Android/ndk` (newest match wins).
On Linux, the helper falls back to downloading the NDK that rusty_v8's build
script pins. The detected NDK is exposed inside the crate source where both the
build script's guard and the GN `android_ndk_root` default expect it, so no
extra download happens even when the crate would fetch a Linux-only archive.
Linux needs libclang 21.1+ and `ld.lld` (`apt install llvm-21-dev lld`); the
host compiler itself is downloaded from Chromium's pinned build so the
compiler-runtime layout matches. macOS uses Homebrew LLVM (`brew install
llvm@23 lld@23`); on Windows, set `OPERIT_LIBCLANG_DIR`
to a directory containing `libclang.dll` (the Gradle build already does).
The helper also provisions the pinned `gn` and `ninja` under the cache and
passes them via `GN`/`NINJA`, because the crate's own downloader opens raw
connections that bypass proxy environment settings. `armv7-linux-androideabi`
is not supported: rusty_v8's build script maps only
the x86_64 and aarch64 Android targets. Build/cache checks:

```bash
python3 -m unittest discover -s tools/tests -p test_android_v8_build.py -v
```

For a manual build after installing the Rust target:

```bash
python3 tools/build_scripts/prepare_android_v8.py \
  --target aarch64-linux-android \
  --env-file tools/release/work/android-v8/aarch64-linux-android.env
```

## Local CLI development and macOS Keychain authorization

Use the local runner to build and restore the CLI's configured signing identity
before running it:

```bash
python3 tools/build_scripts/run_cli.py -- cli character list --json
python3 tools/build_scripts/run_cli.py --release -- cli chat list --json
# Sign/run an existing binary without rebuilding:
python3 tools/build_scripts/run_cli.py --no-build -- cli character list --json
```

On macOS, choose an existing real signing identity via
`OPERIT_MACOS_SIGNING_IDENTITY`, `run_cli.py --identity`, or the Git-ignored
`tools/build_scripts/cli_macos_signing.local` file:

```json
{"identity": "<certificate SHA-1 or certificate name>"}
```

Both the local runner and macOS CLI packaging use the fixed identifier
`com.operit.cli`. Direct `cargo build` does not run this post-build signing step
and can replace a trusted artifact with a new ad-hoc-signed binary. Re-run the
local runner after such a build. Approve the new signed CLI's Keychain access
with **Always Allow** only after checking the requesting program.

The helper does not export certificates/private keys, store passwords, change
Keychain ACLs, or disable encrypted preferences. An explicitly configured
signing failure stops the build rather than falling back to ad-hoc signing.
Local signing is not public-release notarization; public macOS distribution
still needs an appropriate distribution identity and its release workflow.

The role-card CLI acceptance test uses a disposable profile. Its optional AI
fixture runs on loopback and binds every functional model (including title
and memory generation) to that endpoint:

```bash
python3 tools/tests/character_cards_cli_smoke.py --mock-ai
```

## Local Flutter macOS signing

Flutter's direct `build macos` and `run -d macos` commands can use an existing
stable certificate through the optional, Git-ignored file
`apps/flutter/app/macos/Runner/Configs/Signing.local.xcconfig`:

```xcconfig
CODE_SIGN_IDENTITY = <existing certificate SHA-1 or certificate name>
OPERIT_CODE_SIGN_STYLE = Manual
DEVELOPMENT_TEAM = <team ID associated with that certificate>
```

This keeps the app's own bundle identifier, sandbox and encryption entitlements.
It does not export private keys or alter Keychain ACLs. Without this local file,
the project's existing ad-hoc signing behavior is retained. A previously
unauthorized certificate may still require the user to approve its first
Keychain access; this configuration cannot bypass that permission.
