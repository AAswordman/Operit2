#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.local/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../../.." && pwd)"
output_dir="${DERIVED_FILE_DIR:?Xcode DERIVED_FILE_DIR is required}/Arm64Runtime"
archive="${OPERIT_ARM64_ROOTFS_ARCHIVE:-$repo_dir/apps/flutter/app/ios/Runner/arm64-rootfs.tar.gz}"
if [[ ! -f "$archive" || -L "$archive" ]]; then
    printf 'Missing ARM64 root archive: %s. Prepare it with tools/ios-runtime/arm64/build_rootfs_linux.sh.\n' "$archive" >&2
    exit 1
fi
mkdir -p "$output_dir"
sdk="$(xcrun --sdk macosx --show-sdk-path)"
# Avoid leaking Xcode's SDK selection into nested CMake compiler probes.
unset SDKROOT
libraries=()
for arch in ${ARCHS:-$(uname -m)}; do
    case "$arch" in
        arm64) ;;
        x86_64)
            # Keep a linkable Intel slice without compiling the ARM64 executor.
            stub="$output_dir/arm64-runtime-unavailable-x86_64.o"
            library="$output_dir/liboperit_arm64-x86_64.a"
            xcrun --sdk macosx clang -arch x86_64 -x c -c /dev/null -o "$stub"
            xcrun libtool -static -o "$library" "$stub"
            libraries+=("$library")
            continue
            ;;
        *) printf 'Unsupported macOS runtime architecture: %s\n' "$arch" >&2; exit 1 ;;
    esac
    build_dir="$output_dir/build-$arch"
    cmake -S "$repo_dir/hosts/ios/linux-runtime" -B "$build_dir" \
        -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF -DOPERIT_ARM64_LTO=ON \
        -DOPERIT_ARM64_WASM="${OPERIT_ARM64_WASM:-OFF}" \
        -DCMAKE_OSX_SYSROOT="$sdk" -DCMAKE_OSX_ARCHITECTURES="$arch" \
        -DCMAKE_OSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
    cmake --build "$build_dir" --target operit_arm64 \
        --parallel "${OPERIT_ARM64_JOBS:-6}"
    libraries+=("$build_dir/liboperit_arm64.a")
done
xcrun lipo -create "${libraries[@]}" -output "$output_dir/.liboperit_arm64.a.tmp"
if ! cmp -s "$output_dir/.liboperit_arm64.a.tmp" "$output_dir/liboperit_arm64.a"; then
    mv -f "$output_dir/.liboperit_arm64.a.tmp" "$output_dir/liboperit_arm64.a"
else
    rm "$output_dir/.liboperit_arm64.a.tmp"
fi
python3 "$repo_dir/tools/ios-runtime/arm64/package_rootfs.py" \
    --archive "$archive" --output "$output_dir/arm64-rootfs.tar.gz"
