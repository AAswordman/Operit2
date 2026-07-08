# operit-providers

`operit-providers` owns provider-facing code that used to live under the
runtime `api` tree. It contains chat provider orchestration, LLM adapters,
voice providers, market APIs, prompt composition, and provider-side runtime
support contracts.

## Responsibilities

- Prepare provider requests and stream provider responses.
- Own LLM provider adapters and model connection testing.
- Own voice provider abstractions and TTS response processing.
- Own market API service code and market data DTOs.
- Compose system prompts, functional prompts, and prompt hooks used by provider
  calls.
- Define provider-side interfaces for runtime-owned model bindings, prompt
  context, token accounting, ToolPkg AI provider hooks, and timing logs.

## Main Modules

- `src/chat/EnhancedAIService.rs`: provider-backed assistant loop and tool
  integration.
- `src/chat/llmprovider/`: provider adapters, model list fetching, connection
  tests, and structured tool-call bridge.
- `src/chat/enhance/`: conversation services, reference handling, input
  processing, round management, and file binding.
- `src/chat/config/`: system prompts, functional prompts, and provider prompt
  configuration.
- `src/chat/hooks/`: prompt and summary hook registries.
- `src/chat/library/`: memory library support used by provider requests.
- `src/voice/`: voice providers and TTS pipeline steps.
- `src/market/`: market API services and support code.
- `src/runtime_support.rs`: provider-side contract implemented by
  `operit-runtime`.

## Boundary

Provider code may use model, store, tools, util, and host-api crates. It must
not depend on `operit-runtime`; runtime-owned behavior is requested through
`ProviderRuntimeSupport`.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.
