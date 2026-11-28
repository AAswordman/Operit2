# Long conversation rendering

Chat display loads an indexed window of messages, while a single Markdown body
can still contain thousands of formatted paragraphs. Two independent quadratic
operations made those messages block the Flutter UI isolate even after their
parsed Markdown had been cached.

## Span ranges

`CanvasMarkdownNodeRenderer` previously walked the complete `InlineSpan` tree
for every paragraph range. With P paragraphs and S styled spans, this required
O(P × S) work. `MarkdownSpanRangeIndex` now records UTF-16 subtree boundaries
once and uses a binary search to visit only overlapping child ranges. Fully
covered subtrees are retained. Partial spans preserve inherited styles, gesture
recognizers, mouse behavior, locale and spelling properties. Widget spans count
as Flutter's single replacement-character position.

The index is local to one prepared span tree and is rebuilt with the existing
style/content invalidation. It does not introduce another persistent text cache.

## Paragraph layout and semantics

A flat Column also created an unbounded set of sibling render objects and
mergeable semantic configurations. CPU samples after the range fix showed
Flutter's `_marksConflictsInMergeGroup` and debug sibling-list validation as the
dominant remaining costs.

Paragraphs now occupy stable groups of 32. Each group owns a semantic container
with explicit child nodes, so Flutter retains paragraph labels and link actions
without comparing all paragraphs against all other paragraphs. The groups keep
the same enclosing selection region, existing paragraph gaps, complete text and
scroll ownership. Appending paragraphs extends the last group or appends another
group without moving existing groups into a new hierarchy. This is not viewport
virtualization: the complete mounted message still participates in layout.

## Verification

The opt-in [benchmark](../tools/benchmarks/chat_switch_stress.md) exercises the
production native runtime, SQLite archive import, FFI switch and message watch,
display projection and `ChatArea`. It makes no model-provider requests. Imports
are outside measured regions. All seven cases were opened three times before
and after the changes on the same Windows machine and native library.

Measured milliseconds from `switchChat` to completed mounted content layout:

| Conversation | First open before | First open after | Reopen median before | Reopen median after |
| --- | ---: | ---: | ---: | ---: |
| 20 × 2,000 characters | 925 | 842 | 281 | 236 |
| 200 × 2,000 characters | 302 | 308 | 204 | 203 |
| 2,000 × 2,000 characters | 311 | 268 | 222 | 176 |
| 10,000 × 2,000 characters | 254 | 269 | 194 | 172 |
| 2,000 × 20,000 characters | 769 | 670 | 550 | 427 |
| 2,000 messages, 100,000-character tail | 2,188 | 1,448 | 1,969 | 1,063 |
| 2,000 messages, 500,000-character tail | 33,065 | 5,393 | 30,569 | 4,360 |

Every case returned a 20-message window. The 500,000-character baseline received
that window in 74 ms and its large parser call returned in 604 ms; the subsequent
Flutter work accounted for most of the wait. Range indexing alone reduced its
first open to 16,543 ms; paragraph grouping reduced it further to 5,393 ms.
Unrelated parser round-trip measurements can include a blocked UI isolate and
must not be interpreted as server execution durations.

These are headless Flutter **debug** measurements, including debug render-tree
checks and enabled semantics, not Android or release build latency estimates.
They do not include the full application sidebar/composer or the reporting
user's installed plugin configuration. Small-case differences are not treated
as statistically established improvements. A large mounted message still builds
all its paragraphs; the remaining cost is material at 500,000 characters.

The checkout had unrelated model settings compilation errors. Validation used an
isolated source copy with the settings panel from `648f4964e`; chat and Markdown
sources were copied from the working tree. Baseline and optimized runs used the
same native library (SHA-256
`B94DA390B2B258A008DA65829623FCA616906C09876AEA7772D3D64D9F308026`).
Raw before/after results and logs are retained locally in
`apps/flutter/app/build/chat_switch_snapshot/{baseline,grouped}.json`.

78 related tests passed, including range/Unicode correctness, bounded span
traversal, paragraph spacing, paragraph/link semantics, selection across the
32-paragraph boundary, inline math, tables, streaming and keyboard copy.
Four additional failures (HEIC cache, video, initial live-chat alignment and
retaining a final tool row) also reproduced with the original renderer sources.
Directory analysis reported no errors or warnings and one existing file-name
style notice in `Utf16Text.dart`.
