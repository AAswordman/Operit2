# iOS Runtime Tooling

The active terminal backend is the independent AArch64 Linux runtime in
[`hosts/ios/linux-runtime`](../../hosts/ios/linux-runtime/README.md).
[`arm64/`](arm64/README.md) owns its builds, pinned Alpine input, verified archive
import, complete development root generation, static libraries and XCFramework.
Production CPU and Linux compatibility code belongs to the host runtime tree.

Flutter Runner builds the ARM64 static library through `ios/build_arm64_runtime.sh`
and imports `Runner/arm64-rootfs.tar.gz` into a persistent ARM64 root. The native
bridge supplies real PTYs, streaming output, interruption, window sizing, shared
host directories and shell sessions. The Rust terminal implementation identifier
is `arm64`.

The former iSH x86 bridge, source fetch/build scripts, patches and CI build have
been removed. Ignored caches and previous user data are retained; `ish/` has no
active build target. The new runtime has no source or link dependency on iSH or
its ARM64 forks.

Run `python3 tools/tests/run_arm64_simulator_smoke.py` on a booted ARM64 simulator
to exercise the production installer and bridge. Broader runtime commands and
current limitations are in the [ARM64 tooling guide](arm64/README.md) and
[runtime status](../../hosts/ios/linux-runtime/docs/status-and-migration.md).
