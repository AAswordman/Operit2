# Model stream revisions

Each model request creates a named in-memory savepoint before emitting output.
An accepted retry retracts that request's output to the savepoint and resends
the prepared request. Completed earlier model rounds and tool results remain.
Text revision does not undo tool side effects or resume a server request at an
interrupted token. Savepoints do not survive process restart.

`ResponseStreamItem` carries text and revision instructions in one source order.
DeepSeek restores its DSML decoder snapshot and forwards the same revision;
the Responses wrapper forwards ordered parent items directly. Both wrappers
retain terminal failures independently of stream closure.

`EnhancedAIService` applies revisions to the current request buffer and forwards
them to the shared conversation stream. Message parsing rebuilds semantic parts
on rollback. Display transforms restore their saved state, the Rust Markdown
parser restores the retained content, and Flutter restores the saved node tree
and nested XML streams. CLI and constrained-display projections also apply
revisions before exposing semantic text. Late ordered subscribers receive the
current accepted text and active savepoints before live events.

Summary generation and title generation share `collect_response_text`, which
uses ordered collection and rejects transport failure or an unknown rollback.
Summary retries therefore cannot concatenate abandoned thinking or tool markup
with the accepted summary. Consumers that need only lifecycle completion may
observe the chunk channel; scheduled replies read their final body from the
shared stream's revision-aware render snapshot.

Plugin providers currently expose text chunks without a revision protocol.
Their shared text adapter reads the upstream replay and live output together,
including providers that complete before the first subscriber attaches. It
does not fabricate a second revision stream. Unused character-stream wrappers
and the old independent-channel sharing adapter have been removed.

Retry policy permits five retries with Host delays of 1, 2, 4, 8 and 16 seconds.
Each attempt has a 120-second first-output deadline. Actual model output
disarms that deadline. Exhaustion and cancellation do not issue another rollback.
Cancellation completion semantics remain owned by the individual provider.

Regression coverage includes repeated rollback after partial thinking and tool
markup, preserved earlier rounds, UTF-8 content, producer failures, plugin
replay/live subscription, and real Host HTTP disconnects through OpenAI, Claude,
Gemini, DeepSeek and Responses. The DeepSeek case also interrupts a native DSML
parameter. Flutter tests cover saved style stacks and nested XML stream routing.

The focused 2026-11-26 run passed seven util integration tests, three provider
integration tests (including the Host-controlled provider matrix), the DeepSeek
wrapper-to-message-parser regression, the runtime display-transform rollback
regression, and fourteen Flutter rendering tests. No device installation or
cross-device runtime verification was performed in this audit.
