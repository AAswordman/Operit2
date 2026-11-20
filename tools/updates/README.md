# Incremental update packages

`prepare.py` creates direct, byte-exact deltas between original installers. The matcher uses
[HDiffPatch 5.1.3](https://github.com/sisong/HDiffPatch/releases/tag/v5.1.3) for matching.
The build tool downloads the pinned generator with `gh` and checks its SHA-256. End users
need no external patch program. The shared Rust decoder uses Host file ranges and
transactional VFS writes, with at most 1 MiB per data buffer.

The client parses instructions through a 1 MiB decoded buffer and accumulates a 1 MiB
output window. Up to 16,384 COPY/residual source requests are sorted by base offset;
overlapping or nearby requests share a Host range read (at most a 64 KiB gap and a
1 MiB read). Bytes are scattered back to their original output positions before hashing
and writing the window once. This avoids reopening the base and writing the destination
for every tiny matcher instruction. The request cap also bounds memory for one-byte
instructions; the whole installer is never loaded into RAM. Every instruction must
advance the declared target length, replacing the former four-million-instruction cap
that rejected valid published MSI patches. Schemas 1–4 and their signed output bytes
are unchanged.

`ApplicationUpdate` logs separate source verification, base expansion, expanded-base
verification, instruction merging, container restoration, and final verification timings.
The merge line includes instruction count, Host read/write counts and actual read bytes.
Coalescing deliberately reads bounded gaps to reduce file-open and VFS overhead; this
can read more bytes than the sum of requested COPY ranges. These are aggregate records,
not per-instruction logs.

The generator compares streaming matches with precise suffix matches and keeps the smaller
compressed result. Precise matching has a 2 GiB estimated builder memory limit; larger
installers use streaming matching. Both run single-threaded for reproducible output.
Every published patch is reconstructed and SHA-256 verified before release publication.

APK releases use schema 4 and the `O2PATCH4\0` envelope. The generator expands changed
DEFLATE entries before matching native libraries and DEX bytes. Unchanged compressed
entries, APK signatures, local headers, padding and the central directory remain in the
original shell. The Android Host restores the exact raw DEFLATE streams using standard
zlib level 6. Both expanded and original package sizes and SHA-256 digests are checked;
unsupported compression is rejected. APK generation never uses whole-archive binary
matching. Preview.38 includes this decoder so subsequent releases can publish APK-aware
patches. The one-time Preview.37 → Preview.38 transition still requires the older format
understood by Preview.37; its temporary generator is removed after that release is prepared.

The shared client can apply a direct patch or a chain such as `.36 → .37 → .38 → .39`.
It follows the target manifest's published base versions into historical Releases in
`Operit2-nightly`, authenticates each manifest against its GitHub asset digest, and matches
intermediate installers by version, SHA-256 and byte size. Only the same platform,
architecture and installer format can connect. Stable clients exclude preview intermediates.
Discovery is bounded to 64 historical versions and transfers metadata only.

The planner selects the smallest total patch download for the verified original. When a full
installer exists, a chain whose total size equals or exceeds it selects the full installer.
Without a published full installer, a complete chain remains usable. Missing intermediate
Releases or patches cannot form a route; the existing full-package or unavailable state applies.
No schema change or extra full installer publication is required: keep the historical patch
Releases and their manifests available so adjacent deltas remain discoverable.

Each hop downloads and validates one patch, reconstructs and validates its exact target, then
deletes that patch and the previous intermediate. Installation is requested only for the final
verified installer. Download progress counts bytes across the whole selected chain. Failure or cancellation
cleans the disposable job; only the final package is retained on success. Base extraction or
merge/integrity failure records full transfer for that final target version on the next attempt;
cancellation and network interruption do not disable incremental retry.

Local processing progress is reported separately from network transfers through
`FullUpdateProgressEvent::PreparationProgress`. Its phase identifiers are `verifying_base`,
`verifying_patch`, `expanding_base`, `verifying_expanded_base`, `applying_patch`,
`restoring_package`, `verifying_package`, and `saving_package`. Only phases performed by
the selected route are emitted. Portable binary patches share verification, instruction
merging and retention progress; APK and MSI add their Host container operations. Full
downloads also report final verification and retention. The runtime facade carries these
phase identifiers in the existing `stage` field and uses `receivedBytes`/`totalBytes` for
the current phase's completed/total work. No generated transport model changes are needed.

Counters are monotonic within a phase and use actual processed bytes. A zero total means
unknown work and must remain indeterminate. Phase changes and completion are delivered
immediately; intermediate events are throttled to four per second across Host workers.
Each hop starts its own local phase sequence. The Flutter dialog shows a per-step
percentage, phase label and elapsed local processing time, refreshing elapsed time every
second even while a native encoder is busy. Downloading and local processing never reuse
each other's percentage. Neither overall completion nor remaining time is estimated.
CLI/TUI display the same phase counters without showing them as network throughput.

Android reports bytes consumed by entry inflation/deflation plus copied shell bytes.
Windows counts the raw bytes of successfully completed independent CAB folders; its
percentage can pause while one large folder is still compressing. `makecab`'s internal
percentage restarts across passes and is not a valid overall compression counter.
Background viewing, cancellation and explicit installation confirmation use the existing
shared update task. Readiness is published only after verified output is safely retained.

Schema 2 retains the gzip envelope, target/base sizes and hashes, and COPY/ADD instructions.
Opcode 3 carries an old-file range and byte residuals: `new = old + residual (mod 256)`.
This preserves almost-matching code and relocation tables efficiently. The decoder also
accepts schema 1 packages already published. Schema 3 manifests support both the residual
format and the CAB-aware format below. Older clients cannot consume the new schema;
do not replace existing test release manifests or patches before distributing a capable client.

MSI deltas use reversible CAB expansion. The builder reads the compound allocation table,
removes embedded LZX folder streams from the original physical shell, and appends their
uncompressed payloads. It matches these compact images instead of compressed CAB bytes.
The `O2PATCH3\0` envelope binds the original package sizes/hashes, the original expansion
recipe (u32 JSON byte length, then JSON), both expanded sizes/hashes, and the residual
instructions. The recipe is covered by the authenticated patch digest; the client does
not need an MSI parser. Each expanded image contains `O2MSI1\0\0`, a u32 JSON length,
the recipe, the physical shell with recipe ranges removed, and folder payloads in order.

The Windows Host uses the built-in `expand.exe` and `makecab.exe` to decode/recreate the
original LZX streams. It restores original sectors, installer metadata and signed bytes.
All tools run invisibly with literal arguments. Cancellation kills and reaps the cabinet
process. Independent folders run largest-first on at most four workers, capped by the
Host's available CPUs. Each worker has its own temporary cabinet directory and independent
file handles, and writes only its validated, disjoint output ranges. The first worker
failure cancels and joins the other workers before cleanup without cancelling the caller's
download token. Cabinet layout and final installer SHA-256 must match; no alternative package is
substituted. Current support covers non-spanning, unreserved LZX cabinets (15–21 bit
windows), as produced by our WiX configuration. APK uses the entry-aware format described
above; other archive formats retain residual deltas.

For the original Preview.36 → .37 MSI pair (target 123,536,583 bytes):

| Method | Complete downloaded patch |
| --- | ---: |
| Original content chunks | 88,648,183 bytes |
| Residual delta on compressed MSI | 66,121,312 bytes |
| Expanded CAB delta, including reconstruction recipe | **5,451,829 bytes** |

Both Python and the shared Rust decoder with the real Windows Host reconstruct the exact
original Preview.37 MSI (`13dae6e415f088934a5dade5d14dcdb7e0f0c97a3a4de17629b27b875a57ada7`).
The 5,745,337-byte standalone DLL experiment is separate from this complete installer result.
That historical test used serial CAB processing. Scratch expansion needs hundreds of MB
of disk space, is stored under a disposable update job, and is deleted on success,
failure/cancellation or startup recovery. Expanded bases/targets are freed as soon as each
stage finishes. Only the verified original installer is retained as the next update base.

Local Windows debug measurements on the original Preview.40 → Preview.41 release assets,
including exact output-byte verification (2026-11-20):

| Work | Before | After |
| --- | ---: | ---: |
| APK complete reconstruction, cached prior decoder versus batched decoder | 41.29 s | 15.35 s |
| APK merge Host reads, excluding source verification | 213,122 | 10,543 |
| APK merge Host writes | 331,925 | 280 |
| MSI merge Host reads, excluding source verification | 1,622,783 | 22,606 |
| MSI merge Host writes | 3,138,953 | 311 |
| MSI base expansion, serial versus parallel CAB codec | 9.22 s | 2.37 s |
| MSI target restoration, serial versus parallel CAB codec | 103.65 s | 64.82 s |
| MSI complete reconstruction, **both using the batched decoder**, serial versus parallel CAB codec | 119.85 s | 73.59 s |

The unbatched MSI read/write counts come from scanning the published instruction stream;
the earlier complete unbatched MSI runtime was not timed. APK timing includes the production
Android codec running on the Windows test host, not an Android device. APK restoration
still took 10.53 s and MSI merging 5.10 s after these changes. A single large DEFLATE/LZX
stream must retain its original encoder and parameters; independent-folder concurrency
does not parallelize within that stream. No installation, package build or publication
is part of these measurements.

Further profiling of the same Preview.40 → .41 assets identified the remaining work:

| Container | Largest independent stream | Raw bytes | Isolated stream restoration |
| --- | --- | ---: | ---: |
| APK | `lib/arm64-v8a/liboperit_flutter_bridge.so` | 161,796,232 | 5.61 s |
| APK | `lib/arm64-v8a/libapp.so` | 20,775,856 | 1.00 s |
| MSI | CAB folder 1, including `operit_flutter_bridge.dll` (188,203,008 bytes) | 208,085,361 | 82.81 s |
| MSI | CAB folder 0, including Flutter/FFmpeg and the executable | 42,241,968 | 18.93 s |
| MSI | CAB folder 13 | 35,888,762 | 15.20 s |

These are separate diagnostic runs, not new end-to-end client timings. The APK experiment
reads each changed entry through Python's ZIP reader, recompresses with standard zlib level 6,
and compares its compressed length and SHA-256 with the target stream. Its five entries take
7.23 s in total; the bridge library accounts for 78%. The MSI experiment schedules four
workers largest-first using the same Windows cabinet tools and parameters as the Host,
checks each restored compressed stream's SHA-256, and takes 84.01 s including expansion.
The largest folder alone takes 82.81 s, setting the critical path. Machine load and the
different harness prevent comparing these absolute numbers directly with the earlier
client timings. Additional workers cannot split either container's largest existing stream.

Thirteen of the target MSI's seventeen folders, totaling 21,981,862 raw bytes, have exactly
the same compressed-block SHA-256 as folders in the base. The current generator still
expands them all. Keeping identical compressed streams in the shell would avoid redundant
expansion/restoration in future patches. This is a generator optimization opportunity,
not an optimization applied to the published assets or measured end-to-end above. It
does not remove the changed 208 MB folder from the critical path.

Instruction replay also quantifies the source-read tradeoff:

| Input | Requested COPY/residual bytes | Current 64 KiB gaps: reads / bytes | 16 KiB gaps: reads / bytes |
| --- | ---: | ---: | ---: |
| APK | 286,658,279 | 10,543 / 864,155,559 | 22,629 / 442,819,285 |
| MSI | 286,420,976 | 22,606 / 1,510,341,432 | 47,272 / 673,044,170 |

Smaller gaps reduce read amplification while increasing Host calls, so read counts and byte
totals alone do not establish a faster setting. A simulated 4 MiB LRU cache with fixed
64 KiB, 256 KiB, or 1 MiB pages increases physical bytes for both published instruction
streams; it is not used by the merger. Substantial further wall-time reductions require
avoiding expensive match search when reconstructing the largest compression stream;
changing its compression level or arbitrarily splitting it cannot preserve signed bytes.

The shared Rust integration suite covers tiny reordered/overlapping COPYs, wrapping
residuals, literal data, large operations split across windows, bounded physical I/O,
mid-read cancellation, and more than four million valid instructions. On Windows it
also exercises the real CAB codec with eight independent folders and worker failure
cleanup. Progress tests run through public update execution, including the production
APK/CAB codecs, and assert phase order, monotonic counters, completion before readiness
and cleanup after cancellation from a local progress callback. The `Incremental Update Merge`
workflow runs this suite on Linux and Windows and runs focused Flutter tests for phase
transport, progress presentation, narrow layouts and controller timer disposal.
Real-release tests remain opt-in: provide `OPERIT_APK_TEST_{MANIFEST,BASE,PATCH,TARGET}` or
`OPERIT_MSI_TEST_{MANIFEST,BASE,PATCH,TARGET}` and run the corresponding ignored
`apk_real_release_round_trip` or `msi_real_release_round_trip` test with `--nocapture`.

The application observes normalized Host network events. Automatic preparation allows any
connected network, including cellular, when the selected transfer totals less than
30 MB (30,000,000 bytes). Incremental routes use the total patch bytes; full routes use
the full installer bytes. Transfers at or above this threshold, or of unknown size,
require Wi-Fi or Ethernet with a known non-metered connection. It downloads and verifies
a patch when the original is local, or a full installer when published and no original is
available. Patch-only updates without an original remain available for manual selection.
Changing to an ineligible network cancels an unattended transfer. A verified package is
retained for reuse after application restart, including as the next version's baseline.
Preparation never invokes installation; the user must confirm in the update dialog.
