# Summary quality smoke verification — 2026-11-25

The change is evaluated by the summary's ability to preserve and resume the task,
not by prompt length or whether a template is old. The original four-section
structure remains; content wording was adjusted after observed generation errors.

## Method

- Live provider: the locally configured DeepSeek endpoint, `deepseek-v4-flash`,
  thinking enabled at the configured `low` effort.
- Only synthetic conversations were sent. Application preferences were read without
  changes; no existing chats, user files, or runtime logs were submitted to the model.
- Every request retained its original system, message sequence, and tool schemas,
  with a summary directive appended as a final user message. Tools were advertised
  but the runner never executes them.
- `export_summary_prompt` exports the actual Rust production directive. Prompt
  variants and raw responses are retained locally under
  `artifacts/summary-smoke-20261125/` (ignored build artifacts).
- The two cases cover a cache-fix handoff and an order-migration task containing
  corrections, retained older constraints, contradictory assistant claims/test
  evidence, an actual test blocker, and an untrusted instruction in a tool result.
- Fixed structure, required facts/identifiers, completion status, and absence of
  tool calls are checked automatically. Semantic assessment also reads the actual
  output; keyword coverage alone does not establish correctness.

This is an isolated live prompt-quality check, not an end-to-end test of the
application's summary lifecycle, a long-context benchmark, or a cache-hit benchmark.

## Observed comparisons

| Variant | Requests | Automatic structure/fact checks | Semantic observations |
| --- | ---: | --- | --- |
| Initial full template plus trailing pause instruction | 2 | 2 passed | One output turned the generation instruction into a reason to postpone the task; another invented a mandatory dual-write/backfill/gray-release strategy. |
| Original template verbatim, appended to retained chat context | 2 | 1 passed | The cache-handoff case continued the task and returned `finish_reason=tool_calls`; the other case retained key facts but added unsupported test/migration details. This evaluates the old template in the new request layout, not its original standalone-system layout. |
| Long control/evidence preface plus original template | 2 | 2 passed | Improved treatment of test evidence, but outputs still included commentary about the current checkpoint generation. Extra rules did not remove the problem. |
| Short preface plus original template | 4 | 4 passed | Key facts survived, but one output inferred missing API credentials from the absence of recorded requests; another treated generation as the reason the test remained unexecuted. |
| Final short preface plus evidence-based template wording | 4 | 4 passed | Both cases, repeated twice, retained the latest goal, earlier constraints, corrected identifiers, actual validation, unverified items, and next steps. No tool calls, invented missing-credential blocker, compaction-induced postponement, or invented dual-write/gray-release plan was observed. |

The final four generations still have imperfections: occasional unnecessary
meta-commentary, some repetition, and an imprecise chronology in one narrative
section. They must not be described as universally reliable or factually perfect.
The tests establish improvement on these cases, not an all-model quality guarantee.

Total observed usage for all fourteen comparison requests: 25,521 input tokens and
30,095 completion tokens (provider-reported completion includes reasoning tokens).
Cache token counts from these short, repeatedly reused fixtures do not establish
the application's real summary cache-hit rate.

## Final prompt changes

The format remains Core Task Status, Interaction & Scenario, Conversation Progress
& Overview, and Key Information & Context. The changes target the actual errors:

1. Define the source as the conversation before the summary directive; the directive
   is not a new task, progress, pause reason, or next step.
2. Request summary output without continuing the task or calling tools.
3. Replace demands to explain every unstarted phase with reporting only known
   reasons and established blockers.
4. Remove mandatory filler length and the requirement for two sentences per fact.
5. Distinguish plans, assistant claims, and observed validation; absence of recorded
   execution does not imply missing capabilities, credentials, or authorization.
6. Retain explicit corrections and older constraints that remain valid.

Final exported prompt SHA-256:
`4201d9994713a2c6c21787893311e61affff3777aa59ff0f7152b804bb845774`.

Unedited generated examples from the final version:

- [Cache-fix handoff](verification/summary-quality-2026-11-25-cache.md)
- [Corrections, test evidence, and blockers](verification/summary-quality-2026-11-25-corrections.md)

These examples summarize the synthetic fixture's state, not the current status of
the real engineering task.

## Reproduction and code checks

Sources:

- `core/crates/provider/services/examples/export_summary_prompt.rs`
- `tools/tests/chat_summary_quality_smoke.py`
- `tools/tests/fixtures/chat_summary_quality.json`

Export the production prompt with `cargo run --offline -p operit-providers
--example export_summary_prompt` from `core/`, with `RUSTFLAGS=-Awarnings`, and save
stdout as UTF-8. The example's `--template-only` argument exports the base template
for isolated comparisons. Run the Python smoke helper with `--preferences`,
`--prompt`, `--output`, and optionally `--repeat`; it explicitly requires the
DeepSeek Chat Completions single-key/default-parameter test configuration it supports.
It does not reproduce all application provider settings or resolve a live chat's
follow-model binding.

After the final prompt change, all eight provider summary regressions passed,
including the real provider serialization prefix/tools/cache-key checks. The
Python runner passed syntax compilation and the scoped Git diff passed whitespace
checks. The existing context-cache and runtime history-change regressions had
already passed; the prompt wording does not change their state management.
