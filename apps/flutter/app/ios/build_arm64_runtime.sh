#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.local/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../../.." && pwd)"
case "${PLATFORM_NAME:?Xcode PLATFORM_NAME is required}" in
    iphoneos) target=ios ;;
    iphonesimulator) target=ios-simulator ;;
    *) printf 'Unsupported ARM64 runtime platform: %s\n' "$PLATFORM_NAME" >&2; exit 1 ;;
esac
for arch in ${ARCHS:-arm64}; do
    if [[ "$arch" != arm64 ]]; then
        printf 'The iOS runtime supports arm64 devices and arm64 simulators; received %s\n' "$arch" >&2
        exit 1
    fi
done
archive="$script_dir/Runner/arm64-rootfs.tar.gz"
if [[ ! -f "$archive" || -L "$archive" ]]; then
    printf 'Missing ARM64 root archive: %s. Prepare it with tools/ios-runtime/arm64/build_rootfs_linux.sh before building iOS.\n' "$archive" >&2
    exit 1
fi
output_dir="${DERIVED_FILE_DIR:?Xcode DERIVED_FILE_DIR is required}/Arm64Runtime"
mkdir -p "$output_dir"
export OPERIT_ARM64_BUILD_DIR="$output_dir/build"
export OPERIT_ARM64_BUILD_TYPE=Release
export OPERIT_ARM64_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-16.4}"
# Xcode adds this to its script environment; nested CMake/Make must not inherit it.
unset SDKROOT
bash "$repo_dir/tools/ios-runtime/arm64/build.sh" "$target"
if ! cmp -s "$OPERIT_ARM64_BUILD_DIR/liboperit_arm64.a" "$output_dir/liboperit_arm64.a"; then
    cp "$OPERIT_ARM64_BUILD_DIR/liboperit_arm64.a" "$output_dir/.liboperit_arm64.a.tmp"
    mv -f "$output_dir/.liboperit_arm64.a.tmp" "$output_dir/liboperit_arm64.a"
fi
python3 "$repo_dir/tools/ios-runtime/arm64/package_rootfs.py" \
    --archive "$archive" --output "$output_dir/arm64-rootfs.tar.gz"
