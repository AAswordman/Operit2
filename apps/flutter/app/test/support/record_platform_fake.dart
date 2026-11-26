import 'dart:async';
import 'dart:typed_data';

import 'package:operit2/core/host/LocalSpeechRecorder.dart';
import 'package:record/record.dart';

/// Feeds synthetic PCM16 microphone chunks in place of a platform recorder.
class FakeRecordPlatform extends RecordPlatform {
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

  /// Emits [frames] of 20 ms, each sample at one constant [amplitude].
  void emit(int frames, int amplitude) {
    const samplesPerFrame = LocalSpeechRecorder.sampleRate * 20 ~/ 1000;
    final pcm = ByteData(frames * samplesPerFrame * 2);
    for (var i = 0; i < pcm.lengthInBytes; i += 2) {
      pcm.setInt16(i, amplitude, Endian.little);
    }
    samples.add(pcm.buffer.asUint8List());
  }
}
