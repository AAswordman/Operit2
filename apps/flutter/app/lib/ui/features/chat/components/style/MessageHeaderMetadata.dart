// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../common/ChatMessageIdentity.dart';
import '../../viewmodel/ChatViewModel.dart';

/// Returns the primary header title for a cursor-style AI message.
String cursorAiPrimaryTitle(
  ChatUiMessage message,
  ThemePreferenceSnapshot snapshot, {
  AppLocalizations? l10n,
}) {
  if (message.roleName.isEmpty ||
      readChatMessageIdentity(message)?.usesProviderLogo == true) {
    return message.modelName;
  }
  if (snapshot.showRoleName && message.roleName.isNotEmpty) {
    return message.roleName;
  }
  return l10n?.chatMessageResponse ?? 'Response';
}

/// Returns the participant name or the actual model identity for bubble messages.
String bubbleAiPrimaryTitle(
  ChatUiMessage message,
  ThemePreferenceSnapshot snapshot,
) {
  if (message.roleName.isEmpty ||
      readChatMessageIdentity(message)?.usesProviderLogo == true) {
    return message.modelName;
  }
  return snapshot.showRoleName ? message.roleName : '';
}

/// Returns the compact model and provider label when enabled in preferences.
String formatModelProviderLabel(
  ChatUiMessage message,
  ThemePreferenceSnapshot snapshot,
) {
  final modelIsPrimary =
      message.roleName.isEmpty ||
      readChatMessageIdentity(message)?.usesProviderLogo == true;
  final showModel =
      !modelIsPrimary && snapshot.showModelName && message.modelName.isNotEmpty;
  final showProvider =
      snapshot.showModelProvider && message.provider.isNotEmpty;
  if (showModel && showProvider) {
    return '${message.modelName} · ${message.provider}';
  }
  if (showModel) {
    return message.modelName;
  }
  if (showProvider) {
    return message.provider;
  }
  return '';
}

/// Formats token, cache, speed, timing, and timestamp metrics into a compact
/// summary that can wrap when displayed.
String formatMessageStatsText(
  ChatUiMessage message,
  ThemePreferenceSnapshot snapshot, {
  AppLocalizations? l10n,
  DateTime? now,
}) {
  final parts = <String>[];
  if (snapshot.showMessageTokenStats) {
    final tokenText = formatCompactTokenStats(message, l10n: l10n);
    if (tokenText.isNotEmpty) {
      parts.add(tokenText);
    }
  }
  if (snapshot.showMessageTimingStats) {
    final timingText = formatCompactTimingStats(message);
    if (timingText.isNotEmpty) {
      parts.add(timingText);
    }
  }
  if (snapshot.showMessageTimestamp) {
    final timestampText = formatSmartMessageTimestamp(message, now: now);
    if (timestampText.isNotEmpty) {
      parts.add(timestampText);
    }
  }
  return parts.join(' · ');
}

/// Formats input, output tokens, cached input (with hit rate), and token generation speed.
String formatCompactTokenStats(
  ChatUiMessage message, {
  AppLocalizations? l10n,
}) {
  final hasTokens =
      message.inputTokens > 0 ||
      message.outputTokens > 0 ||
      message.cachedInputTokens > 0;
  if (!hasTokens && message.completedAt <= 0) {
    return '';
  }
  final inputText = formatCompactTokenCount(message.inputTokens);
  final cachedCount = formatCompactTokenCount(message.cachedInputTokens);
  final cacheRate = formatCacheHitRate(message);
  final cachedSummary = '$cachedCount $cacheRate';
  final cachedLabel =
      l10n?.chatMessageCacheShort(cachedSummary) ?? 'cache $cachedSummary';
  final outputText = formatCompactTokenCount(message.outputTokens);
  final speedText = formatTokenSpeed(message);
  if (speedText.isEmpty) {
    return '↑$inputText ↓$outputText ($cachedLabel)';
  }
  return '↑$inputText ↓$outputText ($cachedLabel) $speedText';
}

/// Formats the cache hit rate percentage (e.g., "70%", "0%").
String formatCacheHitRate(ChatUiMessage message) {
  if (message.inputTokens <= 0 || message.cachedInputTokens <= 0) {
    return '0%';
  }
  final percent = ((message.cachedInputTokens * 100.0) / message.inputTokens)
      .round()
      .clamp(0, 100);
  return '$percent%';
}

/// Formats a token count using k/M suffixes for compact display.
String formatCompactTokenCount(int count) {
  if (count < 1000) {
    return count.toString();
  }
  if (count < 100000) {
    final value = (count / 1000).toStringAsFixed(1);
    return value.endsWith('.0')
        ? '${value.substring(0, value.length - 2)}k'
        : '${value}k';
  }
  if (count < 1000000) {
    return '${(count / 1000).round()}k';
  }
  final millions = (count / 1000000).toStringAsFixed(1);
  return millions.endsWith('.0')
      ? '${millions.substring(0, millions.length - 2)}M'
      : '${millions}M';
}

/// Formats wait and output durations into a compact timing segment.
String formatCompactTimingStats(ChatUiMessage message) {
  final hasTiming = message.waitDurationMs > 0 || message.outputDurationMs > 0;
  if (!hasTiming && message.completedAt <= 0) {
    return '';
  }
  final waitText = formatDurationSeconds(message.waitDurationMs);
  final outputText = formatDurationSeconds(message.outputDurationMs);
  return '◷ $waitText+$outputText';
}

/// Formats token output speed in tokens per second (e.g., "25.4 t/s", "120 t/s").
String formatTokenSpeed(ChatUiMessage message) {
  final hasData =
      message.outputTokens > 0 ||
      message.inputTokens > 0 ||
      message.waitDurationMs > 0 ||
      message.outputDurationMs > 0 ||
      message.completedAt > 0;
  if (!hasData) {
    return '';
  }
  if (message.outputTokens <= 0 || message.outputDurationMs <= 0) {
    return '0 t/s';
  }
  final speed = (message.outputTokens * 1000.0) / message.outputDurationMs;
  if (!speed.isFinite || speed <= 0) {
    return '0 t/s';
  }
  if (speed >= 100) {
    return '${speed.round()} t/s';
  }
  final formatted = speed.toStringAsFixed(1);
  return formatted.endsWith('.0')
      ? '${formatted.substring(0, formatted.length - 2)} t/s'
      : '$formatted t/s';
}

/// Formats milliseconds into compact seconds or minutes-seconds text.
String formatDurationSeconds(int durationMs) {
  if (durationMs >= 60000) {
    final minutes = durationMs ~/ 60000;
    final seconds = ((durationMs % 60000) / 1000).round();
    return '${minutes}m${seconds.toString().padLeft(2, '0')}s';
  }
  return '${(durationMs / 1000).toStringAsFixed(1)}s';
}

/// Formats the message timestamp relative to the current date to avoid truncation.
String formatSmartMessageTimestamp(ChatUiMessage message, {DateTime? now}) {
  final rawTimestamp = message.completedAt > 0
      ? message.completedAt
      : message.timestamp;
  if (rawTimestamp <= 0) {
    return '';
  }
  final dateTime = DateTime.fromMillisecondsSinceEpoch(rawTimestamp);
  final current = now ?? DateTime.now();
  String twoDigits(int value) => value.toString().padLeft(2, '0');
  final timePart = '${twoDigits(dateTime.hour)}:${twoDigits(dateTime.minute)}';
  final isSameDay =
      dateTime.year == current.year &&
      dateTime.month == current.month &&
      dateTime.day == current.day;
  if (isSameDay) {
    return timePart;
  }
  final monthDayPart =
      '${twoDigits(dateTime.month)}-${twoDigits(dateTime.day)} $timePart';
  if (dateTime.year == current.year) {
    return monthDayPart;
  }
  return '${dateTime.year}-$monthDayPart';
}

/// Builds a multi-line tooltip with full token (with cache, cache rate, and speed), timing, and timestamp details.
String formatMessageMetadataTooltip(
  ChatUiMessage message,
  ThemePreferenceSnapshot snapshot, {
  AppLocalizations? l10n,
}) {
  final lines = <String>[];
  final identityParts = <String>[
    if (snapshot.showRoleName && message.roleName.isNotEmpty) message.roleName,
    if (snapshot.showModelName && message.modelName.isNotEmpty)
      message.modelName,
    if (snapshot.showModelProvider && message.provider.isNotEmpty)
      message.provider,
  ];
  if (identityParts.isNotEmpty) {
    lines.add(identityParts.join(' · '));
  }
  if (snapshot.showMessageTokenStats &&
      (message.inputTokens > 0 ||
          message.outputTokens > 0 ||
          message.cachedInputTokens > 0 ||
          message.completedAt > 0)) {
    final inputStr = message.inputTokens.toString();
    final cacheRate = formatCacheHitRate(message);
    final cachedStr = '${message.cachedInputTokens} ($cacheRate)';
    final outputStr = message.outputTokens.toString();
    final speedText = formatTokenSpeed(message);
    final baseTokenLine =
        l10n?.chatMessageTokensCachedTooltip(inputStr, cachedStr, outputStr) ??
        'Tokens: ↑$inputStr (cached: $cachedStr) · ↓$outputStr';
    lines.add(
      speedText.isNotEmpty ? '$baseTokenLine · $speedText' : baseTokenLine,
    );
  }
  if (snapshot.showMessageTimingStats &&
      (message.waitDurationMs > 0 ||
          message.outputDurationMs > 0 ||
          message.completedAt > 0)) {
    final waitSec = '${(message.waitDurationMs / 1000).toStringAsFixed(2)}s';
    final outSec = '${(message.outputDurationMs / 1000).toStringAsFixed(2)}s';
    lines.add(
      l10n?.chatMessageTimingTooltip(waitSec, outSec) ??
          'Timing: $waitSec wait · $outSec output',
    );
  }
  if (snapshot.showMessageTimestamp) {
    final rawTimestamp = message.completedAt > 0
        ? message.completedAt
        : message.timestamp;
    if (rawTimestamp > 0) {
      final dateTime = DateTime.fromMillisecondsSinceEpoch(rawTimestamp);
      String twoDigits(int value) => value.toString().padLeft(2, '0');
      final timeStr =
          '${dateTime.year}-${twoDigits(dateTime.month)}-${twoDigits(dateTime.day)} '
          '${twoDigits(dateTime.hour)}:${twoDigits(dateTime.minute)}:${twoDigits(dateTime.second)}';
      lines.add(l10n?.chatMessageTimeTooltip(timeStr) ?? 'Time: $timeStr');
    }
  }
  return lines.join('\n');
}

/// Positions a floating child near a target cursor offset while keeping it within screen bounds.
class CursorPopupLayoutDelegate extends SingleChildLayoutDelegate {
  const CursorPopupLayoutDelegate({
    required this.target,
    this.cursorOffset = const Offset(12, 16),
    this.margin = 12,
  });

  final Offset target;
  final Offset cursorOffset;
  final double margin;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) {
    return constraints.loosen();
  }

  @override
  Offset getPositionForChild(Size size, Size childSize) {
    double dx = target.dx + cursorOffset.dx;
    double dy = target.dy + cursorOffset.dy;

    if (dx + childSize.width + margin > size.width) {
      dx = target.dx - childSize.width - 8;
    }
    if (dy + childSize.height + margin > size.height) {
      dy = target.dy - childSize.height - 8;
    }

    final maxDx = (size.width - childSize.width - margin).clamp(
      margin,
      double.infinity,
    );
    final maxDy = (size.height - childSize.height - margin).clamp(
      margin,
      double.infinity,
    );

    return Offset(
      dx.clamp(margin, maxDx).toDouble(),
      dy.clamp(margin, maxDy).toDouble(),
    );
  }

  @override
  bool shouldRelayout(covariant CursorPopupLayoutDelegate oldDelegate) {
    return oldDelegate.target != target ||
        oldDelegate.cursorOffset != cursorOffset ||
        oldDelegate.margin != margin;
  }
}

/// Wraps a dialog widget so it appears at [anchorGlobalPosition] instead of the screen center.
class CursorAnchoredDialog extends StatelessWidget {
  const CursorAnchoredDialog({
    super.key,
    required this.anchorGlobalPosition,
    required this.child,
  });

  final Offset? anchorGlobalPosition;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (anchorGlobalPosition == null) {
      return child;
    }
    return LayoutBuilder(
      builder: (context, _) {
        final box = context.findRenderObject() as RenderBox?;
        final localTarget = box != null && box.hasSize
            ? box.globalToLocal(anchorGlobalPosition!)
            : anchorGlobalPosition!;
        return CustomSingleChildLayout(
          delegate: CursorPopupLayoutDelegate(
            target: localTarget,
            cursorOffset: const Offset(8, 8),
          ),
          child: child,
        );
      },
    );
  }
}

/// Displays a tooltip popup anchored to the current mouse cursor position
/// and dims [child] when idle until hovered or touched.
class MouseFollowingTooltip extends StatefulWidget {
  const MouseFollowingTooltip({
    super.key,
    required this.message,
    required this.child,
    this.waitDuration = const Duration(milliseconds: 250),
    this.idleOpacity = 0.38,
    this.activeOpacity = 1.0,
    this.externalActive = false,
  });

  final String message;
  final Widget child;
  final Duration waitDuration;
  final double idleOpacity;
  final double activeOpacity;
  final bool externalActive;

  @override
  State<MouseFollowingTooltip> createState() => _MouseFollowingTooltipState();
}

class _MouseFollowingTooltipState extends State<MouseFollowingTooltip> {
  static const Duration _touchActiveHoldDuration = Duration(milliseconds: 1500);

  OverlayEntry? _overlayEntry;
  Timer? _showTimer;
  Timer? _touchFadeTimer;
  Offset? _pointerGlobalPosition;
  bool _isHovered = false;
  bool _isTouched = false;

  bool get _isActive => _isHovered || _isTouched || widget.externalActive;

  void _handleEnter(PointerEnterEvent event) {
    _pointerGlobalPosition = event.position;
    if (!_isHovered) {
      setState(() {
        _isHovered = true;
      });
    }
    _scheduleShow();
  }

  void _handleHover(PointerHoverEvent event) {
    _pointerGlobalPosition = event.position;
    if (!_isHovered) {
      setState(() {
        _isHovered = true;
      });
    }
    if (_overlayEntry != null) {
      _overlayEntry!.markNeedsBuild();
    } else {
      _scheduleShow();
    }
  }

  void _handleExit(PointerExitEvent event) {
    if (_isHovered) {
      setState(() {
        _isHovered = false;
      });
    }
    _hideTooltip();
  }

  void _handlePointerDown(PointerDownEvent event) {
    _pointerGlobalPosition = event.position;
    _touchFadeTimer?.cancel();
    if (!_isTouched) {
      setState(() {
        _isTouched = true;
      });
    }
  }

  void _handlePointerEnd() {
    _touchFadeTimer?.cancel();
    _touchFadeTimer = Timer(_touchActiveHoldDuration, () {
      if (!mounted) {
        return;
      }
      setState(() {
        _isTouched = false;
      });
    });
  }

  void _scheduleShow() {
    if (widget.message.isEmpty) {
      return;
    }
    if (_showTimer?.isActive ?? false) {
      return;
    }
    _showTimer = Timer(widget.waitDuration, _showTooltip);
  }

  void _showTooltip() {
    if (!mounted || _pointerGlobalPosition == null || widget.message.isEmpty) {
      return;
    }
    if (_overlayEntry != null) {
      _overlayEntry!.markNeedsBuild();
      return;
    }
    final overlay = Overlay.maybeOf(context);
    if (overlay == null) {
      return;
    }
    _overlayEntry = OverlayEntry(
      builder: (overlayContext) {
        final pos = _pointerGlobalPosition;
        if (pos == null) {
          return const SizedBox.shrink();
        }
        final overlayBox = overlay.context.findRenderObject() as RenderBox?;
        final localPos = overlayBox != null && overlayBox.hasSize
            ? overlayBox.globalToLocal(pos)
            : pos;
        final theme = Theme.of(context);
        final colorScheme = theme.colorScheme;
        return IgnorePointer(
          child: CustomSingleChildLayout(
            delegate: CursorPopupLayoutDelegate(target: localPos),
            child: Material(
              color: Colors.transparent,
              child: Container(
                constraints: const BoxConstraints(maxWidth: 360),
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 6,
                ),
                decoration: BoxDecoration(
                  color: colorScheme.inverseSurface.withValues(alpha: 0.94),
                  borderRadius: BorderRadius.circular(6),
                  boxShadow: <BoxShadow>[
                    BoxShadow(
                      color: Colors.black.withValues(alpha: 0.18),
                      blurRadius: 8,
                      offset: const Offset(0, 2),
                    ),
                  ],
                ),
                child: Text(
                  widget.message,
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colorScheme.onInverseSurface,
                    height: 1.35,
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
    overlay.insert(_overlayEntry!);
  }

  void _hideTooltip() {
    _showTimer?.cancel();
    _showTimer = null;
    _overlayEntry?.remove();
    _overlayEntry = null;
  }

  @override
  void didUpdateWidget(covariant MouseFollowingTooltip oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.message != oldWidget.message) {
      if (widget.message.isEmpty) {
        _hideTooltip();
      } else {
        _overlayEntry?.markNeedsBuild();
      }
    }
  }

  @override
  void deactivate() {
    _touchFadeTimer?.cancel();
    _touchFadeTimer = null;
    _hideTooltip();
    super.deactivate();
  }

  @override
  void dispose() {
    _touchFadeTimer?.cancel();
    _touchFadeTimer = null;
    _hideTooltip();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: _handleEnter,
      onHover: _handleHover,
      onExit: _handleExit,
      child: Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: _handlePointerDown,
        onPointerUp: (_) => _handlePointerEnd(),
        onPointerCancel: (_) => _handlePointerEnd(),
        child: GestureDetector(
          behavior: HitTestBehavior.translucent,
          onLongPressStart: widget.message.isEmpty
              ? null
              : (details) {
                  _pointerGlobalPosition = details.globalPosition;
                  _showTooltip();
                },
          onLongPressEnd: widget.message.isEmpty ? null : (_) => _hideTooltip(),
          onLongPressCancel: widget.message.isEmpty ? null : _hideTooltip,
          child: AnimatedOpacity(
            opacity: _isActive ? widget.activeOpacity : widget.idleOpacity,
            duration: const Duration(milliseconds: 180),
            curve: Curves.easeOutCubic,
            child: widget.child,
          ),
        ),
      ),
    );
  }
}
