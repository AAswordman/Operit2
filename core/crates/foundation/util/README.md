# operit-util

`operit-util` owns shared utility code used by the split core crates. It holds
pure helpers plus small cross-target utilities that do not belong to runtime
or store ownership.

## Responsibilities

- Provide logging, stream, serialization, parsing, markdown, media, archive,
  document, network, QR, OCR, and TTS helper utilities.
- Define canonical runtime path constants and root path helpers.
- Keep common utilities reusable by model, store, tools, providers, JS bridge,
  and runtime.
- Avoid owning repositories, provider services, tool execution, host
  lifecycle, or runtime orchestration.

## Main Modules

- `src/RuntimeStorageLayout.rs`: canonical relative runtime storage layout.
- `src/RuntimeStoreRoot.rs`: configured runtime data root.
- `src/OperitPaths.rs`: runtime path helpers built from the configured root.
- `src/stream/`: shared stream abstractions and operators.
- `src/streamnative/`: native stream splitting helpers.
- `src/AppLogger.rs` and `src/ChainLogger.rs`: logging utilities.
- `src/*Util.rs`: focused helpers for files, archives, media, network, and
  document processing.

## Boundary

This crate may depend on host API types for cross-target helper behavior. It
must not depend on runtime services, store repositories, provider code, tool
registries, or JS engine implementations.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.
