#!/usr/bin/env bash
# Runs inside the Debian 12 container (invoked by build_cli_linux_debian12.py).
# /work is the repository root; rustup and the cargo cache live in bind mounts
# configured by the driver, so repeated builds reuse downloaded artifacts.
set -euo pipefail

usage() {
  echo "usage: container-build.sh <x86_64|aarch64>" >&2
  exit 2
}

[ $# -eq 1 ] || usage
arch="$1"

case "$arch" in
  x86_64)
    rust_target="x86_64-unknown-linux-gnu"
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
    export CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
    export CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++
    export AR_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ar
    ;;
  aarch64)
    rust_target="aarch64-unknown-linux-gnu"
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
    export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
    export CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
    export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
    ;;
  *) usage ;;
esac

rust_version="${RUST_TOOLCHAIN_VERSION:-1.95.0}"

# Nested package scripts call plain `pnpm`; expose the corepack shim on PATH
# (this directory is a bind mount, so the image itself stays read-only for
# the non-root build user).
corepack enable --install-directory /opt/operit-cargo/bin pnpm

if ! command -v cargo >/dev/null 2>&1; then
  rustup_init="${RUSTUP_UPDATE_ROOT:-https://static.rust-lang.org/rustup}/dist/$(uname -m)-unknown-linux-gnu/rustup-init"
  curl --proto '=https' --tlsv1.2 -fsSL "$rustup_init" -o /tmp/rustup-init
  chmod +x /tmp/rustup-init
  /tmp/rustup-init -y --profile minimal --default-toolchain "$rust_version" --no-modify-path
fi
rustup target add "$rust_target"

cd /work
# character_cards packs before workflow (alphabetical order) and borrows
# workflow's node_modules through createRequire; pre-install so plugin asset
# synchronization also works from a clean clone.
(cd plugins/packages/buildin/workflow && corepack pnpm install --frozen-lockfile)

python3 plugins/tools/sync_plugin_packages.py --source runtime --no-hot-reload

cargo build --release --manifest-path apps/cli/Cargo.toml --target "$rust_target"
