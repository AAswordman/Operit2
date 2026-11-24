// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;
import 'dart:typed_data';

import 'LocalSpeechRecorder.dart';

/// Holds one bounded microphone capture until it is finished or cancelled.
class RuntimeWindowRecording {
  final _recorder = LocalSpeechRecorder();
  final _result = Completer<RecordedAudio?>();
  final int _maxDurationMs;
  final int? _silenceMs;
  late final Future<void> _started;
  Timer? _deadline;
  Future<void>? _ending;
  bool _discard = false;

  static const _frameMs = 20;
  static const _samplesPerFrame =
      LocalSpeechRecorder.sampleRate * _frameMs ~/ 1000;
  static const _threshold = 360;
  static const _speechFrames = 8;
  static const _preRollFrames = 12;
  int _frames = 0;
  int _skipSamples = 0;
  int? _lowByte;
  int _energy = 0;
  int _samples = 0;
  int _loud = 0;
  int _quiet = 0;
  bool _heardSpeech = false;
  double _level = 0;

  /// Starts capture on the owner engine and arms its deadline after authorization.
  RuntimeWindowRecording(
    this._maxDurationMs,
    this._silenceMs, {
    bool echoCancellation = false,
  }) {
    _started = _recorder.start(
      onChunk: _observe,
      echoCancellation: echoCancellation,
    );
    unawaited(
      _started.then<void>((_) {
        if (_ending != null) return;
        _deadline = Timer(Duration(milliseconds: _maxDurationMs), () {
          unawaited(finish(discard: _silenceMs != null && !_heardSpeech));
        });
      }, onError: (Object _, StackTrace stack) => finish()),
    );
  }

  /// Resolves after cleanup, with no audio for a discarded capture.
  Future<RecordedAudio?> get result => _result.future;

  /// Reports the smoothed microphone level from zero to one during capture.
  double get level => _ending == null ? _level : 0;

  bool get hasSpeech => _heardSpeech && !_discard;

  /// Coalesces manual stop, silence, timeout and window teardown.
  Future<void> finish({bool discard = false}) {
    if (discard) _discard = true;
    return _ending ??= _finish();
  }

  /// Finalizes the WAV and releases the platform recorder before returning it.
  Future<void> _finish() async {
    _deadline?.cancel();
    RecordedAudio? audio;
    try {
      try {
        await _started;
        if (!_discard) audio = await _recorder.stop(skipSamples: _skipSamples);
      } finally {
        await _recorder.dispose();
      }
      _result.complete(_discard ? null : audio);
    } catch (error, stack) {
      _result.completeError(error, stack);
    }
  }

  /// Meters the input and detects trailing silence using fixed PCM16 frames.
  void _observe(Uint8List bytes) {
    if (_ending != null) return;
    final silenceMs = _silenceMs;
    for (final byte in bytes) {
      final low = _lowByte;
      if (low == null) {
        _lowByte = byte;
        continue;
      }
      _lowByte = null;
      final sample = (byte >= 128 ? byte - 256 : byte) * 256 + low;
      _energy += sample * sample;
      if (++_samples != _samplesPerFrame) continue;
      final audible = _energy > _threshold * _threshold * _samplesPerFrame;
      final meanSquare = _energy / _samplesPerFrame;
      final db = 10 * math.log(meanSquare / (32768 * 32768)) / math.ln10;
      final level = ((db + 42) / 34).clamp(0.0, 1.0).toDouble();
      _level += (level - _level) * (level > _level ? 0.55 : 0.16);
      _energy = 0;
      _samples = 0;
      _frames++;
      if (audible) {
        _quiet = 0;
        if (!_heardSpeech && ++_loud >= _speechFrames) {
          _heardSpeech = true;
          _skipSamples =
              math.max(0, _frames - _speechFrames - _preRollFrames) *
              _samplesPerFrame;
        }
      } else {
        _loud = 0;
        if (silenceMs != null &&
            _heardSpeech &&
            ++_quiet * _frameMs >= silenceMs) {
          unawaited(finish());
          return;
        }
      }
    }
  }
}
