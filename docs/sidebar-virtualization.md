# Sidebar virtualization

The native workspace and character-card sidebar use one scrolling viewport per
tab. Expanded projects, groups, and conversation branches contribute individual
rows, so expanding a directory does not mount its entire widget hierarchy.

## Native workspace

`DrawerContentList.dart` flattens the expanded hierarchy into keyed row factories.
One `SliverChildBuilderDelegate` builds project headers, group headers, conversation
rows, preview controls, and desktop recent shortcuts only within the viewport and
Flutter's nearby cache. Offscreen rows are disposed instead of being kept alive.
Keys and `findChildIndexCallback` preserve visible row identity when directories
expand, collapse, or move.

Expansion sets and the scroll controller belong to `DrawerContent`, outside the
recycled rows. Conversation actions retain their canonical history identity.
Recent shortcuts have a separate key namespace from the same conversations in
workspace groups. Desktop group guides are drawn as adjoining row segments.

## Character cards

`nativeSidebarItems` builds lightweight row factories and passes them to the
existing Compose `ctx.lazyItems` protocol. The first window contains at most 24
hierarchy rows; toolbar and search controls form a static prefix. A 2,000-chat
expanded group initially materializes 22 conversation rows and two directory
headers. Dialogs remain outside the recycled row interval.

The host measures variable-height rows and requests a buffered range through the
existing Compose action channel. Viewport-only updates reuse the row projection
and overlapping Compose instances. Rows and callbacks leaving the materialized
window are released. Scrolling does not read the authoritative catalog or avatar
resources again. Selection, running indicators, search, theme, and domain changes
invalidate the relevant projection through their normal inputs.

## Hidden surfaces and keyboard resizing

`PhoneLayout` parks the complete drawer in the shared `RetainedPage` boundary
after it closes. The drawer mounts on first opening, stays active during its
visible closing animation, and retains its element state when parked. Its dirty
build scope is suspended and its render subtree is detached. Main-content
keyboard resizing therefore does not rebuild, lay out, or paint drawer controls.
Current inherited data and pending conversation updates are applied on reopening.

Visited sidebar tabs use the same boundary. Each tab's surface builder runs
inside its retained scope, so an unselected workspace does not regenerate its
row factories during another tab's updates. Plugin sessions, expansion state,
search text, and scroll positions survive tab changes and drawer closure.

## Verification

Focused Flutter tests cover 2,000 workspace conversations and desktop recent
shortcuts, 200 projects with three groups each, disposal of offscreen rows,
identity during expansion, navigation actions, and retained tab sessions.
Phone tests observe the real drawer's elements and render objects through three
keyboard cycles: zero hidden descendant rebuilds and a detached render subtree,
while main content continues resizing. Reopening applies fresh history without
losing the retained search state. Separate drawer-transition checks retain the
scroll position across repeated close/open cycles.

Compiled plugin tests cover the 2,000-chat window, middle and tail range requests,
callback release, bounded transport, collapse and immediate re-expansion, avatar
paths, live selection updates, search, and conversation actions. These are widget
and compiled SDK tests; device frame-time profiling requires a device run.
