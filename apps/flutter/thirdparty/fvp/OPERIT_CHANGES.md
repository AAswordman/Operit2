# Operit FVP integration

This directory vendors the installed FVP 0.40.0 source used by Operit. Its
existing creation cancellation and asynchronous native teardown APIs are retained
so a clean checkout uses the same APIs as the application. Native SDK binaries
remain external dependencies managed by the plugin's build configuration.

Operit's native data URL handling decodes each input into a private temporary
file before MDK opens it. The file remains available for probing, seeking and
looping until native player teardown finishes. This applies at the existing
native player boundary; the browser player keeps its normal URL handling.

Failed creations use distinct player IDs and deliver their errors when the
controller subscribes, without adding an error to an unsubscribed player stream.
This also prevents concurrent failed creations from overwriting each other's
native cleanup handles.

Decoder regression checks can run on Windows without building the application:
place matching `fvp.dll` and `mdk.dll` on `PATH`, set
`OPERIT_FVP_NATIVE_TEST=1`, and run the following from `apps/flutter/app`:

```
fvm flutter test --no-pub test/fvp_windows_decoder_test.dart
```

These tests use the actual decoder and callback bridge with a generated 32×32
H.264 MP4 fixture. Texture allocation is replaced for the test environment, so
they do not validate GPU presentation or Android device playback.
