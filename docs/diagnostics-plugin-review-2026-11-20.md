# Plugin loading and change-review investigation, 2026-11-20

## Evidence and scope

This investigation uses the corrected `operit-diagnostics-1795154518427.log.txt`,
exported at `2026-11-20T14:01:58.314566` (24,650 lines). Its client history spans
several days, while retained Core records begin at line 9911, at 09:48:58, and
include a new startup at 14:01:28. The export records local times without dates.
Historical dates below are inferred from client midnight rollovers and are
approximate, as the export header explains.

The old `List<dynamic>`/`Uint8List` failure at the beginning belongs to the
November 13 client history. The registry-not-ready errors at lines 3984–4969
belong to approximately November 16. Neither establishes a current-day failure.
This report does not use the separate 16:52 diagnostic export.

## What the corrected log establishes

At the latest startup, all twelve recorded `registerToolPkg` calls complete
successfully between 14:01:28.993 and 14:01:30.483. Change review registers at
14:01:29.004. Application lifecycle hooks and sidebar preparation also complete.
The export does not record the final loading-session state or what the Flutter
overlay received, so registration completion alone cannot establish that the
loading window disappeared.

There are 74 change-review summary-render starts in the retained Core records.
Seventy-one have a JavaScript completion trace; three do not:

| Call suffix | Last JS stage | Native invocation returned |
| --- | --- | --- |
| `293290fed8eb4a11b18e4f2734cc004c` | `compile_main_script`, 09:53:08.358 | 09:53:08.367, invokeMs=10 |
| `557232902f774f7487e703407ee17a41` | `compile_main_script`, 09:53:08.360 | 09:53:08.366, invokeMs=6 |
| `61a031d9c4e3446497699bc326f54ff0` | `compile_main_script`, 09:53:08.362 | 09:53:08.369, invokeMs=8 |

These calls occur during a chat switch. Other instances of the same source
complete immediately afterward, including at 09:53:08.372 and 09:53:08.381;
later renders also succeed. The invocation-return traces rule out interpreting
the last JS stage alone as an indefinitely blocked compiler operation. Native
cancellation can interrupt JavaScript before its JS error handler logs a
terminal stage. The old logging did not correlate native cancellation and
completion with these call IDs, and the JS trace read a parameter that was empty
for these page contexts. Page disposal and a live failed render therefore cannot
be distinguished conclusively in this export.

At 09:05:07.631–09:05:07.995, the client also records a separate, concrete
editor failure: 89 null-check exceptions from
`CodeEditorViewportState._viewport -> _zoom -> _pointerMove`, followed by a
`State.setState -> _pointerUp` null-check exception. The change-review workspace
renders its diff through this shared `CodeEditor` component. The trace identifies
the shared editor, although it does not identify which document was open.

## Reproduced editor gesture lifecycle defect

The editor used raw `Listener` callbacks for pinch gestures. Flutter retains a
pointer's hit-test path for its gesture lifetime, including when a listener's
render object is removed. The old editor retained its active touch map and
trackpad state after deactivation. Subsequent moves therefore entered `_zoom`
and dereferenced the removed viewport's `GlobalKey.currentContext`; the final
pointer-up also tried to update the disposed State.

Two widget reproductions start a real pinch or trackpad zoom, remove the editor,
and then deliver the remaining gesture events. Both fail against the unchanged
source. The touch reproduction produces exactly the diagnostic's `_viewport`,
`_zoom`, `_pointerMove` stack and the `_pointerUp` disposal error, with Flutter
reporting `RenderPointerListener ... DISPOSED` as the target. This confirms the
cause of that error burst, without establishing that it accounts for every
reported loading symptom.

The viewport now retires its gesture ownership during `deactivate`, before the
render subtree disappears: it clears touch/trackpad state and captured geometry,
cancels pending zoom anchoring and stops inertia. Retired trackpad updates cannot
continue a previous gesture, and pointer/pan/scroll callbacks cannot acquire work
while the viewport is inactive. `activate` permits a fresh gesture sequence if
Flutter reattaches the State. The same retirement operation is reused when the
document identity changes.

## Reproduced state-subscription defect

`StateFlow::subscribe` in `PreferencesDataStore.rs` called the initial subscriber
before registering it for subsequent publications. A terminal publication
during that first callback was lost permanently unless another publication
happened later. `core_state_flow_event_stream` uses this exact subscription API
to expose `pluginLoadingProgressFlow` to Flutter.

The controlled reproduction uses the real `operit-store` implementation:

1. Subscribe to an initial `loading` snapshot and pause its callback.
2. Publish the terminal `hidden` state on another thread.
3. Resume the initial callback and allow subscription registration to finish.
4. Compare the observer's last value with the source's current value.

Before the fix, the source was `hidden`, but the observer had only received
`["loading"]`. Both `set_value` and `compare_and_set` reproduced the loss.
An initial callback that synchronously published the next state also lost that
publication. All three initial regression tests failed against the unchanged
source. This is a verified mechanism for an indefinitely stale loading window;
the provided export does not prove that this precise interleaving occurred on
the reporting device or that it caused the change-review symptom.

There was also an ordering problem: writers published a value and later invoked
observers with their captured copy. When writers contended for a busy observer,
an earlier writer could deliver its old snapshot after a newer publication.

## Changes

State subscriptions now reconcile the current state after registration.
Observer delivery reads the current value while owning that observer's callback
lock, so queued writer notifications conflate to the latest published state.
Value and publication revision are read consistently in the writer's lock order.
Each observer retains only its last delivered revision, avoiding another
retained copy of potentially large state values and suppressing duplicate
delivery during subscription handoff. Public method signatures and transport
schemas are unchanged.

The focused regression suite covers both terminal-publication paths, an initial
callback that publishes the next state, and concurrent writers behind a busy
observer. The latter must receive `[0, 1, 3]` after states 2 and 3 have both been
published while its state-1 callback is paused.

Additional diagnostics record:

- `PluginStartup`: registry start/readiness and MCP start/completion, correlated
  by loading-session generation.
- `PluginLoading`: session start/completion, successful hide and rejected hide,
  including generation, phase and aggregate counts.
- `RuntimeActivity`: phase/visibility transitions actually received by Flutter.
- `JsExecutionTiming`: native `js_finish` and `js_cancel` records with call IDs.
- JS execution traces: the canonical bound execution-context key and Compose
  session ID, allowing page ownership to be correlated with missing JS terminal
  traces.

These records observe the existing lifecycle. They do not add timeouts, automatic
retries or alternate execution paths.

## Verification and remaining limits

Run the focused regression suite with:

```text
cd core
cargo test -p operit-store --test state_flow_observation
```

The four state-flow regression tests and six existing flow/combine/ownership
tests pass. The six shared-editor tests (including removal and reattachment),
eighteen change-review viewport tests and fourteen runtime-overlay Flutter tests
also pass: 48 focused tests in total. Library checks for `operit-js-bridge` and
`operit-runtime` succeed. A reduced-feature
store test attempt could not run because existing `LegacyTransactionImport`
imports are unavailable with `node-state` alone; the regression suite was
subsequently run successfully with the store's normal features.

The corrected log has no post-fix device run. Android packaging and installed-app
verification remain outstanding. The editor error burst has matching executable
reproductions; the state-flow defect is a verified mechanism that can retain a
stale loading snapshot. The three review calls above remain inconclusive until
native call settlement and page ownership are captured together.
