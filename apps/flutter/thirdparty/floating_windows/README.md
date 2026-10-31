# floating_windows

Transparent, frameless live Flutter surfaces for floating tools and desktop pets.
The package owns native presentation and secondary engines. The embedding app owns
content, state and its resource lifecycle. This package does not depend on Operit.

```dart
await FloatingWindows.run(
  arguments: arguments,
  windowBuilder: (launch) => MyTransparentContent(payload: launch.payload),
  application: () async => runApp(FloatingWindowsHost(
    windowBuilder: (launch) => MyTransparentContent(payload: launch.payload),
    child: const MyApp(),
  )),
);

final caps = await FloatingWindows.capabilities();
if (caps['supported'] == true && caps['permissionGranted'] == true) {
  final window = await FloatingWindows.open(
    payload: {'content': 'pet'},
    options: const FloatingWindowOptions(width: 180, height: 180),
  );
  await window.update(const FloatingWindowOptions(x: 100, y: 100));
  await window.hide();
  await window.show();
  await window.close();
}
```

Call `WidgetsFlutterBinding.ensureInitialized()` first. Register generated Flutter
plugins and app-specific channels in secondary engines using desktop_multi_window's
window-created callbacks. On Android install
`FloatingWindowsPlugin.setEngineConfigurer` before opening a surface; the callback
registers app-specific channels on the new engine (generated plugins register
through FlutterEngine). The owner engine must remain alive. No background service
or notification is introduced by this package.

Content must paint a transparent root. `FloatingWindows.setBeforeClose(launch, callback)` registers a per-surface drain hook for either backend. For native child engines, optional `FloatingWindows.beforeClose`
should asynchronously drain application-owned sessions before engine destruction.
Drain hooks have a five-second deadline; native closure still runs if a hook throws or times out. Close requests acknowledge before draining, so content can close itself from an
in-flight action. Hide/show preserve the engine and content state. Native close waits at most eight
seconds for the child acknowledgement before asking the owner transport to destroy
the surface; owner cleanup has the same bounded deadline. Dart callers can set
`window.close(timeout: ...)` when they need a shorter deadline. Failed child
startup is reclaimed through the native owner transport. Startup waits have a bounded deadline, including the individual ready call, and retry the desktop transports' handler-not-registered errors.

Geometry uses logical pixels. Width/height must be 16–4096. Updates are partial.
Defaults: 240×240, always on top, pointer input enabled, keyboard focus disabled.
`clickThrough` passes input through the whole rectangle, not alpha-shaped regions.
Android overlays are always above applications; switching alwaysOnTop off rejects.
On Android 12+, pass-through opacity is bounded by the system maximum; overlapping
overlays remain subject to the OS combined-opacity rule.

macOS, Windows, Android and GTK/X11 have native system surfaces. X11 requires
RGBA and a compositor. iOS, OpenHarmony, browsers and unsupported Linux
compositors use `FloatingWindowsHost` application surfaces. These remain inside
the application view; they have independent content lifecycles, hide/show state,
input pass-through and focus policy. Positions are clamped into the current view. Application window IDs stay unique across host disposal and remounting in the
same Dart context; handles from a previous host cannot affect its replacement.
Existing desktop engines remain tracked even if compositor capabilities change
after creation, so ownership is retained until they are actually destroyed.

Always inspect `capabilities().scope`: `system`, `application`, or `unsupported`.
`systemOverlay` never becomes true just because the application host is available.
Mount the application host before issuing commands. It does not prompt for OS
permissions. Android permission denial still requires an explicit user request;
it does not silently switch the plugin to a different scope.

macOS has passed a native lifecycle smoke check and the app's complete Dafeiyu plugin check (settings, resource decoding, feeding action,
hide/show and engine release). Windows, Android and GTK/X11 still need device
validation. Application-scope lifecycle and pointer pass-through are covered by Flutter
widget tests, including a real Chrome run. This
is an experimental local package, not a published release.

The local desktop_multi_window 0.3.1 fork preserves its upstream Apache-2.0 license;
see its OPERIT_PATCHES.md for owner-initiated close and transparent-visual changes.


`FloatingWindowOptions(draggable: true)` enables host-owned whole-surface dragging.
Native hosts record the pointer screen position and window origin on down, then
set the window origin to the original origin plus the latest absolute screen
displacement until release. Local coordinates and accumulated deltas are never
used. The application renderer uses the same anchors in its shared Flutter view.
Recognized drags cancel click recognition.

`window.update(options, payload: patch)` shallow-merges content data in the
existing surface and calls the builder with a new launch value. The engine and
widget identity are preserved. `window.getBounds()` reads actual host geometry,
including pointer-driven movement.
