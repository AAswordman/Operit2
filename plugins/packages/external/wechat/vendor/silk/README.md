# WeChat SILK decoder

`../../resources/silk-decoder.wasm` bundles the Skype SILK SDK v1.0.9 from
`binding/libSilkCodec/silk/src/SILK_SDK_SRC_ARM_v1.0.9` in the
`idranme/silk-wasm` repository at commit `5971995be3ce05d4a1bf820fdf9d7ccdc942e945`.
The SDK notice is in `LICENSE-SKYPE`; the toolchain runtime notice is in
`LICENSE-EMSCRIPTEN`. Neither the npm binary nor its JS glue is used.

Build using Emscripten 4.0.14 (the repo toolchain, or set `EMCC`):

```sh
git clone https://github.com/idranme/silk-wasm.git /tmp/silk-wasm
git -C /tmp/silk-wasm checkout --detach 5971995be3ce05d4a1bf820fdf9d7ccdc942e945
bash plugins/packages/external/wechat/vendor/silk/build.sh /tmp/silk-wasm
```

The shipped WASM has fixed memory and no imports. The adapter accepts the
standard and Tencent SILK V3 headers, checks all packet boundaries and caps
output at two minutes of mono 24 kHz signed 16-bit PCM. Operit runs it via its
declared `ToolPkg.wasm` memory ABI, with a fresh instance per clip and no filesystem
or network access from the decoder. The prebuilt module is tracked so regular
plugin packaging does not need a C toolchain or upstream checkout.
