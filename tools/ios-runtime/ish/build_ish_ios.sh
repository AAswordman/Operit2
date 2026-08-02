#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../.." && pwd)"
source_dir="$script_dir/sources/ish"
rootfs_path="$repo_dir/apps/flutter/app/ios/Runner/ish-root.tar.gz"
configuration="${1:?missing Xcode configuration}"
sdk_name="${2:?missing Xcode SDK name}"
platform_name="${3:?missing Xcode platform name}"
architectures="${4:?missing Xcode architectures}"
build_products_dir="$repo_dir/apps/flutter/app/apple/ish-build/${configuration}-${platform_name}"

python3 "$script_dir/fetch_sources.py"

test -d "$source_dir"
test -f "$rootfs_path"

# Builds one pinned iSH static target into the Runner-owned products directory.
build_target() {
    local project="$1"
    local target="$2"
    xcodebuild \
        -project "$project" \
        -target "$target" \
        -configuration "$configuration" \
        -sdk "$sdk_name" \
        ARCHS="$architectures" \
        CONFIGURATION_BUILD_DIR="$build_products_dir" \
        CODE_SIGNING_ALLOWED=NO \
        CODE_SIGNING_REQUIRED=NO \
        build
}

# Verifies that one static library required by the Runner linker was produced.
verify_static_library() {
    local library_name="$1"

    test -f "$build_products_dir/$library_name"
}

build_target "$source_dir/iSH.xcodeproj" liblinux
build_target "$source_dir/iSH.xcodeproj" libiSHLinux
build_target "$source_dir/iSH.xcodeproj" libiSHLinuxUser
build_target "$source_dir/iSH.xcodeproj" libfakefs
build_target "$source_dir/iSH.xcodeproj" libish_emu
build_target "$source_dir/deps/libarchive/libarchive.xcodeproj" libarchive

verify_static_library liblinux.a
verify_static_library libiSHLinux.a
verify_static_library libiSHLinuxUser.a
verify_static_library libfakefs.a
verify_static_library libish_emu.a
verify_static_library libarchive.a
