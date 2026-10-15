# Operit2 Theme Studio

Operit2 Theme Studio is the official web-based authoring workbench for editing and previewing the `theme` template capability of standard ToolPkg (`.toolpkg`) archives. Themes are declarations in `manifest.themes`, not a separate package type.

## Architecture Highlights

- **1:1 Shared Flutter Rendering (`client-shell-v7`)**: The right-hand stage embeds the standalone Flutter Web preview kernel (`tools/theme/preview`), which renders the exact same `operit_ui` widgets used by the native Operit2 client (desktop sidebar, conversation history, top bar, user/AI message bubbles, 9-slice surfaces, and Agent/Classic chat input).
- **Single Rust/WASM Contract Authority**: Normalization, L0–L4 media validation (PNG/JPEG, restricted static SVG, static TrueType `.ttf`), and deterministic Stored-ZIP `.toolpkg` export are executed by `operit-plugin-theme` compiled to WebAssembly. Rust remains the validation and export authority; the browser extracts ZIP entries and reads ToolPkg declarations for editing.
- **Declarative Template Export**: The editor creates a standard ToolPkg with a `themes` declaration. Its generated files are `manifest.json`, `theme.json`, `preview.png`, referenced media assets (`assets/images/*`, `assets/fonts/*`), and the agreement text referenced by `manifest.license`. ToolPkg itself can also declare scripts and other capabilities; the editor does not classify or reject packages by those capabilities.

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
4. **Import, Auto-Save, Undo/Redo & One-Click `.toolpkg` Export**:
   - **Import**: Read `manifest.themes` from a ToolPkg or open a visual JSON draft (`src/toolpkg_import.ts`). The editor follows each declaration's actual paths and asks which declaration to edit when several are present. It does not require a sole `default` declaration or fixed `theme.json` / `preview.png` paths. Import extracts a draft; export creates a new ToolPkg from that draft rather than rewriting the original archive's other capabilities.
   - **Draft Persistence**: Edits and uploaded binary assets are automatically persisted to browser IndexedDB (`src/draft_store.ts`) with explicit **保存草稿** confirmation and multi-step **Undo / Redo** history.
   - **Export `.toolpkg`**: Clicking **导出 ToolPkg** automatically ensures a fresh `1280×800` light-mode normal-conversation frame is rendered, captures `preview.png` from the Flutter `RepaintBoundary`, checks the declared files and referenced agreement text, and downloads the deterministic `.toolpkg` archive.

## Local Development

1. Build the pinned Flutter Web + Rust/WASM preview release from the repository root:

```powershell
.\.venv\Scripts\python.exe tools/theme/build_preview.py --renderer-release-id development-local-20 --flutter-sdk <flutter-sdk-root> --wasm-bindgen <wasm-bindgen-cli-path>
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
$env:OPERIT_THEME_RENDERER_RELEASE_ID = "<release-id>"
$env:OPERIT_STUDIO_BASE_PREFIX = "/theme"
npm run typecheck
npm run build
```

The static bundle is emitted to `tools/theme/studio/dist/` with the verified preview kernel nested at `dist/theme-preview/<release-id>/`.

For production, build the preview release with `--base-prefix /theme` before building Studio. [Deploy Web Experience](../../../.github/workflows/deploy-web-experience.yml) publishes the application and editor together from `main`, with Studio served at `web.operit.app/theme/`.

Editing an existing ToolPkg stores the edited template in an immutable content-addressed directory, preserving resources shared by other capabilities. Unchanged manifest fields retain their original localized values, author arrays and agreement references. Explicit Chinese text edits update only that translation. Package IDs follow the ordinary nonblank ToolPkg rule. The final merged archive must pass the Rust validator for every declared template before download.
