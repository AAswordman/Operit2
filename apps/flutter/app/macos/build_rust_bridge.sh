#!/bin/sh
set -eu

repo_root="$PROJECT_DIR/../../../.."
crate_dir="$repo_root/apps/flutter/native/operit-flutter-bridge"
out_dir="$PROJECT_DIR/Flutter/ephemeral/rust"
lib_name="liboperit_flutter_bridge.a"

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

  RUSTFLAGS="-Awarnings" cargo rustc \
    --manifest-path "$crate_dir/Cargo.toml" \
    --release \
    --target "$rust_target" \
    -- \
    --crate-type staticlib

  cp "$crate_dir/target/$rust_target/release/deps/$lib_name" "$out_dir/$arch/$lib_name"
done
