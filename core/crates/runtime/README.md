# Runtime

Runtime crates own core business services and application-level runtime behavior.

This domain includes chat runtime, preferences, workspace, host-facing runtime services, transfers, plugin hosting, and runtime events. It should expose narrow local capabilities to node runtime instead of creating node or access services itself.

## Crates

- `application`: current aggregate runtime crate containing `OperitApplication`, chat services, workspace services, provider/tool runtime support, plugin runtime support, and runtime event services.

## Target Split

The target layout separates this aggregate into `contracts`, `application`, `chat`, `preferences`, `workspace`, `host`, `transfer`, `plugin-host`, and `events` crates.
