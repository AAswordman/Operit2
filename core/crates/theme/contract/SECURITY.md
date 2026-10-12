# Theme contract security boundary

Status: P0 source for a declarative theme package, not an enabled public
installer.

## What this crate checks

The contract rejects duplicate-key JSON, unknown fields, unsupported versions,
unregistered paths, invalid numeric/enum values, incompatible Core versions,
file-set drift, byte-count/SHA-256 drift and unclosed asset/license references.
Theme packages contain no CSS, scripts, HTML, executable expressions, arbitrary
widgets, URLs or Host paths.

`inspect_theme_archive` and `validate_theme_package` now contain source-level
L0/L1 and L4 gates: raw ZIP directory checks, exact local/central records, PNG
CRC/container checks, JPEG marker checks, complete bounded image decoding, and
static TrueType table/loca/cmap/outline walks. The original ZIP is retained in
`ValidatedThemePackage` and its receipt records `Untrusted` trust explicitly.

## What these checks do not prove

`ContractCheckedTheme` is only the L2/L3 result. It is not evidence of valid
media, curation, publisher identity or installability. `ValidatedThemePackage`
is structural evidence, not an install receipt, official badge, review result or
publisher identity proof. It does not write to a runtime storage path, select a
theme, or apply a renderer.

The current revision lacks native/WASM package test and browser execution
evidence. Earlier compile checks are not media-validation or parity results.
L5 attestation, origin policy, package-library commit/rollback and
application-state persistence remain separate gates.

An expected byte count and SHA-256 prove that the received bytes match a prior
description; they do not prove benevolence or ownership. SPDX IDs and license
texts do not prove permission to distribute an asset. Human review must verify
licensing and readability. Third-party media/font decoders and Flutter
rendering still have an attack surface even with an allowlist.

## Preview and trust

The standalone probe's source now includes external declarative draft input via
an exact-origin, parent-source, nonce-bound MessagePort. Its own pinned
Rust/WASM worker checks draft/asset closure and L4 before any Flutter decoder.
An author-supplied matching SHA-256 does not skip that worker. A new revision
invalidates previous frame evidence immediately, including when the edit fails.
Contract errors/timeouts stop the request; they are not treated as fixture
success. The worker has a caller-owned 15-second deadline and one in-flight
operation. Ordinary browser Workers do not enforce the declared 512 MiB memory
isolation budget; resource consumption and actual ABI execution are still
acceptance work, not an OS-level sandbox claim.

The `surface-probe` frame receipt is not a full-scene render/export authority.
The pure raw exporter validates the bytes of a supplied PNG but does not prove
that PNG came from the renderer. Final Studio screenshot binding, shader-ready
gates, full-scene UI, market confirmation, featured badges and signature
verification are not implemented. Same Dart source is not a promise of
identical GPU or platform glyph rasterizing.

Ed25519 curated-version attestations and the root-signed trust index are planned
separately. No package author declaration or unsigned featured flag is an
official identity proof. Confirmation/permission/security UI must retain locked
application styling when the installer is introduced.
