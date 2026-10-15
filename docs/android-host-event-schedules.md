# Android host event schedules

`AndroidHostEventScheduler` validates timer and interval commands from Core and their persisted copies before registering or consuming alarms. Invalid kinds, nonpositive delays or intervals, inconsistent timer fields, negative stored fire times, and arithmetic overflow are reported explicitly.

`HostEventScheduleReceiver` consumes the alarm, starts the Core service, and submits delivery to the runtime executor. The receiver finishes its asynchronous broadcast on synchronous exits; an accepted background task finishes it in `finally` when delivery ends. Executor submission is not a delivery acknowledgement.

If service startup, executor submission, or delivery fails after consumption, `HostEventHandoff` attempts to persist an `UndeliveredHostEventFire`. Its result preserves the delivery error and, separately, any error encountered while saving that diagnostic. Logs say `failure retained` only after the write succeeds. A failed diagnostic write says `failure NOT retained` and includes both errors.

Retained records use the same lock and SharedPreferences file as schedule mutations. The newest 32 records are kept; dropping an older record emits a warning. Malformed records and storage failures are surfaced rather than replaced with an empty history.

The Android `operit/runtime` method channel exposes a read-only `hostEventScheduleFailures` method. It returns a list of maps with these fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `scheduleId` | String | The Core schedule identity |
| `firedAtMillis` | Long | When the host began the delivery attempt |
| `stage` | String | `START_CORE`, `SUBMIT_BACKGROUND`, or `DELIVERY` |
| `reason` | String | The original delivery error |

For example, an Android diagnostic caller can read the records without clearing them:

```dart
final failures = await const MethodChannel('operit/runtime')
    .invokeListMethod<Map<Object?, Object?>>('hostEventScheduleFailures');
```

Reading does not start Core, re-arm an alarm, clear a record, or retry delivery. This is a bounded diagnostic history, not a durable delivery queue. Consumption and delivery remain separate operations, so a process death between them is not covered by this mechanism; exactly-once delivery is not guaranteed.

Run the Android JVM tests from `apps/flutter/app/android` with a configured JDK 17 and Android SDK:

```sh
gradle -Ptarget-platform=android-arm64 :app:testDebugUnitTest \
  -x :app:compileFlutterBuildDebug \
  -x :app:syncOperitPlugins \
  -x :app:verifyOperitAndroidRuntimeArtifacts \
  -x :app:cargoBuildOperitFlutterBridgeDebugarm64v8a
```

These tests exercise configuration validation, interval overflow, handoff and retention errors, persisted diagnostic encoding, retention limits, and synchronous broadcast completion. Android framework boundaries in the receiver tests are mocked; JVM results do not establish device alarm behavior or native Core delivery.
