# Conversation summary request caching

Conversation compaction must preserve the model-facing conversation prefix. A
separate HTTP request can reuse a provider's prompt cache; changing the system
prompt, tools, message roles, contents, or protocol metadata prevents that reuse.

## Request construction

Automatic summaries, manual summaries, and summaries inserted into history use
the normal structured conversation projection. Source messages are not numbered,
flattened into a transcript, stripped of reasoning signatures, or cleaned for
display. The previous checkpoint remains a normal history turn. The conversation's
system prompt and tool definitions remain in the request. Summary instructions
are appended as the final user turn.

The application entry is `EnhancedAIService::generateSummaryForConfiguration`;
provider generation accepts a `ConversationContext` through
`ConversationService::generateSummaryWithProgress`. The unused tuple/text-only
summary entry points have been removed so internal callers cannot accidentally
skip conversation preparation, tool schemas, or participant configuration.

The summary retains the existing four-section output structure. Its content rules
require evidence for blockers and distinguish plans, assistant claims, and actual
verification. Missing execution records must not be turned into missing credentials
or a request for user approval. The generation directive is excluded from task
history. A short leading instruction requests summary output without tool calls;
the prompt does not say that the underlying task is paused.

After a chat response (including a tool-limit interruption), the provider runtime
retains its prepared history, actual tools, thinking configuration, parameters,
and model binding. The application acknowledges this context after persisting the
assistant segment, using a SHA-256 fingerprint of the ordinary source projection.
The cache keeps one committed context and at most one pending completion per chat
participant. A pending response does not replace the committed context used by a
background summary. Editing, deleting, adding messages, changing variants, or
selecting another participant changes the source identity and prevents stale reuse.
The fingerprint contains canonical metadata as well as message roles and contents.

History without a matching in-memory preparation, including a reopened session
or a selected historical range, uses ordinary chat preparation and prompt hooks
with the conversation's workspace folders and participant. The system is composed
for that conversation, never replaced with a standalone summarizer system. This
does not promise exact reuse of the earlier request's dynamic prompt after restart.

The configured `SUMMARY` model binding is respected. When its provider and model
match the retained conversation's actual binding, the summary also retains that
request's model parameters and thinking settings. Selecting a different summary
model cannot reuse the original model's cache. `SUMMARY_GENERATION` accounting,
summary hooks, cancellation, progress, checkpoint storage, and display-only
conversation reviews remain part of the summary lifecycle.

Tool schemas remain present for cache alignment. Summary generation sends directly
to the provider without entering the chat tool execution loop. The appended
instruction prohibits tool use, and tool-call output fails summary generation;
no tool is executed. No new history pruning or tool-result truncation is added.

Summary hooks receive the actual prepared conversation prefix and trailing summary
directive. `systemPrompt` describes the conversation's existing first system turn;
`chatHistory` contains the complete prepared source, including system turns and
previous checkpoints. Explicit hook edits to history or system instructions can
change the prefix and therefore affect cache reuse. Existing prepared-history
replacement and source-mutation reconciliation remain supported.

## Verification and diagnostics

Provider regressions compare serialized DeepSeek Chat Completions and Responses
requests, verifying that the summary has the original message prefix and tools.
OpenAI Responses additionally verifies that `prompt_cache_key` stays equal.
Other regressions cover protocol metadata, old checkpoints, tool boundaries,
source edits, participant isolation, pending completions, and summary hooks.

`chat.summary.context` reports whether an exact committed preparation was reused.
`chat.summary.usage` reports the actual provider model, whether the summary model
matches the source model, and input/cached/output token counts. These logs contain
counts and identifiers rather than prompt text or credentials. Provider cache
availability, expiry, routing, and cache granularity still determine real hits;
local prefix equivalence alone cannot guarantee a particular hit rate.

Live quality verification and observed limitations are recorded in
[summary-quality-smoke-2026-11-25.md](summary-quality-smoke-2026-11-25.md).
