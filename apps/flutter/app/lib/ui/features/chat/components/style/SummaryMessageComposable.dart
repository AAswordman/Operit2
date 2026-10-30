// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../common/interactions/MessagePressShield.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../viewmodel/ChatViewModel.dart';
import 'MessageHeaderMetadata.dart';

enum _SummaryDisplayPhase { running, streaming, settled }

/// Renders a summary with seamless water-bottle inline expansion and streaming lifecycle.
class SummaryMessageComposable extends StatefulWidget {
  const SummaryMessageComposable({
    super.key,
    required this.message,
    this.onDelete,
    this.onEdit,
    this.enableDialog = true,
    this.splitMarkdownContent,
  });

  final ChatUiMessage message;
  final VoidCallback? onDelete;
  final ValueChanged<ChatUiMessage>? onEdit;
  final bool enableDialog;
  final MarkdownContentSplitter? splitMarkdownContent;

  @override
  State<SummaryMessageComposable> createState() =>
      _SummaryMessageComposableState();
}

class _SummaryMessageComposableState extends State<SummaryMessageComposable>
    with SingleTickerProviderStateMixin {
  static final Set<int> _animatedSummaryTimestamps = <int>{};

  late final AnimationController _expandController;
  late final Animation<double> _expandAnimation;

  _SummaryDisplayPhase _phase = _SummaryDisplayPhase.settled;
  int _streamedCharacterCount = 0;
  int _elapsedSeconds = 0;
  Timer? _runTimer;
  Timer? _streamTimer;
  Timer? _autoCollapseTimer;
  final ScrollController _scrollController = ScrollController();
  bool _copied = false;

  @override
  void initState() {
    super.initState();
    _expandController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 200),
      reverseDuration: const Duration(milliseconds: 160),
    );
    _expandAnimation = CurvedAnimation(
      parent: _expandController,
      curve: Curves.easeOutCubic,
      reverseCurve: const Cubic(0.33, 0, 0.67, 0),
    );

    final isTextEmpty = widget.message.displayText.trim().isEmpty;
    final isFreshMessage =
        (DateTime.now().millisecondsSinceEpoch - widget.message.timestamp)
                .abs() <
            20000 &&
        !_animatedSummaryTimestamps.contains(widget.message.timestamp);

    if (isTextEmpty) {
      _startRunningPhase();
    } else if (isFreshMessage) {
      _animatedSummaryTimestamps.add(widget.message.timestamp);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          _startStreamingPhase();
        }
      });
    } else {
      _phase = _SummaryDisplayPhase.settled;
    }
  }

  @override
  void didUpdateWidget(covariant SummaryMessageComposable oldWidget) {
    super.didUpdateWidget(oldWidget);
    final hadEmptyText = oldWidget.message.displayText.trim().isEmpty;
    final hasNewText = widget.message.displayText.trim().isNotEmpty;

    if (hadEmptyText && hasNewText) {
      _runTimer?.cancel();
      _runTimer = null;
      if (!_animatedSummaryTimestamps.contains(widget.message.timestamp)) {
        _animatedSummaryTimestamps.add(widget.message.timestamp);
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) {
            _startStreamingPhase();
          }
        });
      }
    } else if (oldWidget.message.timestamp != widget.message.timestamp) {
      _cancelAllTimers();
      _phase = _SummaryDisplayPhase.settled;
      _expandController.value = 0.0;
    }
  }

  @override
  void dispose() {
    _cancelAllTimers();
    _expandController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  void _cancelAllTimers() {
    _runTimer?.cancel();
    _runTimer = null;
    _streamTimer?.cancel();
    _streamTimer = null;
    _autoCollapseTimer?.cancel();
    _autoCollapseTimer = null;
  }

  void _startRunningPhase() {
    _cancelAllTimers();
    setState(() {
      _phase = _SummaryDisplayPhase.running;
      _elapsedSeconds = 1;
    });
    _runTimer = Timer.periodic(const Duration(seconds: 1), (timer) {
      if (!mounted) {
        timer.cancel();
        return;
      }
      setState(() {
        _elapsedSeconds++;
      });
    });
  }

  void _startStreamingPhase() {
    _cancelAllTimers();
    setState(() {
      _phase = _SummaryDisplayPhase.streaming;
      _streamedCharacterCount = 0;
    });

    _expandController.forward().then((_) {
      if (!mounted) return;
      _beginTypewriterStream();
    });
  }

  void _beginTypewriterStream() {
    final fullText = widget.message.displayText;
    if (fullText.isEmpty) {
      _finishStreamingAndAutoCollapse();
      return;
    }

    const streamInterval = Duration(milliseconds: 20);
    const charactersPerTick = 5;

    _streamTimer = Timer.periodic(streamInterval, (timer) {
      if (!mounted) {
        timer.cancel();
        return;
      }
      final nextCount = math.min(
        _streamedCharacterCount + charactersPerTick,
        fullText.length,
      );
      setState(() {
        _streamedCharacterCount = nextCount;
      });

      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }

      if (_streamedCharacterCount >= fullText.length) {
        timer.cancel();
        _streamTimer = null;
        _finishStreamingAndAutoCollapse();
      }
    });
  }

  void _finishStreamingAndAutoCollapse() {
    _streamTimer?.cancel();
    _streamTimer = null;
    setState(() {
      _streamedCharacterCount = widget.message.displayText.length;
    });

    _autoCollapseTimer = Timer(const Duration(milliseconds: 700), () {
      if (!mounted) return;
      _expandController.reverse().then((_) {
        if (!mounted) return;
        setState(() {
          _phase = _SummaryDisplayPhase.settled;
        });
      });
    });
  }

  void _handleManualToggle() {
    if (_phase == _SummaryDisplayPhase.running) return;

    if (_expandController.isAnimating) return;

    if (_expandController.value > 0.5) {
      _expandController.reverse();
    } else {
      setState(() {
        _phase = _SummaryDisplayPhase.settled;
        _streamedCharacterCount = widget.message.displayText.length;
      });
      _expandController.forward();
    }
  }

  Future<void> _handleCopy() async {
    final text = widget.message.displayText;
    if (text.isEmpty) return;
    await Clipboard.setData(ClipboardData(text: text));
    if (!mounted) return;
    setState(() {
      _copied = true;
    });
    Future<void>.delayed(const Duration(seconds: 2), () {
      if (mounted) {
        setState(() {
          _copied = false;
        });
      }
    });
  }

  void _handleUserCollapse() {
    if (_expandController.value > 0.0) {
      _expandController.reverse();
    }
  }

  String _formatSettledCapsuleTitle(
    BuildContext context,
    AppLocalizations l10n,
  ) {
    final hasTiming = widget.message.waitDurationMs > 0 || _elapsedSeconds > 0;
    final durationSeconds = widget.message.waitDurationMs > 0
        ? (widget.message.waitDurationMs / 1000).round()
        : _elapsedSeconds;

    final hasTokens =
        widget.message.inputTokens > 0 || widget.message.outputTokens > 0;
    final inputStr = formatCompactTokenCount(widget.message.inputTokens);
    final outputStr = formatCompactTokenCount(widget.message.outputTokens);

    if (hasTiming && hasTokens) {
      return '${l10n.historyDialogSummary} · 用时 ${durationSeconds}秒 输入$inputStr 输出$outputStr';
    } else if (hasTiming) {
      return '${l10n.historyDialogSummary} · 用时 ${durationSeconds}秒';
    } else if (hasTokens) {
      return '${l10n.historyDialogSummary} · 输入$inputStr 输出$outputStr';
    }
    return l10n.historyDialogSummary;
  }

  _BottleColorScheme _resolveBottleColors(ThemeData theme) {
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final canvasBg = theme.scaffoldBackgroundColor;
    final canvasLuminance = canvasBg.computeLuminance();
    final isDarkSurface = canvasLuminance < 0.25 || isDark;

    final Color cardBackground;
    final Color cardBorder;
    final Color dividerLine;
    final Color shadowColor;
    final double elevation;

    if (isDarkSurface) {
      final baseElevated = Color.alphaBlend(
        colorScheme.surfaceTint.withValues(alpha: 0.06),
        colorScheme.surfaceContainerHigh,
      );
      final containerLum = baseElevated.computeLuminance();
      // Ensure safe luminance distance so it never blends into dark canvas
      if ((containerLum - canvasLuminance).abs() < 0.05) {
        cardBackground = Color.alphaBlend(
          Colors.white.withValues(alpha: 0.12),
          baseElevated,
        );
      } else {
        cardBackground = baseElevated;
      }

      cardBorder = colorScheme.outlineVariant.withValues(alpha: 0.65);
      dividerLine = colorScheme.outline.withValues(alpha: 0.35);
      shadowColor = Colors.black.withValues(alpha: 0.70);
      elevation = 4.0;
    } else {
      cardBackground = colorScheme.surfaceContainerLowest;
      cardBorder = colorScheme.outline.withValues(alpha: 0.30);
      dividerLine = colorScheme.outline.withValues(alpha: 0.25);
      shadowColor = colorScheme.shadow.withValues(alpha: 0.08);
      elevation = 2.0;
    }

    return _BottleColorScheme(
      cardBackground: cardBackground,
      cardBorder: cardBorder,
      dividerLine: dividerLine,
      shadowColor: shadowColor,
      elevation: elevation,
      isDarkSurface: isDarkSurface,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final bottleColors = _resolveBottleColors(theme);

    return AnimatedBuilder(
      animation: _expandAnimation,
      builder: (context, _) => _buildWaterBottleCard(
        context,
        theme,
        l10n,
        bottleColors,
        _expandAnimation.value,
      ),
    );
  }

  /// Morphs one opaque vessel between the status capsule and the full card.
  Widget _buildWaterBottleCard(
    BuildContext context,
    ThemeData theme,
    AppLocalizations l10n,
    _BottleColorScheme bottleColors,
    double expandValue,
  ) {
    final colorScheme = theme.colorScheme;
    final isRunning = _phase == _SummaryDisplayPhase.running;
    final capsuleTitle = isRunning
        ? '${l10n.historyDialogSummary} · 已运行 $_elapsedSeconds秒'
        : _formatSettledCapsuleTitle(context, l10n);
    final details = capsuleTitle.substring(l10n.historyDialogSummary.length);
    final collapsedBackground = isRunning
        ? Color.alphaBlend(
            colorScheme.primaryContainer.withValues(alpha: 0.25),
            theme.scaffoldBackgroundColor,
          )
        : bottleColors.isDarkSurface
        ? bottleColors.cardBackground
        : Color.alphaBlend(
            colorScheme.surfaceContainerHighest.withValues(alpha: 0.55),
            theme.scaffoldBackgroundColor,
          );
    final titleStyle = (theme.textTheme.labelSmall ?? const TextStyle())
        .copyWith(
          color: isRunning ? colorScheme.primary : colorScheme.onSurfaceVariant,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.2,
        );
    final titlePainter = TextPainter(
      text: TextSpan(text: capsuleTitle, style: titleStyle),
      textDirection: Directionality.of(context),
      textScaler: MediaQuery.textScalerOf(context),
    )..layout();
    final capsuleWidth = titlePainter.width + 20 + (isRunning ? 10 : 18);
    final capsuleHeight = math.max(titlePainter.height, 14.0) + 7;
    titlePainter.text = TextSpan(
      text: l10n.historyDialogSummary,
      style: titleStyle,
    );
    titlePainter.layout();
    final expandedNeckWidth = math.max(120.0, titlePainter.width + 38);
    titlePainter.dispose();

    double morph(double collapsed, double expanded) =>
        collapsed + (expanded - collapsed) * expandValue;

    final headerHeight = morph(capsuleHeight, 28);
    final neckHeight = morph(capsuleHeight / 2, 28);
    final neckRadius = morph(capsuleHeight / 2, 12);
    final shoulderRadius = 10 * expandValue;
    final bodyRadius = morph(capsuleHeight / 2, 10);
    final bodyTopRadius = 10 * expandValue;
    const maxContentHeight = 340.0;

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final maxWidth = math.max(0.0, constraints.maxWidth - 20);
          final collapsedWidth = math.min(capsuleWidth, maxWidth);
          final vesselWidth = morph(collapsedWidth, maxWidth);
          final neckWidth = morph(
            collapsedWidth,
            math.min(expandedNeckWidth, maxWidth),
          );
          // Positioned dividers cannot size a Stack. Reserve the full row
          // even when the vessel itself contracts under loose chat constraints.
          return SizedBox(
            width: constraints.maxWidth,
            child: Stack(
              alignment: Alignment.topCenter,
              clipBehavior: Clip.none,
              children: <Widget>[
                Positioned(
                  left: 0,
                  right: 0,
                  top: neckHeight,
                  child: Divider(
                    color: bottleColors.dividerLine,
                    thickness: 0.8,
                    height: 1,
                  ),
                ),
                SizedBox(
                  key: const ValueKey('summary-vessel'),
                  width: vesselWidth,
                  child: CustomPaint(
                    painter: _BottleCardPainter(
                      backgroundColor: Color.lerp(
                        collapsedBackground,
                        bottleColors.cardBackground,
                        expandValue,
                      )!,
                      borderColor: isRunning
                          ? colorScheme.primary
                          : bottleColors.cardBorder,
                      shadowColor: bottleColors.shadowColor,
                      elevation: bottleColors.elevation * expandValue,
                      neckWidth: neckWidth,
                      neckHeight: neckHeight,
                      neckRadius: neckRadius,
                      shoulderRadius: shoulderRadius,
                      bodyRadius: bodyRadius,
                      bodyTopRadius: bodyTopRadius,
                    ),
                    child: ClipPath(
                      clipper: _BottleCardClipper(
                        neckWidth: neckWidth,
                        neckHeight: neckHeight,
                        neckRadius: neckRadius,
                        shoulderRadius: shoulderRadius,
                        bodyRadius: bodyRadius,
                        bodyTopRadius: bodyTopRadius,
                      ),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: <Widget>[
                          Center(
                            child: SizedBox(
                              width: neckWidth,
                              height: headerHeight,
                              child: Material(
                                color: Colors.transparent,
                                child: InkWell(
                                  key: const ValueKey('summary-toggle'),
                                  onTap: isRunning ? null : _handleManualToggle,
                                  child: ClipRect(
                                    child: OverflowBox(
                                      minWidth: 0,
                                      maxWidth: double.infinity,
                                      child: Row(
                                        mainAxisSize: MainAxisSize.min,
                                        children: <Widget>[
                                          if (isRunning) ...<Widget>[
                                            Container(
                                              width: 5,
                                              height: 5,
                                              decoration: BoxDecoration(
                                                color: colorScheme.primary,
                                                shape: BoxShape.circle,
                                              ),
                                            ),
                                            const SizedBox(width: 5),
                                          ],
                                          Text(
                                            l10n.historyDialogSummary,
                                            style: titleStyle,
                                          ),
                                          ClipRect(
                                            child: Align(
                                              widthFactor: 1 - expandValue,
                                              child: Text(
                                                details,
                                                style: titleStyle,
                                              ),
                                            ),
                                          ),
                                          if (!isRunning) ...<Widget>[
                                            const SizedBox(width: 4),
                                            Transform.rotate(
                                              angle: math.pi * expandValue,
                                              child: Icon(
                                                Icons.keyboard_arrow_down,
                                                size: 14,
                                                color: colorScheme
                                                    .onSurfaceVariant,
                                              ),
                                            ),
                                          ],
                                        ],
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ),
                          SizedBox(
                            height: maxContentHeight * expandValue,
                            child: ClipRect(
                              child: OverflowBox(
                                alignment: Alignment.topCenter,
                                minWidth: maxWidth,
                                maxWidth: maxWidth,
                                minHeight: maxContentHeight,
                                maxHeight: maxContentHeight,
                                child: expandValue == 0
                                    ? const SizedBox.shrink()
                                    : IgnorePointer(
                                        ignoring: _expandController.isAnimating,
                                        child: SizedBox(
                                          height: maxContentHeight,
                                          child: _buildExpandedContent(
                                            context,
                                            theme,
                                            l10n,
                                            bottleColors,
                                          ),
                                        ),
                                      ),
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _buildExpandedContent(
    BuildContext context,
    ThemeData theme,
    AppLocalizations l10n,
    _BottleColorScheme bottleColors,
  ) {
    final colorScheme = theme.colorScheme;
    final baseTextStyle = theme.textTheme.bodyMedium ?? const TextStyle();
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        // Header Toolbar: "上下文检查点" on left, 4 standard tools on right
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4.5),
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerHighest.withValues(
              alpha: bottleColors.isDarkSurface ? 0.45 : 0.22,
            ),
            border: Border(
              bottom: BorderSide(
                color: bottleColors.cardBorder.withValues(
                  alpha: bottleColors.isDarkSurface ? 0.45 : 0.20,
                ),
                width: 0.8,
              ),
            ),
          ),
          child: Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  Localizations.localeOf(context).languageCode == 'zh'
                      ? '上下文检查点'
                      : 'Context checkpoint',
                  style: TextStyle(
                    fontSize: 11,
                    color: colorScheme.onSurfaceVariant.withValues(
                      alpha: bottleColors.isDarkSurface ? 0.88 : 0.70,
                    ),
                    fontWeight: FontWeight.w500,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              MessagePressShieldRegion(
                child: _SummaryActionButton(
                  icon: _copied
                      ? Icons.check_rounded
                      : Icons.content_copy_rounded,
                  iconColor: _copied
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant,
                  tooltip: _copied
                      ? (Localizations.localeOf(context).languageCode == 'zh'
                            ? '已复制'
                            : 'Copied')
                      : MaterialLocalizations.of(context).copyButtonLabel,
                  onPressed: _handleCopy,
                ),
              ),
              if (widget.onEdit != null) ...<Widget>[
                const SizedBox(width: 4),
                MessagePressShieldRegion(
                  child: _SummaryActionButton(
                    icon: Icons.edit_outlined,
                    tooltip: l10n.edit,
                    onPressed: () => widget.onEdit!(widget.message),
                  ),
                ),
              ],
              if (widget.onDelete != null) ...<Widget>[
                const SizedBox(width: 4),
                MessagePressShieldRegion(
                  child: _SummaryActionButton(
                    icon: Icons.delete_outline,
                    iconColor: colorScheme.error.withValues(alpha: 0.85),
                    tooltip: l10n.delete,
                    onPressed: () => _showDeleteConfirmation(context, l10n),
                  ),
                ),
              ],
              const SizedBox(width: 4),
              MessagePressShieldRegion(
                child: _SummaryActionButton(
                  icon: Icons.keyboard_arrow_up,
                  tooltip: MaterialLocalizations.of(context).closeButtonLabel,
                  onPressed: _handleUserCollapse,
                ),
              ),
            ],
          ),
        ),

        // Body Content: Streaming or Markdown
        Expanded(
          child: _buildCardBody(context, theme, baseTextStyle, colorScheme),
        ),
      ],
    );
  }

  Widget _buildCardBody(
    BuildContext context,
    ThemeData theme,
    TextStyle baseTextStyle,
    ColorScheme colorScheme,
  ) {
    if (_phase == _SummaryDisplayPhase.streaming) {
      final currentStreamText = widget.message.displayText.substring(
        0,
        math.min(_streamedCharacterCount, widget.message.displayText.length),
      );
      return Scrollbar(
        controller: _scrollController,
        thumbVisibility: true,
        child: SingleChildScrollView(
          controller: _scrollController,
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          child: SelectableText.rich(
            TextSpan(
              children: <InlineSpan>[
                TextSpan(text: currentStreamText),
                const WidgetSpan(
                  alignment: PlaceholderAlignment.middle,
                  child: Padding(
                    padding: EdgeInsets.only(left: 4),
                    child: StreamingCursor(),
                  ),
                ),
              ],
            ),
            style: baseTextStyle.copyWith(
              fontSize: 12.5,
              height: 1.5,
              color: colorScheme.onSurface.withValues(alpha: 0.9),
            ),
          ),
        ),
      );
    }

    return Scrollbar(
      controller: _scrollController,
      thumbVisibility: true,
      child: SingleChildScrollView(
        controller: _scrollController,
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        child: widget.splitMarkdownContent != null
            ? Theme(
                data: theme.copyWith(
                  textTheme: theme.textTheme.copyWith(
                    headlineMedium: baseTextStyle.copyWith(
                      fontSize: 14,
                      fontWeight: FontWeight.w700,
                    ),
                    headlineSmall: baseTextStyle.copyWith(
                      fontSize: 13.5,
                      fontWeight: FontWeight.w700,
                    ),
                    titleLarge: baseTextStyle.copyWith(
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                    ),
                    titleMedium: baseTextStyle.copyWith(
                      fontSize: 12.5,
                      fontWeight: FontWeight.w600,
                    ),
                    titleSmall: baseTextStyle.copyWith(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                    ),
                    bodyMedium: baseTextStyle.copyWith(
                      fontSize: 12.5,
                      height: 1.5,
                    ),
                    bodySmall: (theme.textTheme.bodySmall ?? baseTextStyle)
                        .copyWith(fontSize: 11.5, height: 1.4),
                  ),
                ),
                child: StreamMarkdownRenderer(
                  content: widget.message.displayText,
                  isStreaming: false,
                  textColor: colorScheme.onSurface.withValues(alpha: 0.9),
                  backgroundColor: Colors.transparent,
                  selectionRoot: false,
                  splitMarkdownContent: widget.splitMarkdownContent!,
                ),
              )
            : SelectableText(
                widget.message.displayText,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: colorScheme.onSurface.withValues(alpha: 0.9),
                  height: 1.5,
                  fontSize: 12.5,
                ),
              ),
      ),
    );
  }

  void _showDeleteConfirmation(BuildContext context, AppLocalizations l10n) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(l10n.confirmDeleteSummary),
        content: Text(l10n.confirmDeleteSummaryMessage),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(dialogContext).pop();
              widget.onDelete?.call();
            },
            child: Text(l10n.delete),
          ),
        ],
      ),
    );
  }
}

class _SummaryActionButton extends StatelessWidget {
  const _SummaryActionButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
    this.iconColor,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;
  final Color? iconColor;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Tooltip(
      message: tooltip,
      waitDuration: const Duration(milliseconds: 300),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onPressed,
          borderRadius: BorderRadius.circular(4),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Icon(
              icon,
              size: 15,
              color: iconColor ?? theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ),
    );
  }
}

/// Generates the water bottle path where the neck seamlessly connects to the body.
Path _buildBottlePath({
  required Size size,
  required double neckWidth,
  required double neckHeight,
  required double neckRadius,
  required double shoulderRadius,
  required double bodyRadius,
  required double bodyTopRadius,
}) {
  final path = Path();
  final neckLeft = (size.width - neckWidth) / 2;
  final neckRight = (size.width + neckWidth) / 2;

  final effectiveNeckR = math.min(neckRadius, neckWidth / 2);
  final effectiveShoulderR = math.min(
    shoulderRadius,
    math.min(neckLeft, neckHeight),
  );
  final effectiveBodyR = math.min(
    bodyRadius,
    math.min(size.width / 2, math.max(0.0, size.height - neckHeight)),
  );

  final effectiveBodyTopR = math.min(
    bodyTopRadius,
    math.min(size.width / 2, math.max(0.0, (size.height - neckHeight) / 2)),
  );

  // 1. Top of neck
  path.moveTo(neckLeft + effectiveNeckR, 0);
  path.lineTo(neckRight - effectiveNeckR, 0);

  // 2. Neck top-right corner
  path.arcToPoint(
    Offset(neckRight, effectiveNeckR),
    radius: Radius.circular(effectiveNeckR),
    clockwise: true,
  );

  // 3. Neck right edge down
  path.lineTo(neckRight, neckHeight - effectiveShoulderR);

  // 4. Concave right shoulder
  path.arcToPoint(
    Offset(neckRight + effectiveShoulderR, neckHeight),
    radius: Radius.circular(effectiveShoulderR),
    clockwise: false,
  );

  // 5. Right shoulder horizontal
  path.lineTo(size.width - effectiveBodyTopR, neckHeight);

  // 6. Body top-right corner
  path.arcToPoint(
    Offset(size.width, neckHeight + effectiveBodyTopR),
    radius: Radius.circular(effectiveBodyTopR),
    clockwise: true,
  );

  // 7. Body right edge down
  path.lineTo(size.width, size.height - effectiveBodyR);

  // 8. Body bottom-right corner
  path.arcToPoint(
    Offset(size.width - effectiveBodyR, size.height),
    radius: Radius.circular(effectiveBodyR),
    clockwise: true,
  );

  // 9. Bottom edge left
  path.lineTo(effectiveBodyR, size.height);

  // 10. Body bottom-left corner
  path.arcToPoint(
    Offset(0, size.height - effectiveBodyR),
    radius: Radius.circular(effectiveBodyR),
    clockwise: true,
  );

  // 11. Body left edge up
  path.lineTo(0, neckHeight + effectiveBodyTopR);

  // 12. Body top-left corner
  path.arcToPoint(
    Offset(effectiveBodyTopR, neckHeight),
    radius: Radius.circular(effectiveBodyTopR),
    clockwise: true,
  );

  // 13. Left shoulder horizontal
  path.lineTo(neckLeft - effectiveShoulderR, neckHeight);

  // 14. Concave left shoulder
  path.arcToPoint(
    Offset(neckLeft, neckHeight - effectiveShoulderR),
    radius: Radius.circular(effectiveShoulderR),
    clockwise: false,
  );

  // 15. Neck left edge up
  path.lineTo(neckLeft, effectiveNeckR);

  // 16. Neck top-left corner
  path.arcToPoint(
    Offset(neckLeft + effectiveNeckR, 0),
    radius: Radius.circular(effectiveNeckR),
    clockwise: true,
  );

  path.close();
  return path;
}

class _BottleColorScheme {
  const _BottleColorScheme({
    required this.cardBackground,
    required this.cardBorder,
    required this.dividerLine,
    required this.shadowColor,
    required this.elevation,
    required this.isDarkSurface,
  });

  final Color cardBackground;
  final Color cardBorder;
  final Color dividerLine;
  final Color shadowColor;
  final double elevation;
  final bool isDarkSurface;
}

class _BottleCardPainter extends CustomPainter {
  const _BottleCardPainter({
    required this.backgroundColor,
    required this.borderColor,
    required this.shadowColor,
    required this.elevation,
    required this.neckWidth,
    required this.neckHeight,
    required this.neckRadius,
    required this.shoulderRadius,
    required this.bodyRadius,
    required this.bodyTopRadius,
  });

  final Color backgroundColor;
  final Color borderColor;
  final Color shadowColor;
  final double elevation;
  final double neckWidth;
  final double neckHeight;
  final double neckRadius;
  final double shoulderRadius;
  final double bodyRadius;
  final double bodyTopRadius;

  @override
  void paint(Canvas canvas, Size size) {
    final path = _buildBottlePath(
      size: size,
      neckWidth: neckWidth,
      neckHeight: neckHeight,
      neckRadius: neckRadius,
      shoulderRadius: shoulderRadius,
      bodyRadius: bodyRadius,
      bodyTopRadius: bodyTopRadius,
    );

    if (elevation > 0) {
      canvas.drawShadow(path, shadowColor, elevation, false);
    }

    final fillPaint = Paint()
      ..color = backgroundColor
      ..style = PaintingStyle.fill;
    canvas.drawPath(path, fillPaint);

    final strokePaint = Paint()
      ..color = borderColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = 0.8;
    canvas.drawPath(path, strokePaint);
  }

  @override
  bool shouldRepaint(covariant _BottleCardPainter oldDelegate) {
    return oldDelegate.backgroundColor != backgroundColor ||
        oldDelegate.borderColor != borderColor ||
        oldDelegate.shadowColor != shadowColor ||
        oldDelegate.elevation != elevation ||
        oldDelegate.neckWidth != neckWidth ||
        oldDelegate.neckHeight != neckHeight ||
        oldDelegate.neckRadius != neckRadius ||
        oldDelegate.shoulderRadius != shoulderRadius ||
        oldDelegate.bodyRadius != bodyRadius ||
        oldDelegate.bodyTopRadius != bodyTopRadius;
  }
}

class _BottleCardClipper extends CustomClipper<Path> {
  const _BottleCardClipper({
    required this.neckWidth,
    required this.neckHeight,
    required this.neckRadius,
    required this.shoulderRadius,
    required this.bodyRadius,
    required this.bodyTopRadius,
  });

  final double neckWidth;
  final double neckHeight;
  final double neckRadius;
  final double shoulderRadius;
  final double bodyRadius;
  final double bodyTopRadius;

  @override
  Path getClip(Size size) {
    return _buildBottlePath(
      size: size,
      neckWidth: neckWidth,
      neckHeight: neckHeight,
      neckRadius: neckRadius,
      shoulderRadius: shoulderRadius,
      bodyRadius: bodyRadius,
      bodyTopRadius: bodyTopRadius,
    );
  }

  @override
  bool shouldReclip(covariant _BottleCardClipper oldClipper) {
    return oldClipper.neckWidth != neckWidth ||
        oldClipper.neckHeight != neckHeight ||
        oldClipper.neckRadius != neckRadius ||
        oldClipper.shoulderRadius != shoulderRadius ||
        oldClipper.bodyRadius != bodyRadius ||
        oldClipper.bodyTopRadius != bodyTopRadius;
  }
}
