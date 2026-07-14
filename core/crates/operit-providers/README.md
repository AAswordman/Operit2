# operit-providers

`operit-providers` owns Operit's provider contracts and built-in provider
implementations.

The crate root re-exports `AIService`, `SendMessageRequest`, `AiServiceError`,
token/stream helpers, and `ProviderRuntimeSupport`, so SDK consumers do not
need to import the internal `chat::llmprovider` path.

## Usage

External providers implement the public contracts from the crate root:

```toml
operit-providers = "2.0.0-preview.4"
```

The same crate contains the built-in LLM adapters, text-to-speech,
speech-to-text, and market services, ToolPkg provider integration,
conversation orchestration, store access, and tool integration.

## Responsibilities

- Define provider requests, errors, token counters, and streaming contracts.
- Define provider-side interfaces for runtime-owned model bindings, prompt
  context, token accounting, ToolPkg AI provider hooks, and timing logs.
- Provide Operit's built-in provider implementations and orchestration.

## Main Modules

- `src/chat/llmprovider/AIService.rs`: provider request, result, stream, and
  service contracts.
- `src/runtime_support.rs`: provider-side contract implemented by
  `operit-runtime`.
- `src/chat`: built-in chat providers and conversation orchestration.
- `src/tts`: built-in text-to-speech provider contracts and implementations.
- `src/stt`: built-in speech-to-text provider contracts and implementations.
- `src/market`: provider market services.

## Boundary

Runtime-owned behavior is requested through `ProviderRuntimeSupport`;
`operit-providers` does not depend on `operit-runtime`.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.
