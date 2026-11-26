// ignore_for_file: file_names

import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

class _WavePulse {
  const _WavePulse({
    required this.normX,
    required this.startTime,
    required this.intensity,
  });

  final double normX;
  final double startTime;
  final double intensity;
}

class _WaveParticle {
  const _WaveParticle({
    required this.start,
    required this.control,
    required this.end,
    required this.startTime,
    required this.flightDuration,
    required this.size,
  });

  final Offset start;
  final Offset control;
  final Offset end;
  final double startTime;
  final double flightDuration;
  final double size;
}

/// Drives the Dots wave, the swell under new words and the particles that
/// fly from the wave crest up to each word of the voice input card.
class ChatVoiceWave extends ChangeNotifier {
  static const double _waveWidthFactor = 0.88;
  static const double _maxWaveWidth = 488 * _waveWidthFactor;

  /// Lowest crest the wave may reach, whatever the height of its card.
  static const double _leastCrestHeight = 36;
  static const double _atRestHeight = 12;

  /// Share of the crest height that a voice level of one raises.
  static const double _crest = 0.34;
  static const double _spread = 0.20;
  static const double _syllableSwell = 0.60;
  static const double _glow = 0.90;
  static const double _glowRadius = 7;
  static const double _gridSpacing = 5.6;
  static const double _bigDotsRatio = 0.32;
  static const double _letterDotSize = 2.1;
  static const double _arcHeight = 16;
  static const double _typeComesIn = 0.78;
  static const double _flightDuration = 0.88;
  static const double _staggerWindow = 0.36;
  static const double _dotsStay = 0.35;
  static const double _trailLength = 0.065;
  static const double _pulseLifetime = 0.9;

  /// Wave motion speed in silence, as a fraction of its speed under loud speech.
  static const double _restSpeed = 0.22;

  /// Voice level treated as silence, so room noise leaves the wave at rest.
  static const double _noiseFloor = 0.08;

  /// Rates per second at which speech swells the wave and silence settles it.
  static const double _swellRate = 12.0;
  static const double _settleRate = 2.5;

  /// Duration of the blur-to-text transition once a word is revealed.
  static const Duration tokenRevealDuration = Duration(milliseconds: 550);

  /// Delay before a flown-in word sharpens from a blur into readable text.
  static Duration get tokenRevealDelay => Duration(
    milliseconds:
        ((_flightDuration * _typeComesIn + _staggerWindow * 0.45) * 1000)
            .round(),
  );

  final math.Random _random = math.Random();
  final List<_WavePulse> _pulses = <_WavePulse>[];
  final List<_WaveParticle> _particles = <_WaveParticle>[];
  bool _active = false;
  bool _error = false;

  /// Real animation time, shared by word swells and particle flights.
  double _time = 0;

  /// Shape motion time, which slows to a crawl while nobody speaks.
  double _motion = 0;
  double _presence = 0;
  double _smoothedVoice = 0;
  double _crestHeight = _leastCrestHeight;

  /// Height the crest may reach above the baseline. The painter sets it from
  /// the height of the card it paints the wave in, so the wave fills the card.
  set crestHeight(double value) {
    _crestHeight = math.max(_leastCrestHeight, value);
  }

  /// Whether the wave should be shown for the current voice phase.
  bool get active => _active;

  set active(bool value) {
    if (_active == value) return;
    _active = value;
    notifyListeners();
  }

  /// Whether the wave is tinted with the error color.
  bool get error => _error;

  set error(bool value) {
    if (_error == value) return;
    _error = value;
    notifyListeners();
  }

  /// Whether anything still needs an animation frame.
  bool get isAnimating => _active || _presence > 0 || _particles.isNotEmpty;

  /// Advances the wave by [deltaSeconds] of frame time toward [voiceLevel],
  /// the microphone level from zero to one. The wave rests low and slow in
  /// silence, swells and stirs with the voice, then settles gently.
  void tick(double deltaSeconds, {required double voiceLevel}) {
    _time += deltaSeconds;
    final voice = math.max(0.0, voiceLevel - _noiseFloor) / (1 - _noiseFloor);
    final follow = voice > _smoothedVoice ? _swellRate : _settleRate;
    _smoothedVoice +=
        (voice - _smoothedVoice) * math.min(1.0, deltaSeconds * follow);
    final stir = math.min(1.0, _smoothedVoice * 1.5);
    _motion += deltaSeconds * (_restSpeed + (1 - _restSpeed) * stir);
    final ease = math.min(1.0, deltaSeconds * 7.5);
    _presence += ((_active ? 1.0 : 0.0) - _presence) * ease;
    _pulses.removeWhere(
      (pulse) => _time - pulse.startTime > _pulseLifetime,
    );
    _particles.removeWhere(
      (particle) =>
          _time - particle.startTime >
          particle.flightDuration + _dotsStay,
    );
    if (!_active && _presence <= 0.01 && _particles.isEmpty) {
      _presence = 0;
      _smoothedVoice = 0;
    }
    notifyListeners();
  }

  /// Samples the wave height at [u], from zero to one across the wave.
  double sampleHeight(double u) {
    final envelope = math.pow(
      math.max(0.0, math.sin(u * math.pi)),
      0.72,
    ).toDouble();
    // Offset stays positive, so the resting wave never collapses to nothing.
    final breath =
        0.62 +
        0.38 * math.sin(u * 8.5 + _motion * 2.1) * 0.65 +
        0.20 * math.cos(u * 16.0 - _motion * 3.0);
    var pulseBoost = 0.0;
    for (final pulse in _pulses) {
      final decay = math.exp(-(_time - pulse.startTime) * 3.0);
      final distance = (u - pulse.normX).abs();
      final gaussian = math.exp(
        -(distance * distance) / (2 * _spread * _spread * 0.35),
      );
      pulseBoost += gaussian * decay * pulse.intensity * _syllableSwell;
    }
    final boost = math.min(
      1.25,
      math.max(0.0, _smoothedVoice * _crest * 2.4 + pulseBoost),
    );
    final height =
        _atRestHeight + (_crestHeight - _atRestHeight) * boost * breath.abs();
    return math.min(_crestHeight * 1.15, height * envelope * _presence);
  }

  /// Swells the wave under [word] laid out at [bounds] in a card of
  /// [cardSize] and launches particles that fly up to it.
  void launchWordFlight({
    required String word,
    required Rect bounds,
    required Size cardSize,
  }) {
    final wordWidth = math.max(18.0, bounds.width);
    final wordHeight = math.max(16.0, bounds.height);
    final centerX = bounds.left + wordWidth * 0.5;
    final normX = (centerX / cardSize.width).clamp(0.10, 0.90).toDouble();
    _pulses.add(
      _WavePulse(
        normX: normX,
        startTime: _time,
        intensity: 0.80 + _random.nextDouble() * 0.25,
      ),
    );
    final waveWidth = math.min(
      cardSize.width * _waveWidthFactor,
      _maxWaveWidth,
    );
    final waveStart = (cardSize.width - waveWidth) * 0.5;
    final originCenter = waveStart + waveWidth * normX;
    final originY = cardSize.height - 9;
    final count = (word.length * 3).clamp(11, 20).toInt();
    for (var index = 0; index < count; index += 1) {
      final targetX =
          bounds.left +
          _random.nextDouble() * wordWidth +
          (_random.nextDouble() - 0.5) * 5;
      final targetY =
          bounds.top + wordHeight * (0.24 + _random.nextDouble() * 0.54);
      final startX =
          originCenter +
          (_random.nextDouble() - 0.5) * math.max(44.0, wordWidth * 1.4);
      final startY = originY - _random.nextDouble() * 18;
      final spreadArc = (_random.nextDouble() - 0.5) * 42;
      _particles.add(
        _WaveParticle(
          start: Offset(startX, startY),
          control: Offset(
            (startX + targetX) * 0.5 + spreadArc,
            math.min(startY, targetY) -
                _arcHeight -
                _random.nextDouble() * 22,
          ),
          end: Offset(targetX, targetY),
          startTime: _time + _random.nextDouble() * _staggerWindow,
          flightDuration:
              _flightDuration + (_random.nextDouble() - 0.5) * 0.22,
          size: _letterDotSize * (0.80 + _random.nextDouble() * 0.42),
        ),
      );
    }
    notifyListeners();
  }
}

/// Paints the Dots wave and its particles. The canvas covers the card and
/// reaches [bottomInset] below it, down to the edge of the composer surface.
class ChatVoiceWavePainter extends CustomPainter {
  ChatVoiceWavePainter({
    required this.wave,
    required this.color,
    required this.errorColor,
    required this.isDark,
    required this.bottomInset,
  }) : super(repaint: wave);

  /// Space left free between the crest and the top of the card.
  static const double _topMargin = 6;

  final ChatVoiceWave wave;
  final Color color;
  final Color errorColor;
  final bool isDark;

  /// Part of the canvas that lies below the card, inside the surface's inset.
  final double bottomInset;

  @override
  void paint(Canvas canvas, Size size) {
    if (wave._presence <= 0.01 && wave._particles.isEmpty) return;
    final paintColor = wave._error ? errorColor : color;
    final waveWidth = math.min(
      size.width * ChatVoiceWave._waveWidthFactor,
      ChatVoiceWave._maxWaveWidth,
    );
    final startX = (size.width - waveWidth) * 0.5;
    wave.crestHeight = size.height - bottomInset - _topMargin;
    final bottomY = size.height - 2;
    if (wave._presence > 0.02) {
      _paintGlow(canvas, size, paintColor, waveWidth, startX, bottomY);
      _paintGrid(canvas, paintColor, waveWidth, startX, bottomY);
    }
    _paintParticles(canvas, paintColor);
  }

  /// Blurs a filled copy of the wave to give it a soft halo.
  void _paintGlow(
    Canvas canvas,
    Size size,
    Color paintColor,
    double waveWidth,
    double startX,
    double bottomY,
  ) {
    final path = ui.Path()..moveTo(startX, size.height);
    for (var index = 0; index <= 48; index += 1) {
      final u = index / 48;
      path.lineTo(startX + waveWidth * u, bottomY - wave.sampleHeight(u));
    }
    path
      ..lineTo(startX + waveWidth, size.height)
      ..close();
    canvas.drawPath(
      path,
      Paint()
        ..color = paintColor.withValues(
          alpha: (isDark ? 0.30 : 0.09) *
              ChatVoiceWave._glow *
              wave._presence,
        )
        ..maskFilter = ui.MaskFilter.blur(
          ui.BlurStyle.normal,
          ChatVoiceWave._glowRadius * 1.6,
        ),
    );
  }

  /// Draws the dot grid, batching cells that share an opacity.
  void _paintGrid(
    Canvas canvas,
    Color paintColor,
    double waveWidth,
    double startX,
    double bottomY,
  ) {
    const step = ChatVoiceWave._gridSpacing;
    final cols = (waveWidth / step).floor();
    final sparseStep = (wave._motion * 5.0).floor();
    final bigDotStep = (wave._motion * 6.0).floor();
    final bigDots = <int, ui.Path>{};
    final plusMarks = <int, ui.Path>{};
    final smallDots = <int, ui.Path>{};
    for (var col = 0; col <= cols; col += 1) {
      final u = col / math.max(1, cols);
      final x = startX + u * waveWidth;
      final maxHeight = wave.sampleHeight(u);
      final rows = (maxHeight / step).ceil();
      for (var row = 0; row < rows; row += 1) {
        final y = bottomY - row * step;
        final energy = math.max(
          0.0,
          1 - (row * step) / math.max(1.0, maxHeight),
        );
        final alpha = ((0.25 + energy * 0.75) * wave._presence)
            .clamp(0.12, 0.95)
            .toDouble();
        final centerWeight = math.sin(u * math.pi).clamp(0.0, 1.0).toDouble();
        final bigDotThreshold =
            ChatVoiceWave._bigDotsRatio * energy * centerWeight;
        if (_cellHash(col, row, bigDotStep) < bigDotThreshold) {
          _pathOf(bigDots, alpha).addOval(
            Rect.fromCircle(
              center: Offset(x, y),
              radius: 1.9 + energy * 0.85,
            ),
          );
        } else if (_cellHash(col, row, sparseStep) <
            0.42 + (1 - energy) * 0.25) {
          final halfLength = 1.45 + energy * 0.75;
          _pathOf(plusMarks, alpha * 0.85)
            ..moveTo(x - halfLength, y)
            ..lineTo(x + halfLength, y)
            ..moveTo(x, y - halfLength)
            ..lineTo(x, y + halfLength);
        } else {
          _pathOf(smallDots, alpha * 0.88).addOval(
            Rect.fromCircle(
              center: Offset(x, y),
              radius: 0.85 + energy * 0.65,
            ),
          );
        }
      }
    }
    _fillBuckets(canvas, bigDots, paintColor);
    _strokeBuckets(canvas, plusMarks, paintColor);
    _fillBuckets(canvas, smallDots, paintColor);
  }

  /// Fills each opacity bucket of dots with a single draw call.
  void _fillBuckets(Canvas canvas, Map<int, ui.Path> paths, Color paintColor) {
    final paint = Paint();
    for (final entry in paths.entries) {
      paint.color = paintColor.withValues(
        alpha: (entry.key + 0.5) / _alphaSteps,
      );
      canvas.drawPath(entry.value, paint);
    }
  }

  /// Strokes each opacity bucket of plus marks with a single draw call.
  void _strokeBuckets(
    Canvas canvas,
    Map<int, ui.Path> paths,
    Color paintColor,
  ) {
    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.05
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    for (final entry in paths.entries) {
      paint.color = paintColor.withValues(
        alpha: (entry.key + 0.5) / _alphaSteps,
      );
      canvas.drawPath(entry.value, paint);
    }
  }

  /// Draws each particle as a glowing head with a short fading trail.
  void _paintParticles(Canvas canvas, Color paintColor) {
    final trailPaint = Paint()..strokeCap = StrokeCap.round;
    final headPaint = Paint();
    for (final particle in wave._particles) {
      final elapsed = wave._time - particle.startTime;
      if (elapsed < 0) continue;
      if (elapsed <= particle.flightDuration) {
        final rawT = elapsed / particle.flightDuration;
        final head = _flightPosition(particle, _smoothFlightEase(rawT));
        final tailRaw = math.max(0.0, rawT - ChatVoiceWave._trailLength);
        final tail = _flightPosition(particle, _smoothFlightEase(tailRaw));
        final alpha = math.min(1.0, rawT * 4.5) * 0.88;
        trailPaint
          ..strokeWidth = particle.size * 0.75
          ..shader = ui.Gradient.linear(tail, head, <Color>[
            paintColor.withValues(alpha: 0),
            paintColor.withValues(alpha: alpha * 0.38),
          ]);
        canvas.drawLine(tail, head, trailPaint);
        headPaint.color = paintColor.withValues(alpha: alpha);
        canvas.drawCircle(head, particle.size * 0.55, headPaint);
      } else {
        final stayProgress =
            (elapsed - particle.flightDuration) / ChatVoiceWave._dotsStay;
        headPaint.color = paintColor.withValues(
          alpha: math.max(0.0, 1 - stayProgress) * 0.65,
        );
        canvas.drawCircle(
          particle.end,
          particle.size * 0.48 * (1 - stayProgress * 0.3),
          headPaint,
        );
      }
    }
  }

  @override
  bool shouldRepaint(ChatVoiceWavePainter oldDelegate) =>
      oldDelegate.wave != wave ||
      oldDelegate.color != color ||
      oldDelegate.errorColor != errorColor ||
      oldDelegate.isDark != isDark ||
      oldDelegate.bottomInset != bottomInset;
}

const int _alphaSteps = 32;

/// Returns the path that collects every cell drawn at [alpha].
ui.Path _pathOf(Map<int, ui.Path> paths, double alpha) {
  final bucket = (alpha * _alphaSteps)
      .floor()
      .clamp(0, _alphaSteps - 1)
      .toInt();
  return paths.putIfAbsent(bucket, ui.Path.new);
}

/// Evaluates one quadratic Bezier curve at [t].
Offset _flightPosition(_WaveParticle particle, double t) {
  final inverse = 1 - t;
  return Offset(
    inverse * inverse * particle.start.dx +
        2 * inverse * t * particle.control.dx +
        t * t * particle.end.dx,
    inverse * inverse * particle.start.dy +
        2 * inverse * t * particle.control.dy +
        t * t * particle.end.dy,
  );
}

/// Blends an ease-in-out with an ease-out for a gentle flight.
double _smoothFlightEase(double t) {
  final easeOut = 1 - math.pow(1 - t, 3).toDouble();
  final easeInOut = t < 0.5
      ? 4 * t * t * t
      : 1 - math.pow(-2 * t + 2, 3) / 2;
  return 0.55 * easeInOut + 0.45 * easeOut;
}

/// Hashes one grid cell into a stable pseudo-random value for a time step.
double _cellHash(int col, int row, int timeStep) {
  final value =
      math.sin(col * 127.1 + row * 311.7 + timeStep * 19.19) * 43758.5453;
  return value - value.floorToDouble();
}
