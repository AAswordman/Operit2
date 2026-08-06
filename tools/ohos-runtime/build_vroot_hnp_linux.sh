#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../.." && pwd)"
runtime_dir="$script_dir"
source_dir="$runtime_dir/sources/harmonix"
output_dir="$repo_dir/apps/flutter/app/ohos/entry/hnp/arm64-v8a"
harmonix_commit="2d671a26d2f049af12c3583da6403b77c3b66023"

# Verifies that one required directory exists before the QEMU-vroot build starts.
require_directory() {
    local path="$1"
    test -d "$path" || {
        echo "Required directory does not exist: $path" >&2
        exit 1
    }
}

: "${OHOS_SDK_HOME:?OHOS_SDK_HOME must point to the OpenHarmony SDK version directory}"
require_directory "$OHOS_SDK_HOME/native"
require_directory "$OHOS_SDK_HOME/llvm"

mkdir -p "$runtime_dir/sources" "$output_dir"
if [ ! -d "$source_dir/.git" ]; then
    git clone https://github.com/harmoninux/Harmonix.git "$source_dir"
fi
git -C "$source_dir" fetch --depth=1 origin "$harmonix_commit"
git -C "$source_dir" checkout --detach "$harmonix_commit"

export OHOS_ARCH="aarch64"
export OHOS_ABI="arm64-v8a"
export OHOS_SDK_HOME
make -C "$source_dir/build-hnp"

test -f "$source_dir/build-hnp/harmonix-public.hnp"
cp "$source_dir/build-hnp/harmonix-public.hnp" "$output_dir/harmonix-public.hnp"
cp "$source_dir/build-hnp/harmonix-public.hnp" "$output_dir/harmonix-private.hnp"
