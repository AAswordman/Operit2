# Terminal rendering and binary PTY events

The workspace terminal uses the vendored Flutter xterm renderer on every platform.
Its character positions come from a fixed cell grid. Single-glyph paragraphs are
cached by their complete visible attributes, rather than a hash alone. Equivalent
font configurations compare equal, so unrelated widget rebuilds preserve caches.

Each visible buffer row owns a retained `OffsetLayer` and a recorded picture.
Exact cell snapshots detect changes even through `BufferLine.data`; an unchanged
row reuses its picture when another row, the cursor, or the selection changes.
Scrolling moves existing row layers. Leaving the viewport releases those rows'
cached resources. Font, scale, theme, and terminal-owner changes invalidate all
row pictures. The engine may raster-cache stable row pictures; the renderer no
longer marks the whole terminal as changing every frame. This is a compositor
hint, not a guarantee of a specific GPU frame time on every rendering backend.

Continuous backgrounds are recorded as spans, and plain spaces need no glyph
draw. The cursor, selection, highlights, and composing text remain a separate
overlay. Updates with unchanged buffer height only request paint; changes to the
scroll extent still request layout. Input-method geometry updates are deduplicated
and sent after the frame has established the displayed scroll and cursor position.

PTY presentation output uses `TerminalHost.nextPtySessionOutput`: bytes wake the
pending reader, and EOF wakes an idle reader after all queued bytes drain. Native
descriptor readers feed a bounded session-owned binary buffer. Browser serial
producers signal a registered shared-memory waiter; Apple ARM64 sessions deliver
bytes through native callbacks. The shared runtime has no timed output polling.

`RuntimeTerminalPtyOutputStream` carries `CoreBuffer` and generated Flutter clients
expose `Stream<Uint8List>`. Input accepts `Vec<u8>` / `Uint8List` as well. Shared
stream collectors clone the immutable byte owner rather than each payload. Core Link
encodes these as binary values; neither direction wraps terminal bytes in Base64.
UTF-8 decoding remains incremental at the terminal parser boundary. The view's
resize debounce is independent of output delivery.

Peer protocol v32 records this terminal service wire contract change. Peers and generated SDK clients
must be updated together. The synchronous Host read and exit-status query remain
available for explicit callers; the presentation reader does not poll them.

Regression checks cover dense viewport row reuse, exact cell mutation, viewport
eviction, compositing/disposal, font value equality, binary frames, registration
races, idle EOF wakeups, and final-output ordering. Device profiling at minimum
font size remains necessary to quantify frame-time and GPU improvements.
