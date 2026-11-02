#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../.." && pwd)"
source_dir="$repo_dir/hosts/ios/linux-runtime"
target="${1:-host}"
if [[ $# -gt 0 ]]; then shift; fi
build_dir="${OPERIT_ARM64_BUILD_DIR:-$script_dir/build/$target}"
build_type="${OPERIT_ARM64_BUILD_TYPE:-Release}"
jobs="${OPERIT_ARM64_JOBS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || printf '4')}"
case "$target" in
    xcframework)
        # These slices must use separate caches even when the caller overrides its output.
        package_dir="${OPERIT_ARM64_BUILD_DIR:-$script_dir/build}"
        mkdir -p "$package_dir"
        OPERIT_ARM64_BUILD_DIR="$package_dir/ios" bash "$script_dir/build.sh" ios "$@"
        OPERIT_ARM64_BUILD_DIR="$package_dir/ios-simulator" bash "$script_dir/build.sh" ios-simulator "$@"
        stage_dir="$(mktemp -d "$package_dir/.xcframework.XXXXXX")"
        trap 'rm -rf "$stage_dir"' EXIT
        xcodebuild -create-xcframework \
            -library "$package_dir/ios/liboperit_arm64.a" -headers "$source_dir/include" \
            -library "$package_dir/ios-simulator/liboperit_arm64.a" -headers "$source_dir/include" \
            -output "$stage_dir/OperitArm64.xcframework"
        rm -rf "$package_dir/OperitArm64.xcframework"
        mv "$stage_dir/OperitArm64.xcframework" "$package_dir/OperitArm64.xcframework"
        exit 0
        ;;
    host)
        cmake -S "$source_dir" -B "$build_dir" -DCMAKE_BUILD_TYPE="$build_type" "$@"
        cmake --build "$build_dir" --config "$build_type" --parallel "$jobs"
        ;;
    ios|ios-simulator)
        sdk=iphoneos
        if [[ "$target" == ios-simulator ]]; then sdk=iphonesimulator; fi
        cmake -S "$source_dir" -B "$build_dir" -DCMAKE_SYSTEM_NAME=iOS \
            -DCMAKE_OSX_SYSROOT="$(xcrun --sdk "$sdk" --show-sdk-path)" \
            -DCMAKE_C_COMPILER="$(xcrun --sdk "$sdk" --find clang)" \
            -DCMAKE_CXX_COMPILER="$(xcrun --sdk "$sdk" --find clang++)" \
            -DCMAKE_OSX_ARCHITECTURES=arm64 \
            -DCMAKE_OSX_DEPLOYMENT_TARGET="${OPERIT_ARM64_DEPLOYMENT_TARGET:-15.0}" \
            -DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY \
            -DCMAKE_BUILD_TYPE="$build_type" -DBUILD_TESTING=OFF -DOPERIT_ARM64_LTO=OFF "$@"
        cmake --build "$build_dir" --config "$build_type" --target operit_arm64 --parallel "$jobs"
        ;;
    *) printf 'usage: %s [host|ios|ios-simulator|xcframework] [cmake options...]\n' "$0" >&2; exit 2 ;;
esac
