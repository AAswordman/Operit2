// ignore_for_file: file_names

import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitTheme.dart';
import 'ChatVoiceInputSession.dart';
import 'ChatVoiceWave.dart';

/// Shows the voice card in place of [composer] while [session] is not idle.
/// The two cross-fade while the composer grows into the card and shrinks back.
class ChatVoiceInputSwitcher extends StatefulWidget {
  const ChatVoiceInputSwitcher({
    super.key,
    required this.session,
    required this.textStyle,
    required this.textOrigin,
    required this.surfaceBottomInset,
    required this.composer,
  });

  /// Duration of the voice card fading in.
  static const Duration swapDuration = Duration(milliseconds: 220);

  /// Duration of the composer fading out during a swap.
  static const Duration swapOutDuration = Duration(milliseconds: 100);

  /// Duration of the height change that accompanies a swap.
  static const Duration resizeDuration = Duration(milliseconds: 280);

  final ChatVoiceInputSession session;
  final TextStyle textStyle;

  /// Offset of the composer's first text line inside its content box, so the
  /// card shows words exactly where the composer shows typed text.
  final Offset textOrigin;

  /// Space between the composer's content and the bottom edge of its surface.
  /// The card's wave covers it, so the wave ends flush with the surface.
  final double surfaceBottomInset;
  final Widget composer;

  @override
  State<ChatVoiceInputSwitcher> createState() => _ChatVoiceInputSwitcherState();
}

class _ChatVoiceInputSwitcherState extends State<ChatVoiceInputSwitcher> {
  static final Tween<Offset> _swapOffset = Tween<Offset>(
    begin: const Offset(0, 0.08),
    end: Offset.zero,
  );

  Timer? _resizeEnd;
  bool _resizing = false;

  @override
  void initState() {
    super.initState();
    widget.session.addListener(_syncResize);
    _resizing = _isActive;
  }

  @override
  void didUpdateWidget(ChatVoiceInputSwitcher oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.session == widget.session) return;
    oldWidget.session.removeListener(_syncResize);
    widget.session.addListener(_syncResize);
    _syncResize();
  }

  @override
  void dispose() {
    widget.session.removeListener(_syncResize);
    _resizeEnd?.cancel();
    super.dispose();
  }

  bool get _isActive =>
      widget.session.phase != ChatVoiceInputPhase.idle;

  /// Eases height changes only around a swap, so the composer's own expansion
  /// is not animated a second time.
  void _syncResize() {
    if (_isActive) {
      _resizeEnd?.cancel();
      _resizeEnd = null;
      if (!_resizing) setState(() => _resizing = true);
      return;
    }
    if (!_resizing || _resizeEnd != null) return;
    _resizeEnd = Timer(
      ChatVoiceInputSwitcher.swapDuration +
          ChatVoiceInputSwitcher.resizeDuration,
      () {
        if (!mounted) return;
        setState(() {
          _resizing = false;
          _resizeEnd = null;
        });
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    return _HeightTransition(
      animated: _resizing,
      duration: ChatVoiceInputSwitcher.resizeDuration,
      child: ListenableBuilder(
        listenable: widget.session,
        builder: (context, child) => AnimatedSwitcher(
          duration: ChatVoiceInputSwitcher.swapDuration,
          reverseDuration: ChatVoiceInputSwitcher.swapOutDuration,
          switchInCurve: Curves.easeOutCubic,
          switchOutCurve: Curves.easeInCubic,
          transitionBuilder: _swapTransition,
          layoutBuilder: _stackSwapping,
          child: _isActive
              ? ChatVoiceInputCard(
                  session: widget.session,
                  textStyle: widget.textStyle,
                  textOrigin: widget.textOrigin,
                  surfaceBottomInset: widget.surfaceBottomInset,
                )
              : child!,
        ),
        child: widget.composer,
      ),
    );
  }

  /// Keeps both children bottom-aligned on the composer's own constraints, so
  /// the card is revealed from the bottom as the height grows. Nothing is
  /// clipped here: the card's wave may reach below the box, and the composer
  /// surface clips it to its own rounded edge.
  static Widget _stackSwapping(
    Widget? currentChild,
    List<Widget> previousChildren,
  ) => Stack(
    alignment: Alignment.bottomCenter,
    fit: StackFit.passthrough,
    clipBehavior: Clip.none,
    children: <Widget>[...previousChildren, ?currentChild],
  );

  /// Fades a swapping child while it rises the last few pixels into place.
  static Widget _swapTransition(Widget child, Animation<double> animation) =>
      FadeTransition(
        opacity: animation,
        child: SlideTransition(
          position: _swapOffset.animate(animation),
          child: child,
        ),
      );
}

/// Eases between the heights its child takes while [animated] is set, and
/// reports the child's size untouched otherwise. [AnimatedSize] cannot do
/// this: a zero duration makes it re-dirty itself during its own layout.
class _HeightTransition extends StatefulWidget {
  const _HeightTransition({
    required this.animated,
    required this.duration,
    required this.child,
  });

  final bool animated;
  final Duration duration;
  final Widget child;

  @override
  State<_HeightTransition> createState() => _HeightTransitionState();
}

class _HeightTransitionState extends State<_HeightTransition>
    with SingleTickerProviderStateMixin {
  final GlobalKey _renderKey = GlobalKey();

  /// Created by the first resize, so a transition that never animates owns no
  /// ticker and its dispose never reaches into the deactivated tree.
  AnimationController? _animation;
  AnimationController get _controller => _animation ??= AnimationController(
    vsync: this,
    duration: widget.duration,
  )..addListener(_applyProgress);

  @override
  void didUpdateWidget(_HeightTransition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.duration != oldWidget.duration) {
      _animation?.duration = widget.duration;
    }
    if (!widget.animated) _animation?.stop();
  }

  @override
  void dispose() {
    _animation?.dispose();
    super.dispose();
  }

  /// Runs from layout, so it only starts the animation and never touches the tree.
  void _startResize() => _controller.forward(from: 0);

  void _applyProgress() {
    final renderObject = _renderKey.currentContext?.findRenderObject();
    if (renderObject is! _RenderHeightTransition) return;
    renderObject.progress = Curves.easeOutCubic.transform(_controller.value);
  }

  @override
  Widget build(BuildContext context) => _HeightTransitionRender(
    key: _renderKey,
    animated: widget.animated,
    onHeightChanged: _startResize,
    child: widget.child,
  );
}

class _HeightTransitionRender extends SingleChildRenderObjectWidget {
  const _HeightTransitionRender({
    super.key,
    required this.animated,
    required this.onHeightChanged,
    required super.child,
  });

  final bool animated;
  final VoidCallback onHeightChanged;

  @override
  _RenderHeightTransition createRenderObject(BuildContext context) =>
      _RenderHeightTransition(
        animated: animated,
        onHeightChanged: onHeightChanged,
      );

  @override
  void updateRenderObject(
    BuildContext context,
    _RenderHeightTransition renderObject,
  ) {
    renderObject
      ..animated = animated
      ..onHeightChanged = onHeightChanged;
  }
}

/// Keeps its child bottom-aligned and, while easing, reports a height between
/// the one last shown and the child's own height. Painting, hit testing and
/// baselines follow the child's offset through [RenderShiftedBox].
class _RenderHeightTransition extends RenderShiftedBox {
  _RenderHeightTransition({
    required bool animated,
    required this.onHeightChanged,
  }) : _animated = animated,
       super(null);

  /// Runs from layout when the child's height has to be eased to.
  VoidCallback onHeightChanged;

  final LayerHandle<ClipRectLayer> _clipLayer = LayerHandle<ClipRectLayer>();

  bool _animated;

  /// Whether height changes are eased. Otherwise the child's size is reported as is.
  bool get animated => _animated;

  set animated(bool value) {
    if (value == _animated) return;
    _animated = value;
    markNeedsLayout();
  }

  double _progress = 1;

  /// Eased progress of the running height animation.
  set progress(double value) {
    if (value == _progress) return;
    _progress = value;
    markNeedsLayout();
  }

  double? _fromHeight;
  double? _toHeight;
  double _shownHeight = 0;
  bool _laidOut = false;

  @override
  void performLayout() {
    final child = this.child;
    if (child == null) {
      size = constraints.smallest;
      return;
    }
    child.layout(constraints, parentUsesSize: true);
    size = constraints.constrain(_settleTarget(child.size));
    _shownHeight = size.height;
    _laidOut = true;
    final childParentData = child.parentData as BoxParentData;
    childParentData.offset = Offset(0, size.height - child.size.height);
  }

  @override
  Size computeDryLayout(BoxConstraints constraints) {
    final child = this.child;
    if (child == null) return constraints.smallest;
    final natural = child.getDryLayout(constraints);
    return constraints.constrain(
      Size(natural.width, _easedHeight(natural.height)),
    );
  }

  @override
  double? computeDryBaseline(BoxConstraints constraints, TextBaseline baseline) {
    final child = this.child;
    final result = super.computeDryBaseline(constraints, baseline);
    if (child == null || result == null) return result;
    return (
      result +
      computeDryLayout(constraints).height -
      child.getDryLayout(constraints).height
    );
  }

  /// Tracks the child's height and returns the height to report for it.
  Size _settleTarget(Size natural) {
    if (!_animated) {
      _fromHeight = null;
      _toHeight = null;
      return natural;
    }
    if (_toHeight == null) {
      // The first layout of an eased stretch starts from the height shown so far.
      _fromHeight = _laidOut ? _shownHeight : natural.height;
      _toHeight = natural.height;
      _restart();
    } else if (_toHeight != natural.height) {
      // The child moved while easing, so continue from the height shown now.
      _fromHeight = _shownHeight;
      _toHeight = natural.height;
      _restart();
    }
    return Size(natural.width, _easedHeight(natural.height));
  }

  double _easedHeight(double naturalHeight) {
    final from = _fromHeight;
    final to = _toHeight;
    if (from == null || to == null) return naturalHeight;
    return from + (to - from) * _progress;
  }

  /// Restarts the ease from the height currently shown, when one is needed.
  void _restart() {
    _progress = 0;
    if (_fromHeight != _toHeight) onHeightChanged();
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    final child = this.child;
    if (child != null && size.height < child.size.height) {
      // While easing, the child reaches above the box. Only that part is
      // hidden: the card's wave may still reach below the box, into the
      // composer surface's own inset.
      _clipLayer.layer = context.pushClipRect(
        needsCompositing,
        offset,
        Rect.fromLTRB(0, 0, size.width, double.maxFinite),
        super.paint,
        oldLayer: _clipLayer.layer,
      );
      return;
    }
    _clipLayer.layer = null;
    super.paint(context, offset);
  }

  @override
  void dispose() {
    _clipLayer.layer = null;
    super.dispose();
  }
}

/// In-place voice card shown inside the composer while the user speaks.
class ChatVoiceInputCard extends StatefulWidget {
  const ChatVoiceInputCard({
    super.key,
    required this.session,
    required this.textStyle,
    required this.textOrigin,
    required this.surfaceBottomInset,
  });

  final ChatVoiceInputSession session;
  final TextStyle textStyle;

  /// Offset of the composer's first text line inside its content box.
  final Offset textOrigin;

  /// Space between the card and the bottom edge of the composer surface.
  final double surfaceBottomInset;

  @override
  State<ChatVoiceInputCard> createState() => _ChatVoiceInputCardState();
}

class _ChatVoiceInputCardState extends State<ChatVoiceInputCard>
    with TickerProviderStateMixin {
  /// Lines of transcript kept visible; older lines scroll out of the card.
  static const int _maxTextLines = 3;
  static const double _thinkingDotStagger = 0.18 / 1.1;

  /// Diameter of the action key, sized like the composer's action button.
  static const double _keySize = 42;

  final GlobalKey _cardKey = GlobalKey();
  final ChatVoiceWave _wave = ChatVoiceWave();

  /// Created once the wave first animates, so a card that never shows a wave
  /// owns no ticker.
  Ticker? _waveTicker;
  late final AnimationController _pulseController = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 650),
  );
  late final CurvedAnimation _pulseCurve = CurvedAnimation(
    parent: _pulseController,
    curve: Curves.easeInOut,
  );
  late final Animation<double> _pulseOpacity = Tween<double>(
    begin: 0.35,
    end: 1,
  ).animate(_pulseCurve);
  late final AnimationController _thinkingController = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1100),
  );
  Duration? _lastWaveTick;
  final List<GlobalKey> _tokenKeys = <GlobalKey>[];
  int _flownTokens = 0;
  bool _flightFrameScheduled = false;

  @override
  void initState() {
    super.initState();
    widget.session.addListener(_handleSessionChanged);
    _wave.addListener(_handleWaveChanged);
    _syncAnimations();
    _scheduleFlights();
  }

  @override
  void didUpdateWidget(ChatVoiceInputCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.session == widget.session) return;
    oldWidget.session.removeListener(_handleSessionChanged);
    widget.session.addListener(_handleSessionChanged);
    _flownTokens = 0;
    _syncAnimations();
    _scheduleFlights();
  }

  @override
  void dispose() {
    widget.session.removeListener(_handleSessionChanged);
    _wave.removeListener(_handleWaveChanged);
    // The card usually leaves while its wave is still fading out.
    _waveTicker?.dispose();
    _pulseCurve.dispose();
    _pulseController.dispose();
    _thinkingController.dispose();
    _wave.dispose();
    super.dispose();
  }

  /// Matches the wave, pulse and thinking animations to the current phase.
  void _syncAnimations() {
    final phase = widget.session.phase;
    final showsThinking =
        phase == ChatVoiceInputPhase.thinking ||
        phase == ChatVoiceInputPhase.typing;
    _wave
      ..active =
          phase == ChatVoiceInputPhase.listening || showsThinking
      ..error = phase == ChatVoiceInputPhase.error;
    if (phase == ChatVoiceInputPhase.listening) {
      _pulseController.repeat(reverse: true);
    } else {
      _pulseController.stop();
    }
    if (showsThinking) {
      _thinkingController.repeat();
    } else {
      _thinkingController.stop();
    }
  }

  void _handleSessionChanged() {
    if (!mounted) return;
    _syncAnimations();
    setState(() {});
    _scheduleFlights();
  }

  void _handleWaveChanged() {
    if (!_wave.isAnimating) return;
    final ticker = _waveTicker ??= createTicker(_onWaveTick);
    if (ticker.isActive) return;
    _lastWaveTick = null;
    ticker.start();
  }

  void _onWaveTick(Duration elapsed) {
    final last = _lastWaveTick;
    _lastWaveTick = elapsed;
    final deltaSeconds = last == null
        ? 0.0
        : ((elapsed.inMicroseconds - last.inMicroseconds) / 1e6)
              .clamp(0.0, 0.05)
              .toDouble();
    _wave.tick(deltaSeconds, voiceLevel: _waveVoiceLevel());
    if (!_wave.isAnimating) _waveTicker?.stop();
  }

  /// Resolves the level the wave follows. While listening it is the microphone
  /// itself, so silence keeps the wave at rest and speech raises it.
  double _waveVoiceLevel() => switch (widget.session.phase) {
    ChatVoiceInputPhase.listening => widget.session.level,
    ChatVoiceInputPhase.thinking => 0.12,
    ChatVoiceInputPhase.typing => 0.45,
    ChatVoiceInputPhase.error => 0.1,
    ChatVoiceInputPhase.empty || ChatVoiceInputPhase.idle => 0,
  };

  /// Launches flights for words laid out since the previous frame.
  void _scheduleFlights() {
    if (_flightFrameScheduled) return;
    if (widget.session.tokens.length == _flownTokens) return;
    _flightFrameScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _flightFrameScheduled = false;
      _launchFlights();
    });
  }

  void _launchFlights() {
    if (!mounted) return;
    final tokens = widget.session.tokens;
    if (tokens.length < _flownTokens) {
      _flownTokens = tokens.length;
      return;
    }
    final cardObject = _cardKey.currentContext?.findRenderObject();
    if (cardObject is! RenderBox || !cardObject.attached) return;
    // Particles start at the wave's baseline, which lies below the card.
    final waveSize = Size(
      cardObject.size.width,
      cardObject.size.height + widget.surfaceBottomInset,
    );
    for (var index = _flownTokens; index < tokens.length; index += 1) {
      final tokenObject = _tokenKeys[index].currentContext?.findRenderObject();
      if (tokenObject is! RenderBox || !tokenObject.attached) continue;
      _wave.launchWordFlight(
        word: tokens[index].text,
        bounds: MatrixUtils.transformRect(
          tokenObject.getTransformTo(cardObject),
          Offset.zero & tokenObject.size,
        ),
        cardSize: waveSize,
      );
    }
    _flownTokens = tokens.length;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    while (_tokenKeys.length < widget.session.tokens.length) {
      _tokenKeys.add(GlobalKey());
    }
    final textOrigin = widget.textOrigin;
    return Stack(
      key: _cardKey,
      // The wave reaches below the card, up to the composer surface's edge.
      clipBehavior: Clip.none,
      children: <Widget>[
        Positioned(
          left: 0,
          right: 0,
          top: 0,
          bottom: -widget.surfaceBottomInset,
          child: IgnorePointer(
            child: CustomPaint(
              painter: ChatVoiceWavePainter(
                wave: _wave,
                color: scheme.onSurface,
                errorColor: scheme.error,
                isDark: theme.brightness == Brightness.dark,
                bottomInset: widget.surfaceBottomInset,
              ),
            ),
          ),
        ),
        Padding(
          // Words start where the composer's own text starts, and the key
          // lines up with its action button.
          padding: EdgeInsetsDirectional.fromSTEB(
            textOrigin.dx,
            textOrigin.dy,
            4,
            4,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Expanded(
                    child: GestureDetector(
                      onTap: widget.session.advance,
                      child: _buildText(scheme),
                    ),
                  ),
                  _buildStatusPill(scheme),
                ],
              ),
              const SizedBox(height: 2),
              Align(
                alignment: AlignmentDirectional.centerEnd,
                child: _buildActionKey(scheme),
              ),
            ],
          ),
        ),
      ],
    );
  }

  /// Shows the recognized words, or the placeholder for the current phase.
  Widget _buildText(ColorScheme scheme) {
    final session = widget.session;
    final tokens = session.tokens;
    final baseStyle = widget.textStyle;
    if (tokens.isNotEmpty) {
      final lineHeight = (baseStyle.fontSize ?? 14) * (baseStyle.height ?? 1);
      return ConstrainedBox(
        constraints: BoxConstraints(maxHeight: lineHeight * _maxTextLines),
        child: ClipRect(
          // Sized to the words themselves, so the first line stays at the top.
          // Once words outgrow the limit, the oldest lines leave through the top.
          child: OverflowBox(
            alignment: Alignment.bottomLeft,
            fit: OverflowBoxFit.deferToChild,
            maxHeight: double.infinity,
            child: Wrap(
              children: <Widget>[
                for (var index = 0; index < tokens.length; index += 1)
                  _VoiceToken(
                    key: _tokenKeys[index],
                    text: tokens[index].spaced
                        ? ' ${tokens[index].text}'
                        : tokens[index].text,
                    style: baseStyle,
                  ),
              ],
            ),
          ),
        ),
      );
    }
    final l10n = AppLocalizations.of(context)!;
    final problem = session.problem;
    final (String message, Color color) = switch (session.phase) {
      ChatVoiceInputPhase.error => (
        problem!.isMissingConfiguration
            ? l10n.chatSpeechInputConfigurationRequired
            : l10n.chatSpeechInputFailed('${problem.error}'),
        scheme.error,
      ),
      ChatVoiceInputPhase.empty => (
        l10n.chatSpeechNoTextRecognized,
        scheme.onSurfaceVariant,
      ),
      _ => (l10n.chatVoiceListening, scheme.onSurfaceVariant),
    };
    return Text(
      message,
      style: baseStyle.copyWith(color: color),
      maxLines: _maxTextLines,
      overflow: TextOverflow.ellipsis,
    );
  }

  /// Shows the elapsed listening time, or the thinking label after a stop.
  Widget _buildStatusPill(ColorScheme scheme) {
    final phase = widget.session.phase;
    final elapsed = widget.session.elapsed;
    final showsThinking =
        phase == ChatVoiceInputPhase.thinking ||
        phase == ChatVoiceInputPhase.typing;
    final label = showsThinking
        ? AppLocalizations.of(context)!.chatVoiceThinking
        : '${elapsed.inMinutes}:${(elapsed.inSeconds % 60).toString().padLeft(2, '0')}';
    return AnimatedOpacity(
      opacity: phase == ChatVoiceInputPhase.listening || showsThinking
          ? 1
          : 0,
      duration: const Duration(milliseconds: 200),
      child: Padding(
        // Centers the label on the first text line of the card.
        padding: const EdgeInsetsDirectional.only(top: 1, start: 12),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            FadeTransition(
              opacity: _pulseOpacity,
              child: Container(
                width: 5.5,
                height: 5.5,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: scheme.onSurface,
                ),
              ),
            ),
            const SizedBox(width: 6),
            Text(
              label,
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w500,
                fontFeatures: const <ui.FontFeature>[
                  ui.FontFeature.tabularFigures(),
                ],
                color: scheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Shows the stop key while listening, thinking dots, or the error key.
  Widget _buildActionKey(ColorScheme scheme) {
    final session = widget.session;
    final phase = session.phase;
    final isError = phase == ChatVoiceInputPhase.error;
    final iconColor = isError ? scheme.error : scheme.onSurface;
    final VoidCallback? onTap = switch (phase) {
      ChatVoiceInputPhase.listening => () => unawaited(session.stop()),
      ChatVoiceInputPhase.thinking || ChatVoiceInputPhase.typing => null,
      _ => () => unawaited(session.start()),
    };
    return _VoiceKey(
      size: _keySize,
      background: isError
          ? scheme.error.withValues(alpha: 0.14)
          : scheme.onSurface.withValues(alpha: 0.08),
      onTap: onTap,
      tooltip: phase == ChatVoiceInputPhase.listening
          ? AppLocalizations.of(context)!.chatVoiceStopListening
          : '',
      child: switch (phase) {
        ChatVoiceInputPhase.listening => Container(
            width: 13,
            height: 13,
            decoration: BoxDecoration(
              color: iconColor,
              borderRadius: BorderRadius.circular(3.2),
            ),
          ),
        ChatVoiceInputPhase.thinking || ChatVoiceInputPhase.typing =>
          _buildThinkingDots(iconColor),
        ChatVoiceInputPhase.error => CustomPaint(
            size: const Size(18, 18),
            painter: _ExclamationMarkPainter(color: iconColor),
          ),
        _ => _buildMicIcon(scheme),
      },
    );
  }

  /// Shows the theme's microphone asset, or the default microphone icon.
  Widget _buildMicIcon(ColorScheme scheme) {
    final snapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final asset = resolveThemeInputActionAsset(
      snapshot,
      isStop: false,
      isSend: false,
    );
    if (asset != null && asset.isNotEmpty) {
      final size = resolveThemeInputActionAssetSize(
        snapshot,
        isStop: false,
        isSend: false,
      );
      return SizedBox(
        width: size,
        height: size,
        child: ResolvedThemeImage(
          resolver: ThemeAssetScope.of(context),
          reference: asset,
          fit: BoxFit.contain,
        ),
      );
    }
    return Icon(Icons.mic, size: 18, color: scheme.onSurface);
  }

  Widget _buildThinkingDots(Color color) {
    return AnimatedBuilder(
      animation: _thinkingController,
      builder: (context, child) => Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          for (var index = 0; index < 3; index += 1)
            _buildThinkingDot(
              color,
              _thinkingDotProgress(
                (_thinkingController.value + index * _thinkingDotStagger) % 1,
              ),
              trailingGap: index < 2,
            ),
        ],
      ),
    );
  }

  Widget _buildThinkingDot(
    Color color,
    double progress, {
    required bool trailingGap,
  }) {
    return Padding(
      padding: EdgeInsetsDirectional.only(end: trailingGap ? 3.5 : 0),
      child: Container(
        width: 4,
        height: 4,
        transformAlignment: Alignment.center,
        transform: Matrix4.diagonal3Values(
          0.85 + 0.2 * progress,
          0.85 + 0.2 * progress,
          1,
        ),
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: color.withValues(alpha: 0.28 + 0.72 * progress),
        ),
      ),
    );
  }

  /// Follows the reference keyframes: rise to full by 40%, fall back by 80%.
  static double _thinkingDotProgress(double value) {
    if (value <= 0.4) return Curves.easeInOut.transform(value / 0.4);
    if (value <= 0.8) {
      return 1 - Curves.easeInOut.transform((value - 0.4) / 0.4);
    }
    return 0;
  }
}

/// One transcript unit that sharpens from a blur once its particles land.
class _VoiceToken extends StatefulWidget {
  const _VoiceToken({super.key, required this.text, required this.style});

  final String text;
  final TextStyle style;

  @override
  State<_VoiceToken> createState() => _VoiceTokenState();
}

class _VoiceTokenState extends State<_VoiceToken>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: ChatVoiceWave.tokenRevealDelay + ChatVoiceWave.tokenRevealDuration,
  );
  late final CurvedAnimation _reveal = CurvedAnimation(
    parent: _controller,
    curve: Interval(
      ChatVoiceWave.tokenRevealDelay.inMicroseconds /
          _controller.duration!.inMicroseconds,
      1,
      curve: const Cubic(0.22, 1, 0.36, 1),
    ),
  );

  @override
  void initState() {
    super.initState();
    _controller.forward();
  }

  @override
  void dispose() {
    _reveal.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _reveal,
      child: Text(widget.text, style: widget.style),
      builder: (context, child) {
        final progress = _reveal.value;
        if (progress >= 1) return child!;
        return Transform.translate(
          offset: Offset(0, 3 * (1 - progress)),
          child: ImageFiltered(
            imageFilter: ui.ImageFilter.blur(
              sigmaX: 7 * (1 - progress),
              sigmaY: 7 * (1 - progress),
            ),
            child: Opacity(opacity: progress, child: child),
          ),
        );
      },
    );
  }
}

/// Circular action key that shrinks slightly while pressed.
class _VoiceKey extends StatefulWidget {
  const _VoiceKey({
    required this.size,
    required this.background,
    required this.onTap,
    required this.tooltip,
    required this.child,
  });

  final double size;
  final Color background;
  final VoidCallback? onTap;
  final String tooltip;
  final Widget child;

  @override
  State<_VoiceKey> createState() => _VoiceKeyState();
}

class _VoiceKeyState extends State<_VoiceKey> {
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    final key = AnimatedScale(
      scale: _pressed ? 0.92 : 1,
      duration: const Duration(milliseconds: 120),
      child: Listener(
        onPointerDown: (_) => setState(() => _pressed = true),
        onPointerUp: (_) => setState(() => _pressed = false),
        onPointerCancel: (_) => setState(() => _pressed = false),
        child: Material(
          color: widget.background,
          shape: const CircleBorder(),
          child: InkWell(
            customBorder: const CircleBorder(),
            onTap: widget.onTap,
            child: SizedBox(
              width: widget.size,
              height: widget.size,
              child: Center(child: widget.child),
            ),
          ),
        ),
      ),
    );
    return widget.tooltip.isEmpty
        ? key
        : Tooltip(message: widget.tooltip, child: key);
  }
}

/// Draws the bare exclamation mark of the error key.
class _ExclamationMarkPainter extends CustomPainter {
  const _ExclamationMarkPainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.scale(size.width / 24, size.height / 24);
    final paint = Paint()
      ..color = color
      ..strokeWidth = 2.4
      ..strokeCap = StrokeCap.round;
    canvas.drawLine(const Offset(12, 6), const Offset(12, 14), paint);
    canvas.drawCircle(const Offset(12, 18), 1, Paint()..color = color);
  }

  @override
  bool shouldRepaint(_ExclamationMarkPainter oldDelegate) =>
      oldDelegate.color != color;
}
