# Hosts

`hosts` contains concrete implementations of `operit-host-api` traits. Runtime
code consumes these implementations through `HostManager`, host capability
traits, and owner-host interaction requests.

## Directory Meaning

Top-level host directories describe the runtime platform or a shared host
implementation area:

```text
hosts/android   Android native host implementations
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

Workspace browser follows the same ownership model as the interactive terminal:
the owner app owns the real interactive capability, while remote controllers use
CoreProxy calls and watch streams.

For the current Flutter owner, the real browser capability is the workspace
WebView session inside:

```text
apps/flutter/app/lib/ui/features/chat/components/workspace/browser
```

That layer owns WebView construction, browser chrome, permission UI, user
interaction, tab state, JavaScript execution, navigation, and visible attach.
It also handles owner-host browser session requests from
`RuntimeHostInteractionService`.

The runtime-facing path is:

```text
Remote controller
  -> CoreProxy call/watch
  -> RuntimeBrowserService
  -> RuntimeHostInteractionService browser_session request
  -> owner app workspace browser registry
  -> real owner-host WebView session
```

Browser status returns through `RuntimeBrowserService.browserSessionEvents`.
The stream carries semantic session events such as navigation start, progress,
URL change, completion, error, and close. It does not stream pixels or replay a
host-fetched HTML document.

Future product owners such as CLI service hosts, server apps, or VS Code
extensions should implement the same owner-host browser command contract. They
should not duplicate server code for the command protocol, and they should keep
their concrete browser/view capability inside the product owner that can
actually present or operate the interactive surface.

For terminals, platform code such as `hosts/android/src/terminal.rs` owns PTY
execution while Flutter owns the xterm view and attach logic. Workspace browser
uses the same split: the owner app owns the browser session and user surface;
runtime services expose semantic control and event streams over Link.
