# operit-providers

`operit-providers` owns Operit's provider contracts and built-in provider
implementations.

The crate root re-exports `AIService`, `SendMessageRequest`, `AiServiceError`,
token/stream helpers, and `ProviderRuntimeSupport`, so SDK consumers do not
need to import the internal `chat::llmprovider` path.

## Usage

External providers implement the public contracts from the crate root:

```toml
operit-providers = "2.0.0-preview.8"
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

## Model Discovery Structure

`ModelListFetcher.rs` owns discovery orchestration and the only network execution
point: `defaultHttpHost().executeHttpRequest(...)`. Its private `model_list`
modules only prepare request data and parse response data; they do not create
HTTP clients or introduce platform-specific branches.

- `model_list/mod.rs`: selects the discovery protocol once from the provider type
  and routes request preparation and response parsing.
- `model_list/request.rs`: shared URL/header configuration, key-pool selection,
  Bearer authentication, Anthropic authentication, and OpenCode routing.
- `model_list/endpoint.rs`: structural model-resource path derivation that retains
  gateway prefixes and explicit API versions.
- `model_list/response.rs`: catalog-declared item selection and shared model
  metadata parsing.
- `model_list/gemini.rs`: Gemini query-key authentication, versioned model-list
  paths, and generation-model filtering.
- `model_list/tests.rs`: request and model-parsing unit tests.

Private functions use Rust `snake_case`; the existing public `ModelListFetcher`
entry point remains unchanged.

## OpenAI-Compatible Model Discovery Paths

Operit1's Kotlin `ModelListFetcher.extractBaseUrl()` preserves the path before
an API version segment. Directly replacing a URL's path with the catalog path
loses that gateway prefix, so catalog discovery derives model paths structurally
instead of overwriting the endpoint route.

- Explicit `chat/completions`, `responses`, `messages`, and `models` resource
  suffixes select the sibling `models` resource, retaining all preceding path
  segments and their API version.
- A base ending in a complete numeric version segment (`v1`, `v2`, or `V1`)
  appends `models` to that configured version.
- Other configured base paths scope the catalog operation path. Shared suffix
  and prefix segments are merged to avoid duplicating provider namespaces.
- Root endpoints use the catalog model path. Existing query and fragment removal
  remains unchanged.

Examples for an OpenAI-compatible catalog using `/v1/models`:

| Configured endpoint path | Model-list path |
| --- | --- |
| `/a/b/c/v1/chat/completions` | `/a/b/c/v1/models` |
| `/a/b/c/v1/responses` | `/a/b/c/v1/models` |
| `/a/b/c/v2` | `/a/b/c/v2/models` |
| `/a/b/c/chat/completions` | `/a/b/c/models` |
| `/a/b/c` | `/a/b/c/v1/models` |

Provider-specific namespaces such as `/api/v1`, `/compatible-mode/v1`, and
`/api/coding/paas/v4` remain part of the configured route. Unlike Kotlin's
versionless extraction branch, configured gateway paths are not reduced to the
origin. Paths are matched by complete segments, not URL substrings.

Source contract checks are in `tools/tests/openai_model_endpoint.test.mjs`.
Rust regression cases in `model_list/tests.rs` cover nested paths, Responses,
provider namespaces, trailing slashes, ports, and percent-encoded prefixes.

## Gemini Model Discovery

Both `GOOGLE` and `GEMINI_GENERIC` use the Kotlin discovery protocol: the selected
API key is URL-encoded into the `key` query parameter. Discovery does not generate
an `Authorization: Bearer` header for these providers. Explicit custom headers
remain user-controlled.

The configured origin is retained. Root endpoints use the catalog operation
path; explicit `/v1` and `/v1beta` endpoints retain their version and select its
`models` resource. Unsupported endpoint paths and missing keys produce errors
before network execution.

Only models explicitly declaring `generateContent` are listed. Their identifiers
use the response's non-empty `baseModelId`, matching Kotlin's base-model identity,
and are sorted by ID. Missing or malformed generation metadata is reported as an
error rather than assuming a model's capability or inventing an identifier.

Source contract checks are in `tools/tests/google_model_catalog.test.mjs`.
These checks do not compile Rust or contact a provider. An upstream error such as
`API_KEY_SERVICE_BLOCKED` is preserved in the returned HTTP error; changing the
request protocol cannot establish that a real key has permission to use the
provider service.

## Anthropic Model Discovery

`ModelListFetcher` sends both `ANTHROPIC` and `ANTHROPIC_GENERIC` catalog
requests through the shared HTTP host, using `x-api-key` authentication and
`anthropic-version: 2023-06-01`, matching the Kotlin model-list implementation.
The API key comes from the provider's configured key selection, including
rotation through enabled key-pool entries. Explicit custom headers replace
matching header names case-insensitively. Missing required authentication or
an empty version header returns a configuration error before any HTTP request.

Source contract checks are in `tools/tests/anthropic_model_catalog.test.mjs`;
request-header unit tests are in `model_list/tests.rs`.

## Boundary

Runtime-owned behavior is requested through `ProviderRuntimeSupport`;
`operit-providers` does not depend on `operit-runtime`.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.

## Tool Continuation and Response Completion

Enhanced chat consumes the finalized `ToolExecutionBatch.message` directly;
there is no second formatting or independent truncation pass for model history.
A nonempty continuation must carry a nonblank finalized message. Response
completion diagnostics record sizes and execution identifiers, not response text.

OpenAI-compatible streaming and non-streaming producers record terminal failures
before closing their shared response channels. `RevisableTextStream` exposes that
failure through boxed consumers. Enhanced chat checks it after collection for both
initial responses and tool-result responses, before normal completion, token-usage
persistence, or further tool dispatch. The existing runtime error lifecycle receives
the error; partial text is retained and user cancellation remains distinct.
