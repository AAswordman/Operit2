# Operit2 Data Rescue

Operit2 includes a standalone data-maintenance entry that can run without Core, SQLite, plugin loading, normal bootstrap parsing, or user preferences. It is intended for recovery when the normal application cannot start.

## Entrances

- Android: long-press the `Operit2` launcher icon and choose `数据救援`; the shortcut starts the separate maintenance process.
- iOS: use the `Data Rescue` Home Screen shortcut while the app is fully closed. While the app is already running, iOS displays an explicit instruction to quit and relaunch through that shortcut.
- OpenHarmony: launch the separately exported `DataRescueAbility`.
- Windows: open the `Operit2` folder in the Start Menu and choose `Operit2 Data Rescue`, or start the executable with `--data-rescue`.
- macOS: use the native `Operit2 Data Rescue` menu command, or start the executable with `--data-rescue`.
- Linux: use the installed `Operit2 Data Rescue` desktop entry, or start the executable with `--data-rescue`.
- Web: open `data-rescue.html` from the deployed site, or use `?mode=data-rescue`.
- CLI: run `operit2 rescue describe`, `export`, `inspect`, `import`, or `resume`.

The Flutter maintenance screen automatically prepares the Host-recorded data locations and never asks users to type or confirm storage paths. Interrupted imports use their durable journal locations. Missing, damaged, or unconfirmed native location records produce an explicit diagnostic instead of selecting another directory. The Flutter maintenance screen is a separate route and does not construct `CoreApplicationService`, `RuntimeBootstrapManager`, the normal logger path, or the normal business UI. Native rescue processes register the file picker and offline rescue ABI without registering the application plugin set. Browser maintenance uses a separate Worker and an exclusive Web Lock.

## Archive and restore

Native platforms use the portable `operit.files.v1` archive layout. Browser storage uses the independent `operit.browser.v1` layout because OPFS, IndexedDB, and browser origin storage are not the same storage contract as native files. Archives are ordinary ZIP files and do not require a password. Preferences are decrypted with the existing source key before export, without changing live files. Both data rescue and regular snapshots contain plaintext preferences that users can inspect and edit. The archive contains every selected regular file, all configured identity roots, workspaces, client bootstrap data, application-owned files, registered portable host secrets, empty directories, and portable Unix permission bits. Windows retains the destination security ACL. Every payload is checked for declared size and SHA-256 before restore.

Temporary archive transfers are excluded from the application staging namespace and the complete runtime tree, including identity-owned staging. Native archive leases retain files only while their Host owner is alive. Explicit disposal and final Host release remove both incomplete and sealed files. Subsequent archive operations collect abandoned leased transfers; opening a configured data-rescue session also collects legacy unleased transfers under exclusive access from every identity, including removed identities. Active leased transfers remain readable. These temporary files are separate from the retained original data created by a restore.

Restore is staged and verified before publication. The maintenance lease is exclusive, and normal runtime access uses a shared lease. A pending publication journal blocks normal startup. The original directories are retained under a generated `.operit-retained-*` path on native platforms and `data-rescue/retained-*` in the browser. The journal is durable and recovery is explicit; the implementation does not silently discard or substitute damaged data.

If the normal process is still holding the lease, the maintenance screen reports the concrete lock error. Android exposes an explicit stop-normal-process command from its maintenance activity. iOS requires a cold launch through the Home Screen shortcut. Other platforms require the normal process to be closed by the user or by the platform's normal lifecycle controls.

An import retains the displaced native roots and application-owned entries. A durable interrupted publication blocks normal startup and is only advanced by an explicit resume operation. Incomplete pre-publication staging is discarded when the user closes the maintenance session. Corrupt archive metadata, unsupported filesystem objects such as symbolic links, overlapping roots, or modified pending payloads produce an explicit error before the corresponding publication step. External disk failure, device loss, storage hardware failure, and OS-level revocation remain outside software guarantees; keep the exported archive on a separate device or storage medium.

Native and browser archives are deliberately not cross-importable in this version. The logical data contract differs between native filesystem data and browser OPFS/IndexedDB records.

## Progress and bounded storage I/O

The restore entry is labeled `从备份恢复`. Export, source reading, manifest inspection, full integrity checking, staged restore, restored-file verification, publication, interrupted-import completion, and destination saving report measured stage counters. The screen shows completed bytes or items and the current stage percentage. Enumeration reports discovered items until its total is known; a system picker is shown as waiting for the user. Percentages describe the current stage, and task success is reported only after that task has completed.

The native bridge starts background jobs and polls a separate progress/result state, so filesystem work cannot occupy the platform request queue. Browser workers post progress during synchronous WASM work; the browser transport keeps task completion pending through settings publication and journal completion.

Archive content is never collected into a Dart byte array. Source ranges are limited to 1 MiB and submitted in awaited 64 KiB chunks. Native ZIP, hash verification, staging, publication copying, and desktop saving use bounded 64 KiB ranges. Browser destination saving consumes a response stream with backpressure. Android stages document sources on a worker and streams destination writes. iOS opens scoped source references and selects an output folder, then the native storage Host performs the measured write. OpenHarmony document copies report each completed bounded write.

Browser Blob/File database values remain file-backed during restore. Records and nested binary exports are processed sequentially. IndexedDB ArrayBuffer and typed-array values must still be reconstructed as contiguous values for the database API, one record at a time; their size can affect peak memory. JSON metadata and each preferences document have an explicit 16 MiB limit. These metadata limits do not limit ordinary archive payload or attachment sizes. Oversized metadata is reported explicitly.
