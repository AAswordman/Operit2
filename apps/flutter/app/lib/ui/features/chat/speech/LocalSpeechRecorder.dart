// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:record/record.dart';

import '../../../../core/logging/ClientLogger.dart';

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

class _PcmCaptureDiagnostics {
  int _chunkCount = 0;
  int _totalBytes = 0;
  int _sampleCount = 0;
  int _peak = 0;
  double _sumSquares = 0;

  /// Records actual PCM16 values from one microphone stream chunk.
  void add(Uint8List chunk) {
    final bytes = ByteData.sublistView(chunk);
    for (var offset = 0; offset + 1 < chunk.length; offset += 2) {
      final sample = bytes.getInt16(offset, Endian.little);
      final magnitude = sample == -32768 ? 32768 : sample.abs();
      _peak = math.max(_peak, magnitude);
      _sumSquares += sample * sample;
    }

    _chunkCount += 1;
    _totalBytes += chunk.length;
    _sampleCount += chunk.length ~/ 2;
  }

  /// Formats PCM16 root-mean-square energy as dBFS.
  static String _formatDbFs(double sumSquares, int sampleCount) {
    if (sampleCount == 0 || sumSquares == 0) {
      return '-inf dBFS';
    }
    final rms = math.sqrt(sumSquares / sampleCount);
    return '${(20 * math.log(rms / 32768) / math.ln10).toStringAsFixed(1)} dBFS';
  }

  /// Describes all PCM16 values observed during the active recording.
  String summary() {
    return 'chunks=$_chunkCount bytes=$_totalBytes samples=$_sampleCount '
        'peak=$_peak rms=${_formatDbFs(_sumSquares, _sampleCount)}';
  }
}

class LocalSpeechRecorder {
  static const String _logTag = 'LocalSTT';
  static const MethodChannel _windowsAudioInputChannel = MethodChannel(
    'operit/audio-input',
  );
  static const int _sampleRate = 16000;
  static const int _channelCount = 1;
  static const int _bitsPerSample = 16;
  static const int _streamBufferSize = 256;
  static const int _minimumPcmDurationMs = 250;
  static const int _minimumPcmBytes =
      _sampleRate *
      _channelCount *
      (_bitsPerSample ~/ 8) *
      _minimumPcmDurationMs ~/
      1000;

  final AudioRecorder _recorder = AudioRecorder();
  BytesBuilder? _pcmBuilder;
  StreamSubscription<Uint8List>? _streamSubscription;
  Completer<void>? _streamDone;
  _PcmCaptureDiagnostics? _pcmDiagnostics;
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
    final diagnostics = _PcmCaptureDiagnostics();
    final inputDevice = await _resolveInputDevice();
    if (inputDevice != null) {
      ClientLogger.i(
        'recording start sampleRate=$_sampleRate channels=$_channelCount '
        'inputRole=default inputId=${inputDevice.id} inputLabel=${inputDevice.label}',
        tag: _logTag,
      );
    }
    final stream = await _recorder.startStream(
      RecordConfig(
        encoder: AudioEncoder.pcm16bits,
        sampleRate: _sampleRate,
        numChannels: _channelCount,
        streamBufferSize: _streamBufferSize,
        device: inputDevice,
      ),
    );
    _pcmBuilder = pcmBuilder;
    _streamDone = streamDone;
    _pcmDiagnostics = diagnostics;
    _streamError = null;
    _streamErrorStackTrace = null;
    final subscription = stream.listen(
      (Uint8List chunk) {
        pcmBuilder.add(chunk);
        diagnostics.add(chunk);
      },
      onError: (Object error, StackTrace stackTrace) {
        _streamError = error;
        _streamErrorStackTrace = stackTrace;
      },
      onDone: streamDone.complete,
    );
    _streamSubscription = subscription;
  }

  /// Resolves the Windows default capture endpoint for local recording.
  Future<InputDevice?> _resolveInputDevice() async {
    if (kIsWeb || defaultTargetPlatform != TargetPlatform.windows) {
      return null;
    }
    final device = await _windowsAudioInputChannel
        .invokeMapMethod<String, String>('resolveDefaultCaptureDevice');
    if (device == null) {
      throw StateError('Windows did not return a default capture device');
    }
    final id = device['id'];
    final label = device['label'];
    if (id == null || id.isEmpty || label == null || label.isEmpty) {
      throw StateError('Windows returned an invalid default capture device');
    }
    return InputDevice(id: id, label: label);
  }

  /// Stops the microphone stream and returns a complete WAV payload.
  Future<RecordedAudio> stop() async {
    final pcmBuilder = _pcmBuilder;
    final streamDone = _streamDone;
    final diagnostics = _pcmDiagnostics;
    if (pcmBuilder == null || streamDone == null) {
      throw StateError('No speech recording is active');
    }

    await _recorder.stop();
    await streamDone.future;
    if (diagnostics != null) {
      ClientLogger.d(
        'recording complete ${diagnostics.summary()}',
        tag: _logTag,
      );
    }
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
    if (pcmBytes.length < _minimumPcmBytes) {
      throw StateError('录音时间过短，请说完后再停止');
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
    _pcmDiagnostics = null;
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
