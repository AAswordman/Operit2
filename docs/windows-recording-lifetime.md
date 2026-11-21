# Windows recording and Flutter engine ownership

The Windows crash reported on 2026-11-22 is an access violation in
`record_windows_plugin.dll`. The saved report
`native-20261121-200500-742-101268-66220.txt` identifies module-relative address
`0x4488d`, reading address `0x10`. Symbolizing the local Debug DLL resolves that
instruction to `flutter::EventChannel::SetStreamHandler`, at the call to the
binary messenger's `SetMessageHandler`.

The installed `record_windows` 1.0.7 implementation stores its binary messenger,
root-window provider, and callback queue in static members. Each registered
Flutter engine overwrites the messenger and window provider. A floating pet or
detached window therefore replaces the main engine's recording transport even
when that window never records audio. Closing it leaves a stale messenger and a
window provider capturing its destroyed registrar. A previous saved crash,
`native-20261121-200129-504-74880-99692.txt`, resolves relative address `0x43e5f`
to `flutter::FlutterView::GetNativeWindow`, consistent with the second stale
reference. The static ownership and both instruction locations are confirmed;
the original interactive crash sequence has not been replayed in the app.

A subsequent report, `native-20261121-201904-779-89864-81708.txt`, exposes a
separate teardown-order error. Its `flutter_windows.dll+0x15d6c` instruction
resolves to `FlutterDesktopMessengerSetCallback` accessing the stopped engine's
message dispatcher. The dumped stack contains recording-plugin
`BinaryMessengerImpl::SetMessageHandler`, `MethodChannel::SetMethodCallHandler`,
plugin deletion, and `PluginRegistrarManager::OnRegistrarDestroyed`, followed by
`FlutterWindowsEngine::Stop` and child-controller destruction. Unregistering
channels from the plugin destructor is too late with the configured engine.

The app now uses `apps/flutter/thirdparty/record_windows` through a path override.
Each engine owns its messenger, method channel, and cancellable dispatcher. The
dispatcher keeps a native view handle and resolves its root after startup view
attachment, without retaining the registrar. The Windows runner now registers
the recording plugin with its existing per-engine shutdown hooks, for both main
and secondary engines. That hook cancels queued work and disposes recorders and
channels before controller destruction. The subsequent plugin destructor never
calls the stopped engine. Shutdown is idempotent.

Recorded events own their payload and hold a weak reference to stream state.
They cannot dereference a released recorder or event handler. Recorder ownership
also follows its existing Media Foundation COM reference count, so disposal does
not delete a recorder while a native callback still holds it. These changes are
inside the Windows host implementation and preserve the public Dart API.

Run the focused native regression with the repository Python environment:

```powershell
.\.venv\Scripts\python.exe -m unittest tools.tests.test_windows_record_lifetime
```

The fixture compiles only recording-plugin sources and the existing Flutter C++
wrapper. It checks main and child engine isolation, equal recorder IDs across
engines, recorder creation after child destruction, queued state events across
disposal, immediate channel unregistration, recreation, view attachment, and
callback cancellation across a worker thread. It also invalidates the simulated
engine before destroying the plugin, rejecting every late messenger call. It
does not open a microphone.

A complete app rebuild is required to replace the crashing native DLL. Actual
microphone recording while repeatedly creating and closing desktop-pet windows
remains a device validation step.
