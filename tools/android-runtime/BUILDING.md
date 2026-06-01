# Building Android Runtime

This document builds the phone runtime artifacts from source on Fedora WSL.
Docker is not used.

## Inputs

- BusyBox 1.38.0
- PRoot 5.4.0
- talloc 2.4.3
- Bash 5.3
- Android NDK r29-beta4
- Alpine latest-stable minirootfs

The Operit PRoot changes are stored in:

```text
tools/android-runtime/patches/proot-5.4.0-operit-android.patch
```

`build_android_tools_wsl.sh` copies the clean PRoot 5.4.0 source tree into the
WSL build directory, applies that patch, and compiles the patched copy.

## One-Shot Build

Run from Windows PowerShell:

```powershell
.\tools\android-runtime\fetch_sources.ps1
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ./tools/android-runtime/fetch_ndk_wsl.sh'
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ANDROID_NDK_HOME="$HOME/.cache/operit-android-runtime/android-ndk-r29-beta4" ./tools/android-runtime/build_android_tools_wsl.sh'
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ./tools/android-runtime/build_alpine_rootfs_wsl.sh'
```

## PRoot-Only Build

Run this after editing `tools/android-runtime/patches/proot-5.4.0-operit-android.patch`:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ANDROID_NDK_HOME="$HOME/.cache/operit-android-runtime/android-ndk-r29-beta4" OPERIT_ANDROID_RUNTIME_COMPONENTS="proot" ./tools/android-runtime/build_android_tools_wsl.sh'
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

Regenerate the PRoot patch by comparing clean upstream PRoot 5.4.0 against the
Operit-modified PRoot tree:

```bash
diff -ruN proot-5.4.0-clean proot-5.4.0-operit \
  --exclude=.git \
  > tools/android-runtime/patches/proot-5.4.0-operit-android.patch
```

The patch must apply to a clean PRoot 5.4.0 tree with:

```bash
patch -d proot-5.4.0 -p1 --batch < tools/android-runtime/patches/proot-5.4.0-operit-android.patch
```
