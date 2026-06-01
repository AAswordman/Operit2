#!/usr/bin/env bash
set -euo pipefail

cache_dir="$HOME/.cache/operit-android-runtime"
zip_path="$cache_dir/android-ndk-r29-beta4-linux.zip"
ndk_dir="$cache_dir/android-ndk-r29-beta4"

mkdir -p "$cache_dir"
curl -L -o "$zip_path" "https://dl.google.com/android/repository/android-ndk-r29-beta4-linux.zip"
rm -rf "$ndk_dir"
unzip -q "$zip_path" -d "$cache_dir"

cat "$ndk_dir/source.properties"
