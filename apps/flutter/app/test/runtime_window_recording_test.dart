import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/LocalSpeechRecorder.dart';
import 'package:operit2/core/host/RuntimeWindowRecording.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:record/record.dart';

import 'support/record_platform_fake.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(ClientLogger.initialize);

  for (final echoCancellation in [true, false]) {
    test(
      'capture detects microphone volume and trims silence with echoCancellation=$echoCancellation',
      () async {
        final original = RecordPlatform.instance;
        final input = FakeRecordPlatform();
        RecordPlatform.instance = input;
        debugDefaultTargetPlatformOverride = TargetPlatform.android;
        addTearDown(() {
          RecordPlatform.instance = original;
          debugDefaultTargetPlatformOverride = null;
        });

        final recording = RuntimeWindowRecording(
          20000,
          300,
          echoCancellation: echoCancellation,
        );
        final config = await input.started.future;
        expect(config.echoCancel, echoCancellation);
        expect(
          config.androidConfig.audioSource,
          echoCancellation
              ? AndroidAudioSource.voiceCommunication
              : AndroidAudioSource.defaultSource,
        );
        await Future<void>.delayed(Duration.zero);

        input.emit(50, 0);
        input.emit(7, 1200);
        await Future<void>.delayed(Duration.zero);
        expect(recording.hasSpeech, isFalse);

        input.emit(1, 1200);
        await Future<void>.delayed(Duration.zero);
        expect(recording.hasSpeech, isTrue);
        expect(recording.level, greaterThan(0.35));

        input.emit(15, 0);
        final audio = await recording.result;
        final wav = ByteData.sublistView(audio!.bytes);
        const bytesPerFrame = LocalSpeechRecorder.sampleRate * 20 ~/ 1000 * 2;
        expect(wav.getUint32(40, Endian.little), (12 + 8 + 15) * bytesPerFrame);
        expect(wav.getInt16(44 + 12 * bytesPerFrame, Endian.little), 1200);
      },
    );
  }
}
