# ESP32 Edge firmware

The ESP32-2432S028 is an input, output and transport node. It uploads user text
and microphone PCM to Core, displays Core text and plays Core audio. Core owns
recognition, synthesis, model requests, chat context, history and plugin execution.
The firmware contains no model HTTP client, provider keys or local chat engine.
See [Edge architecture](../../docs/architecture.md) for the shared Space contracts.

## Source ownership

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | Startup, device actions and UI event loop |
| `src/edge_chat.rs` | Current Core chat session, bounded display and text forwarding |
| `src/device_speech.rs` | Core speech Stream / ReverseStream transport and session identity |
| `src/audio.rs`, `src/wav.rs` | Capture/playback orchestration and incremental PCM16 WAV parsing |
| `src/device_space_ui.rs`, `src/space_join_ui.rs` | Pairing, Space admission and local decisions |
| `src/runtime_profile.rs` | Board-specific Peer and scene budgets |
| `src/runtime_storage_*.rs` | Bounded NVS snapshots, compression and commit recovery |
| `src/settings.rs`, `src/wifi.rs` | Wi-Fi configuration and local setup page |
| `src/ui.rs`, `ui_port/` | Rust/C adapter, fixed pages and two-row RGB565 rendering |
| `board/` | Display, touch, GPIO, I2S and DAC resources |
| `editor/` | Same-source C/Wasm preview, constrained Rust simulator and build tools |

The browser uses the fixed C UI. The inactive draft designer, AI editing panels
and their deployment UI have been removed. Source editing and the simulator's
HTTP/USB inspection tools remain available for secondary development.

## Connections and local state

Wi-Fi settings come from the setup page or `OPERIT_WIFI_SSID` and
`OPERIT_WIFI_PASSWORD`. mDNS advertises `_operit-edge._tcp`. Serial Link is
available offline; TCP Link requires a station connection. The plain HTTP setup
page is retained; model HTTPS requests belong to Core. This board uses IPv4;
its ESP-IDF build disables IPv6. This product omits the optional HTTP discovery
directory (`peer-directory`); mDNS, saved endpoints, TCP and UART remain available.
The SDK keeps directory support enabled by default for other products.

Pairing uses the receiving device's six-digit code for both UART and TCP. No
Edge token is required. Pairing and Space admission are separate: an additional
paired Core does not replace the current Space or gain its permissions. Space
admission requires approval. Leaving a Space retains direct pairings; the local
unpair action removes stored pairings. The firmware permits two simultaneous
incoming sessions; stored peer count is instead constrained by NVS capacity.

An admitted device uses the durable `edge` identity without `storage.provide`.
Identity, pairing, Space control and small encrypted device preferences persist;
chat text and transient scenes do not. `edge.preferences/read|set|remove` remains
available for generic device configuration, with 24 keys and a 6144-byte plaintext
limit. An old `chat` preference no longer enables direct model requests; it may
be removed through that endpoint. Layout partition addresses remain unchanged.

When an old Space administrator is unavailable, leave that Space through the
local device UI, pair the replacement Core, then submit and approve a fresh
join. Re-pairing alone never transfers the old administrator's authority.

## Text, speech and display

The UI retains current-chat display, text send, new-chat requests, microphone
capture, expression, connection status, pairing and Space controls. Conversation
catalogs, cross-chat selection, remote history paging and plugin diagnostic
screens have been removed. Core retains complete history and manages plugins.

`chatMessagesWindowFlow` supplies at most 12 message rows, 640 visible UTF-8 bytes
and 15 hard lines. The device scrolls within this latest window and updates live
replies incrementally. The C text buffer is 1024 bytes including sender labels;
no transcript is written to NVS. Reconnection restores subscriptions and the
current chat ID within the same Space, without replaying sends or new-chat actions.

The browser forwards up to 16384 UTF-8 bytes of text. Its preview uses the C
renderer's 511-byte draft buffer. Native inputs above that draft capacity are
rejected explicitly. Long text uses the existing chunked Link push to Core;
partial or expired sends are never committed as a truncated message or retried.

`operit.device.speech` retains the existing `requests` watch and `results` push.
Core processes recognition/synthesis, while Edge handles bounded audio chunks,
backpressure, cancellation and playback. Disconnects retire session identities;
late results cannot enter a new request. Full audio round trips need a Core
speech processor, independently of the text chat route.

Core-owned ToolPkg scripts may upload generic `edge.scene` assets. ESP32 renders
and handles local scene input while retaining the system exit strip. Scene
limits are 8 KiB per pack and 16 KiB total cache. No ToolPkg executes on ESP32.

## Build and verification

Prepare Node.js, Emscripten, the ESP32 Rust/ESP-IDF toolchain and espflash as
explained in the [editor README](editor/README.md). From the repository root:

```powershell
npm run build --prefix edge/boards/esp32/editor
npm test --prefix edge/boards/esp32/editor
npm run check --prefix edge/boards/esp32/editor
npm run check:firmware --prefix edge/boards/esp32/editor
npm run build:firmware --prefix edge/boards/esp32/editor
```

There is one firmware composition, including the Core text/audio interfaces.
`check:firmware` builds Xtensa release, checks the ABI and validates the generated
application image against the 3,997,696-byte app partition. It does not flash.
`build:firmware` also packages `dist/operit-esp32.bin` and the fixed 4 MiB flash
container. Normal firmware updates preserve NVS and the reserved layout partition.
Keep the matching ELF for crash address resolution.

Measured on 2026-11-27, the complete Core text/audio application is
2,555,136 bytes (2.44 MiB), down 992 bytes from the previous
2,556,128-byte image. The unchanged application partition has 1,442,560 bytes
free. The 1 MiB target remains unmet by 1,506,560 bytes. The fixed 4 MiB flash
container is not the application size.

Node-local preferences reject transaction commit records and omit the replica
codec. Shared immutable replay views avoid copying metadata with every snapshot.
Ordered preference maps eliminate sorting copies before encoding. The storage
edit and timeout paths share their implementations across callers. Link values
use the existing MessagePack scanner with a bounded container stack; successful
Call payloads move through metadata validation without rebuilding their trees.
The simulator reuses the board TCP adapter and its fallible 512-byte read buffer,
including cancellation and close behavior, instead of a desktop 16 KiB buffer.
Typed Link visitors decode only their requested data shapes; ignored fields
already validated by the frame parser need no second traversal. Chat and device
Space views use typed presentation records, avoiding JSON field lookup and
temporary arrays in the local UI. Private stream identifiers stay off the UI API.
Startup connects directly to the configured SSID, without an extra full scan
whose results served only disabled logs. Unused device log calls, logger setup
and UI metric queries have been removed; UI errors and USB health remain.
Timeouts share one poll routine without boxing the operation or erasing its Send
property across await; full Core HTTP directory builds remain supported. Historical
Space admission decoding belongs to its enum variant, removing the duplicate
command enum. Status JSON serializes the existing record directly; UI diagnostics
parse C bytes without an intermediate string.

Verification: final Xtensa release, ELF ABI, partition and C/Wasm packaging
passed. Runtime without HTTP directories: 151 passed; Core/Edge admission: 46 passed;
persistence: 373 passed (3 ignored); editor: 109 passed; firmware-module simulation:
58 passed; TypeScript checks passed. Link's 62 passes are from the preceding revision.

The Linux simulator passed a targeted real-TCP transport workflow against Core:
pairing and approval, storage-full/interrupted-write recovery, 65-second PCM byte
volume, long recognition text, repeated audio rounds, playback backpressure,
text streaming, long UTF-8 input, reconnect, restart and offline cancellation.
The deterministic model and speech providers do not validate recognition quality.
The workflow kept the 256 KiB heap and 16/24/20 KiB guarded task stacks; sampled
peak heap was 203,360 bytes with zero allocation failures. Earlier runs exhausted
the heap while expanding the editor IPC JSON output beside audio state. Output
now grows fallibly in small blocks and serializes each reply once. A later run
exhausted the heap in the desktop TCP adapter's infallible 16 KiB allocation;
reusing the board TCP adapter passed the same workflow with the fixed heap and
stack limits unchanged. The restart fixture waits for the
re-established authenticated carrier before cancelling a persisted request.
Two bundled-plugin blocks were excluded only in the targeted run; their
assertions remain in the full suite. The full workflow still fails when a Core market package reports a
successful import but is absent from the loaded-plugin list. The separate Windows
workflow still exits with 0xC0000005; a debugger locates its failure inside the
Windows BCrypt initialization path on a bounded stack. Neither full workflow is
reported as passed. The fixture CLI has native inference disabled; its manifest
and lockfile were restored after building.

The simulator compiles actual firmware protocol/storage modules, constrains its
Rust heap to 256 KiB and uses guarded task stacks. C/Wasm tests run the same UI
sources. Desktop timing and allocator results do not measure ESP-IDF, Wi-Fi,
DMA or whole-board memory. No physical test is claimed for this cleanup.

The board no longer allocates the unused 76800-byte RGB332 Wi-Fi mirror at
startup. TFT writes go directly to SPI; UI ownership stays with the face host.
USB inspection and the same-source C/Wasm preview remain available.

The renderer has no heap or full-screen framebuffer: two RGB565 rows use
1280 bytes, with at most eight strip flushes per pump. The 4168-glyph Noto font
uses seven-byte records and one-bit masks (112198 data bytes). Copyright and
SIL Open Font License 1.1 remain in `ui_port/mini_font.h` and `ui_port/OFL.txt`.

## Xtensa build overlays

The firmware build prepares checksum-pinned Tokio 1.53.1 and ring 0.17.14
sources under the ignored `.embuild/` directory. The scripts are
`edge/boards/esp32/editor/prepare-tokio.py` and `prepare-ring.py`; they never
modify Cargo's shared registry cache. Original dependency licenses and
copyright notices stay in the extracted sources.

Serde derive and rmp-serde use their unmodified registry releases. Their former
size overlays, preparation scripts and overlay-only comparison fixtures have
been removed. Shared Link and simulator protocol tests remain.

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
The Xtensa build omits embedded Rust caller-location file/line tables with
`-Z location-detail=none`. Rust panics abort immediately without a Rust panic
hook or panic text. ESP-IDF native abort backtraces remain; resolve available
function symbols on the host with the matching ELF. Embedded
caller locations are unavailable, and source file/line lookup requires separately
retained debug information (the default release build strips it). This
requires the pinned ESP nightly toolchain, as do the existing build-std settings.

Peer transport decoding selects the existing Call / Watch / Push envelope with
the shared constant-stack field scanner, then decodes its typed body. Owned
frames retain binary fields as shared ranges. Compatibility tests compare every
request lifecycle and success/error response against the original Serde codec,
including rejected positional struct variants. Serializer/decoder entrypoints
remain out of line so Fat LTO can share each instantiation across callers.
The built-in font retains all 4,168 glyphs and their displayed pixels in
seven-byte records and one-bit masks; table tests check field widths and bitmap
boundaries. Digit advance is fixed at 16 pixels.

The ESP32 standard library uses `backtrace-trace-only` to omit the device-side
DWARF symbol reader. Native abort backtrace addresses can be resolved on the
host using the matching ELF. Keep the ELF
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

## Native INMP441 and onboard speaker

The board audio adapter uses the existing speech protocol, not a second speech
service. The supported wiring is INMP441 VDD=3.3V, GND=GND, L/R=GND (left),
SCK/BCLK=GPIO27, WS=GPIO22, SD=GPIO35. GPIO21 remains the display backlight;
do not connect the microphone clock there. The passive speaker uses the board's
two-pin amplified speaker connector driven by DAC GPIO26.

Capture runs on I2S1 at 16 kHz, with 32-bit stereo slots (64 bit clocks/frame).
Only the left slot is converted to mono PCM16, with DC removal, bounded gain and
saturation. A priority-10 native task drains DMA independently of Host workflows.
Its fixed 32-block queue retains 512 ms of high-16-bit mono input in 16,384 bytes.
The original PCM filter uses exactly those same high bits, so conversion is unchanged. Its 3 KiB stack, queue and scratch storage are
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

Microphone acquisition uploads bounded PCM blocks to Core; playback consumes
Core-provided WAV segments. Production capture does not compute diagnostic
peaks, clipping counts or sums, and the local tone/capture self-test has been
removed. DMA overrun detection, backpressure, cancellation and driver errors
remain. USB `audio --text status|record|stop` exposes the same transport state.
With an admitted Core speech processor attached, USB `audio --text "speak 你好"`
requests Core synthesis and plays the returned PCM16 WAV.

Firmware release builds omit Rust console log formatting and IDF info/warning
logs that are disabled at runtime once Link owns UART; early startup failures,
C error logs, UI errors, allocation evidence and USB diagnostics remain available.

Recorded build, simulator and hardware results are maintained in
[Verification and current limitations](#verification-and-current-limitations).
