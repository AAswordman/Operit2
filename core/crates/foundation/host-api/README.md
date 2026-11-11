# operit-host-api

Runtime composition of platform capabilities and the shared VFS.

`HostManager` registers platform providers and exposes `context.vfs()` for file
consumers. The returned filesystem retains this runtime's storage roots,
filesystem backend and mount catalog. Business services should receive this
filesystem rather than collecting native roots and constructing their own mapper.

The crate reexports `operit-host-contract` so existing platform integrations can
use the established import location. Contracts are defined only in that lower
crate. `operit-vfs` depends on contracts and never on this assembly crate, tools,
providers, persistence or application services.

Native filesystem providers are backend adapters. Their locator-based API is
restricted to platform code and VFS implementation by `operit-tools`' native
filesystem boundary check. Core services cannot extract backend locators or
collect physical storage roots.

See [VFS and host boundaries](../../../../docs/vfs-host-boundaries.md).
