# Shared code editor

`CodeEditor.dart` is the common entry point for workspace editing and plugin
source presentation. The input client, virtual text renderer, line layout,
syntax cache, gutter, scroll controller, and zoom transform were moved here
from the workspace editor.

```dart
CodeEditor(
  controller: controller,
  filePath: 'main.dart',
  onChanged: saveDraft,
)

CodeEditor(text: source, filePath: path, readOnly: true)
```

Callers can supply focus, undo and scroll controllers, font style, gutter
visibility, screen-space padding, scale and scale callbacks, zoom bounds, and
whether gestures pan or select. Read-only views retain selection/copy but do
not attach a keyboard or apply text edits. `wrapToViewport` reflows the text
width during zoom; `contentWidth` sets a fixed logical width for wide content.

`contentBuilder` replaces the body for decorated rows or paired diff columns.
It runs inside `CodeEditorViewportScope`: its single vertical scrollable must
use that scope's controller and physics. Such bodies own their gutters and
row decorations. Rows must use the live incoming width (for example, flex
columns), so `wrapToViewport: true` can reflow them at every zoom level.
For unwrapped rows, `contentColumns` supplies source text, each column's width
fraction and reserved gutter space. The editor measures and caches their
horizontal extent with `CodeTextLayout`, including the platform text scale;
callers do not estimate widths from character counts. They share the editor's
zoom, focal anchoring, panning and inertia. Diff parsing, row decorations and fold actions remain caller-owned.

Workspace Markdown previews use this same panning mode for touch and mouse
drags, including inertial scrolling after release and interruption on pointer
down. Vertical panning updates only the scroll position; the viewport rebuilds
only when its horizontal offset changes. Markdown rows remain lazily mounted
and retain their parsed content while the viewport moves. The preview's selection
region wraps the viewport so its mouse selection recognizer does not take
ownership of panning gestures.

Editable text keeps Flutter's native touch scrolling and mouse range selection.
Its virtual layout submits a source-origin correction only when measured row
geometry changes the scroll offset. A zero correction would make Flutter
restart its ballistic activity on every layout frame, even with stable text.

Trackpad pan/zoom sequences initially remain ordinary scrolling gestures. The
viewport captures a zoom anchor and disables the child's native drag only when
the sequence reports an actual scale change. Entering a pinch after panning
uses the latest focal point rather than replaying the accumulated pan.

Editable inputs delegate native editing selectors through Flutter's text-input
connection. Read-only inputs have no input connection, so their keyboard
navigation remains in Flutter's shortcut/action pipeline. They must not consume
the intents used to forward navigation keys to the platform input method.

`CodeEditorText` renders a source row using the same syntax-span builder as the
editor, with optional `CodeTextHighlight` intervals for intraline changes.
It uses an explicit source style and the same forced line strut as the text
layout, keeping empty lines and fallback-font glyphs at the editor's row height.

Viewport deactivation retires the current touch/trackpad sequence and cancels
pending zoom geometry and inertia. Flutter can continue dispatching events to
the cached hit-test target of an existing pointer after that target leaves the
tree; these events no longer own an active editor interaction. A reattached
viewport acquires a fresh gesture sequence before it uses its new geometry.
