# Conversation model selection

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

`chatModelBinding` and `setChatModelBinding` route by the explicit chat ID.
`ProviderRuntimeSupport.modelBindingForChat` uses the same stored selection for
execution, display, message configuration and context estimates. A missing
provider or model removes the obsolete selection from the persisted conversation
and the conversation uses the current global model. Re-adding the same IDs does
not restore the old selection. Cleanup uses the existing extension deletion API
and runs before execution admission acquires its metadata lease. The existing
persistence transactions, synchronization and execution protection are unchanged.
Catalog, storage and decoding errors propagate;
they do not change model selection. Auxiliary function bindings remain separate.
An explicit per-request provider/model pair still has its existing semantics.

Plugin participants that use the conversation default receive its effective
model as `defaultModelBinding`. Participants with their own fixed model retain
their plugin-owned binding and existing composer restrictions.

The routed API additions advance the peer protocol revision to 29.
