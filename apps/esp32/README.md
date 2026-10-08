# ESP32 Edge pairing

The ESP32 advertises `_operit-edge._tcp` over mDNS. Use the Core application's
device scan to discover it, start pairing, and enter the code displayed on the
ESP32. Core and Edge must be on the same reachable LAN. Opening the browser
editor automatically starts a separately identified `esp32-edge-simulator`.

The firmware registers a native `device.status` plugin. Its `read` action
returns board id, expression, IP address, and Wi-Fi name over the paired Link.
The pairing code is excluded from plugin results. Core ToolPkg UI is not
rendered on this display unless it has an explicit Edge adaptation.

## Local configuration

The ESP32 uses the same Edge token as the Core/CLI main link. Put that token in
the local `.edge-token` file under `apps/esp32`; the firmware build reads it
automatically. The file is ignored by Git and must never be committed or
included in logs. `OPERIT_EDGE_TOKEN` can still override it for CI or a
deliberate one-off build:

```powershell
cargo build --release
```

Leave the setup page's token field empty to keep using this same token. Enter a
different value only when the Core/CLI token is intentionally being rotated.

Use the ESP32 toolchain and partition table documented in the repository's
`BUILDING.md`. Select the actual serial port for your device rather than assuming
the port in a developer's local configuration. Preserve existing Wi-Fi/NVS data
when updating only the application image.

## Memory and connection lifetime

- The firmware's Edge worker uses a 32 KiB stack; the previous 6 KiB stack
  overflowed while processing pairing requests.
- The optional RGB332 diagnostic mirror is disabled to leave heap space for
  pairing. The physical TFT remains enabled.
- Native Core requests share a process-lifetime Tokio executor so the saved
  pairing socket and background receivers survive between commands. The Rust
  DLL requires a native rebuild and application restart, not Dart hot reload.

## Verification and current limitations

Native scheduler regression tests cover socket reuse across requests, receiver
survival after request completion, and non-Send request futures. On the
ESP32-2432S028, three consecutive PairStart requests completed after flashing
the stack/mirror fix without rebooting. This is **not** verification of complete
correct-code pairing, Space admission, reconnect, or chat.

The fixed self-drawn UI handles pairing, status, Space admission, a bounded
chat text view, unpair confirmation, and local Space-exit confirmation. Core still owns chat history and
permissions. A paired device remains in chat when offline. The host/computer
composer can send drafts; an on-device keyboard, images, conversation lists,
animations and dynamic layouts are not currently implemented.

## Recovering from an unavailable Space administrator

Pairing and Space membership are independent. Unpairing does not silently leave
or merge a Space. On the device, open **Status → 退出空间 → 确认退出** to leave an
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

The display has a 2048-byte static text buffer for the 640-byte payload plus
sender labels. Cursor history is bounded; it does not retain past page text.
UART `device:screen` exposes `chatCachedBytes`, `chatCachedLines`, and
`chatScrollLine` for checking the real device without a screenshot request.

## UI backend

The only renderer is `ui_port/operit_mini_ui.c`. Rust uses `src/ui.rs` and the
`operit_ui_*` C ABI; board display ownership and RGB565 flushing use
`activateUi` / `flushUi`. There is no third-party widget runtime, allocator
pool, backend marker or fallback build. Native firmware and Wasm compile the
same C sources.

The 320×240 renderer uses two-row RGB565 strips (1280 bytes). A compile-time
assertion limits its static state, strip and inspection buffer to 10 KiB.
This is not a whole-board RAM measurement. The browser framebuffer and the
Rust runtime/network allocations are separate.

```powershell
npm run build --prefix tools/esp32-editor
npm test --prefix tools/esp32-editor
npm run build:firmware --prefix tools/esp32-editor
```

Preview builds require Node.js and Emscripten but no firmware build cache.
Firmware builds also require the ESP32 Rust/ESP-IDF toolchain and espflash.
Building does not flash the device. `npm run dev` builds and flashes; updating
with `npm run flash` preserves NVS and the reserved layout partition.

## Xtensa build overlays

The firmware build prepares checksum-pinned Tokio 1.53.1 and ring 0.17.14
sources under the ignored `.embuild/` directory. The scripts are
`tools/esp32-editor/prepare-tokio.py` and `prepare-ring.py`; they never modify
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

`npm run build:firmware` runs `check-xtensa-tokio.py` against the final ELF to
check the Tokio current-thread constructor and ring seal/open stack frames.
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
