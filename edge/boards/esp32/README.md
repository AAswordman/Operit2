# ESP32 Edge firmware

Space responsibilities are defined in [Edge architecture](../../docs/architecture.md).
The [bounded Space and display contracts](../../docs/core-entry-protocol.md)
keep Peer and Space authority on each node. Core projects business messages into
bounded display text; ESP stores its own identity, pairing, Space control and native
configuration. The device does not run tools or interpret tool result/status data.
This document covers the ESP32-2432S028 implementation and firmware build.
Tooling and acceptance commands are maintained in the
[editor README](editor/README.md).

The ESP32 advertises `_operit-edge._tcp` over mDNS. Use the Core application's
device scan to discover it, start pairing, and enter the code displayed on the
ESP32. LAN discovery finds local devices; a remote Edge can also be paired by
its reachable address. Opening the browser editor automatically starts a
separately identified `esp32-edge-simulator`.

The firmware owns native device services. `edge.status/read` returns board id,
expression, IP address and Wi-Fi name over the paired Link. The pairing code is
excluded from service results. ToolPkg runs on a phone/desktop as an optional
configuration editor; it is not an Edge runtime dependency.

## Local configuration

Edge pairing uses the six-digit code shown on the receiving device, for both
serial and network connections. No Edge token, `.edge-token` file, or
`OPERIT_EDGE_TOKEN` environment variable is needed. Legacy token settings are
ignored; the shared listener configuration no longer contains a token field.

Configure Wi-Fi from the setup page or with `OPERIT_WIFI_SSID` and
`OPERIT_WIFI_PASSWORD`. Pairing still verifies the devices and shared keys;
joining a Space remains a separate, approved action. A Core receiving a remote
pairing uses the same code-confirmed shared handshake.

Use the ESP32 toolchain and partition table documented in the repository's
`BUILDING.md`. Select the actual serial port for your device rather than assuming
the port in a developer's local configuration. Preserve existing Wi-Fi/NVS data
when updating only the application image.

## Independent operation and device preferences

An ESP admitted to another Space receives the durable `edge` identity. It does
not need `storage.provide` to synchronize its own small preferences or send chat.
That capability controls full business replicas, not local storage or operation.

The native application chooses its data: encrypted configuration persists;
chat context is an eight-message / 6144-text-byte RAM window, with at most 2048
bytes per message. No chat database, ToolPkg or business replica journal is
created. Reboot retains configuration and starts a fresh local conversation.
Other members disconnecting does not clear a direct conversation in the same
Space. A Space change retires the old context before validating new settings,
even when both Spaces use identical model configuration. Late responses from a
retired context cannot enter the new conversation.

Set the device's `chat` preference with `endpoint`, `model`, optional `apiKey`
and `systemPrompt`. The endpoint is a complete OpenAI-compatible streaming chat
completion URL reachable **from the device**. Firmware sends HTTPS/HTTP itself
through its Host provider and renders SSE replies; it does not call Core or
require a Binding. The HTTP worker verifies HTTPS using the ESP-IDF certificate
bundle and keeps I/O off the UI task. Internet/service availability is still
required; this does not install an on-device language model.

Use `space preference-set <esp-id> chat <JSON>` and `space sync` in CLI, or call
native `edge.preferences/read|set|remove` through the existing generic device
API from a configuration ToolPkg. Preferences are scoped to the target device
and Space, at most 24 keys and a 6144-byte plaintext file. The protocol exchanges
only the current per-key state and deletion markers, not unbounded history.
See [CLI commands](../../../docs/cli-network-control.md).

Without a `chat` configuration, the optional remote Core mode below remains
available. Existing STT/TTS Stream/ReverseStream processing is retained. The
ESP32-2432S028 firmware includes the board audio adapter described below.
Independent text chat does not imply a connected STT/TTS processor or verified
physical voice I/O.

## Memory and connection lifetime

- The firmware's asynchronous Edge worker uses a 32 KiB stack; the previous
  6 KiB stack overflowed while processing pairing requests. The separate
  blocking worker reserves 24 KiB, leaving 8 KiB more internal heap for node
  state and Link operations. Health diagnostics expose the responding task's
  stack high-water mark for hardware verification. The main UI task reserves
  16 KiB for synchronous authorization and storage decoding; the previous
  12 KiB reservation overflowed on the constrained-cache read path.
- The display frame loop reads scene activity and revision as scalar fields,
  avoiding a temporary JSON object on every frame while admission is running.
- Idle UART input and TCP accept poll every 10 ms, reducing wakeups on the
  shared worker so FreeRTOS idle work can run. Active byte-stream backpressure
  keeps the existing shorter retry interval.
- The optional RGB332 diagnostic mirror is disabled to leave heap space for
  pairing. The physical TFT remains enabled.
- Runtime NVS writes stage one changed record and stream unchanged records from
  a compact sorted index. They do not clone the full index during Space admission.
  Index growth is reserved before writing; the in-memory change becomes visible
  only after the alternate bank is published. Failed writes preserve the old
  snapshot. The ONV3 envelope and 7000-byte logical storage limit are unchanged.
  Packed records now use format 2, with an immutable 847-byte dictionary of
  public JSON field names stored in firmware Flash. The dictionary does not
  consume a separate heap buffer. Legacy raw (0) and LZSS (1) records remain
  readable. Opening storage compacts legacy records one at a time in memory;
  the next successful ordinary transaction publishes the new representation.
  A failed transaction keeps the previous bank authoritative. Decoded record
  contents, identities, pairing secrets and Space policy remain unchanged.
  Firmware predating record format 2 cannot read newly published snapshots;
  preserve a matching firmware/NVS backup when testing a downgrade.
  Node-local edits move unshared parsed records into the transaction instead
  of cloning retained JSON trees. A weak mutation witness detects unchanged
  edits without pinning the old tree; external immutable snapshots still receive
  copy-on-write protection. Failed edits reload the unchanged durable snapshot.
  Records serialize by reference with unindented JSON on constrained node-local
  stores to avoid large temporary formatting buffers; desktop formatting remains
  unchanged. Constrained serialization measures the complete encoding first and
  fallibly reserves its exact size once, avoiding simultaneous old/new buffers
  during growth. Capacity failure returns an error before NVS publication.
  Edits and record replacements stage schema migration in the same atomic write,
  so a failed first publication cannot leave a schema-only record behind.
  Background approval polls persist only reconciliation changes; completed or
  otherwise unchanged requests are not rewritten on every poll. Constrained
  gateways retain two recent terminal application receipts, pruning older ones
  before admitting more work. Pending, approving and approved handoffs remain
  durable. Full Core keeps its complete application history. Record deletions
  use the same transaction and atomic publication as record updates.
- The NVS Host opts out of retaining parsed preference caches after their last
  store handle closes. Node-state reads also release the shared decoded cache
  when returning an immutable snapshot, so long-lived policy services cannot
  pin large JSON trees. Committed and unchanged node-local edits release the
  cache as well, including while a store handle remains open. Concurrent handles
  still share a transaction lock; subsequent reads reload durable bytes from
  the compressed NVS cache.
  During node-local updates, the transaction releases the old decoded cache
  before serialization. Failed publication invalidates that cache so every live
  handle reloads the unchanged durable snapshot; existing immutable snapshots
  remain valid. Temporary policy Stores also honor this memory policy.
  Desktop storage keeps its existing persistent cache policy.
  Approval and policy replay scans validate the current node-state schema and
  decode typed records
  directly from durable bytes, without an intermediate JSON tree or large tree
  nodes for typed application records. Single-record reads skip unrelated
  payloads, including retained join history. Single-record updates borrow the
  unchanged JSON payloads and serialize the replacement directly, reserving the
  measured output size once. They retain the shared transaction lock and atomic
  publication boundary; failed writes leave all durable records intact. Whole
  node-state replacements validate the old schema and retain replication metadata
  without expanding the old business payloads. Policy projection tracks inserted
  commands instead of retaining a second full JSON snapshot for comparison.
  Immutable control imports borrow the old encoded map and decode existing
  commands one at a time for equality. Authenticated endpoint imports also
  compare old commands one at a time while holding the incoming batch, and
  release validation state before publication. Every command and duplicate in the batch
  is validated before one atomic publication; unchanged syncs do not write NVS.
  Older schemas still use the existing migration transaction; newer schemas are
  rejected. Obsolete Space policy caches are validated record by record before
  any deletion; current Space authority is retained, including on retry after
  a failed deletion.
- Constrained receiving endpoints retain the source membership, profiles and
  topology needed for live consent and approval recovery. After first receiving
  and observing source departures, they omit the duplicate source command log
  from the durable application. Applicants keep their original submission for
  retries, even if a gateway returns a compacted record. Terminal endpoint
  receipts retain identity and status without an obsolete accepted snapshot.
  Review polling compacts legacy receipts before growing recovered membership;
  the current authority log and active consent remain intact.
- Node-local member retirement releases obsolete profile and topology records
  before rewriting the remaining membership revision. Its membership marker is
  removed last so failed cleanup can retry. Startup completes persisted removal
  effects from the authority log even if the departing peer is offline. Full
  Core membership tombstones and history retain their existing behavior.
- Constrained Hosts serialize policy materialization across temporary store
  instances, preventing UI, approval and authenticated transport checks from
  simultaneously retaining complete replay working sets. Authorization replay
  borrows encoded payloads from an immutable byte snapshot and decodes commands
  directly, avoiding a generic JSON value tree during replay; full snapshot
  exports decode projection entries directly into their final operation vector,
  avoiding a second collection of decoded payloads. Constrained replay/export
  vectors reserve their known entry bound without geometric growth; reservation
  failures return an error. Immutable conflicts,
  legacy admission compatibility, ordering and revocation checks remain enforced.
  UI connection indicators poll with the existing 500 ms route refresh. While
  an admission or pairing UI operation is busy, background route, settings and
  pairing-indicator refreshes keep their previous display values instead of
  allocating another working set. Each actual routed operation still validates
  current authorization.
- Device Space UI workflows run on the blocking worker. A device-local async
  permit covers initialization, review, pairing, joining and local exit, including
  their network waits. Periodic preference synchronization takes that same
  permit only when available and skips busy ticks, avoiding a second authority
  working set during approval. Display refreshes also defer while it is held.
  Shared peer connection, availability, inbound request dispatch and reply
  delivery remain independent so local workflows can await network responses.
  Approval polling validates historical receipts one at a time, retains only
  small summaries, and reloads live requests individually. Completed history does
  not accumulate in a second typed collection during reconciliation; malformed
  entries still fail validation before pruning or other writes.
- Constrained Edge snapshot and control-exchange replies serialize typed
  authority records directly into the existing standard Call response. Each
  encoded operation releases its payload before the next operation is written;
  the serializer runs once and preserves the standard map/sequence format.
  Snapshot import also releases each accepted control payload before the next
  durable operation, after validating the complete incoming policy.
  Incoming Space snapshots and control batches decode
  their queued envelope with the existing JSON record decoder, then consume it
  into typed authority records. Operation payloads move unchanged into those
  records, and the consumed input frame is released before policy replay or
  publication. This avoids an intermediate Link value tree and reuses the
  durable decoder to fit the application partition. The shared channel still validates current
  authorization before execution and transmission; the prepared snapshot path
  retains the ordinary destination, route-kind, direct-origin, TTL and pairing
  checks. Control exchange also retains current Space scope and admitted-member
  checks, and shares the ordinary authority import and removal effects. Other
  handlers use the ordinary dispatcher.
- Routed Call/Watch/Push envelopes move the owned argument subtree through
  serialization and parsing. Only the small route metadata is reconstructed,
  including for MessagePack struct sequences; metadata validation and correlation
  checks still run before dispatch. The existing envelope encoding is unchanged.
- Authenticated requests re-read only their current session credential, using
  direct typed decoding on the constrained Host. This avoids copying all credentials
  while preserving authorization checks after pairing revocation.
- Constrained Hosts retain standard MessagePack bytes in pending inbound and
  worker queues. The authenticated reader inspects only Call correlation and
  control fields, skipping the business payload; queued Calls expand on their
  execution worker and Call replies expand at their waiting consumer. Heartbeat
  controls still bypass business dispatch. Completed replies are encoded before
  the final authorization check and transferred to the
  encrypted writer without rebuilding a value tree. Calls keep independent
  execution, while Watch/Push ordering and the cipher lock across each response
  batch remain intact. Credential revocation is checked at the existing entry
  and transmission boundaries. Desktop request queues keep their value storage.
- Peer transport decoding reads the existing request/response envelopes directly,
  avoiding the generic untagged-union buffer for encrypted payloads. Owned byte
  frames retain binary fields as shared slices of the received allocation;
  borrowed decoding still copies. The explicit decode scope restores its parent
  on nested decoding and failure, and releases its source when decoding ends. Decryption
  reuses uniquely owned ciphertext and copies shared buffers before mutation.
  Departure exchange releases the remote snapshot before constructing the local
  reply. Space snapshot replies consume each authority payload while building
  the Link value tree, instead of retaining two complete decoded copies. Control
  exchange drops the received operation vector before building its response.
  Snapshot reconciliation reads local membership without exporting a second
  local snapshot and releases received authority payloads before replaying local
  removal effects. Exchanges without remote leave/remove commands skip that
  otherwise redundant removal replay. Policy validation sorts references to
  received operations rather than cloning their payloads. ESP-IDF MessagePack
  output grows in bounded blocks while the source tree is alive, avoiding a
  second large allocation during buffer doubling. Serialization still runs
  once, including stream source capture. Wire encoding, nonce checks and
  authorization remain unchanged.
- Device UI watch subscriptions open one at a time. Their boxed route and
  authorization futures are constructed after the opening permit is acquired;
  established streams still receive independently. Waiting opens can cancel
  without sending a request, while an in-flight acknowledgement always finishes
  before its stream is closed.
- Native Core requests share a process-lifetime Tokio executor so the saved
  pairing socket and background receivers survive between commands. The Rust
  DLL requires a native rebuild and application restart, not Dart hot reload.

## Optional remote Core chat routing

After admission, the shared Edge return client sends annotated calls and watches
through the adjacent Core's Binding resolver. Resolved execution remains in the
existing Space route namespace; plugin execution and permission checks stay on
Core. Embedded streams reopen through their annotated source and source arguments,
not as ordinary chat properties. Push sessions retain their existing router path.

The first session generates a volatile chat ID and calls the existing explicit
`ensureRoutedChat` creation route. Core selects an authorized executor and owns the
Binding and chat metadata. Message/state subscriptions and sending are enabled
only after successful creation acknowledgement. Read and send operations never
allocate a missing Binding implicitly.

Firmware and simulator share entry-identity polling. Replacing the Core entry or
its admitted session reopens subscriptions even without an observed offline gap.
The selected chat ID survives a reconnect within the same Space, but is reset
when the Space changes or the device explicitly leaves/unpairs. Closed primary
watches and unexpectedly closed embedded streams trigger bounded subscription
recovery; user sends and non-idempotent new-chat requests are never replayed.
Only bounded display data and the selected ID live in volatile Edge memory; no
chat business replica is persisted on the device.

## Verification and current limitations

The fixed UI provides pairing-code input, status, Space admission, a bounded
chat view and conversation metadata list, plugin diagnostics, unpair
confirmation and local Space-exit confirmation. Text drafts can be supplied by
the host/editor; a full on-device text keyboard is not implemented. Images use
text placeholders on the physical device, and arbitrary dynamic layouts are
not supported.

The simulator compiles actual firmware modules and runs protocol/session tests;
C/Wasm tests exercise the same renderer. Build the preview before those tests.
The hardware smoke tool requires an already paired/admitted device and does not
complete first-pairing or full-chat acceptance. See the editor's verification
commands and distinguish simulator, build and physical-device results.

### Build and simulator verification (2026-10-28)

The Edge changes target upstream `review/pr356-edge-space-sync-memory` at
`269d910d`. After rebasing the remaining device changes onto that dedicated
branch and consolidating repeated node-local schema checks, the complete
`npm run test:device-budget --prefix edge/boards/esp32/editor -- --firmware` gate passed:
firmware and Xtensa ABI checks, shared C/Wasm UI, TypeScript, fixed heap/stack
probes, and three authenticated TCP scenarios covering pairing/admission/restart,
standalone chat across restart, and remote chat with storage recovery and speech.
The standalone scenario peaked at 208,224 / 262,144 Rust heap bytes; remote chat,
storage recovery and speech peaked at 212,840 / 262,144 bytes. Every sampled state
reported zero allocation failures. Desktop regression passed 61 simulator tests,
29 board tests, 116 editor tests and eight serial debug tests.

The built image uses **3,945,104 / 3,997,696 bytes** (**52,592 free**).
Its `.flash.text` uses **3,324,810 / 3,342,304 bytes** (**17,494 free**): code mapping
and image partition capacity are separate limits. This image has not yet been
flashed for another hardware acceptance run.

Speech integration verified 65 seconds of PCM/WAV byte volume, recognition text
over 8 KiB and ordered multi-segment TTS with deterministic providers. These
checks cover transport behavior; physical long recording and actual recognition
quality require hardware validation. Simulator heap/stack measurements exclude
ESP-IDF, Wi-Fi, TLS and native driver overhead.

### Hardware verification (2026-10-28)

The earlier hardware-tested image preserved identity, pairing, Wi-Fi and the
existing Space. With the [audio wiring below](#native-inmp441-and-onboard-speaker),
the user confirmed an audible test tone. Duplicate-start rejection, intentional
stop/restart and 20 consecutive short self-tests passed, with 44,544–46,080
captured left-channel samples per test and no RX/queue overflow, observed reset
or recorded allocation failure. Lowest free heap was 16,684 bytes; free heap
after tests was 70,456–75,768 bytes. Capture resources were released on close.

Those timed self-tests precede the WAV/PCM layering cleanup and upstream rebase.
They establish short acquisition and restart behavior, without verifying queued
recording tails, microphone acoustic quality, physical long recording or a Core
STT/TTS round trip. The board and Windows Core had different Spaces; the existing
Windows Space was preserved. A successful DMA write alone does not establish
audible playback, and nonzero microphone samples do not establish recognition
quality. See the [current chain and acceptance scope](../../docs/core-entry-protocol.md).

## Recovering from an unavailable Space administrator

Pairing and Space membership are independent. Unpairing does not silently leave
or merge a Space. On the device, open **Settings → Device Space → Space and
pairing management → Leave current device Space → Confirm** to leave an
unavailable old Space. This uses the shared local Space-exit transition, creates
a singleton policy immediately, and retains identity, Wi-Fi and direct pairing.
Obsolete local review records and completed applications from the abandoned
Space are retired. Explicit exit also removes the unused `space_join_*`
approval files left behind by older firmware; they are not parsed or migrated.
No business files or full Core policy audit are erased. Non-replicating
endpoints discard obsolete Space membership/profile/topology caches and policy
projections for retired Spaces, while preserving current authority and pending
admission recovery. Space snapshots include only current members'
profiles/topology, leaving unrelated Core history local.

If the new Core already has a pending application to the old Space, cancel that
application and submit a fresh one after the exit. Re-pairing alone does not
transfer the old administrator's authority. The new application still needs
explicit approval on the device. Space exit is a local UI action, not an RPC
that an unadmitted Core can use to reset another device.

## Chat display windows

Conversation history remains on Core. The device uses the generic
`chatMessagesWindowFlow` watch with a 640-byte visible-text budget, 15 hard text lines, and at most
12 message rows per window: roughly three screens, depending on text wrapping.
The separate line budget also bounds messages made of many very short lines.
Only presentation fields and live response streams are transferred, not token
statistics, raw tool results, or a business-storage replica.

Swipe down to read older text, or up to return toward the live reply. Scrolling
inside the cached window is local; crossing a window boundary requests another
window by message timestamp and UTF-8 text offset. A long single reply can be
paged in full. Each new window replaces the old one in RAM. Live replies update
incrementally and retain a bounded tail instead of freezing at the text limit.
Chat text/windows are never written to NVS.

The display has a 1024-byte static text buffer for the 640-byte payload plus
sender labels. Cursor history is bounded; it does not retain past page text.
UART `device:screen` exposes `chatCachedBytes`, `chatCachedLines`, and
`chatScrollLine` for checking the real device without a screenshot request.

### Conversation metadata

The list subscribes only when opened and retains at most 24 metadata entries.
The C UI shows three rows per page. Selecting an entry changes only this Edge's
view, not the Core application's selected conversation. Viewing and display
caches remain volatile; no conversation database is replicated to the device.

### Tool-call cards

Chat tool calls use a compact, two-row wrapper with the existing plugin icon.
Only the tool name and invocation state are painted: 调用中 (amber), 成功
(green), 失败 (red), or 未完成 (muted). The package namespace is omitted from
its painted name; arguments and raw result bodies stay on Core. Ordinary model
reply prose is still displayed normally, including summaries of tool results.

The chat-owned projection pairs persisted results by call ID and emits bounded
private presentation tokens in the existing display text/stream. They do not
expose a plugin invocation endpoint or change Binding permissions. Tokens are
atomic across Core history pages; clipped live-tail tokens are discarded. Live
status events replace a pending wrapper rather than adding a second result
message; reset/rollback and the final persisted snapshot restore its status.
Cards share the normal chat scroll and require no new static tool cache or UI
heap. `device:screen` / simulator inspection include visible `toolCards` with
name, status and the plugin icon for verification.

## UI backend

The only renderer is `ui_port/operit_mini_ui.c`. Rust uses `src/ui.rs` and the
`operit_ui_*` C ABI; board display ownership and RGB565 flushing use
`activateUi` / `flushUi`. There is no third-party widget runtime, allocator
pool, backend marker or fallback build. Native firmware and Wasm compile the
same C sources.

LVGL is no longer a firmware or preview dependency. The Cargo component list,
`components_esp32.lock`, SDK defaults and `ui_port/CMakeLists.txt` select only
the self-drawn UI. Optional plugin scenes use `operit-edge-scene`; the device
retains the system bar and exit action. Scene rendering does not make Host
responsible for plugin business logic.

An existing ESP-IDF build cache can still contain `managed_components/lvgl__lvgl`
and `build/esp-idf/lvgl__lvgl` or `lvgl_port` archives from earlier firmware.
Those generated files are not dependencies in the current build graph. To
remove the old component downloads and archives, clean the generated firmware
target directory, then rebuild. This also removes the cached firmware ELF and
other build artifacts; it does not erase or flash the device:

```powershell
cargo clean --manifest-path edge/boards/esp32/Cargo.toml --target-dir D:/esp32
npm run build:firmware --prefix edge/boards/esp32/editor
```

The editor uses an `esp32` target directory at the root of the repository's
drive; adjust `D:/esp32` to that location, or to your direct Cargo build's
custom target directory. A package-only clean may miss outputs from retired
build fingerprints. ESP-IDF's bundled examples may still mention LVGL;
those SDK examples are not linked into this firmware.

The 320×240 renderer uses two-row RGB565 strips (1280 bytes). A compile-time
assertion limits its static state, strip and inspection buffer to 10 KiB.
This is not a whole-board RAM measurement. The browser framebuffer and the
Rust runtime/network allocations are separate.

```powershell
npm run build --prefix edge/boards/esp32/editor
npm test --prefix edge/boards/esp32/editor
npm run build:firmware --prefix edge/boards/esp32/editor
```

Preview builds require Node.js and Emscripten but no firmware build cache.
Firmware builds also require the ESP32 Rust/ESP-IDF toolchain and espflash.
Building does not flash the device. `npm run dev` builds and flashes; updating
with `npm run flash` preserves NVS and the reserved layout partition.

## Xtensa build overlays

The firmware build prepares checksum-pinned Tokio 1.53.1 and ring 0.17.14
sources under the ignored `.embuild/` directory. The scripts are
`edge/boards/esp32/editor/prepare-tokio.py` and `prepare-ring.py`; they never modify
Cargo's shared registry cache. The original dependency licenses and copyright
notices stay in the extracted sources.

The ESP LLVM 21 windowed backend can address incoming stack arguments
incorrectly after realigning the stack beyond the Xtensa 16-byte ABI. The
Xtensa-only overlays reduce Tokio's performance padding and ring's scalar
Poly1305 padding to that alignment. The ring Rust/C layouts are changed
together; cryptographic arithmetic, keys and wire protocols are unchanged.
These are dependency-source overlays, not an unmodified upstream build, and
must be reviewed when updating the pinned versions or compiler. Desktop
builds do not use them.

The firmware release uses Fat LTO and size optimization across its Rust packages
to fit the existing 4 MB board and application partition. ESP32 console timestamps use direct UTC clock fields;
desktop logs retain their local-time formatter. A single codegen unit keeps the
Windows linker command within its length limit. UUID retains `opt-level = 1`
for the Xtensa hex-parser workaround. Tokio now uses `opt-level = "z"`, with
its pinned alignment overlay and final ELF ABI checks still required.

Peer transport decoding selects the existing Call / Watch / Push envelope with
the shared constant-stack field scanner, then decodes its typed body. Owned
frames retain binary fields as shared ranges. Compatibility tests compare every
request lifecycle and success/error response against the original Serde codec,
including rejected positional struct variants. Serializer/decoder entrypoints
remain out of line so Fat LTO can share each instantiation across callers.
The built-in font retains all 4,168 glyphs and bitmap bytes in packed nine-byte
records; table tests check each field width and bitmap boundary.

The ESP32 standard library uses `backtrace-trace-only`: panic diagnostics and
raw backtrace addresses remain available, while symbol lookup runs on the host
with the matching ELF. This avoids linking the device-side DWARF symbol reader;
it does not disable panic reporting or the ESP-IDF abort backtrace. Keep the ELF
from each flashed build for `xtensa-esp32-elf-addr2line -f -C -e <elf> <addresses>`.

The ESP32 Host scheduler enables Tokio's `rt` executor support without
`rt-multi-thread`: firmware drives current-thread runtimes on its bounded Host
workers. Non-ESP32 platforms retain the multi-thread feature and the upstream
worker naming behavior. Keeping this dependency split explicit prevents desktop
executor code from consuming the ESP32 code-mapping budget after shared Host
changes; it does not add another scheduler or change device task admission.

On Xtensa, the overlays keep the Tokio constructor and ring seal/open as named,
non-inlined functions so LTO cannot erase the ABI inspection boundaries. These
attributes do not change their Rust signatures or cryptographic operations.
The prepared ring files are checked against the expected pinned patch, including
these boundaries; edited or incomplete overlays fail the build.

`npm run build:firmware` runs `check-xtensa-tokio.py` against the final ELF to
check those functions' stack frames and incoming stack argument offsets. Missing
symbols, stack realignment and incorrect offsets remain build failures.
A successful build alone is not a replacement for hardware pairing and chat
regression tests.

## Third-party font

`ui_port/mini_font.h` contains standalone Noto Sans CJK SC Regular bitmap and
glyph tables: 14 px text covering ASCII, GB2312 first-level Chinese and
punctuation, plus 28 px digits. No widget-library callbacks or headers are
required. Adobe copyright and SIL Open Font License 1.1 are retained in that
file and `ui_port/OFL.txt`.

## Reserved layout storage

The layout Flash partition remains at its original address, so normal firmware
updates still preserve existing data. The unused layout loader, slot writer and
C descriptor generator have been removed; the fixed renderer neither loads nor
writes these packages. `/ui/capabilities` reports `dynamicLayout: false` and
`imagePreview: false`. Editor Wi-Fi/USB layout deployment is rejected before
contacting the device, while independent project editing/export is retained.

Image messages use a visible text placeholder. No dormant image decoder,
preview subscription or image-chunk UI ABI remains in the firmware.

## Core plugin page

The fixed plugin screen reads a six-item, two-column page from the current
chat's execution Core using read-only Binding routes. Only enabled ToolPkg
containers are listed; refresh and previous/next page actions never enable,
disable or install a plugin. Plugin summaries and test results are volatile.

The white lightning button at the top right runs connectivity tests for **all**
enabled containers, paging six at a time and executing sequentially. A plugin
cell opens a modal with separate connectivity and tool-call tests. Connectivity
executes that container's `test_connection` export in the existing Core ToolPkg
main runtime. Tool testing executes its `test_tool_call` export; the daily-life
plugin exercises `daily_life:get_current_date` through the existing tool dispatcher.
UI/prompt-only plugins explicitly report that they have no business tools.
Missing exports, failed tests, disabled packages and exceptions are failures,
never fallback successes based on file readability.

The diagnostic route and read-only list use the authenticated Space and chat Binding.
There are no separate chat read/write permissions. Edge cannot choose an
arbitrary function/tool name or arguments. Only bounded success/message data
returns, not tool payloads or JavaScript stack/source. Green/red numbers are the
Edge-to-Core round trip **including that test function**, not a pure network
ping or a guarantee that every plugin tool can run. Core's diagnostic timeout
is 6 seconds; Edge's UI deadline is 8 seconds and drains outstanding Link
acknowledgements without cancelling an ambiguous transaction. No timed-out or
retired session/page/probe result can overwrite a newer attempt. Batch runs are
not automatically replayed after reconnect. Results remain volatile.

### ESP32 plugin browser

The shared device/WASM screen lists enabled Core ToolPkg plugins in two columns,
with **专属 / 一般** tabs. General is selected by default. A ToolPkg manifest may
opt into the exclusive category with `"edge": {"exclusive": true}`; without this
explicit marker it remains general. This is display classification, not a new
execution runtime, permission grant, or a native ESP32 plugin implementation.
The marker applies to all Edge platforms; an old `esp32` marker must be renamed
to `edge`. Device telemetry/debug services are not listed as user plugins.

Selecting a package reads its ID, localized description (192 UTF-8 bytes maximum),
and active tools (three names per page, 64 bytes each) through the chat's existing
Core Binding. Long details can be scrolled; tool pages use 前组 / 后组. Empty tool
lists omit the tools section and tool-test button. The white top-right lightning
sequentially tests connectivity for all enabled plugins in the selected category.
Individual tests continue to run the existing fixed Core diagnostic exports;
no scripts, arguments or result payloads are copied to the device.

## Device-space settings

All pairing controls live under Settings → Device Space. The Edge can discover
nearby nodes, initiate pairing, enter the receiver's six-digit code, display its
own incoming pairing code, and review assigned Space join requests. Management
provides outgoing join/cancel, local Space exit and one-sided local unpair.
Incoming prompts remain available in settings without taking over chat or a
plugin scene. Wrong codes can be edited and retried within the service's attempt
and expiration limits; network work runs on the existing Host scheduler.

The firmware and desktop simulator compile the same `device_space_ui.rs`
controller and the same fixed C renderer. Pending outgoing consent survives
restart through the node service's existing records, and approved membership is
refreshed automatically. Pairing, approval and membership remain separate.
A retained remote pairing is replaced only after a fresh code succeeds; unpair
itself remains local-only. No new Link wire encoding, transport or Host API is
introduced.

### Long input and speech transport

The C draft remains a 511-byte UTF-8 display/input buffer. The browser owns its
complete draft separately (up to 16,384 bytes); native input without that owner
rejects overflow explicitly. Remote chat sends above 2,048 bytes use the existing
Link Push / ReverseStream transport, in UTF-8 chunks of at most 2,048 bytes. Core
validates length, explicit commit and EOF, then delegates to `sendUserMessage`.
Incomplete input never submits its received prefix. This requires the matching
Core route. Edge retains a bounded temporary input string, not a persisted full
transcript. Direct model input retains its existing 2,048-byte limit.

Speech capture and playback use four-slot queues of at most 4,096-byte audio
blocks. Capture awaits capacity and invalidates outstanding sends on disconnect
or cancellation. Speech protocol v2 also returns recognition text in 2,048-byte
chunks; only the final result publishes the complete bounded draft (16,384 bytes
maximum). The Core example processor owns up to 300 seconds of PCM and WAV
assembly. Device TTS input is limited to 4,096 UTF-8 bytes and Core calls the
existing provider in text segments of at most 1,024 bytes. Provider recording
limits and actual recognition/playback quality remain separate from transport
validation. The native adapter below is specific to the documented wiring;
transport tests do not substitute for physical recording/playback acceptance.


## Native INMP441 and onboard speaker

The board audio adapter uses the existing speech protocol, not a second speech
service. The supported wiring is INMP441 VDD=3.3V, GND=GND, L/R=GND (left),
SCK/BCLK=GPIO27, WS=GPIO22, SD=GPIO35. GPIO21 remains the display backlight;
do not connect the microphone clock there. The passive speaker uses the board's
two-pin amplified speaker connector driven by DAC GPIO26.

Capture runs on I2S1 at 16 kHz, with 32-bit stereo slots (64 bit clocks/frame).
Only the left slot is converted to mono PCM16, with DC removal, bounded gain and
saturation. A priority-10 native task drains DMA independently of Host workflows.
Its fixed 32-block queue retains 512 ms of high-16-bit mono input and raw signal
statistics in 16,768 bytes; the original PCM filter uses exactly those same high
bits, so conversion is unchanged. Its 3 KiB stack, queue and scratch storage are
released on close. Four DMA descriptors of 256 stereo frames provide a fixed
8 KiB / 64 ms hardware ring. The ISR and its callback remain in IRAM during
Flash cache suspension. DMA and PCM queue overflow fail capture explicitly.
Audio also shares the existing device-work guard with background Space/NVS
materialization. The existing Host worker sends PCM incrementally through
`DeviceSpeech` with backpressure and request identity; there is no second Rust
runtime or full-recording buffer. Statistics are published once per block and
speech phase updates remain responsive. Completed text waits for local state
work to finish before entering chat. Press the
chat microphone again to stop; capture is limited to 300 seconds. Recognition
requires a connected Core speech processor, and its completed text enters the
existing chat send path once.

Playback uses the GPIO26 DAC continuous driver / I2S0 and starts as PCM arrives.
The initial gain is one quarter. Supported Core output is uncompressed PCM16 WAV,
8–48 kHz, mono or stereo (downmixed). WAV headers, metadata and sample frames may
cross transport blocks; only fixed parser/sample buffers are retained. MP3, Opus,
and other compressed audio are rejected explicitly rather than sent as PCM noise.
The Core provider must be configured to return a supported WAV format.

The firmware application owns WAV parsing in `src/wav.rs` and speech/session
orchestration in `src/audio.rs` and `src/device_speech.rs`. The streaming decoder
emits mono PCM16 independently of hardware; the simulator tests this same source.
`edge/boards/esp32/board` owns native I2S/DAC resources, acquisition buffering and signal
conversion, including the final PCM16-to-DAC8 conversion. It does not parse WAV
or Core speech packets.

In Settings → Device operations, **Audio self-test** works without a Space or
speech processor: it plays a quiet 250 ms tone, then captures roughly three seconds
of microphone sample statistics. It does not save or upload the recording.
USB diagnostics expose the same test and status:

```powershell
python edge/boards/esp32/editor/device-debug.py audio --port COM24 --text test
python edge/boards/esp32/editor/device-debug.py audio --port COM24 --text status
```

Use the actual attached COM port and close any other UART owner first. Tests
report sample count, nonzero samples, left/right raw peak, clipping, sums and driver
errors. A successful DMA write does not prove audible sound; nonzero microphone
samples do not prove recognition quality. Hardware results must be recorded
separately from desktop unit/transport tests.

With an admitted Core speech processor attached, USB `audio --text "speak 你好"`
requests synthesis through the existing processor and plays returned PCM16 WAV.
The standalone hardware self-test does not establish this Core round trip.
Firmware release builds omit Rust console log formatting and IDF info/warning
logs that are disabled at runtime once Link owns UART; early startup failures,
C error logs, UI errors, allocation evidence and USB diagnostics remain available.

Recorded build, simulator and hardware results are maintained in
[Verification and current limitations](#verification-and-current-limitations).
