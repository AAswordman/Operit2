# operit-local-models

`operit-local-models` owns Operit's local model contracts. It is the sibling
crate to `operit-providers`: provider code talks to remote APIs and external
services, while this crate describes model files installed on the user's device
and the local inference sessions that consume them.

The first target users are STT and TTS. The same crate is shaped for local LLM
engines such as llama.cpp and MNN, so model download, verification, registry
state, and engine selection stay in one local-model boundary.

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

The active built-in catalog starts with Sherpa ONNX speech models:

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
- Sources: pinned Hugging Face revisions with declared size and SHA-256 for
  every file.

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
