# operit-core-application

`operit-core-application` is the composition root for the Core tree.

It creates and owns the current runtime application, local proxy client, node router, route runtime installation, and access services. Host surfaces should receive handles from this crate instead of constructing runtime, proxy, node, and access pieces independently.

## Boundary

- Owns `CoreApplication`, `CoreApplicationConfig`, and lifecycle-facing handles.
- Creates `OperitApplication`, `LocalCoreProxy`, `CoreNodeLocalRuntime`, `CoreNodeRouter`, `LinkAccessStore`, and `RuntimeRemoteLinkService`.
- Accepts a preconfigured local client through `startWithLocalClient()` or `startWithSharedLocalClient()` when a host factory must finish platform wiring first.
- Exposes local Core calls through `localClient()`.
- Exposes node routing through `nodeRouter()`.
- Exposes access and device-space control through `accessServices()`.
- Updates Link Access device metadata through `updateAccessIdentity()`.
- Starts native remote Link servers through `serveRemoteLink()`.
- Clears the global route runtime through `shutdown()` or `shutdownNow()`.

## Call Paths

- Local host calls: `CoreApplication.localClient()` -> generated proxy -> local runtime.
- Rust route calls: route macro wrapper -> shared link route runtime -> `CoreNodeRouter`.
- Access control: pairing/session/peer transport -> access services -> node transport.

## Host Surfaces

- CLI command, TUI, Link, and Web Access surfaces create `CoreApplication` and consume its handles.
- Flutter holds `CoreApplication` over its shared local client and updates the application-owned Link identity before serving pairing and PeerLink traffic.
