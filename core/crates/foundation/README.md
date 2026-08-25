# Foundation

Foundation crates provide shared contracts and data types used by every other core domain.

They must stay free of application ownership, runtime construction, node routing, proxy runtime, and access server responsibilities.

## Crates

- `host-api`: host capability contracts and `HostManager`.
- `model`: shared data models for chat, memory, prompts, workflow, STT/TTS, and nodes.
- `util`: common filesystem, network, logging, serialization, archive, media, and stream utilities.
- `link`: core link protocol types, `CoreValue`, calls, watches, pushes, events, frames, and route runtime contracts.

## Target Split

The target layout also adds a `contracts` crate for cross-domain traits once runtime, node, access, tool, provider, and persistence contracts are narrowed.
