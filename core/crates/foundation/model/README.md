# operit-model

`operit-model` owns shared data structures used across Operit2 core crates.
It is the place for reusable value models, request/response DTOs, catalog
records, chat entities, memory data, configuration data, and prompt models.

## Responsibilities

- Define serializable models shared by runtime, store, tools, providers, proxy
  generation, and host-facing code.
- Keep data structures independent from runtime service orchestration.
- Provide model/catalog helpers that are pure data transformations.
- Expose common prompt and tool DTOs used across provider and tool boundaries.

## Main Modules

- `src/Chat*.rs`: chat history, messages, variants, window state, and turn
  options.
- `src/Character*.rs`: character cards, groups, tags, memory bindings, and
  export/import payloads.
- `src/Memory*.rs`: memory records, search settings, debug data, and export
  models.
- `src/Model*.rs`: provider profiles, model configs, model catalogs, pricing,
  request specs, and standard parameters.
- `src/ToolPrompt.rs`: shared tool prompt DTOs used by provider and tool code.
- `collects/`: bundled model and TTS catalog data.

## Boundary

This crate must not depend on `operit-runtime`, `operit-store`,
`operit-tools`, `operit-providers`, or `operit-js-bridge`. Runtime behavior
belongs in parent crates and services.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.
