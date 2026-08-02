# iSH Runtime Tooling

This directory owns the iOS iSH runtime inputs and build steps.

- `fetch_sources.py` downloads verified ZIP archives for iSH and its required
  upstream dependencies into `downloads/`, then extracts them into `sources/`.
- `build_alpine_rootfs_linux.sh` produces the x86 Alpine rootfs on Linux with
  `bash`, `python3`, `py3-pip`, `nodejs`, `npm`, and `ca-certificates`.
- `build_ish_ios.sh` builds the iSH static targets used by the Flutter iOS
  Runner. It requires the rootfs staged by the Linux rootfs build.

`downloads/`, `sources/`, and `build/` are build-owned directories and are
ignored by Git. The Runner exposes one terminal type: `shell`, backed by the
iSH Alpine Linux environment with Python and Node.js installed.

iSH is GPLv3 with its `LICENSE.IOS` App Store exception. Distributing an iOS
build that includes this runtime requires providing the corresponding iSH
source and license text to users.
