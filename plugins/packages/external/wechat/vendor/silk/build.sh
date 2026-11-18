#!/usr/bin/env bash
set -euo pipefail

here=$(cd -- "$(dirname -- "$0")" && pwd)
repo=$(git -C "$here" rev-parse --show-toplevel)
revision=5971995be3ce05d4a1bf820fdf9d7ccdc942e945
source=${1:?Usage: bash build.sh /path/to/silk-wasm-checkout}
test "$(git -C "$source" rev-parse HEAD)" = "$revision"
git -C "$source" diff --quiet HEAD -- binding/libSilkCodec/silk
sdk="$source/binding/libSilkCodec/silk/src/SILK_SDK_SRC_ARM_v1.0.9"
compiler=${EMCC:-"$repo/.ci-tools/emsdk/upstream/emscripten/emcc"}
mkdir -p "$here/../../resources"
LC_ALL=C "$compiler" -O3 -flto -DNDEBUG -I"$sdk/interface" -I"$sdk/src" \
  "$here/decode.c" "$sdk"/src/*.c --no-entry -s STANDALONE_WASM=1 \
  -s INITIAL_MEMORY=12582912 -s STACK_SIZE=131072 \
  -s EXPORTED_FUNCTIONS='["_malloc","_silk_decode"]' \
  -o "$here/../../resources/silk-decoder.wasm"
