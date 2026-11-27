#!/bin/sh
set -eu

# Xcode supplies its own PATH; preserve the CI-selected WebAssembly tools for
# plugin packaging, which runs again within this build phase.
if [ -n "${OPERIT_WASM_TOOL_PATH:-}" ]; then
  export PATH="$OPERIT_WASM_TOOL_PATH:$PATH"
fi

repo_root="$PROJECT_DIR/../../../.."
crate_dir="$repo_root/apps/flutter/native/operit-flutter-bridge"
out_dir="$PROJECT_DIR/Flutter/ephemeral/rust"
lib_name="liboperit_flutter_bridge.a"

# Prepare the Core asset index and the matching Flutter application resources.
"$repo_root/.venv/bin/python" "$repo_root/plugins/tools/sync_plugin_packages.py" \
  --source runtime --no-hot-reload --flutter-assets

for arch in $ARCHS; do
  case "$arch" in
    arm64)
      rust_target="aarch64-apple-darwin"
      ;;
    x86_64)
      rust_target="x86_64-apple-darwin"
      ;;
    *)
      echo "Unsupported macOS Rust bridge architecture: $arch" >&2
      exit 1
      ;;
  esac

  mkdir -p "$out_dir/$arch"
  rustup target add "$rust_target"

  "$repo_root/.venv/bin/python" "$repo_root/tools/build_scripts/cargo_build.py" cargo rustc \
    --manifest-path "$crate_dir/Cargo.toml" \
    --release \
    --target "$rust_target" \
    --crate-type staticlib

  cp "$crate_dir/target/$rust_target/release/$lib_name" "$out_dir/$arch/$lib_name"
done
