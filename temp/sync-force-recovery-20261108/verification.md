# Forced consistency recovery verification — 2026-11-08

The task began at 9495703c7. A concurrent operation merged unrelated upstream changes and staged work; staging was preserved. Scope totals include staged and unstaged changes in the 20 edited synchronization files only.

- Rust/Dart code including tests: +488 / -559; net -71 lines.
- Entire scoped change including protocol manifest and documentation: +495 / -562; net -67 lines.
- Plugin storage: 57 passed, 1 existing ignored case. Corruption scenarios cover missing internal/business/declaration tables, invalid columns/declarations/initialization, stale CAS, transaction rollback, and forwarding the recovered baseline.
- Preference snapshot recovery: 11 passed, including malformed-file recovery with raw-file CAS.
- Transaction model and migration laws: 22 passed.
- Chat SQL: 25 passed, 1 existing ignored case.
- Node consistency and scheduling helpers: 15 passed.
- Device-space scheduling/coordinator integration: 12 passed, including same-pass race recovery and network-only automatic retries.
- Flutter activity overlay: 12 passed; focused Dart analysis reports no issues.
- Final `cargo check -p operit-node-runtime` passed, including application dependencies and protocol fingerprint validation.
- Extended transaction contracts: 23 passed / 6 failed. Extended transaction-filtered store unit tests: 13 passed / 2 failed. The exact 6+2 failure names were independently reproduced using pre-change store sources from 9495703c7 under `temp/sync-force-baseline`; shared foundation/host dependencies were verified unchanged from that revision.

Peer transport protocol is v3; consistency protocol is 7. Both phones need the updated runtime. No APK was rebuilt and no physical-device synchronization was performed in this task.
