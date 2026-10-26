# V86 Runtime Suite

This builder produces a 32-bit Buildroot V86 runtime suite for the Web host.
The kernel supplies BusyBox commands; the compressed initramfs supplies `node`
and `python3` plus their runtime libraries. It also contains fixed offline
packages: Node has `lodash`, `dayjs`, `zod`, and `uuid`; Python has `requests`,
`rich`, `click`, `packaging`, and `python-dateutil` with their pure-Python
dependencies. The guest intentionally has no `npm`, `npx`, `pip`, or `pip3`.
The serial agent transfers workspace content before starting the requested
interpreter process. iOS uses iSH with an Alpine rootfs, not this V86 suite.

Run it through Fedora WSL:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && ./tools/web-runtime/build_v86_runtime_wsl.sh'
```

The Fedora image needs a native i686 toolchain and ELF dependency resolver:

```bash
sudo dnf install -y gcc gcc-c++ glibc-devel.i686 libstdc++-devel.i686 ncurses-devel.i686 pax-utils patchelf cpio unzip
```

The command writes the kernel, compressed initramfs, and hash manifest to
`apps/flutter/app/web/runtime/generated/v86/`. Publish a verified release to the immutable
R2 location consumed by the Web host:

```powershell
.venv\Scripts\python.exe tools\web-runtime\publish_v86_runtime_r2.py
```

This requires `CLOUDFLARE_API_TOKEN` with write access to
`operit-model-assets`. The browser reads the versioned release through
`https://models.operit.app`; immutable response caching prevents another image
transfer while the published runtime URL is unchanged.

From Fedora WSL, use the token stored by `assistance_web` without exposing it
in the shell command:

```powershell
wsl -d FedoraLinux-43 -u root -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && OPERIT_CLOUDFLARE_ENV_FILE=/mnt/d/Code/prog/assistance_web/.env.local bash tools/web-runtime/publish_v86_runtime_r2_fedora.sh'
```

The builder stores the verified Buildroot kernel, Alpine Node rootfs, Python source,
and compiled i686 Python installation in
`~/.cache/operit-v86-runtime/buildroot-suite/`. Repeated builds reuse those
artifacts instead of downloading or compiling the runtime again. To deliberately
refresh the suite sources, run:

```powershell
wsl -d FedoraLinux-43 -- bash -lc 'cd /mnt/d/Code/prog/assistance2 && OPERIT_RUNTIME_REFRESH=1 ./tools/web-runtime/build_v86_runtime_wsl.sh'
```

## Node.js 24

Node and its musl dependencies come from Alpine v3.23 x86. They are staged
separately from the glibc-linked Python runtime; the Node ELF has a private
RPATH rather than a global `LD_LIBRARY_PATH`. The builder checks the installed
Node major before packaging and does not fall back to the old Node 20 image.

After building, run `node tools/web-runtime/v86_runtime_smoke.mjs` before
publishing. The smoke test covers Node 24, a real `node:sqlite` query, spawning
another Node process, and spawning the glibc Python runtime. The published Web
runtime does not change until the new image is built, tested, and published.
