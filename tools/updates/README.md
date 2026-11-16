# Incremental update packages

`prepare.py` creates direct, byte-exact deltas between original installers. The matcher uses
[HDiffPatch 5.1.3](https://github.com/sisong/HDiffPatch/releases/tag/v5.1.3) for matching.
The build tool downloads the pinned generator with `gh` and checks its SHA-256. End users
need no external patch program. The shared Rust decoder uses Host file ranges and
transactional VFS writes, with at most 1 MiB per data buffer.

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
verified installer. Progress counts bytes across the whole selected chain. Failure or cancellation
cleans the disposable job; only the final package is retained on success. Base extraction or
merge/integrity failure records full transfer for that final target version on the next attempt;
cancellation and network interruption do not disable incremental retry.

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
process. Cabinet layout and final installer SHA-256 must match; no alternative package is
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
Recompression took about 100 seconds locally; the Rust debug integration test took about
147 seconds including hashing, patching and cleanup. Background preparation can absorb
this work before the user confirms installation. Scratch expansion needs hundreds of MB
of disk space, is stored under a disposable update job, and is deleted on success,
failure/cancellation or startup recovery. Expanded bases/targets are freed as soon as each
stage finishes. Only the verified original installer is retained as the next update base.

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
