# Hosts

`hosts` contains concrete implementations of `operit-host-api` traits. Runtime
code consumes these implementations through `HostManager`, host capability
traits, and owner-host interaction requests.

## Directory Meaning

Top-level host directories describe the runtime platform or a shared host
implementation area:

```text
hosts/android   Android native host implementations
hosts/ohos      OpenHarmony native host implementations
hosts/apple     Apple native host implementations
hosts/linux     Linux native host implementations
hosts/windows   Windows native host implementations
hosts/web       WebAssembly/browser-platform host implementations
hosts/server    Server-target host implementations
hosts/common    Native/server host code shared by multiple platform or app hosts
```

`hosts/web` means the WebAssembly/browser platform host. It does not mean an
in-app browser, a WebView surface, Web Access, or a workspace browser tab.

`hosts/common` is for host capability implementations reused by multiple
runtime owners. A crate in `hosts/common` must still be a host implementation
behind `operit-host-api`; it is not a place for app UI, Access state, remote
session management, or Link protocol ownership.

## Boundary Rules

Host crates implement platform capabilities behind `operit-host-api`.
Examples include file system, HTTP, terminal, managed runtime, storage, web
visit, browser automation, and runtime event delivery.

Host crates may be registered by product owners such as Flutter native bridge,
CLI, server app, or a future VS Code extension host service. Shared server-side
host capability code belongs here when multiple owners can use the same
implementation.

Host crates must not own:

```text
Access pairing, session, signing, or device trust
Remote product state
operit-link envelope semantics
Flutter UI state or WebView widgets
Web Access UX and static asset serving
Core business state owned by operit-runtime
```

Those boundaries are defined in:

```text
docs/link-access-architecture.md
core/CRATE_BOUNDARIES.md
core/crates/operit-host-api/README.md
```

## Workspace Browser

Workspace browser follows the interactive terminal boundary: the controller app
only uses CoreProxy calls, watch streams, and push input streams, Core invokes a host trait, and the
runtime owner app supplies the concrete platform capability through Core owner
interaction.

The command path is:

```text
controller app (local workspace, remote Web, CLI, or VS Code)
  -> CoreProxy and operit-link
  -> RuntimeBrowserService
  -> BrowserSessionHost
  -> RuntimeHostInteractionService browser_session request
  -> runtime owner app subscriber
  -> real runtime owner app WebView session
```

The Flutter owner implementation is under:

```text
apps/flutter/app/lib/core/host/browser
```

`FlutterBrowserSessionBridge` implements the Rust `BrowserSessionHost` trait,
then requests the concrete WebView operation through
`RuntimeHostInteractionService`. The bridge is not a browser backend. Network,
JavaScript, cookies, forms, media, permissions, and page lifecycle remain owned
by the real runtime app WebView.

The display path is:

```text
runtime owner app WebView
  -> semantic DOM and session events
  -> RuntimeBrowserService.browserSessionEvents
  -> operit-link watch stream
  -> controller app same-origin projection view
```

The projection never navigates its iframe to the external page. It renders an
inert snapshot received from Core and sends click, input, submit, keyboard, and
scroll interactions back through `RuntimeBrowserService`. This keeps the real
WebView session alive when a workspace view is removed and keeps all app-to-app
browser data inside Link.

The workspace view implementation is under:

```text
apps/flutter/app/lib/ui/features/chat/components/workspace/browser
```

It may own browser chrome, bookmarks, history presentation, and the projection
renderer. It must not own or import `RuntimeBrowserOwner`,
`RuntimeBrowserSessionRegistry`, a host trait implementation, or a real external
page WebView controller.

Future product owners such as CLI service hosts, server apps, or VS Code
extensions should implement the same owner-host browser command contract. They
must reuse the Core command and event contract and connect it to their real
runtime app browser capability.

For terminals, platform code such as `hosts/android/src/terminal.rs` owns PTY
execution while Flutter owns the xterm view and attach logic. Workspace browser
uses the same split: the runtime owner app owns the executing WebView, while the
workspace owns a renderer attached through Core session streams.
