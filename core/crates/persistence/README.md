# Persistence

Persistence crates own durable state, repositories, identity data, bindings, space data, and synchronization logs.

This domain stores and retrieves state. It does not own realtime route selection, peer transport, business stream execution, provider execution, or tool execution.

## Crates

- `store`: current aggregate persistence crate containing database access, repositories, preferences, node identity stores, binding stores, space stores, and sync stores.

## Target Split

The target layout separates this aggregate into `core`, `preferences`, `chat`, `memory`, `workspace`, `node`, and `sync` crates.
