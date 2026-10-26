// ignore_for_file: file_names

import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

/// Whether the new-chat intro currently owns the empty-chat stage.
///
/// The empty chat area hides its static wordmark while this is true so the
/// watermark and the intro animation never render on top of each other.
/// [ChatScreenContent] is stateless, so the flag lives here as a
/// feature-local notifier instead of constructor plumbing.
final ValueNotifier<bool> newChatIntroActive = ValueNotifier<bool>(false);

/// Armed right before a brand-new conversation is created and consumed by
/// the overlay when that conversation becomes current. Switching to any
/// existing conversation leaves this false, so the intro never plays for it.
final ValueNotifier<bool> newChatIntroArmed = ValueNotifier<bool>(false);

/// The new-chat intro: particles assemble into the wordmark, dissolve,
/// then a typewriter greeting with shimmer takes over. Plays once when a
/// freshly created (still empty) conversation becomes current — creation
/// call sites arm [newChatIntroArmed], and switching to an existing
/// conversation never plays it.
class NewChatIntroOverlay extends StatefulWidget {
  const NewChatIntroOverlay({
    super.key,
    required this.currentChatId,
    required this.isChatEmpty,
    required this.isSwitching,
  });

  final String? currentChatId;
  final bool isChatEmpty;
  final bool isSwitching;

  /// Creates the mutable state for the intro overlay.
  @override
  State<NewChatIntroOverlay> createState() => _NewChatIntroOverlayState();
}

class _NewChatIntroOverlayState extends State<NewChatIntroOverlay>
    with TickerProviderStateMixin {
  static const Duration _introDuration = Duration(milliseconds: 4600);
  static const int _assembleMs = 1500;
  static const int _holdMs = 700;
  static const int _dissolveMs = 700;
  // Typing starts a clean beat after the dissolve ends, so the particle
  // wordmark and the greeting never share the stage.
  static const int _typeStartMs = 3100;
  static const int _typeStepMs = 110;
  static const String _greeting = '有什么可以帮你？';
  static const String _subtitle = '新对话已就绪，随时开始';

  // Constructed eagerly, not lazily during disposal of an unused overlay.
  // One finite timeline drives both particles and the typing caret. A repeating
  // ambient controller would keep rebuilding this full-screen canvas forever
  // after the greeting has finished.
  late final AnimationController _intro;

  bool _awaitingSettle = false;
  bool _playing = false;
  bool _dismissed = false;

  /// Ink-sampled wordmark points in wordmark-local pixels, rasterized once
  /// in initState. Layout boxes (getBoxesForRange) cover whole text runs, so
  /// sampling them fills the word's bounding rectangle instead of the
  /// letterforms — pixel sampling matches the approved mockup.
  final List<Offset> _markTargets = <Offset>[];
  Size _markSize = Size.zero;

  /// Creates the finite timeline while the element is still active.
  @override
  void initState() {
    super.initState();
    _intro = AnimationController(vsync: this, duration: _introDuration);
    _precomputeMarkTargets();
  }

  /// Rasterizes the wordmark offscreen and samples opaque pixels on a grid.
  Future<void> _precomputeMarkTargets() async {
    const word = 'Operit';
    const markFontSize = 60.0;
    const sampleStep = 2;
    final builder =
        ui.ParagraphBuilder(ui.ParagraphStyle(textDirection: TextDirection.ltr))
          ..pushStyle(
            ui.TextStyle(
              color: const Color(0xFFFFFFFF),
              fontSize: markFontSize,
              fontWeight: ui.FontWeight.w800,
              letterSpacing: 2,
            ),
          )
          ..addText(word);
    final paragraph = builder.build()
      ..layout(const ui.ParagraphConstraints(width: 1000));
    final markWidth = paragraph.maxIntrinsicWidth.ceil() + 8;
    final markHeight = paragraph.height.ceil() + 8;
    final recorder = ui.PictureRecorder();
    Canvas(recorder).drawParagraph(paragraph, Offset.zero);
    final picture = recorder.endRecording();
    final image = picture.toImageSync(markWidth, markHeight);
    picture.dispose();
    paragraph.dispose();
    // Keep the dimensions before disposing the image. CanvasKit invalidates
    // image getters after dispose, which made the Web frontend throw a
    // null-check error while sampling the intro wordmark.
    final imageWidth = image.width;
    final imageHeight = image.height;
    final bytes = await image.toByteData();
    image.dispose();
    if (bytes == null || !mounted) {
      return;
    }
    final pixels = bytes.buffer.asUint8List();
    final targets = <Offset>[];
    for (var y = 0; y < imageHeight; y += sampleStep) {
      for (var x = 0; x < imageWidth; x += sampleStep) {
        if (pixels[(y * imageWidth + x) * 4 + 3] > 128) {
          targets.add(Offset(x.toDouble(), y.toDouble()));
        }
      }
    }
    if (targets.isEmpty) {
      return;
    }
    setState(() {
      _markSize = Size(imageWidth.toDouble(), imageHeight.toDouble());
      _markTargets
        ..clear()
        ..addAll(targets);
    });
  }

  /// Starts the intro when a new empty conversation settles as current.
  ///
  /// Controller mutations are deferred to a post-frame callback: touching
  /// tickers from didUpdateWidget notifies AnimatedBuilder while the parent
  /// rebuild is still laying out, which crashes with "looking up a
  /// deactivated widget's ancestor" when the subtree is being re-keyed.
  @override
  void didUpdateWidget(covariant NewChatIntroOverlay oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentChatId != widget.currentChatId) {
      // The intro only plays for a conversation that was just created — the
      // creation call sites arm [newChatIntroArmed] before the new chat id
      // arrives here. Plain switches between existing conversations (even
      // empty ones) must not claim the stage or play.
      _awaitingSettle = newChatIntroArmed.value;
      newChatIntroArmed.value = false;
      _schedule(_hideImmediate);
      // Claim the stage with the chat switch, before the new chat's empty
      // area becomes visible. The write must be deferred like every other
      // mutation here: this didUpdateWidget can run from a LayoutBuilder's
      // layout callback, and notifying ChatArea's ValueListenableBuilder
      // synchronously asserts with "markNeedsBuild during build". The
      // switch keeps the chat area hidden, so a post-frame claim still
      // lands before anything is on screen.
      if (_awaitingSettle) {
        _schedule(() => newChatIntroActive.value = true);
      }
    }
    if (_awaitingSettle && !widget.isSwitching) {
      _awaitingSettle = false;
      if (widget.isChatEmpty && widget.currentChatId != null) {
        _schedule(_start);
      } else {
        // Settled on a non-empty chat — the intro will not play.
        _schedule(() => newChatIntroActive.value = false);
      }
    }
    if (_playing && !widget.isChatEmpty) {
      _schedule(_hideImmediate);
    }
  }

  void _schedule(void Function() action) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        action();
      }
    });
  }

  @override
  void dispose() {
    // Deferred: notifying ChatArea's ValueListenableBuilder from teardown
    // would mark an element that is being deactivated this same frame.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      newChatIntroActive.value = false;
    });
    _intro.dispose();
    super.dispose();
  }

  void _start() {
    _dismissed = false;
    _playing = true;
    newChatIntroActive.value = true;
    _intro
      ..reset()
      ..forward();
    setState(() {});
  }

  void _hideImmediate() {
    if (!_playing && _dismissed) {
      return;
    }
    _playing = false;
    _dismissed = true;
    _intro.stop();
    // A chat switch that is still settling owns the stage claim already;
    // releasing here would flash the static wordmark before the incoming
    // chat's intro (or its non-empty content) takes over.
    if (!_awaitingSettle) {
      newChatIntroActive.value = false;
    }
    setState(() {});
  }

  /// Builds the particle canvas plus the greeting column.
  @override
  Widget build(BuildContext context) {
    if (!_playing) {
      return const SizedBox.shrink();
    }
    final colorScheme = Theme.of(context).colorScheme;
    return AnimatedBuilder(
      animation: _intro,
      builder: (context, _) {
        // Driven by controller value rather than lastElapsedDuration: value
        // is defined on every frame (never null) and matches the mockup
        // timeline exactly while forward() runs.
        final t = (_intro.value * _introDuration.inMilliseconds).round();
        final greetingVisible = t >= _typeStartMs - 120;
        final typedCount = t <= _typeStartMs
            ? 0
            : math.min(
                _greeting.length,
                ((t - _typeStartMs) / _typeStepMs).floor(),
              );
        final subtitleVisible =
            t >= _typeStartMs + _greeting.length * _typeStepMs + 250;
        // Blink only while the finite intro is running; the settled greeting
        // stays visible without requesting another frame.
        final caretOn = _intro.isCompleted || t % 1100 < 605;
        return Stack(
          fit: StackFit.expand,
          children: <Widget>[
            CustomPaint(
              size: Size.infinite,
              painter: _ParticleWordPainter(
                progress: _phaseProgress(t),
                dissolve: _dissolveProgress(t),
                color: colorScheme.primary,
                markTargets: _markTargets,
                markSize: _markSize,
              ),
            ),
            Center(
              child: Opacity(
                opacity: greetingVisible ? 1 : 0,
                child: ThemeNewChatGreeting(
                  greeting: _greeting.substring(0, typedCount),
                  subtitle: _subtitle,
                  caretVisible: caretOn,
                  subtitleVisible: subtitleVisible,
                ),
              ),
            ),
          ],
        );
      },
    );
  }

  double _phaseProgress(int elapsedMs) {
    return (elapsedMs / _assembleMs).clamp(0.0, 1.0);
  }

  double _dissolveProgress(int elapsedMs) {
    // Hold the assembled wordmark crisp briefly before dissolving, so the
    // "Operit" glyph is legible instead of blurring straight into the fade.
    final started = elapsedMs - (_assembleMs + _holdMs);
    if (started <= 0) {
      return 0;
    }
    return (started / _dissolveMs).clamp(0.0, 1.0);
  }
}

/// Draws particles that fly in from the edges and assemble into the
/// wordmark glyphs, then dissolve in place. Glyph targets come from the
/// ink-sampled wordmark points prepared by the overlay state.
class _ParticleWordPainter extends CustomPainter {
  _ParticleWordPainter({
    required this.progress,
    required this.dissolve,
    required this.color,
    required this.markTargets,
    required this.markSize,
  });

  final double progress;
  final double dissolve;
  final Color color;
  final List<Offset> markTargets;
  final Size markSize;

  Size? _builtFor;
  final List<_Particle> _particles = <_Particle>[];
  final math.Random _random = math.Random(7);

  void _build(Size size) {
    _builtFor = size;
    _particles.clear();
    if (markTargets.isEmpty) {
      return;
    }
    // All particles live inside a centered stage around the wordmark. The
    // overlay spans the whole chat area, so sampling starts across the full
    // canvas blankets the entire window with dots at desktop scale.
    final stage = Size(
      math.min(size.width, 480.0),
      math.min(size.height, 280.0),
    );
    final stageCenter = Offset(size.width / 2, size.height / 2);
    final stageOrigin = stageCenter - Offset(stage.width / 2, stage.height / 2);
    final markTopLeft =
        stageOrigin +
        Offset(
          (stage.width - markSize.width) / 2,
          (stage.height - markSize.height) / 2,
        );
    // Spawn ring around the stage: particles fly in from just outside the
    // wordmark zone instead of scattering from every corner of the screen.
    final ringRadius = math.max(stage.width, stage.height) * 0.75;
    for (final local in markTargets) {
      if (_random.nextDouble() < 0.2) {
        continue;
      }
      final angle = _random.nextDouble() * math.pi * 2;
      final radius = ringRadius * (0.55 + _random.nextDouble() * 0.25);
      _particles.add(
        _Particle(
          target:
              markTopLeft +
              local +
              Offset(_random.nextDouble(), _random.nextDouble()),
          start:
              stageCenter +
              Offset(math.cos(angle) * radius, math.sin(angle) * radius),
          delay: 0.4 + _random.nextDouble() * 0.6,
          size: 1.2 + _random.nextDouble() * 1.4,
        ),
      );
    }
  }

  @override
  void paint(Canvas canvas, Size size) {
    if (_builtFor != size) {
      _build(size);
    }
    final paint = Paint()..isAntiAlias = true;
    for (final particle in _particles) {
      final local = (progress / particle.delay).clamp(0.0, 1.0);
      final eased = 1 - math.pow(1 - local, 3).toDouble();
      final position = Offset.lerp(particle.start, particle.target, eased)!;
      paint.color = color.withValues(
        alpha: (0.55 + 0.45 * eased) * (1 - dissolve),
      );
      canvas.drawCircle(position, particle.size, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _ParticleWordPainter oldDelegate) {
    return oldDelegate.progress != progress ||
        oldDelegate.dissolve != dissolve ||
        oldDelegate.markTargets.length != markTargets.length;
  }
}

class _Particle {
  _Particle({
    required this.start,
    required this.target,
    required this.delay,
    required this.size,
  });

  final Offset start;
  final Offset target;
  final double delay;
  final double size;
}
