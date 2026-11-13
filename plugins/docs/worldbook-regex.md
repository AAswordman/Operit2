# Worldbook regex and message transforms

Worldbook 1.3.0 adds ordered replacement rules. Open **Worldbook manager → 正则规则**
to add, edit, enable, reorder, test, import, or export them. Existing keyword
regex still controls which worldbook entries activate; replacement rules change
the selected content after activation.

The manager, previews, validation errors, import compatibility warnings, tool
results, and variable-template examples follow the host's Chinese or English
language setting. User-authored names, patterns, replacements, and worldbook
content keep their original text when the interface language changes.

## Execution modes

| Display only (`markdownOnly`) | Model only (`promptOnly`) | Effect |
| --- | --- | --- |
| false | false | Changes new user/assistant text before its terminal save. Editing applies it only when `runOnEdit` is enabled. |
| true | false | Prepares a native replacement program once and applies it incrementally during generation. Completed display copies use the same native program semantics. Stored text, edit input, copy actions, and model history retain the source. |
| false | true | Changes model-facing text for both real sends and independent token estimates. |
| true | true | Runs independently for display and model-facing text. |

New rules start in display-only mode. Changing rules refreshes existing displayed
messages. It does not rewrite old saved messages. Regex errors are reported; an
invalid configuration is never silently ignored or partially imported.

Supported placements are user text (1), assistant text (2), worldbook entry
content (5), and assistant reasoning (6). Worldbook placement runs source rules
and then prompt rules before injection wrappers are added. Display-only worldbook
placement does not alter injected entries. Structured tool calls, tool results,
and status parts are excluded from message transformations.

Rules execute from top to bottom. A role binding selects the actual execution
participant or the saved message author; leaving the binding empty selects all
participants. Depth zero selects the newest message. Message display/edit depth
counts later saved messages; prompt depth counts later conversational prompt
turns, excluding system and tool turns. A tool-using assistant can produce
multiple prompt turns. A new message and an injected worldbook entry use depth
zero. Empty depth limits and `-1` mean unrestricted.

## Import and replacement compatibility

Import accepts a standalone SillyTavern rule, a rule array, `{ "regex_scripts": [] }`,
or JSON character cards with `extensions.regex_scripts` / `data.extensions.regex_scripts`.
Rules are validated as a complete batch and appended with fresh IDs. Choose the
local character binding after importing a character card. Export preserves the
current order in a JSON array. Rules live in the plugin's VFS config directory as
`regex_scripts.json`; existing worldbook entries and variables keep their own files.

Patterns accept plain patterns or `/pattern/flags`. A plain pattern
does not implicitly enable global replacement; add `g` to replace every match.
Replacement text supports `$0`, numbered captures, `$<name>`, and `{{match}}`.
`trimStrings` removes literal substrings from captured values. Empty replacements
and whitespace are preserved. Replacement text is never evaluated as JavaScript.

`substituteRegex` supports no macro expansion (0), direct expansion (1), or escaped
expansion (2) in the search pattern. Replacement text and trim strings can use the
existing Worldbook variable macros, including `{{getglobalvar::key}}`,
`{{getvar::key}}`, and the Worldbook global/character/local variable forms.
Reading those variables for display does not initialize or advance variable state.
The isolated editor preview has no live conversation variable context.

Display execution uses Rust regex automata over valid Unicode text. Flags `g`,
`i`, `m`, `s`, and `u` are accepted; Unicode matching is always enabled. Lookaround,
backreferences, patterns that can match empty text, unsupported flags, and patterns
exceeding the automaton size limit fail explicitly before generation starts.
Unicode character classes and case folding follow Rust regex semantics. Unicode
word-boundary assertions are not supported by this DFA; native plugins can use
`(?-u:\b)` for ASCII boundaries. Source/model rules and the isolated editor preview
execute JavaScript RegExp, so a preview is not validation of native compatibility.
Display pattern, replacement-literal and trim macros resolve once at preparation;
captured generated text is inserted literally and is not evaluated as a macro template.

Compatibility follows [SillyTavern's regex engine](https://github.com/SillyTavern/SillyTavern/blob/release/public/scripts/extensions/regex/engine.js)
and [rule editor](https://github.com/SillyTavern/SillyTavern/blob/release/public/scripts/extensions/regex/index.js)
for the supported fields. SillyTavern slash-command placement (3) is rejected.
Deprecated placement (0), arbitrary SillyTavern macros, embedded scripts, and
browser HTML rendering are outside this feature. Regex output uses Operit's
existing Markdown renderer.

## Plugin API

`ToolPkg.registerMessageStreamTransform({ id, function })` registers a native
display-program preparer. The plugin receives `prepare_display` for an uncached history
window, or with one context before starting a live provider. `eventPayload` holds
`chatId` and an ordered `messages` array containing `timestamp`, `sender`,
`participantId`, `depth`, and `isEdit`. It can read configuration once for the batch,
resolve variables, choose scopes and construct any ordered program supported by
the typed SDK. It returns a `ToolPkg.TextTransformBatch` containing a shared
`programs` table and one `messagePrograms` index per context, or `null` / `undefined`
to decline the batch. Invalid index counts and out-of-range indexes are errors.
Each shared pipeline is transmitted, decoded and compiled once:

```ts
export async function prepareDisplay(event: ToolPkg.MessageStreamTransformEvent): Promise<ToolPkg.TextTransformBatch> {
  return { programs: [{ rules: [{
    pattern: "<private>.*?</private>", replacement: "", flags: "gs",
    kinds: ["markdown", "thinking"], trimStrings: []
  }] }], messagePrograms: event.eventPayload.messages.map(() => 0) };
}

ToolPkg.registerMessageStreamTransform({ id: "private-text", function: prepareDisplay });
```

The runtime retains compiled programs for sixteen exact context windows, including
chat, timestamps, sender, participant, depth and edit state. Preparers must return
the same result for an unchanged context and plugin data. VFS commits under device
or space plugin directories automatically clear prepared programs and refresh
mounted rows, including writes, appends, replacements, renames and deletions.
Plugins only save their files; no manual invalidation API is required. Package
enable/disable also clears programs. Late preparation cannot populate a newer revision.

Rules use Rust regex syntax and execute in registration order, then rule order.
Replacement tokens are `$0`, `$1`, `$<name>`, and `$$` for a literal dollar sign.
Compilation resolves capture references once. Every observer has independent
matching state and shares the immutable compiled automata. Rust keeps that state
across chunks and snapshots it at explicit savepoints; rollback restores pending
matches as well as displayed text. No plugin calls occur in the chunk loop.
Literal-only patterns use a direct streaming text matcher without compiling a
regex engine; only a possible matching suffix remains buffered. Replacement
references are parsed once without regex matching.

Unmatched text is emitted as soon as the automaton proves it safe. Only a possible
match stays buffered. A future-dependent expression such as `a.*z` may have to
retain text until `z` or the end of the semantic part arrives. Rules never match
across Markdown, thinking or opaque tool/status boundaries. Original semantic
delimiters and tool payloads pass through unchanged, including unfinished tools.
Completed messages and presentation bubbles use the same compiled execution rules.

`ToolPkg.registerMessageTransformHook({ id, function })` registers an awaited,
ordered text transformer before persistence. The existing `registerChatMessageHook` remains a message
notification API. Transform hooks receive:

```ts
{
  eventName: "before_persist",
  eventPayload: {
    chatId: string,
    timestamp: number,
    sender: string,
    participantId: string | null,
    partId: string,
    kind: "markdown" | "thinking",
    text: string,
    depth: number,
    isEdit: boolean
  }
}
```

Return a string to replace text, including `""` to clear it; return `null` or
`undefined` to leave it unchanged. Promises are awaited, and each handler receives
the previous handler's result. Other result types and handler failures stop the
operation visibly. A shared pre-hook timeout bounds each message transformation.
The host retains message IDs, attributes, semantic parts, and tool structure.
The persistence hook covers new user text, terminal assistant saves (including
partial replies saved after provider failure), and explicit edits. Intermediate
streaming checkpoints remain canonical generation snapshots.

The former whole-text `before_display` callback is removed. Display replacements
use `registerMessageStreamTransform` during generation and completed projections,
including response-presentation bubbles. Preparers must be read-only and safe to
repeat for completed projections. Committed VFS changes to plugin configuration or
data automatically cause existing static rows to be projected again.
Plugin enable/disable also invalidates those projections. Changing a transformer
during generation takes effect when the completed message is displayed.

Model-facing replacements use the existing `registerPromptFinalizeHook` and
`registerPromptEstimateFinalizeHook` APIs at `before_send_to_model`. Worldbook
registers the same read-only function in both families after its entry-injection
handler. Current input and its matching history turn are reconciled so a rule
does not run twice against the same input. See [prompt-hooks.md](prompt-hooks.md)
for the send/estimate contract.
