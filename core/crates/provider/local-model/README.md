# operit-local-models

`operit-local-models` owns Operit's local model contracts. It is the sibling
crate to `operit-providers`: provider code talks to remote APIs and external
services, while this crate describes model files installed on the user's device
and the local inference sessions that consume them.

The crate implements STT/TTS asset management and embedded CPU text inference
using llama.cpp and MNN. Model download, verification, installation inventory,
and engine selection stay in one local-model boundary.

The crate intentionally models local assets, not cloud API providers. A model
entry names its kind, compatible engine, license, declared files, byte sizes,
checksums, and concrete source ids. Installation downloads the declared files,
verifies them, writes runtime data, and records the installed model in the
registry.

## Responsibilities

- Define local model manifests, file metadata, engine identifiers, model kinds,
  and installed registry records.
- Provide built-in manifest entries for local models shipped through external
  model repositories.
- Submit manifest-declared files to the injected `HttpHost` download manager
  and verify installed bytes against declared checksums.
- Verify and delete installed local models through registry-aware operations.
- Provide storage path helpers for model files under the runtime data tree.
- Define STT, TTS, and local chat inference session interfaces.
- Define `LocalModelRuntimeSupport`, the local-model-side contract implemented
  by `operit-runtime`.
- Keep model assets out of the application binary; models are installed as
  runtime data.

## Main Modules

- `src/LocalModelManifest.rs`: model kind, engine kind, manifest records, and
  file checksum helpers.
- `src/LocalModelCatalog.rs`: built-in local model manifests.
- `src/LocalModelHub.rs`: Hugging Face and ModelScope repository search, file
  tree inspection, and automatic driver manifest inference.
- `src/LocalModelDownload.rs`: manifest-driven local model installation and
  download progress mapping.
- `src/LocalModelRegistry.rs`: installed model records and registry snapshot
  queries.
- `src/LocalModelRegistryStore.rs`: registry persistence without creating a
  network client.
- `src/LocalModelStorage.rs`: storage path construction and path segment
  validation.
- `src/LocalInference.rs`: local STT, TTS, and chat request/session contracts.
- `src/runtime_support.rs`: runtime-owned operations requested by the local
  model crate.

## Built-in Catalog

The chat catalog includes pinned `qwen3-0.6b-gguf` (llama.cpp, Q8_0) and
`qwen3-0.6b-mnn` (MNN) bundles, with per-file sizes and SHA-256 hashes.
Hub imports recognize complete GGUF files and MNN text model bundles. GGUF
imports select one quantization, preferring Q4_K_M; split GGUF shards are not
supported. MNN bundles must include their external embeddings unless the
model uses tied embeddings in the weight file.

The speech catalog includes Sherpa ONNX models:

- `sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20`: bilingual
  streaming STT on Windows, Linux, macOS, Android, OpenHarmony, and iOS.
- `vits-zh-aishell3-int8`: Chinese multi-speaker TTS.
- `sherpa-onnx-vits-zh-ll`: Chinese five-speaker TTS with Sherpa rule FSTs.
- `matcha-icefall-zh-baker`: Chinese single-speaker Matcha TTS with Sherpa
  rule FSTs and the k2-fsa Vocos vocoder.
- `kitten-nano-en-v0_8-int8`: English Kitten TTS.
- `sherpa-onnx-web-paraformer-small-zh-en`: browser Paraformer STT bundle.
- `sherpa-onnx-web-vits-piper-en-us-libritts-r-medium`: browser VITS TTS
  bundle with 904 speakers.
- Engine: `SherpaOnnx` version `1.13.2`.
- Licenses: model-specific; entries carry their own license field.
- Sources: pinned Hugging Face and ModelScope repositories (plus HF-Mirror
  resolution) with declared size and SHA-256 for every file.

Native Sherpa drivers and browser bundle drivers are separate enum variants.
Catalog status and installation both validate the current platform against the
driver variant, so a Web bundle cannot be selected by a desktop or mobile
runtime and a native model cannot be selected by the browser runtime.

Windows, Linux, macOS, Android, and OpenHarmony install a checksum-verified
engine archive for the exact platform and architecture. iOS links the official
Sherpa ONNX and ONNX Runtime XCFrameworks into the signed application, and Web
uses the browser runtime carried by its installed model bundle. Embedded engine
targets still receive an installed engine registry record, but do not download
a second runtime archive during model installation.

Additional STT, TTS, chat, and embedding manifests can be added as catalog
entries or loaded by the runtime through the same manifest structure.

## Installation Model

`LocalModelInstaller` performs native filesystem installation for a runtime data
root:

1. Validate the manifest id, version, and relative file paths.
2. Submit all declared files to `HttpHost` with bounded concurrency, progress
   events, and a cancellation token.
3. Download into temporary files managed by the host.
4. Replace the target file after size and checksum validation.
5. Write the installed model registry record.

The same installer can verify installed files and delete an installed model
directory together with its registry record.

## Boundary

`operit-local-models` depends on the contracts in `operit-host-api`, but not on
any concrete Android, Apple, Linux, OpenHarmony, Windows, or web host.
It does not depend on `operit-runtime`, `operit-store`, `operit-tools`,
`operit-providers`, or UI code. Runtime-owned behavior is requested through
`LocalModelRuntimeSupport`.

`operit-providers` remains responsible for API providers, HTTP voice providers,
market API calls, and provider orchestration. Local engines such as Sherpa,
Piper, llama.cpp, and MNN belong behind this crate's local session contracts.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.

## Local chat inference and routing

In Settings > Local models, download a chat model, open **Inference & settings**,
set CPU threads and (for llama.cpp) context/batch sizes, and run a prompt.
**Use for chat** saves its runtime settings and selects it for the CHAT function.
**Release model memory** frees cached weights. Only one LLM is cached at a time;
inference is serialized and runs off the runtime event loop.

Weights, partial downloads, model caches, custom imported manifests, and the
installation registry belong to this CoreNode. `runtime/models/local/**` and
`runtime/config/preferences/local_model_registry.preferences.json` are excluded
from Space file synchronization. Sharing a provider/model reference does not
copy a model bundle to other devices.

Each installed chat model receives an opaque inference Binding. A saved model
profile carries this Binding alongside its `modelId@version` reference.
`ChatServiceCore.generateLocalLlm` uses `target:runtime.execute` route permission
to send only the prompt, generation settings, and model reference to the device
owning the Binding. The conversation and tools can remain on the calling node.
Installing the same model and version locally preserves an explicitly saved remote
inference Binding; changing the inference device requires an explicit selection.
The target resolves its own exact installed files. An unavailable remote owner
returns an error rather than silently choosing local weights.

The current bridge returns a complete text result, including input/output token
counts. It does not yet stream individual tokens or cancel native generation.
GPU backends, multimodal inputs, and split GGUF files are not supported.

## Building and testing the embedded engines

The default `native-llm` feature builds static CPU backends with CMake 3.22+,
a C++17 compiler, and Ninja for Android/OpenHarmony cross builds. The first native build downloads SHA-256 checked source archives
for llama.cpp `b11829` and MNN `3.6.1`. Set `OPERIT_LLAMA_SOURCE` and
`OPERIT_MNN_SOURCE` to pre-extracted source directories for offline builds.
Android builds require `ANDROID_NDK_HOME` (or `ANDROID_NDK_ROOT`/`ANDROID_NDK`);
OpenHarmony builds require `OHOS_NATIVE_HOME`. Their SDK CMake toolchains are
selected by build.rs. Actual native inference has been verified on Windows x64;
mobile and Apple targets still require device build/runtime validation.

Contract-only consumers can use `default-features = false`. WebAssembly does
not build or expose either native engine, but can call a native model device
through the same routed provider reference.

```powershell
cargo test --manifest-path core/Cargo.toml -p operit-local-models --lib
cargo test --manifest-path core/Cargo.toml -p operit-runtime --lib LocalLlmService::tests
```

Two ignored smoke tests use real local weights. Set `OPERIT_LLAMA_TEST_MODEL`
to a complete GGUF instruct model and `OPERIT_MNN_TEST_CONFIG` to an installed
MNN bundle's config.json, then run:

```powershell
cargo test --manifest-path core/Cargo.toml -p operit-local-models --lib native -- --include-ignored --nocapture --test-threads=1
```
