# Android Runtime Tooling

This directory prepares the Android phone runtime binaries:

- BusyBox 1.38.0
- PRoot 5.4.0
- talloc 2.4.3 for PRoot
- Bash 5.3 source archive
- Android NDK r29-beta4 for WSL builds
- Alpine latest-stable minirootfs with bash, python3, nodejs, npm, uv, pnpm, and ca-certificates

Build instructions are in [BUILDING.md](BUILDING.md).

`build_android_tools_wsl.sh` applies `patches/proot-5.4.0-operit-android.patch`
to a fresh PRoot 5.4.0 source copy before compiling. That patch contains the
Android arm64 syscall and 16 KB page compatibility changes used by Operit.

Intermediate build files are written under Fedora WSL:

```text
~/.cache/operit-android-runtime/build/assistance2
```

Compiled binaries are copied to:

```text
apps/flutter/app/android/app/src/main/jniLibs/<abi>/liboperit_busybox.so
apps/flutter/app/android/app/src/main/jniLibs/<abi>/liboperit_proot.so
apps/flutter/app/android/app/src/main/jniLibs/<abi>/liboperit_loader.so
apps/flutter/app/android/app/src/main/assets/android-runtime/<abi>/rootfs.tar.gz.bin
apps/flutter/app/android/app/src/main/assets/android-runtime/<abi>/rootfs.tar.gz.bin.sha256
```
