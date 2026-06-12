# Android Runtime Tooling

This directory prepares the Android phone runtime binaries:

- BusyBox 1.38.0
- Termux PRoot v5.1.107.78
- talloc 2.4.3 for PRoot
- Bash 5.3 source archive
- Android NDK r29-beta4 for WSL builds
- Alpine latest-stable minirootfs with bash, python3, nodejs, npm, uv, pnpm, and ca-certificates

Build instructions are in [BUILDING.md](BUILDING.md).

PRoot edits live in:

```text
tools/android-runtime/sources/termux-proot-operit
```

`build_android_tools_wsl.sh` keeps
`patches/termux-proot-operit-android.patch` and that editable source tree in
sync before compiling. When the patch exists and the editable source tree has
not been created, the script creates the editable tree from clean Termux PRoot
and applies the patch. When both exist, the script regenerates the patch from
the editable tree. If the patch does not exist yet, the script creates the
editable tree from clean Termux PRoot and then writes a new patch. After the
editable tree exists, change PRoot by editing that tree, then run the normal
PRoot build.

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
