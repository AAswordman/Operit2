# Current plugin Chat send contract

## Public send and execution control

The current SDK exposes exactly these three methods for sending and cancellation:

```ts
Tools.Chat.sendMessage(request: Chat.SendRequest): Promise<MessageSendResultData>;
Tools.Chat.sendMessageStreaming(request: Chat.SendRequest): AsyncIterable<Chat.SendEvent>;
Tools.Chat.cancel(chatId: string): Promise<Chat.CancelResult>;
```

Conversation creation, listing, extension access and functional `Chat.call` are
separate existing conversation/model capabilities. `startTurn`, `waitTurn`,
`cancelTurn`, `finishSequence`, callback-based chat streaming and their execution
handles are not part of the current author contract.

The Rust authority is `core/crates/plugin/sdk/src/js_sdk/chat.rs`; TypeScript
exports are in `plugins/types/chat.d.ts`. Native handles and finalization records
are internal runtime-support types, not plugin SDK results.

## One accepted send

`kind: "submit"` preserves original text, complete attachments and an explicit
nullable reply target. Its `turn` selects execution or real user-only persistence.
An explicitly supplied participant is validated by the registered configuration
owner. Group execution records the input before planning, then supplies each actual
planned participant on its continuation.

```ts
const request: Chat.SendRequest = {
  kind: "submit",
  chatId,
  runtime: "main",
  input: { text: "Hello", attachments: [], replyToMessageTimestamp: null },
  turn: { kind: "execute" },
  notifyReply: false,
};
const receipt = await Tools.Chat.sendMessage(request);
```

`kind: "continue"` identifies an existing committed user timestamp and an explicit
participant. It cannot resubmit text, attachments or a reply target. Core verifies
that the source is a real user row and has not been superseded by another user row.
Every continuation requires `summaryPolicy: "defer" | "after_reply"`. Both policies
suppress pre-reply and mid-reply automatic summaries. `after_reply` checks the
completed conversation only after a successful persisted assistant reply and
native send cleanup. Cancelled, blocked, consumed and nonpersistent outcomes do
not trigger that check. Summary failures reject the send while preserving its
already committed messages. No automatic continuation follows this summary.

Both modes use one admission and the existing `startUserMessage` pipeline. Native
execution owns the canonical lease, input-hook dispatch, persistence receipt,
completion publication and lease release. An accepted send is not dependent on
plugin code eventually calling a separate finish method.

## Semantic observation

```ts
try {
  for await (const event of Tools.Chat.sendMessageStreaming(request)) {
    switch (event.type) {
      case "part":
        await applyPartsSnapshot(event.messageTimestamp, event.parts);
        break;
      case "completed":
        await showReceipt(event.result);
        break;
    }
  }
} catch (error) {
  console.error(error);
  throw error;
}
```

A `part` event is an **atomic replacement of this message's complete ordered part
list**, not a text delta. Parts identify markdown, thinking, tool calls, tool
results and status; tool identities and attributes are already structured.
Removed parts disappear from the next replacement. Consumers must replace their
message state, not append every snapshot.

The existing `AssistantMarkupStreamState` performs incremental parsing. Provider
revision events reset that same parser to the revised snapshot. Fully parsed
completion and cancellation snapshots are delivered before the final receipt.
Multiple physical assistant messages retain independent snapshots identified by
their actual message timestamps.

The observer retains the latest snapshot per assistant message and one terminal
result. Snapshot content/metadata accounting is capped at **8 MiB per send**;
overflow rejects observation explicitly. Slow consumption coalesces unconsumed
revisions to the latest authoritative state. It does not create an append-only
chunk queue. Only one `next()` may be pending on the JavaScript iterator.

Streaming submission starts on the first pull. A request is captured when the
iterator is created, so caller mutation cannot change subsequent native admission.
Returning from an unstarted iterator does not send a request.

The `completed` event carries the actual finalized receipt. Its outcome still
distinguishes committed completion/cancellation, nonpersistent output, blocked
input and plugin-consumed input. A consumed request is not proof that the consuming
plugin's own later work completed. Native failures reject iteration instead of
manufacturing a completion event.

## Observation disposal and cancellation

Breaking a loop, calling the iterator's `return()`, or throwing from its consumer
releases observation. Explicit disposal wakes a pending native pull. **None of
these operations cancels model generation.** Native send finalization still runs.

`cancel(chatId)` captures the calling engine's enabled authenticated package and
its exact active execution in this chat before asynchronous model access. A queued
request cannot cancel another package or a newer execution. The acknowledgement
contains `chatId` and `cancelRequested`; the terminal receipt reports the actual
result of the originating send. No active owned execution produces a truthful
`cancelRequested: false` acknowledgement.

## Character-group adapter

`NativeGroupTurnTransport` uses only `sendMessage` and `cancel`. Controller request
keys correlate local attempts; they do not grant native authority. Native-authored
persisted message snapshots are checked against the selected participant.

Each actual send is finalized by the host. Group `finish`/`abandon` methods are
plugin-local planning operations, not additional `Tools.Chat` APIs. Original user
input is persisted once with `record_only` before invoking the response planner.
Every participant uses a continuation referencing that acknowledged timestamp.
Intermediate replies use `summaryPolicy: "defer"`; the final planned reply uses
`"after_reply"` and is the only reply eligible for a completion notification.
Empty or speak-false plans leave a user-only record and do not check summaries.
Continuation finalization
does not increment the original user-submission counter again.

The package registers `group-input-submit` in the production chat input hook.
Host-origin submissions bound to a group are consumed and scheduled through the
retained group controller. The hook waits for the actual input persistence receipt
before returning `Consume` and clearing the composer, while planning and generation
remain outside the hook. Input persistence failure keeps the composer intact;
planner or response failure preserves the already recorded original input and
propagates the original error.
Native sends with `source: "Sequence"` do not recursively enter group planning.
Package-local execution controls retain the exact chat/submission identity for
status, cancellation, and explicit resumption. This registration is covered by
controlled-boundary tests; it is not a claim of live model or cross-platform
integration acceptance.

## Verification scope

Repeatable checks without Rust/Flutter compilation:

```powershell
node --test core/crates/plugin/javascript-bridge/tests/chat-send-contracts.test.mjs
node core/crates/runtime/application/scripts/check-chat-configuration.mjs
node --test plugins/packages/buildin/character_cards/tests/group-execution.test.mjs plugins/packages/buildin/character_cards/tests/native-send-transport.test.mjs
node plugins/packages/buildin/character_cards/scripts/check.mjs
```

The JavaScript iterator is executed against a controlled native ABI. Plugin
transport tests execute actual plugin source with controlled native receipts and
snapshots. Source assertions and Rust syntax parsing are not Rust type checking,
a live AI request or cross-platform runtime acceptance. Native mailbox Rust tests
are present; running the runtime test target requires its native storage host
dependencies. The wasm production check does not execute those native tests.
The character-card main bundle is generated from its source; complete
installed-package acceptance still requires packaging and running the application.

## Conversation context and background ownership

A conversation-wide context refresh reads the current binding and estimates the
request window for every currently bound participant. The stored conversation
window is the largest of those estimates. A refresh with an explicit participant
estimates that participant alone. Historical assistant authors do not select the
conversation's current configuration.

Regenerating an assistant revision requires that revision's persisted participant
identity. Missing identity, catalog initialization errors, and extension read
errors stop regeneration with the original failure instead of selecting a
replacement participant. Manual summary insertion calls the summary model without
resolving a conversation execution participant first, including histories that
contain only user messages.

The shared JavaScript runtime retains a call for each pending Promise reaction
subscribed under that call. Primary hook completion can return `Consume` while
those reactions keep the session detached. Each delivered reaction releases its
reference, and explicit cancellation still prevents its handler from running.
Plugins do not need a fixed-duration keepalive timer for observed background work.

## Participant history policy

Configuration execution results require a complete `historyPolicy` object with
`isolateParticipants` and `labelUserMessages`. Ordinary model conversations set both
to false. Character cards isolate participants; groups also label user messages.
Core applies this policy to both model requests and context window estimates.

An isolated assistant message is attributed by its persisted participant ID under
the authenticated configuration owner. The current participant's own messages keep
their assistant and tool boundaries. Other participants' ordered Markdown speech
becomes user reference context prefixed `[From role: name]`, using the message's
persisted display identity. Thinking, status and tool protocol from other
participants are excluded. Group user input is prefixed `[From user]`. Projection
starts at the latest summary and does not infer historical authors from renamed
cards, duplicate display names or the current group roster. Missing required author
or display identity is an error.

After the final reply, Core refreshes the conversation window across its current
participants and checks each configured model's enabled summary threshold. An
exceeded threshold schedules one existing canonical summary task with
`autoContinue: false`; persistence, cancellation and post-summary context refresh
use the shared runtime lifecycle. This API contains no role-card group concepts.


The native storage host uses standard-library `File::lock` and `File::try_lock`
(Rust 1.89 minimum) rather than `fs2`. Contended leases remain owned by the active
process; other lock errors propagate through the host API. Each host/application
lockfile records the removed storage dependency. Native storage tests include
cross-process lease retention. Runtime wasm production checks remain independent
of these native test fixtures; `--tests` additionally includes bundled native
SQLite and requires a target with its C standard library.
