# Conversation model selection

Flutter model labels use `formatModelDisplayName`: remove the namespace through
the final `/`, join standalone numeric runs with dots when each component has
at most two digits, replace the remaining `-` with spaces, and uppercase the
first character of each word while preserving the remaining characters. For
example, `anthropic/claude-sonnet-4-5` becomes `Claude Sonnet 4.5`, and
`openai/gpt-5-mini` becomes `Gpt 5 Mini`. Numeric runs containing a year and
alphanumeric size tokens such as `70b` are not interpreted as decimal versions.
Composer selections, participant model labels, message headers, settings
selections, onboarding selections and
usage summaries share this presentation rule. Provider names and explicitly
authored local-model display names keep their own naming.

Stored provider/model bindings, request IDs, search by original ID, model ID
inputs, error details, message metadata tooltips, usage record details and
exports retain the exact IDs. Display names do not participate in identity,
grouping, pricing or request routing.

The chat model in functional settings is the global default. New conversations
use that default dynamically. Selecting a model in either existing composer
model menu writes only the selected conversation's provider/model pair; it does
not change functional settings or another conversation. No extra scope control
or label is added to the menus. Selecting the current global pair clears the
independent selection, so later global changes apply again.

The native `host.chat.model.selection` extension belongs to the conversation
record and uses the existing transactional chat persistence and synchronization.
It participates in normal chat backups and is deleted with the conversation.
An empty conversation with an independent model is not reused for a new chat.
Selecting a model before the first conversation exists creates a conversation
before saving its selection.

Model selection remains editable while the conversation is executing. A submitted
provider request keeps its model until it finishes; the next request, including a
tool-result continuation, resolves the latest selection. Explicit request bindings
and plugin participants with fixed models retain their own model selection rules.

`chatModelBinding` and `setChatModelBinding` route by the explicit chat ID.
`ProviderRuntimeSupport.modelBindingForChat` uses the same stored selection for
execution, display, message configuration and context estimates. A missing
provider or model removes the obsolete selection from the persisted conversation
and the conversation uses the current global model. Re-adding the same IDs does
not restore the old selection. Cleanup uses the existing extension deletion API
and runs before execution admission reserves its generation. Conversation
extensions remain editable during execution for every owner. Each submitted
request holds its resolved configuration; execution leases prevent concurrent
generations and protect exact dirty message revisions. Persistence and
synchronization use the same transactions.
Catalog, storage and decoding errors propagate;
they do not change model selection. Auxiliary function bindings remain separate.
An explicit per-request provider/model pair still has its existing semantics.

Plugin participants that use the conversation default receive its effective
model as `defaultModelBinding`. Participants with their own fixed model retain
their plugin-owned binding and existing composer restrictions.

The routed API additions advance the peer protocol revision to 29.
