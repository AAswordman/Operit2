# Provider

Provider crates own AI provider integrations and provider-facing services.

This domain covers remote LLM providers, provider chat orchestration, media providers, market services, memory provider helpers, and local model capability. It should be consumed by runtime services through explicit handles.

## Crates

- `services`: current aggregate provider crate for LLM providers, chat provider services, STT/TTS, market, memory helpers, and runtime support.
- `local-model`: local model catalogs, registries, storage, downloads, engine manifests, and local inference support.

## Target Split

The target layout separates `services` into `contracts`, `llm`, `chat`, `media`, `memory`, and `market` crates.
