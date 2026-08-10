#!/usr/bin/env bash
set -Eeuo pipefail

# Reports the command and source line that caused the iSH build to terminate.
report_build_failure() {
    local exit_code="$?"

    printf 'iSH build failed at %s:%s while running: %s\n' \
        "${BASH_SOURCE[1]}" "${BASH_LINENO[0]}" "$BASH_COMMAND" >&2
    exit "$exit_code"
}

trap report_build_failure ERR

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../.." && pwd)"
source_dir="$script_dir/sources/ish"
rootfs_path="$repo_dir/apps/flutter/app/ios/Runner/ish-root.tar.gz"
configuration="${1:?missing Xcode configuration}"
sdk_name="${2:?missing Xcode SDK name}"
platform_name="${3:?missing Xcode platform name}"
architectures="${4:?missing Xcode architectures}"
build_products_dir="$repo_dir/apps/flutter/app/apple/ish-build/${configuration}-${platform_name}"
linux_configuration="${configuration}Linux"
ish_meson_build_dir="$build_products_dir/meson-ish"
linux_meson_build_dir="$build_products_dir/meson-linux"

python3 "$script_dir/fetch_sources.py"

test -d "$source_dir"
test -f "$rootfs_path"

for patch_path in "$script_dir"/patches/*.patch; do
    /usr/bin/patch -d "$source_dir" -p1 -i "$patch_path"
done

# Builds one pinned iSH static target into the Runner-owned products directory.
build_target() {
    local project="$1"
    local target="$2"
    local target_configuration="$3"
    local meson_build_dir="$4"

    xcodebuild \
        -project "$project" \
        -target "$target" \
        -configuration "$target_configuration" \
        -sdk "$sdk_name" \
        ARCHS="$architectures" \
        CONFIGURATION_BUILD_DIR="$build_products_dir" \
        MESON_BUILD_DIR="$meson_build_dir" \
        CODE_SIGNING_ALLOWED=NO \
        CODE_SIGNING_REQUIRED=NO \
        build
}

# Verifies that one static library required by the Runner linker was produced.
verify_static_library() {
    local library_name="$1"

    printf 'Verifying iSH static library: %s\n' "$library_name"
    test -f "$build_products_dir/$library_name"
}

# Verifies that one static library exports a required C symbol.
verify_static_library_symbol() {
    local library_name="$1"
    local symbol_name="$2"

    printf 'Verifying iSH static library symbol: %s in %s\n' "$symbol_name" "$library_name"
    xcrun --sdk "$sdk_name" nm -gU "$build_products_dir/$library_name" \
        | awk -v expected="_$symbol_name" '$NF == expected { found = 1 } END { exit !found }'
}

# Builds one iSH source file into a universal static library.
build_ish_source_static_library() {
    local source_path="$1"
    local library_name="$2"
    local sdk_root
    local source_name
    local architecture
    local object_path
    local architecture_library
    local -a architecture_libraries=()

    sdk_root="$(xcrun --sdk "$sdk_name" --show-sdk-path)"
    source_name="$(basename "$source_path" .c)"
    for architecture in $architectures; do
        object_path="$build_products_dir/$source_name-$architecture.o"
        architecture_library="$build_products_dir/$library_name-$architecture.a"
        xcrun --sdk "$sdk_name" clang \
            -arch "$architecture" \
            -isysroot "$sdk_root" \
            -I "$source_dir" \
            -I "$source_dir/deps/libarchive/libarchive" \
            -c "$source_path" \
            -o "$object_path"
        xcrun --sdk "$sdk_name" libtool -static \
            -o "$architecture_library" \
            "$object_path"
        architecture_libraries+=("$architecture_library")
    done

    xcrun --sdk "$sdk_name" lipo -create \
        "${architecture_libraries[@]}" \
        -output "$build_products_dir/$library_name.a"
}

build_target "$source_dir/iSH.xcodeproj" libish "$configuration" "$ish_meson_build_dir"
build_target "$source_dir/iSH.xcodeproj" liblinux "$linux_configuration" "$linux_meson_build_dir"
build_target "$source_dir/iSH.xcodeproj" libiSHLinux "$linux_configuration" "$linux_meson_build_dir"
build_target "$source_dir/iSH.xcodeproj" libiSHLinuxUser "$linux_configuration" "$linux_meson_build_dir"
build_target "$source_dir/iSH.xcodeproj" libiSHApp "$linux_configuration" "$linux_meson_build_dir"
build_target "$source_dir/iSH.xcodeproj" libfakefs "$linux_configuration" "$linux_meson_build_dir"
build_target "$source_dir/iSH.xcodeproj" libish_emu "$linux_configuration" "$linux_meson_build_dir"
build_target "$source_dir/deps/libarchive.xcodeproj" libarchive "$configuration" "$linux_meson_build_dir"
build_ish_source_static_library "$source_dir/tools/fakefs.c" libiSHFakefs
build_ish_source_static_library "$source_dir/util/fchdir.c" libiSHFchdir

verify_static_library liblinux.a
verify_static_library libiSHLinux.a
verify_static_library libiSHLinuxUser.a
verify_static_library libish.a
verify_static_library libiSHApp.a
verify_static_library_symbol libiSHApp.a iosfs_mount_app_directory
verify_static_library libfakefs.a
verify_static_library libish_emu.a
verify_static_library libarchive.a
verify_static_library libiSHFakefs.a
verify_static_library libiSHFchdir.a
