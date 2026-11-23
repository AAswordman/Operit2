import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/LocalSpeechRecorder.dart';
import 'package:operit2/core/host/RuntimeWindowRecording.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:record/record.dart';

class _PcmInput extends RecordPlatform {
  final samples = StreamController<Uint8List>();
  final started = Completer<RecordConfig>();

  @override
  Future<void> create(String id) async {}

  @override
  Future<bool> hasPermission(String id, {bool request = true}) async => true;

  @override
  Stream<RecordState> onStateChanged(String id) => const Stream.empty();

  @override
  Future<Stream<Uint8List>> startStream(String id, RecordConfig config) async {
    started.complete(config);
    return samples.stream;
  }

  @override
  Future<String?> stop(String id) async => null;

  @override
  Future<void> cancel(String id) async {}

  @override
  Future<void> dispose(String id) async {
    await samples.close();
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);

  void emit(int frames, int amplitude) {
    const samplesPerFrame = LocalSpeechRecorder.sampleRate * 20 ~/ 1000;
    final pcm = ByteData(frames * samplesPerFrame * 2);
    for (var i = 0; i < pcm.lengthInBytes; i += 2) {
      pcm.setInt16(i, amplitude, Endian.little);
    }
    samples.add(pcm.buffer.asUint8List());
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(ClientLogger.initialize);

  test(
    'echo-cancelled capture signals speech before returning a trimmed WAV',
    () async {
      final original = RecordPlatform.instance;
      final input = _PcmInput();
      RecordPlatform.instance = input;
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      addTearDown(() {
        RecordPlatform.instance = original;
        debugDefaultTargetPlatformOverride = null;
      });

      final recording = RuntimeWindowRecording(
        20000,
        300,
        echoCancellation: true,
      );
      final config = await input.started.future;
      expect(config.echoCancel, isTrue);
      expect(
        config.androidConfig.audioSource,
        AndroidAudioSource.voiceCommunication,
      );
      await Future<void>.delayed(Duration.zero);

      input.emit(50, 0);
      input.emit(7, 1200);
      await Future<void>.delayed(Duration.zero);
      expect(recording.hasSpeech, isFalse);

      input.emit(1, 1200);
      await Future<void>.delayed(Duration.zero);
      expect(recording.hasSpeech, isTrue);

      input.emit(15, 0);
      final audio = await recording.result;
      final wav = ByteData.sublistView(audio!.bytes);
      const bytesPerFrame = LocalSpeechRecorder.sampleRate * 20 ~/ 1000 * 2;
      expect(wav.getUint32(40, Endian.little), (12 + 8 + 15) * bytesPerFrame);
      expect(wav.getInt16(44 + 12 * bytesPerFrame, Endian.little), 1200);
    },
  );
}
