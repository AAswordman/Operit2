# Prompt and summary hooks

## Sending and independent estimation

History and finalization hooks have separate send and estimate registrations:

| Operation | Send registration | Independent estimate registration |
| --- | --- | --- |
| Prepare history | `registerPromptHistoryHook` | `registerPromptEstimateHistoryHook` |
| Finalize the request | `registerPromptFinalizeHook` | `registerPromptEstimateFinalizeHook` |

History hooks receive `before_prepare_history` and `after_prepare_history`.
Finalization hooks receive `before_finalize_prompt` and `before_send_to_model`.
Mutations are passed to subsequent handlers and stages before token calculation.
The current input is reconciled into the finalized history using the same logic
as a real send.

Independent estimates execute the estimate history and finalization families;
they do not execute the corresponding send families. System prompt composition
and tool prompt composition remain shared. Estimates do not run input submission
hooks, generate a response, or save a conversation message.

A plugin that changes both real requests and estimates must register both
handlers explicitly. Use the same function when the transformation is read-only
and identical. Worldbook, the context limiter, plan mode and goal mode register
estimate finalizers. Debug dumps and device-information collection remain tied
to real sends rather than running again for independent estimates.
Keep side effects out of estimate handlers, including advancing a workflow or
recording that a message was sent. The estimate context identifies the actual
conversation and selected execution participant in `metadata.executionContext`.

The token calculation performed immediately before a real model request already
uses its finalized history; it does not execute estimate handlers again.

## Empty text and nonparticipation

An omitted field leaves that field unchanged. An explicit empty string clears
the field, and whitespace is preserved exactly. This applies to string results
and the `rawInput`, `processedInput`, `systemPrompt` and `toolPrompt` object fields
in prompt hooks. Empty history arrays explicitly clear the corresponding array.
Return `null` or `undefined` to contribute nothing.

These hooks modify model-facing content. They do not provide a generic display
transformation or a message persistence interceptor.

## Summary generation

`registerSummaryGenerateHook` runs at:

1. `before_prepare_summary_prompt`
2. `before_send_to_model`
3. `after_generate_summary`

The payload includes `chatId`, `chatHistory`, `preparedHistory`, `systemPrompt`,
`summaryPrompt`, `summaryResult`, `previousSummary`, model parameters and metadata.
The histories retain turn kinds, tool identities and metadata.

An object result can replace `chatHistory`, `preparedHistory`, `systemPrompt`,
`summaryPrompt` and `summaryResult`; metadata entries are merged. Every handler
sees changes from the preceding handler, and metadata carries across stages.
Both before stages include a proposed `preparedHistory`. An edited prepared
array is used directly, including an empty array. When only source components
change, the prepared request is rebuilt from those components. A custom prepared
history remains intact through subsequent stages that leave prompt content unchanged.
Invalid history structures or result types fail the
operation rather than silently discarding the plugin's requested changes.

A string result replaces `summaryPrompt` at either before stage and
`summaryResult` at `after_generate_summary`. Empty strings are accepted as
explicit replacements; the normal summary-content validation still applies to
the final result. Return `null` or `undefined` to leave the summary unchanged.
