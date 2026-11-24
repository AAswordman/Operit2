# Chat text selection

Chat prose, code blocks and tables share the surrounding Markdown selection
region. A standalone surface creates its own region when no selection registrar
is present. Keyboard copy is dispatched through Flutter's standard editing
shortcuts and the focused region's `CopySelectionTextIntent` action. Selecting
message text transfers focus from the chat composer to that region. Nested
independent regions can receive competing focus requests during mouse dragging,
leaving selected code visible while Command+C copies the empty outer region.

Code blocks preserve source row identities separately from syntax spans. Their
selection delegate reconstructs text between selected row endpoints, retaining
line separators and intervening empty lines while excluding line numbers.
Selected rows stay mounted when scrolling; ordinary unselected rows retain lazy
viewport rendering.

Thinking panels keep their disclosure headers outside text selection. Their
expanded Markdown bodies participate in the surrounding region, including when
the body is streaming. `SelectionContainer.disabled` covers only disclosure
headers and their controls, allowing expanded text to join the selection.

Workspace and plugin code editors share the editing/scrolling contract described
in `apps/flutter/app/lib/ui/common/code_editor/README.md`. Ordinary trackpad
panning stays in the scrolling path until the event sequence changes scale.
Read-only source views use Flutter keyboard actions without attaching an IME.

`mac_text_interactions_test.dart` checks physical mouse drag sequences followed
by Command+C after moving focus from an editable input to chat prose, code,
tables and expanded thinking. Code coverage includes both wrap modes, forward
and reverse multi-line selections, partial-line selections, blank lines, and
copying after scrolling selected rows out of the lazy viewport. It also checks
read-only source keyboard navigation and trackpad pan-to-pinch transitions in
both editable and read-only editors under Flutter's macOS configuration.
