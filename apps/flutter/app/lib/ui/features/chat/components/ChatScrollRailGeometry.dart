// ignore_for_file: file_names

import 'dart:math' as math;

/// Shares bounded rail geometry between painting, pointer selection and previews.
class ChatScrollRailGeometry {
  /// Creates a centered viewport onto evenly spaced message ticks.
  const ChatScrollRailGeometry({
    required this.entryCount,
    required this.viewportHeight,
  }) : assert(entryCount > 0),
       assert(viewportHeight > 0 && viewportHeight < double.infinity);

  static const double tickSpacing = 16;
  static const double interactionWidth = 88;
  static const double hitWidth = 48;
  static const double rightInset = 8;
  static const double maximumTickWidth = 18;
  // Reach full width before the pointer touches the longest tick's left edge.
  static const double fullExpansionDistance = maximumTickWidth + 4;
  static const double proximityDistance = 72;
  static const double waveRadius = 44;

  final int entryCount;
  final double viewportHeight;

  /// Preserves a separate tick for every message, even in long conversations.
  double get contentHeight => entryCount * tickSpacing;

  double get height =>
      math.min(contentHeight, math.max(tickSpacing, viewportHeight - 100));

  double get maxScrollOffset => math.max(0, contentHeight - height);

  /// Centers the rail independently of the number of messages in history.
  double get top => (viewportHeight - height) / 2;

  /// Provides at least a finger-sized target without enlarging the painted rail.
  double get hitHeight => math.min(viewportHeight, math.max(48.0, height + 16));

  /// Centers the touch target around the actual tick positions.
  double get hitTop => (viewportHeight - hitHeight) / 2;

  /// Keeps the wave visible in both short conversations and long histories.
  double get waveSpread => math.min(waveRadius, math.max(12.0, height / 3));

  /// Resolves the visible message after accounting for independent rail scrolling.
  int entryIndexAt(double y, {double scrollOffset = 0}) =>
      ((y.clamp(top, top + height - 0.001) - top + scrollOffset) / tickSpacing)
          .floor()
          .clamp(0, entryCount - 1);

  /// Uses the same scroll offset for ticks, hit testing, and preview placement.
  double entryCenter(int index, {double scrollOffset = 0}) =>
      top + (index + 0.5) * tickSpacing - scrollOffset;

  /// Smoothly increases horizontal proximity and saturates beside the rail.
  double proximityAt(double x) {
    final distance = (interactionWidth - rightInset - x).abs();
    final value =
        ((proximityDistance - distance) /
                (proximityDistance - fullExpansionDistance))
            .clamp(0.0, 1.0);
    return value * value * (3 - 2 * value);
  }

  /// Forms a vertical bell-shaped wave with its peak at the pointer position.
  double tickWidth({
    required double restingWidth,
    required double tickY,
    required double pointerY,
    required double proximity,
  }) {
    final distance = (tickY - pointerY) / waveSpread;
    final wave = math.exp(-0.5 * distance * distance);
    return restingWidth + (maximumTickWidth - restingWidth) * proximity * wave;
  }
}
