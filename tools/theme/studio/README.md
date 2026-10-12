# Operit2 Theme Studio

Operit2 Theme Studio is the official web-based authoring workbench for creating, inspecting, previewing, and exporting declarative Operit2 Theme v1 packages (`.optheme`).

## Architecture Highlights

- **1:1 Shared Flutter Rendering (`client-shell-v6`)**: The right-hand stage embeds the standalone Flutter Web preview kernel (`apps/flutter/theme_preview`), which renders the exact same `operit_theme` widgets used by the native Operit2 client (desktop sidebar, conversation history, top bar, user/AI message bubbles, 9-slice surfaces, and Agent/Classic chat input).
- **Single Rust/WASM Contract Authority**: Normalization, L0–L4 media validation (PNG/JPEG, restricted static SVG, static TrueType `.ttf`), and deterministic Stored-ZIP `.optheme` export are executed by `operit-theme-contract` compiled to WebAssembly. No schema, media, or ZIP rules are reimplemented in browser JavaScript.
- **Zero Executable Code in Themes**: Exported `.optheme` archives contain only `manifest.json`, `theme.json`, `preview.png`, referenced media assets (`assets/images/*`, `assets/fonts/*`), and SPDX license texts (`licenses/*`).

## Core Authoring Features

1. **Interactive Canvas Inspector & 6 Target Layers**:
   - Toggle **点选元素 (Inspect)** or click layer chips (`画布背景`, `导航侧栏`, `对话顶栏`, `用户气泡`, `AI 气泡`, `输入框`, `字体排版`, `M3 调色板`) to jump directly to the schema parameters for that component.
   - Switch between **正常对话** (default, showing user and AI message bubbles) and **空白会话**, and preview across Desktop (`1280×800`), Phone (`390×844`), Tablet (`768×1024`), and Wide (`1440×900`) viewports in Light or Dark mode.
2. **Docked Dual-Mode Stage Color HUD**:
   - Docked below the preview canvas so it never obscures the chat input bar.
   - Enabled in **Light + Dark dual-mode** by default: edit both `palette.light` and `palette.dark` (or component-specific light/dark overrides) side-by-side with a custom color popover, alpha slider, quick preset swatches, and one-click M3 seed palette generation.
3. **Media Assets, 9-Slice Surface Editor & Input Action Icons**:
   - Upload background images, user/AI bubble 9-slice skins (`tiled_nine_slice` or `nine_patch` with visual crop/slice/scale controls), and input box 9-slice skins (PNG/JPEG or restricted static SVG).
   - Customize the 5 right-side chat input action icons (`settingsAsset`, `attachmentAsset`, `micAsset`, `sendAsset`, `stopAsset`) and adjust their display sizes (`12–44 px` / `12–48 px`).
   - Upload custom static TrueType fonts (`assets/fonts/*.ttf`) or select built-in `noto-sans-sc` / `cascadia-mono` for global, user, and AI typography.
4. **Import, Auto-Save, Undo/Redo & One-Click `.optheme` Export**:
   - **Import**: Load an existing `.optheme` package or `theme.json` file (`src/optheme_import.ts`) to inspect or iterate on an existing theme.
   - **Draft Persistence**: Edits and uploaded binary assets are automatically persisted to browser IndexedDB (`src/draft_store.ts`) with explicit **保存草稿** confirmation and multi-step **Undo / Redo** history.
   - **Export `.optheme`**: Clicking **导出 .optheme** automatically ensures a fresh `1280×800` light-mode normal-conversation frame is rendered, captures `preview.png` from the Flutter `RepaintBoundary`, verifies SPDX license closure, and downloads the deterministic `.optheme` archive.

## Local Development

1. Build the pinned Flutter Web + Rust/WASM preview release from the repository root:

```powershell
..venvScriptspython.exe tools/theme/build_preview.py --renderer-release-id development-local-20 --flutter-sdk <flutter-sdk-root> --wasm-bindgen <wasm-bindgen-cli-path>
```

2. Start the local Vite development server (bound to `127.0.0.1:7474`):

```powershell
cd tools/theme/studio
npm ci
npm run dev
```

To use a custom preview release ID during development or build, set `OPERIT_THEME_RENDERER_RELEASE_ID=<release-id>`.

## Production Build

```powershell
cd tools/theme/studio
OPERIT_THEME_RENDERER_RELEASE_ID=<release-id> OPERIT_STUDIO_BASE_PREFIX=/studio npm run build
```

The static bundle is emitted to `tools/theme/studio/dist/` with the verified preview kernel nested at `dist/theme-preview/<release-id>/`.
