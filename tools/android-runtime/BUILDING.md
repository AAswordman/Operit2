# Building Android Runtime

This document builds the phone runtime artifacts from source on Fedora WSL.
Docker is not used.

## Inputs

- BusyBox 1.38.0
- Termux PRoot v5.1.107.78
- talloc 2.4.3
- Bash 5.2.37
- Android NDK r29-beta4
- Alpine v3.23 minirootfs

The Operit PRoot changes are stored in:

```text
tools/android-runtime/patches/termux-proot-operit-android.patch
```

The editable Operit PRoot source tree is:

```text
tools/android-runtime/sources/termux-proot-operit
```

`build_android_tools_wsl.sh` keeps the editable tree and patch synchronized.
When the patch exists and the editable tree has not been created, the script
creates the editable tree from clean Termux PRoot and applies the patch. When
both exist, the script regenerates the patch from the editable tree. If the
patch does not exist yet, the script creates the editable tree from clean Termux
PRoot and then writes a new patch. The PRoot build then copies the editable
tree into the WSL build directory and compiles that copy.

## Exact Source Pins

The Android runtime build was validated on September 27, 2026. The fetch script
records the upstream URL and SHA-256 for each fixed source archive:

| Component | Version | Upstream source | Fetch definition |
| --- | --- | --- | --- |
| BusyBox | 1.38.0 | `https://busybox.net/downloads/busybox-1.38.0.tar.bz2` | `fetch_sources.ps1` |
| Termux PRoot | v5.1.107.78 | `https://github.com/termux/proot` tag `v5.1.107.78` | `fetch_sources.ps1` |
| talloc | 2.4.3 | `https://www.samba.org/ftp/talloc/talloc-2.4.3.tar.gz` | `fetch_sources.ps1` |
| Bash | 5.2.37 | `https://ftp.gnu.org/gnu/bash/bash-5.2.37.tar.gz` | `fetch_sources.ps1` |
| Android NDK | r29-beta4 | `https://dl.google.com/android/repository/android-ndk-r29-beta4-linux.zip` | `fetch_ndk_wsl.sh` |
| Fedora WSL | 43 | FedoraLinux-43 distribution | commands in this document |

`fetch_sources.ps1` is the source of truth for the fixed archive checksums.
The Alpine rootfs script uses the `v3.23` branch at
`https://dl-cdn.alpinelinux.org/alpine` and reads the release version, archive
name, and SHA-256 from that branch's `latest-releases.yaml` for each target
architecture. Record the resolved Alpine release in build logs when producing
an artifact.

The dependency lockfiles are checked in with the runtime sources. Do not
regenerate them with a different package mirror or toolchain and commit the
result as part of an unrelated runtime change.
## One-Shot Build

Run from Windows PowerShell:

```powershell
.\tools\android-runtime\fetch_sources.ps1
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ./tools/android-runtime/fetch_ndk_wsl.sh'
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ANDROID_NDK_HOME="$HOME/.cache/operit-android-runtime/android-ndk-r29-beta4" ./tools/android-runtime/build_android_tools_wsl.sh'
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ./tools/android-runtime/build_alpine_rootfs_wsl.sh'
```

## PRoot-Only Build

Run this after editing `tools/android-runtime/sources/termux-proot-operit`:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ANDROID_NDK_HOME="$HOME/.cache/operit-android-runtime/android-ndk-r29-beta4" OPERIT_ANDROID_RUNTIME_COMPONENTS="proot" ./tools/android-runtime/build_android_tools_wsl.sh'
```

To synchronize the editable tree and patch without compiling:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ANDROID_NDK_HOME="$HOME/.cache/operit-android-runtime/android-ndk-r29-beta4" OPERIT_ANDROID_RUNTIME_COMPONENTS="proot" OPERIT_ANDROID_RUNTIME_SYNC_ONLY=1 ./tools/android-runtime/build_android_tools_wsl.sh'
```

Limit ABI output during local testing:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ANDROID_NDK_HOME="$HOME/.cache/operit-android-runtime/android-ndk-r29-beta4" OPERIT_ANDROID_RUNTIME_ABIS="arm64-v8a" OPERIT_ANDROID_RUNTIME_COMPONENTS="proot" ./tools/android-runtime/build_android_tools_wsl.sh'
```

## Output Paths

Intermediate files stay inside Fedora WSL:

```text
~/.cache/operit-android-runtime/build/assistance2
```

Android app artifacts are copied into:

```text
apps/flutter/app/android/app/src/main/jniLibs/<abi>/libbash.so
apps/flutter/app/android/app/src/main/jniLibs/<abi>/liboperit_busybox.so
apps/flutter/app/android/app/src/main/jniLibs/<abi>/liboperit_flutter_bridge.so
apps/flutter/app/android/app/src/main/jniLibs/<abi>/liboperit_loader.so
apps/flutter/app/android/app/src/main/jniLibs/<abi>/liboperit_proot.so
apps/flutter/app/android/app/src/main/assets/android-runtime/<abi>/rootfs.tar.gz.bin
apps/flutter/app/android/app/src/main/assets/android-runtime/<abi>/rootfs.tar.gz.bin.sha256
```

These app artifacts are generated files and are ignored by git.

## Patch Maintenance

Edit PRoot under:

```text
tools/android-runtime/sources/termux-proot-operit
```

Then run the PRoot build command above. The build script regenerates:

```text
tools/android-runtime/patches/termux-proot-operit-android.patch
```

To verify the patch manually:

```bash
patch -d proot-5.1.107.78 -p1 --batch < tools/android-runtime/patches/termux-proot-operit-android.patch
```
