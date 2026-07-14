// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

import 'package:record/record.dart';

class RecordedAudio {
  final Uint8List bytes;
  final String fileName;
  final String contentType;

  /// Creates one recorded audio payload for speech recognition.
  const RecordedAudio({
    required this.bytes,
    required this.fileName,
    required this.contentType,
  });
}

class LocalSpeechRecorder {
  static const int _sampleRate = 16000;
  static const int _channelCount = 1;
  static const int _bitsPerSample = 16;

  final AudioRecorder _recorder = AudioRecorder();
  BytesBuilder? _pcmBuilder;
  StreamSubscription<Uint8List>? _streamSubscription;
  Completer<void>? _streamDone;
  Object? _streamError;
  StackTrace? _streamErrorStackTrace;

  /// Starts a 16 kHz mono PCM16 microphone stream.
  Future<void> start() async {
    if (_pcmBuilder != null) {
      throw StateError('A speech recording is already active');
    }
    final permitted = await _recorder.hasPermission();
    if (!permitted) {
      throw StateError('Microphone permission was not granted');
    }

    final pcmBuilder = BytesBuilder();
    final streamDone = Completer<void>();
    final stream = await _recorder.startStream(
      const RecordConfig(
        encoder: AudioEncoder.pcm16bits,
        sampleRate: _sampleRate,
        numChannels: _channelCount,
      ),
    );
    _pcmBuilder = pcmBuilder;
    _streamDone = streamDone;
    _streamError = null;
    _streamErrorStackTrace = null;
    final subscription = stream.listen(
      pcmBuilder.add,
      onError: (Object error, StackTrace stackTrace) {
        _streamError = error;
        _streamErrorStackTrace = stackTrace;
      },
      onDone: streamDone.complete,
    );
    _streamSubscription = subscription;
  }

  /// Stops the microphone stream and returns a complete WAV payload.
  Future<RecordedAudio> stop() async {
    final pcmBuilder = _pcmBuilder;
    final streamDone = _streamDone;
    if (pcmBuilder == null || streamDone == null) {
      throw StateError('No speech recording is active');
    }

    await _recorder.stop();
    await streamDone.future;
    final streamError = _streamError;
    final streamErrorStackTrace = _streamErrorStackTrace;
    _clearActiveRecording();
    if (streamError != null) {
      Error.throwWithStackTrace(
        streamError,
        streamErrorStackTrace ?? StackTrace.current,
      );
    }

    final pcmBytes = pcmBuilder.takeBytes();
    if (pcmBytes.isEmpty) {
      throw StateError('The recorded PCM stream is empty');
    }
    if (pcmBytes.length.isOdd) {
      throw StateError('The recorded PCM16 stream has an incomplete sample');
    }
    final timestamp = DateTime.now().microsecondsSinceEpoch;
    return RecordedAudio(
      bytes: _buildWaveFile(pcmBytes),
      fileName: 'operit-stt-$timestamp.wav',
      contentType: 'audio/wav',
    );
  }

  /// Clears all state associated with the completed stream.
  void _clearActiveRecording() {
    _pcmBuilder = null;
    _streamSubscription = null;
    _streamDone = null;
    _streamError = null;
    _streamErrorStackTrace = null;
  }

  /// Releases recording resources and cancels an active capture.
  Future<void> dispose() async {
    final subscription = _streamSubscription;
    if (_pcmBuilder != null) {
      await _recorder.cancel();
      await subscription?.cancel();
      _clearActiveRecording();
    }
    await _recorder.dispose();
  }

  /// Builds a canonical PCM WAV file from little-endian PCM16 samples.
  static Uint8List _buildWaveFile(Uint8List pcmBytes) {
    const headerLength = 44;
    const bytesPerSample = _bitsPerSample ~/ 8;
    const blockAlign = _channelCount * bytesPerSample;
    const byteRate = _sampleRate * blockAlign;
    final wavBytes = Uint8List(headerLength + pcmBytes.length);
    final header = ByteData.view(wavBytes.buffer, 0, headerLength);
    _writeAscii(wavBytes, 0, 'RIFF');
    header.setUint32(4, 36 + pcmBytes.length, Endian.little);
    _writeAscii(wavBytes, 8, 'WAVE');
    _writeAscii(wavBytes, 12, 'fmt ');
    header.setUint32(16, 16, Endian.little);
    header.setUint16(20, 1, Endian.little);
    header.setUint16(22, _channelCount, Endian.little);
    header.setUint32(24, _sampleRate, Endian.little);
    header.setUint32(28, byteRate, Endian.little);
    header.setUint16(32, blockAlign, Endian.little);
    header.setUint16(34, _bitsPerSample, Endian.little);
    _writeAscii(wavBytes, 36, 'data');
    header.setUint32(40, pcmBytes.length, Endian.little);
    wavBytes.setRange(headerLength, wavBytes.length, pcmBytes);
    return wavBytes;
  }

  /// Writes one ASCII marker into a byte buffer.
  static void _writeAscii(Uint8List target, int offset, String value) {
    for (var index = 0; index < value.length; index += 1) {
      target[offset + index] = value.codeUnitAt(index);
    }
  }
}
