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

`CodeEditorText` renders a source row using the same syntax-span builder as the
editor, with optional `CodeTextHighlight` intervals for intraline changes.
It uses an explicit source style and the same forced line strut as the text
layout, keeping empty lines and fallback-font glyphs at the editor's row height.
