#!/usr/bin/env bash
# Validates the actual Flutter/WebKit scene against a supplied Linux engine.
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo 'Usage: run_native_scene.sh ENGINE_ARTIFACTS ICU_DATA' >&2
  exit 2
fi
engine="$(realpath "$1")"
icu="$(realpath "$2")"
test_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app="$(cd "$test_dir/../../../../app" && pwd)"
cd "$app"
assets="$app/build/linux_native_scene/flutter_assets"
fvm flutter assemble --output="$assets" \
  '--define=BuildMode=debug' '--define=TargetPlatform=linux-x64' \
  "--define=TargetFile=$test_dir/../../test/linux_native_scene_fixture.dart" \
  kernel_snapshot_program
build_id="$(cat "$assets/.last_build_id")"
cp "$app/.dart_tool/flutter_build/$build_id/app.dill" "$assets/kernel_blob.bin"
cmake -S "$test_dir/native_scene" -B build/linux_native_scene/host -G Ninja \
  -DFLUTTER_ENGINE_ARTIFACTS="$engine"
cmake --build build/linux_native_scene/host --parallel 4
for renderer in opengl software; do
  output="$app/build/linux_native_scene/results/$renderer"
  GDK_BACKEND=x11 FLUTTER_LINUX_RENDERER="$renderer" \
    xvfb-run -a dbus-run-session -- build/linux_native_scene/host/native_scene_driver "$assets" "$icu" "$output"
  GDK_BACKEND=x11 FLUTTER_LINUX_RENDERER="$renderer" OPERIT_SCENE_CLOSE_LIVE=1 \
    xvfb-run -a dbus-run-session -- build/linux_native_scene/host/native_scene_driver "$assets" "$icu" "$output/close-live"
done
