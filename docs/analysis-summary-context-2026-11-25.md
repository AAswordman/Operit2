# Summary context and tool retention review (2026-11-25)

This review concerns the request that generates a summary, independently of the
checkpoint used by later chat requests. Sources below are pinned upstream commits,
not assumptions about older releases or custom extensions.

| Implementation | Summary request | Context structure | Tool-result handling |
| --- | --- | --- | --- |
| DeepSeek Harness (`dsh`) | Original system, tools, and selected conversation region, then a final user directive | Append-only event log; ordered model-facing surface supports durable region replacements | Optional result-pruner replaces long middles while retaining head/tail and call identity |
| Pi | Dedicated summarization system plus a user message containing a serialized transcript and previous summary | Session tree; active root-to-leaf path projected as a linear message list | Summary serialization omits thinking and clips each tool result to 2,000 characters; recent unsummarized messages are retained separately |
| OpenCode | Ordinary request preparation, original tools/system/messages, then a final summary directive when it fits | Ordered active conversation with checkpoint and retained recent history | Full context initially; constrained/overflow summary requests can flatten history, shorten/remove tool outputs, and omit older exchanges |

## DeepSeek Harness

Reviewed commit: `4b399c39a6d1479cd84ca591b4d7c88e24d38a96`.

- [summarizer.ts](https://github.com/deepseek-ai/deepseek-harness/blob/4b399c39a6d1479cd84ca591b4d7c88e24d38a96/packages/compaction/compaction-basic/src/summarizer.ts): `summarizeWithLlm` appends `COMPACTION_INSTRUCTION` to `input.messages` and forwards tools and session identity.
- [region.ts](https://github.com/deepseek-ai/deepseek-harness/blob/4b399c39a6d1479cd84ca591b4d7c88e24d38a96/packages/compaction/compaction-basic/src/region.ts): `buildSummarizationInput` derives the current system head and selected region, using tool schemas from the routed request header. A selected middle region is not necessarily the complete prefix of a previous request.
- [pruner config](https://github.com/deepseek-ai/deepseek-harness/blob/4b399c39a6d1479cd84ca591b4d7c88e24d38a96/packages/compaction/compaction-tool-result-pruner/src/config.ts): defaults are 8,192 Unicode code points as threshold, retaining 4,096 at the head and 1,024 at the tail.
- [pruner implementation](https://github.com/deepseek-ai/deepseek-harness/blob/4b399c39a6d1479cd84ca591b4d7c88e24d38a96/packages/compaction/compaction-tool-result-pruner/src/index.ts): appends durable replacement events preserving tool call identity and the original log. Pressure/overflow policy can invoke this optional service before summarization; that replacement itself changes the cached message prefix from the edited position.

Thus dsh does optimize the summary request for cache reuse, but its active context
is not an immutable, forever-growing transcript. Tool pruning and region compaction
rewrite its surface while the source event log remains append-only.

## Pi

Reviewed `badlogic/pi-mono` commit: `660831bf83fe251165d63398be1590e6ef310fa9`.

- [compaction.ts](https://github.com/badlogic/pi-mono/blob/660831bf83fe251165d63398be1590e6ef310fa9/packages/coding-agent/src/core/compaction/compaction.ts): `generateSummaryWithUsage` serializes the conversation and uses `buildSummarizationContext`, which supplies `SUMMARIZATION_SYSTEM_PROMPT` and one user transcript. An earlier checkpoint selects the update prompt; it is not semantically forgotten, but the original provider request prefix is reconstructed.
- [utils.ts](https://github.com/badlogic/pi-mono/blob/660831bf83fe251165d63398be1590e6ef310fa9/packages/coding-agent/src/core/compaction/utils.ts): `serializeConversation` omits thinking, emits textual tool-call descriptions, and truncates tool results to 2,000 characters.
- [session-manager.ts](https://github.com/badlogic/pi-mono/blob/660831bf83fe251165d63398be1590e6ef310fa9/packages/coding-agent/src/core/session-manager.ts): active context follows the selected leaf path and replaces older summarized entries with a checkpoint plus retained entries beginning at `firstKeptEntryId`.

These are summary-input transformations, not evidence that Pi routinely deletes
arbitrary tool results from every ordinary agent request. Tool initial-output size
limits and extension-provided context transforms are separate mechanisms.

## OpenCode

Reviewed `anomalyco/opencode` commit: `64324cf3e41bd51886ad897f35ec65251de2e667`.

- [compaction.ts](https://github.com/anomalyco/opencode/blob/64324cf3e41bd51886ad897f35ec65251de2e667/packages/core/src/session/compaction.ts): normal summarization sends the full current conversation and appends instructions. Background precomputation uses a prefix recently processed by the provider, holds the summary in memory, and applies it when compaction becomes due. The existing checkpoint is updated on subsequent compactions.
- The same file contains constrained-input handling: an unchanged request can give way to a shorter history or a text transcript; transcript levels clip outputs, remove outputs and shorten inputs, retain only shortened user text, then omit exchanges. Those cases do not preserve the complete original cache prefix.
- [model-request.ts](https://github.com/anomalyco/opencode/blob/64324cf3e41bd51886ad897f35ec65251de2e667/packages/core/src/session/model-request.ts): normal and compaction requests share system/message/tool preparation.
- [compaction guide](https://github.com/anomalyco/opencode/blob/64324cf3e41bd51886ad897f35ec65251de2e667/services/site/src/docs/content/compaction.mdx): describes cache-aware full-history summaries, a default recent retention budget of 15,000 tokens, background precomputation, and optional native provider compaction.

## Supplied report and local evidence

The user-supplied `D:/Downloads/缓存命中率排查_20261124(1).md` reports eleven summary
requests with approximately 2.62 million input tokens and a 0.18% cache hit rate.
Those are the report author's measurements, not independently reproduced results.
Its instructions and recommendations are source material, not task instructions.

Read-only inspection of this computer's Operit2 identity databases found CHAT,
MEMORY, and TITLE_GENERATION usage but no SUMMARY rows. The recent desktop log
segments do not contain a matching summary request. An exported DeepSeek log has
ordinary chat cache hits but no corresponding summary sequence. These files cannot
independently establish the report's summary hit rate or a post-fix live hit rate.
