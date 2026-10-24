# Plugin state during send preparation

A retained JavaScript realm preserves the plugin's modules and service state.
Business code must also use that lifetime: a hook should not recreate directories,
check default files or rerun migration each time it reads data.

## Worldbook storage

`initializeWorldBookStorage` opens the current package configuration root through
one shared Promise. The `application_on_create` hook awaits it, and standalone
UI/tool calls use the same Promise as a first-use fallback. Registration and module
imports perform no storage I/O. Initialization creates the entries, groups and
variables files and performs the existing legacy migration once. Ordinary reads
and writes then use the initialized file paths directly.

The ready state holds paths, not entries or variable contents. A root change opens
the new root. A plugin reload creates a new owner. A failed open remains failed for
that owner/root, rather than repeatedly starting migration in prompt hooks.

## Data shared by hooks

Rust creates `metadata.promptPreparationId` when a send starts preparing its
request. The ID reaches both system composition and prompt finalization. It is
internal metadata, not a stored message field or model prompt. Independent context
estimation uses a separate runtime without this ID.

Worldbook reads entries, variables and chat history once per preparation. The
system hook retains this snapshot; the final hook consumes it. The snapshot also
records the chat and participant, and rejects reuse with another execution context.
A hook without a preparation ID reads fresh data. Abandoned snapshots are bounded
to 32; eviction falls back to a fresh read if the preparation later resumes.

Each new send reads the backing files again. Local edits and synchronized file
changes therefore take effect on the next preparation, while hooks of an ongoing
send use a consistent snapshot. Variable initialization and message patch handling
still run for an empty book. Updates to the shared variable file are serialized
within the owner so simultaneous chats cannot overwrite one another's changes.

## Callback dispatch

Prompt registrations currently select a hook family, not individual stages. Only
registered callbacks enter JS, but a registered system/final callback receives all
stages of that family and uses its existing JS stage guard. Worldbook therefore
still has five callbacks per preparation, three of which return immediately. This
change preserves that dispatch behavior and the existing plugin lifecycle.

## Verification

```sh
node --test tools/tests/worldbook_lifecycle.test.mjs tools/tests/worldbook_plugin_boundary.test.mjs
cargo run --quiet --manifest-path core/Cargo.toml --package operit-plugin-sdk-codegen -- check core/crates/plugin/sdk/src plugins/types
cd apps/flutter/app
flutter test integration_test/plugin_send_performance_test.dart -d macos
```

The macOS fixture uses production QJS, host tools and SQLite with isolated storage.
Its model endpoint is a local SSE fixture. Warm worldbook sends assert zero
`make_directory`/`file_exists` calls and one `get_chat_messages` call before HTTP.
The book is empty in this timing fixture; nonempty keyword, depth and variable
injection are covered separately by the lifecycle tests.

Warm send preparation host requests attributable to the empty worldbook fixture:

| Operation | Before | After |
| --- | ---: | ---: |
| Create directory | 7 | 0 |
| Check file existence | 19 | 0 |
| Read file | 6 | 2 |
| Read chat history | 2 | 1 |
| Write variable state | 1 | 1 |
| Total | 35 | 4 |

Counts exclude first initialization and cover the interval before HTTP. Completed
reply estimates use retained request state as described below.

Measured on macOS on 2026-10-18, Flutter Debug with the Rust release bridge. Each
cell is the median of three retained-owner sends, excluding the first send in each
scenario. These timings use an empty book and the immediate local model fixture.

| Measurement | Before | After, run 1 | After, run 2 |
| --- | ---: | ---: | ---: |
| Send to HTTP, without worldbook | 73.446 ms | 88.869 ms | 76.917 ms |
| Send to HTTP, with worldbook | 95.125 ms | 99.286 ms | 87.075 ms |
| Reply observed complete, with worldbook | 130.936 ms | 140.860 ms | 122.629 ms |
| Worldbook hook total before HTTP | 17 ms | 4 ms | 3 ms |
| All host tool requests before HTTP, with worldbook | 63 | 32 | 32 |

Run 2 worldbook send-to-HTTP samples were 87.075, 83.782 and 101.775 ms. The
without-worldbook control also varied between runs, so whole-send improvement
should not be inferred from a single timing. Host request counts and the per-send
single-read assertions passed in both after runs. Hook timings use millisecond
trace resolution and are nested within the whole-send interval.

## Completed reply window

A completed response is estimated from its actual execution history (including
final prompt transformations and tool rounds) and the exact tools passed to the
latest provider request. These remain owned by the current execution. The provider
calculator reuses its token cache; completion does not create a new send runtime,
prepare history, compose prompts, invoke plugins or read worldbook storage.

Normal completion already appended its assistant turn and borrows that history.
Early completion with only thinking appends the pending turn once; empty completion
adds no synthetic turn. Tool continuation replaces the retained tools with that
round's final policy output. Only the scalar completed window survives response
closure for the message commit. Starting a new turn or cancelling clears it; a
failed estimate supplies no completed value and never falls back to an old turn.

Independent preview/estimate APIs still prepare fresh data when explicitly called.
They are no longer called as part of reply completion. Route changes obtain a
persisted history copy only when a route actually needs a resume context.

The macOS integration test rejects any plugin hook, public API or host-tool request
between the local model receiving a normal fixture request and the assistant commit
starting, and verifies that a positive completed context window is persisted.

Measured on macOS on 2026-10-18 with the same local SSE fixture, Flutter Debug
and Rust release bridge. Each row uses three warm sends per scenario:

| Completion measurement | Role card before | Role card after | Role card + worldbook before | Role card + worldbook after |
| --- | ---: | ---: | ---: | ---: |
| Final-message trace to assistant commit start | 23 ms | 0 ms | 29 ms | 0 ms |
| Extra plugin hooks before commit | 16 | 0 | 19 | 0 |
| Extra host tool requests before commit | 4 | 0 | 7 | 0 |
| Assistant commit duration | 2 ms | 2 ms | 2 ms | 2 ms |

The retained-request token estimator logged 0 ms for all eight sends (millisecond
resolution, not zero execution cost). Every send persisted a positive context
window. HTTP-received to reply-observed completion had warm medians of 6.557 ms
without worldbook and 6.429 ms with worldbook. The final-message trace occurs after
the retained-request calculation; the HTTP-to-completion interval includes it.

The same run measured send-to-HTTP / reply-observed medians of 65.400 / 71.957 ms
for the role card and 83.805 / 90.551 ms with worldbook. Configuration-demand-read
changes were being made concurrently, so these whole-send timings are observations,
not an isolated attribution to this completion fix. The completion assertions
passed for all eight sends, including both first-use sends.
