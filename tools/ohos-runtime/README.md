# OpenHarmony Vroot Runtime Tooling

This directory produces the OpenHarmony PC terminal runtime:

- `qemu-harmonix-aarch64`, the QEMU-vroot executable built from Harmonix;
- an HNP package containing the executable and its OpenHarmony libraries;
- the Alpine 3.22.1 aarch64 rootfs and a SHA-256 checksum. It includes
  `bash`, `python3`, `nodejs`, `npm`, `uv`, `pnpm`, and `ca-certificates`, so
  tools and plugins have their standard command-line runtimes immediately.

The generated HNP files are copied to:

```text
apps/flutter/app/ohos/entry/hnp/arm64-v8a/
```

The generated Alpine files are copied to:

```text
apps/flutter/app/ohos/entry/src/main/resources/rawfile/ohos-vroot/
```

Both locations are build outputs and are ignored by Git.

Build from Fedora WSL after the OpenHarmony SDK is installed on Windows:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && OHOS_SDK_HOME=/mnt/c/Users/12809/harmony-tools/harmonyos-sdk/18 ./tools/ohos-runtime/build_vroot_hnp_linux.sh'
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ./tools/ohos-runtime/build_alpine_rootfs_linux.sh'
```

`build_vroot_hnp_linux.sh` checks out Harmonix commit
`2d671a26d2f049af12c3583da6403b77c3b66023` and uses its published QEMU-vroot
patch series. Harmonix is MIT-licensed; the bundled QEMU source remains GPL-2.0-or-later.
