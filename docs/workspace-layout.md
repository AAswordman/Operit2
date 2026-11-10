# Workspace layout

The workspace presentation follows the host's interaction model, independently
of the available window or content width:

- Desktop hosts (Windows, macOS, Linux, and browsers on those platforms) always
  use a split pane. Opening the workspace reserves space beside the conversation;
  the divider lets the user resize the workspace.
- Phone and tablet hosts always cover the conversation body with the workspace.
  Increasing the window width or rotating the device does not enable split mode.
  The host toolbar and its workspace toggle remain available.

`OperitWindowPlatform.dart` owns the desktop platform classification.
`WorkspaceLayoutMetrics.dart` selects an explicit `SidePanelLayout` for
`WorkspaceShell`. Navigation breakpoints only decide where the shell attaches
and how its tab chrome is rendered; they do not select the workspace presentation.

In desktop split mode, the initial workspace width is 45% of the available host
width, bounded below by 360 logical pixels and above by the space left after
reserving room for the conversation. Dragging respects the existing workspace
and conversation size limits. A smaller host temporarily constrains
the visible pane width without discarding the user's dragged width. Window
resizing, application zoom, and changes to the navigation or conversation
sidebar widths do not switch the workspace to an overlay.

The shared `AdaptiveSidePanel` retains its adaptive mode and `breakpoint` API for
plugin callers. The workspace always supplies an explicit split or overlay mode.
