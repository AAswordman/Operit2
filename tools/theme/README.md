# Theme Tools & Web Studio

This directory contains the build recipes, MessagePort protocol bridge, and standalone Web Theme Studio for Operit2 Theme v1 (`.optheme`).

## Directory Layout

- `prepare_builtin_fonts.py`: Reproducible subsetting recipe for the built-in static TrueType fonts (`noto-sans-sc` and `cascadia-mono`) and their OFL license catalog in `apps/flutter/packages/operit_theme/assets/fonts/`.
- `build_preview.py`: Explicit build recipe that compiles the Rust/WASM contract validator (`operit-theme-contract`) and the standalone Flutter Web preview kernel (`apps/flutter/theme_preview`) into an immutable `build/theme-preview/<rendererReleaseId>/` release directory.
- `studio_message_port.ts` / `studio_contract_bridge.ts`: Versioned browser `MessagePort` transport and Web Worker client for the Rust/WASM theme contract.
- `studio_message_port.test.ts`: Node TypeScript test suite covering handshake, revision ordering, scene mode, frame capture, and binary export boundaries.
- `studio/`: Standalone Vite + TypeScript Web Theme Studio application (see [`studio/README.md`](studio/README.md)).

## Building the Preview Kernel

Run from the repository root with the project Python environment, the application's Flutter SDK, and the `wasm-bindgen` CLI version locked in `core/Cargo.lock`:

```powershell
..venvScriptspython.exe tools/theme/build_preview.py --renderer-release-id development-local-20 --flutter-sdk <flutter-sdk-root> --wasm-bindgen <wasm-bindgen-cli-path>
```

Each build writes a new immutable release under `build/theme-preview/<rendererReleaseId>/` (including `renderer-release.json` and SHA-256 digests) and refuses to overwrite an existing release directory.

## Running Local Theme Studio

```powershell
cd tools/theme/studio
npm ci
npm run dev
```

Open <http://127.0.0.1:7474/>. Studio mounts the pinned `build/theme-preview/<rendererReleaseId>/` directory same-origin, renders live frames through the shared Flutter `operit_theme` widgets (`client-shell-v6`), and exports verified `.optheme` packages through the Rust/WASM contract worker.

## Static Bundle & GitHub Pages Deployment

Running `npm run build` inside `tools/theme/studio` verifies the pinned preview release in `build/theme-preview/<rendererReleaseId>/`, emits `dist/theme-studio-config.json` and `dist/.nojekyll`, and copies the verified Flutter Web + Rust/WASM preview release into `dist/theme-preview/<rendererReleaseId>/`.

For subpath deployment (such as `/studio/` on `gh-pages`), pass `--base-prefix /studio` to `tools/theme/build_preview.py` and set `OPERIT_STUDIO_BASE_PREFIX=/studio` during `npm run build`. Automated CI deployment is configured in [`.github/workflows/deploy-theme-studio.yml`](../../.github/workflows/deploy-theme-studio.yml) and [`.github/workflows/deploy-web-experience.yml`](../../.github/workflows/deploy-web-experience.yml).
