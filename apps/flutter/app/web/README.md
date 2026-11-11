# Operit2 Web Access

This directory owns the Web Access frontend boundary.

The runtime's synchronous Host channels and plugin workers require
`SharedArrayBuffer` in a secure, cross-origin isolated browser context. Startup
checks these prerequisites before acquiring runtime ownership. The deployment
server must send these headers for every application asset:

```text
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Resource-Policy: same-origin
```

The deployment server or reverse proxy for `web.operit.app` must emit these headers. The Link Access server exposed by the CLI does not serve the Web frontend.

## Flutter Web development

The FVM-pinned Flutter SDK natively reads `web_dev_config.yaml` from the
Flutter app root. It configures the built-in development server at
`http://127.0.0.1:4835` and sends the three cross-origin isolation headers above.
No separate Node process or proxy port is required.

Run from `apps/flutter/app`:

```powershell
fvm flutter run -d edge
```

For Chrome, use `fvm flutter run -d chrome`. To open a browser manually, use
`fvm flutter run -d web-server` and open `http://127.0.0.1:4835`.
These devices share the same native server configuration; Flutter's debug
connection and hot reload use that origin directly.

For VS Code, select `Operit2: Web (isolated)` from Run and Debug and press F5.
It launches Edge through the FVM Flutter SDK without a pre-launch proxy task.

These headers enable cross-origin isolation, not unrestricted CORS access to
remote APIs. Remote API servers must still authorize the development origin
through their own CORS policies. `web_dev_config.yaml` only configures Flutter's
development server; deployed hosting must send the headers independently.

## Runtime layout

This is the standard Flutter Web entry directory for `web.operit.app`. The
standalone browser runtime is kept under `runtime/` and is split by purpose:

- `runtime/src/` contains TypeScript sources and its compiler configuration.
- `runtime/generated/` contains generated JavaScript/WASM artifacts and the v86
  guest runtime.
- `runtime/vendor/` contains third-party static files such as MessagePack.

Native apps and the CLI do not package or serve this directory. The settings
entry and `operit2 cli web open` open `https://web.operit.app/` without starting a
local server or putting credentials in the URL. Connecting to another device is
a separate node listening/pairing operation, not a Web Access server lifecycle.
