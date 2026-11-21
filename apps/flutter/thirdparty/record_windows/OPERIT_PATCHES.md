# Operit patches

Based on the installed `record_windows` 1.0.7 package from pub.dev. The upstream
BSD license is preserved in LICENSE. The Dart recording API and Windows method
and event channel names are unchanged.

- Scope the binary messenger, method channel, and callback queue to one plugin
  instance. Creating another Flutter engine cannot replace their ownership.
- Capture the native view handle at registration and resolve its root window
  when dispatching. No queued callback dereferences a registrar or Flutter view
  object after engine destruction; view attachment during startup is supported.
- Add `RecordWindowsPluginCApiShutdownWithRegistrar` for the Windows host's
  pre-engine shutdown hook. Cancel the engine's pending callback queue and
  unregister recorder event and method channels before engine shutdown. Plugin
  destruction releases native resources without calling the stopped engine.
- Queue weak stream state and owned event payloads instead of raw recorder or
  stream-handler pointers. Recorder disposal can release channels immediately.
- Respect Media Foundation's COM reference counting when releasing recorders.
  In-flight source-reader callbacks retain their recorder until they finish.
- Reject duplicate recorder creation within an engine without replacing its
  live stream handlers. Different engines can use the same recorder ID.
- Remove the upstream workspace resolution flag for the local dependency.

The native fixture in `tools/tests/test_windows_record_lifetime.py` compiles the
production plugin against the configured Flutter Windows wrapper. It exercises
independent engines, child closure, delayed state delivery, recorder disposal
and recreation, startup view attachment, and worker dispatch after cancellation.
It does not start Dart or open a microphone. See
[`docs/windows-recording-lifetime.md`](../../../../docs/windows-recording-lifetime.md)
for the crash evidence and remaining device validation.
