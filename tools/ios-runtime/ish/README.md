# iSH Runtime Tooling

This directory owns the iOS iSH runtime inputs and build steps.

- `fetch_sources.py` downloads verified ZIP archives for iSH and its required
  upstream dependencies into `downloads/`, then extracts them into `sources/`.
- `build_alpine_rootfs_linux.sh` produces the Alpine v3.23 x86 rootfs on Linux
  with `bash`, `python3`, `py3-pip`, `nodejs` (Node.js 24), `npm`, and
  `ca-certificates`.
- `build_ish_ios.sh` builds the iSH static targets used by the Flutter iOS
  Runner. It requires the rootfs staged by the Linux rootfs build.

`downloads/`, `sources/`, and `build/` are build-owned directories and are
ignored by Git. The Runner exposes one terminal type: `shell`, backed by the
iSH Alpine Linux environment with Python and Node.js installed.

iSH is GPLv3 with its `LICENSE.IOS` App Store exception. Distributing an iOS
build that includes this runtime requires providing the corresponding iSH
source and license text to users.

## Reusing local iOS build products

`build_ish_ios.sh` records completed builds in
`apps/flutter/app/apple/ish-build/<configuration>-<platform>/.operit-ish-build-complete`.
Before fetching or patching sources, it checks that the required headers, all
static libraries, requested architecture slices, and the managed mount symbol
are available. A warm cache avoids the nested iSH Xcode build during Flutter or
VS Code launches. Runner also declares the scripts, patches, rootfs, libraries,
and completion marker as build-phase inputs/outputs for incremental builds.

Cache entries must match the configuration, SDK version, platform, and exact
source signature in **all configurations**, including Debug. Changes to scripts
or patches invalidate the cache so native crash fixes cannot be silently skipped.
Unchanged warm launches still reuse the verified libraries. To rebuild explicitly,
run from the repository root (with the rootfs already staged):

```bash
OPERIT_ISH_FORCE_REBUILD=1 bash tools/ios-runtime/ish/build_ish_ios.sh \
  Debug iphonesimulator iphonesimulator arm64
```

Use the appropriate configuration, SDK, platform, and architectures for the
intended target. Forcing a build invokes the native toolchain and is not the
normal cached launch path. The shell cache-decision tests use isolated fixtures
and do not download sources or invoke Xcode:

```bash
python3 -m unittest discover -s tools/tests -p 'test_ish_build_cache.py' -v
```

## Native crash regressions

`0001-operit-managed-runtime-mount.patch` supplies `do_mount()` with a full,
zeroed, writable Linux page. Linux writes the final byte of this buffer before
parsing the hostfs options; a host path string must never be passed directly as
that buffer. The bridge rejects paths that cannot fit and frees the temporary
page on both success and failure. Hostfs copies the path into its own storage.

`0004-emulator-tlb-task-migration.patch` keeps the emulator TLB per simulated
CPU instead of Darwin thread-local storage, which can retain stale addresses
when Linux tasks migrate between host pthreads. It remains a required build input.

`0005-namespace-darwin-kernel-formatters.patch` gives the embedded kernel's
printf/scanf implementations private names. Linux's format language differs from
Darwin libc (in particular, it does not support floating point). Exporting them
under libc names can route V8 GC tracing and other host formatting through the
kernel and crash the application. The native formatter regression links the
actual namespace macros with separate host and kernel translation units and
checks both call paths.

Run the runtime and cache regressions with:

```bash
python3 -m unittest discover -s tools/tests -p 'test_ish*.py' -v
```

The native mount tests compile the actual implementation extracted from the
patch with Clang AddressSanitizer and UndefinedBehaviorSanitizer. They cover
read-only paths, page-length boundaries, repeated/conflicting mounts, every
allocation failure, and mount failure/retry cleanup. Patch application is also
checked against locally cached, pinned upstream archives, without downloading
sources. Edit the tracked patches rather than generated files under `sources/`.

## Node.js version contract

The rootfs builder uses the Alpine v3.23 repositories, constrains `nodejs` to
major 24, and verifies the installed APK database before packaging. Changing CI's
host Node does not update `ish-root.tar.gz`: rebuild the rootfs on Linux and
include that artifact in the iOS build.

Installed apps currently reuse the persistent iSH fakefs root, so replacing the
bundled archive does not upgrade an existing installation. That root may contain
user-created files; a future migration must preserve them rather than deleting
the directory as part of an upgrade.
