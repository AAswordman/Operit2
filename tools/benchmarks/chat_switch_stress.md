# Chat history switch and render stress benchmark

This opt-in benchmark imports synthetic conversations into a fresh runtime directory,
then calls the production `switchChat` and `chatMessagesFlow` APIs over the actual
native FFI transport. It mounts the production `ChatArea`, display projection cache,
plugin surface catalog, native Markdown parser and Flutter message widgets.
No provider request is made. User application storage is not opened.

The fixture covers 20, 200, 2,000 and 10,000 messages of approximately 2,000
characters each; 2,000 messages of 20,000 characters each; and conversations whose
last message contains 100,000 or 500,000 characters. Each conversation is opened
three times. Fixture construction and archive import are outside timing.

Measurements start immediately before `switchChat`. The report separates command
completion, the first nonempty decoded message window, and the first frame in which
all mounted Markdown surfaces have completed parsing and their rows have released
pending layout. Three ready frames confirm stability. The harness also verifies
the last message's end marker exists in the rendered text. The report records
window size, mounted rows, native parser round trips, and available engine frame
build/raster timings. Reopen samples share application caches; first-open samples
are not guaranteed to have a cold operating-system disk cache.

The host application shell, sidebar, composer, `AIChatScreen` subscription management,
and the reporting user's installed plugins and device are not reproduced. The
harness connects the genuine switch and observation calls to the genuine transcript
widget; it does not time an automated click in the full application. Findings must
be described with those boundaries. Profile runs measure an actual native window;
headless debug results are diagnostic and are not release-device latency estimates.

From `apps/flutter/app`, with a current release native bridge DLL:

```powershell
$env:OPERIT_CHAT_STRESS_LIBRARY = 'ABSOLUTE_PATH_TO_RELEASE_BRIDGE_DLL'
fvm flutter test test/chat_switch_stress_test.dart --no-pub --reporter expanded
```

For the Windows profile runner:

```powershell
$env:OPERIT_CHAT_STRESS_OUTPUT = 'ABSOLUTE_OUTPUT_DIRECTORY'
fvm flutter drive --profile --no-pub -d windows --driver=test_driver/chat_switch_stress_driver.dart --target=integration_test/chat_switch_stress_test.dart --dart-define=OPERIT_CHAT_STRESS_LIBRARY=ABSOLUTE_PATH_TO_RELEASE_BRIDGE_DLL
```

Use `--dart-define=OPERIT_CHAT_STRESS_CASE=small` to run only the small baseline,
or choose another exact fixture id from the source. Without an explicitly supplied
library the ordinary widget test is skipped. The harness saves incremental results
under `build/chat_switch_stress/latest.json`, and each run keeps its independent
runtime database and native logs for inspection. The profile driver also exports
its integration report to `OPERIT_CHAT_STRESS_OUTPUT`.

The native Windows Host resolves packaged resources next to the executable.
For a headless run, the Flutter tester's `data/flutter_assets` must point to the
same packaged assets used by the native library; otherwise builtin plugins will
fail to load and the result does not cover their initialization. A profile
application uses its normal bundle layout. Preserve the native library and
packaged assets across baseline/optimized comparisons.

For CPU sampling, launch a single case with `--start-paused --no-dds` and
`--dart-define=OPERIT_CHAT_STRESS_CPU_PROFILE=true`, then resume its VM isolate
through the VM service. The harness exports `cpu-<case>-<repeat>.json` when a VM
service is enabled. This mode adds profiling overhead and should not replace the
ordinary before/after timing run. Native events and watch callbacks must be
drained by the test's pump loop; awaiting them inside a blocked fake-async test
zone can introduce a harness-only stall.

See [the rendering change and measured results](../../docs/chat-rendering-performance.md)
for the November 2026 investigation and the boundaries of those measurements.
