#!/bin/sh
set -eu

repo_root="$PROJECT_DIR/../../../.."
crate_dir="$repo_root/apps/flutter/native/operit-flutter-bridge"
out_dir="$PROJECT_DIR/Flutter/ephemeral/rust/$PLATFORM_NAME"
lib_name="liboperit_flutter_bridge.a"

# Core embeds plugin assets at compile time. Prepare both built-in and optional
# official packages before Cargo runs, including on the first clean build.
"$repo_root/.venv/bin/python" "$repo_root/plugins/tools/sync_plugin_packages.py" \
  --source runtime --no-hot-reload

for arch in $ARCHS; do
  case "$PLATFORM_NAME:$arch" in
    iphoneos:arm64)
      rust_target="aarch64-apple-ios"
      ;;
    iphonesimulator:arm64)
      rust_target="aarch64-apple-ios-sim"
      ;;
    iphonesimulator:x86_64)
      rust_target="x86_64-apple-ios"
      ;;
    *)
      echo "Unsupported iOS Rust bridge target: platform=$PLATFORM_NAME arch=$arch" >&2
      exit 1
      ;;
  esac

  mkdir -p "$out_dir/$arch"
  rustup target add "$rust_target"

  RUSTFLAGS="-Awarnings" cargo rustc \
    --manifest-path "$crate_dir/Cargo.toml" \
    --release \
    --target "$rust_target" \
    --crate-type staticlib

  cp "$crate_dir/target/$rust_target/release/$lib_name" "$out_dir/$arch/$lib_name"
done
