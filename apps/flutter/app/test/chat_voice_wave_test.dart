import 'dart:math' as math;

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/voice_input/ChatVoiceWave.dart';

/// Advances [wave] by [frames] frames of 60 fps at [level].
void _run(ChatVoiceWave wave, int frames, {double level = 0}) {
  for (var frame = 0; frame < frames; frame += 1) {
    wave.tick(1 / 60, voiceLevel: level);
  }
}

/// Returns the tallest point of the wave.
double _peak(ChatVoiceWave wave) {
  var peak = 0.0;
  for (var sample = 0; sample <= 100; sample += 1) {
    peak = math.max(peak, wave.sampleHeight(sample / 100));
  }
  return peak;
}

/// Returns how far the wave shape moves across [frames] at [level].
double _shift(ChatVoiceWave wave, int frames, {double level = 0}) {
  final before = List<double>.generate(
    101,
    (sample) => wave.sampleHeight(sample / 100),
  );
  _run(wave, frames, level: level);
  var shift = 0.0;
  for (var sample = 0; sample <= 100; sample += 1) {
    shift = math.max(
      shift,
      (wave.sampleHeight(sample / 100) - before[sample]).abs(),
    );
  }
  return shift;
}

void main() {
  // Height of the resting wave at its tallest point, whatever its phase.
  final Matcher rests = inInclusiveRange(5, 13);

  test('rests as a low wave that barely moves in silence', () {
    final wave = ChatVoiceWave()..active = true;
    addTearDown(wave.dispose);
    _run(wave, 90);

    expect(_peak(wave), rests);
    expect(_shift(wave, 6), lessThan(0.3));
  });

  test('swells with the voice, stirs faster and settles gently', () {
    final wave = ChatVoiceWave()..active = true;
    addTearDown(wave.dispose);
    _run(wave, 90);
    final rest = _peak(wave);
    final restShift = _shift(wave, 6);

    _run(wave, 30, level: 0.9);
    final spoken = _peak(wave);
    expect(spoken, greaterThan(rest + 8));
    expect(_shift(wave, 6, level: 0.9), greaterThan(restShift * 3));

    // Silence eases the wave down instead of dropping it at once.
    _run(wave, 4);
    expect(_peak(wave), greaterThan(rest + (spoken - rest) / 2));

    _run(wave, 120);
    expect(_peak(wave), rests);
  });

  test('treats room noise below its floor as silence', () {
    final wave = ChatVoiceWave()..active = true;
    addTearDown(wave.dispose);
    _run(wave, 90);
    _run(wave, 90, level: 0.05);

    expect(_peak(wave), rests);
  });
}
