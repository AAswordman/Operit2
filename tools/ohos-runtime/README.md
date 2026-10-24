# OpenHarmony Vroot Runtime Tooling

This directory produces the OpenHarmony PC terminal runtime:

- `qemu-harmonix-aarch64`, the QEMU-vroot executable built from Harmonix;
- an HNP package containing the executable and its OpenHarmony libraries;
- the Alpine 3.23.6 aarch64 base plus an offline APK repository and checksum.
  The owner installs `bash`, `python3`, `nodejs` (Node.js 24), `npm`, `uv`, `pnpm`,
  certificates, curl, Git, SSH, Tesseract language data, Poppler and fonts through
  apk. Package upgrades preserve the user's rootfs and installed package database.

Raw assets copy in bounded chunks when their checksum changes. Extraction and
package installation occur on first terminal use; successful installs remove
staging and the extracted APK repository. Active network DNS comes from the OHOS
owner. OCR and PDF preview share this terminal environment.

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
wsl -d FedoraLinux-43 -u root -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ./tools/ohos-runtime/build_alpine_rootfs_linux.sh'
```

`build_vroot_hnp_linux.sh` checks out Harmonix commit
`2d671a26d2f049af12c3583da6403b77c3b66023` and uses its published QEMU-vroot
patch series. Harmonix is MIT-licensed; the bundled QEMU source remains GPL-2.0-or-later.

The rootfs builder requires root so archive ownership and executable modes are
correct. Run the ARM64 integration checks with qemu-user installed:

```powershell
wsl -d FedoraLinux-43 -u root -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && bash tools/ohos-runtime/test_rootfs_linux.sh'
```

The checks cover offline installation, user package/data preservation, runtime
startup, PDF page order/scale, and actual OCR output. They do not emulate OHOS
HNP installation or its process sandbox; those require a connected device.
