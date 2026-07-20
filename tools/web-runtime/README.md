# V86 Runtime Suite

This builder produces a 32-bit Buildroot V86 runtime suite for the Web host.
The kernel supplies BusyBox commands; the compressed initramfs supplies `node`
and `python3` plus their runtime libraries. It also contains fixed offline
packages: Node has `lodash`, `dayjs`, `zod`, and `uuid`; Python has `requests`,
`rich`, `click`, `packaging`, and `python-dateutil` with their pure-Python
dependencies. The guest intentionally has no `npm`, `npx`, `pip`, or `pip3`.
The serial agent transfers workspace content before starting the requested
interpreter process. iOS uses app-linked native frameworks and does not use V86
or WebKit.

Run it through Fedora WSL:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ./tools/web-runtime/build_v86_runtime_wsl.sh'
```

The Fedora image needs a native i686 toolchain and ELF dependency resolver:

```bash
sudo dnf install -y gcc gcc-c++ glibc-devel.i686 libstdc++-devel.i686 ncurses-devel.i686 pax-utils cpio unzip
```

The command writes the kernel, compressed initramfs, and hash manifest to
`apps/web_access/v86/runtime/`. The Web host consumes that manifest and
guest image.

The builder stores the verified Buildroot kernel, Node archive, Python source,
and compiled i686 Python installation in
`~/.cache/operit-v86-runtime/buildroot-suite/`. Repeated builds reuse those
artifacts instead of downloading or compiling the runtime again. To deliberately
refresh the suite sources, run:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && OPERIT_RUNTIME_REFRESH=1 ./tools/web-runtime/build_v86_runtime_wsl.sh'
```
