# record Ohos

Ohos specific implementation for record package called by record_platform_interface.

## Usage

```yaml
dependencies:
  record: ^5.0.0
  record_ohos: 1.0.0
```

## Usage

```dart
import 'package:record/record_ohos.dart';

final record = AudioRecorder();

// Check and request permission if needed
if (await record.hasPermission()) {
  // Start recording to file
  await record.start(const RecordConfig(), path: 'aFullPath/myFile.m4a');
  // ... or to stream
  final stream = await record.startStream(const RecordConfig(AudioEncoder.pcm16bits));
}

// Stop recording...
final path = await record.stop();
// ... or cancel it (and implicitly remove file/blob).
await record.cancel();

record.dispose(); // As always, don't forget this one.
```
