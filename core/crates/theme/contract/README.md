# operit-theme-contract

Runtime-independent Theme v1 (`.optheme`) archive, schema, integrity, media, preview, and deterministic export crate.

It depends only on public ecosystem crates and performs no filesystem I/O, Host calls, runtime startup, plugin execution, persistence, or theme application.

## Public Entrypoints

- `normalize_draft`: Validate a declarative v1 authoring draft against `theme-v1.schema.json`, fill only schema-documented defaults, and enforce semantic slice/version rules. Returns `ThemeSpecV1` (whose `canonical_bytes()` emits deterministic sorted-key UTF-8 JSON).
- `parse_theme` / `parse_manifest`: Strictly validate expanded `theme.json` and `manifest.json` documents (unknown fields, duplicate JSON keys, non-integer versions, and invalid RGBA tokens are rejected).
- `validate_package_files`: Verify L1–L3 manifest/file digests, byte budgets, path canonicality, and exact asset/SPDX license closure. Returns `ContractCheckedTheme`.
- `validate_theme_package` / `validate_theme_package_with_expectation`: Execute the complete fail-closed L0–L4 pipeline (bounded Stored/Deflate ZIP inspection, JSON contract & hash closure, PNG/JPEG/restricted static SVG decoding, 9-slice pixel bounds, and static TrueType table/`loca`/`cmap`/glyph outline traversal). Returns `ValidatedThemePackage` with `ThemeValidationReceipt` (`trust: Untrusted`).
- `validate_theme_preview_draft` / `validate_theme_preview_transfer`: Validate in-memory authoring drafts and contiguous binary asset buffers using the exact same L2–L4 checks as package import. Returns `ValidatedThemePreview`.
- `export_theme_package` / `export_theme_package_from_transfer`: Emit a deterministic Stored-ZIP `.optheme` archive (fixed entry order, DOS timestamp, and permissions) and immediately self-verify the generated archive through `validate_theme_package`.
- `inspect_theme_archive`: Bounded L0/L1 ZIP pre-inspection returning whole-archive byte length, SHA-256, and entry summary prior to user confirmation.
- `wasm` feature bindings (`src/wasm.rs`): Exposes `normalizeThemeDraft`, `validateThemePreviewDraft`, and `exportThemePackage` for the Theme Studio and Flutter Web preview workers.

## Verification

```powershell
cargo test --manifest-path core/Cargo.toml -p operit-theme-contract
cargo check --manifest-path core/Cargo.toml -p operit-theme-contract --features wasm --target wasm32-unknown-unknown
```

See [`docs/theme-v1-contract.md`](../../../../docs/theme-v1-contract.md) and [`SECURITY.md`](SECURITY.md).
