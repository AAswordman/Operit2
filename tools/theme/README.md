# Theme Tools & Web Studio

This directory contains the build recipes, MessagePort protocol bridge, and standalone Web Theme Studio for ToolPkg's `theme` template capability (`manifest.themes`). The package format is ToolPkg (`.toolpkg`), not a separate theme format.

## Directory Layout

- `prepare_builtin_fonts.py`: Reproducible subsetting recipe for the built-in static TrueType fonts (`noto-sans-sc` and `cascadia-mono`) and their OFL license catalog in `apps/flutter/packages/operit_ui/assets/fonts/`.
- `build_preview.py`: Explicit build recipe that compiles the Rust/WASM contract validator (`operit-plugin-theme`) and the standalone Flutter Web preview kernel (`tools/theme/preview`) into an immutable `build/theme-preview/<rendererReleaseId>/` release directory.
- `studio_message_port.ts` / `studio_contract_bridge.ts`: Versioned browser `MessagePort` transport and Web Worker client for the Rust/WASM theme contract.
- `studio_message_port.test.ts`: Node TypeScript test suite covering handshake, revision ordering, scene mode, frame capture, and binary export boundaries.
- `studio/`: Standalone Vite + TypeScript Web Theme Studio application (see [`studio/README.md`](studio/README.md)).

## Building the Preview Kernel

Run from the repository root with the project Python environment, the application's Flutter SDK, and the `wasm-bindgen` CLI version locked in `core/Cargo.lock`:

```powershell
.\.venv\Scripts\python.exe tools/theme/build_preview.py --renderer-release-id development-local-20 --flutter-sdk <flutter-sdk-root> --wasm-bindgen <wasm-bindgen-cli-path>
```

Each build writes a new immutable release under `build/theme-preview/<rendererReleaseId>/` (including `renderer-release.json` and SHA-256 digests) and refuses to overwrite an existing release directory.

## Running Local Theme Studio

```powershell
cd tools/theme/studio
npm ci
npm run dev
```

Open <http://127.0.0.1:7474/>. Studio mounts the pinned `build/theme-preview/<rendererReleaseId>/` directory same-origin, renders live frames through the shared Flutter `operit_ui` widgets (`client-shell-v7`), and exports verified `.toolpkg` packages through the Rust/WASM contract worker.

## Static Bundle & GitHub Pages Deployment

Running `npm run build` inside `tools/theme/studio` verifies the pinned preview release in `build/theme-preview/<rendererReleaseId>/`, emits `dist/theme-studio-config.json` and `dist/.nojekyll`, and copies the verified Flutter Web + Rust/WASM preview release into `dist/theme-preview/<rendererReleaseId>/`.

The production editor is published at `web.operit.app/theme/`. Pass `--base-prefix /theme` to `tools/theme/build_preview.py` and set `OPERIT_STUDIO_BASE_PREFIX=/theme` during `npm run build`. The preview's Flutter base href and the editor's build prefix must match.

The single publishing workflow is [Deploy Web Experience](../../.github/workflows/deploy-web-experience.yml). Run it manually from `main` to build and publish both the main application at `/` and Theme Studio at `/theme/`. Deployment runs share the `github-pages` concurrency group and do not cancel a running deployment. The workflow updates the existing `gh-pages` branch with a regular push, preserving its history, then explicitly requests a GitHub Pages build with `pages: write` permission. GitHub Pages must already use that branch as its publishing source and have `web.operit.app` configured as its custom domain.
