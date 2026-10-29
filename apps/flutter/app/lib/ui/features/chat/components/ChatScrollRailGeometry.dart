// ignore_for_file: file_names

import 'dart:math' as math;

/// Shares bounded rail geometry between painting, pointer selection and previews.
class ChatScrollRailGeometry {
  /// Creates a centered rail that always fits inside the transcript viewport.
  const ChatScrollRailGeometry({
    required this.entryCount,
    required this.viewportHeight,
  }) : assert(entryCount > 0),
       assert(viewportHeight > 0 && viewportHeight < double.infinity);

  static const double interactionWidth = 88;
  static const double hitWidth = 48;
  static const double rightInset = 8;
  static const double maximumTickWidth = 24;
  // Reach full width before the pointer touches the longest tick's left edge.
  static const double fullExpansionDistance = maximumTickWidth + 4;
  static const double proximityDistance = 72;
  static const double waveRadius = 44;

  final int entryCount;
  final double viewportHeight;

  /// Keeps dense histories within the available height instead of overflowing.
  double get height =>
      math.min(entryCount * 8.0, math.max(8.0, viewportHeight - 100));

  /// Limits paint work to the number of visually distinguishable tick rows.
  int get tickCount => math.min(entryCount, math.max(1, (height / 3).floor()));

  /// Centers the rail independently of the number of messages in history.
  double get top => (viewportHeight - height) / 2;

  /// Provides at least a finger-sized target without enlarging the painted rail.
  double get hitHeight => math.min(viewportHeight, math.max(48.0, height + 16));

  /// Centers the touch target around the actual tick positions.
  double get hitTop => (viewportHeight - hitHeight) / 2;

  /// Keeps the wave visible in both short conversations and long histories.
  double get waveSpread => math.min(waveRadius, math.max(12.0, height / 3));

  /// Returns the exact message row nearest to a pointer's vertical position.
  int entryIndexAt(double y) =>
      (((y - top) / height) * entryCount).floor().clamp(0, entryCount - 1);

  /// Places previews using the same history-to-rail mapping as pointer selection.
  double entryCenter(int index) => top + (index + 0.5) * height / entryCount;

  /// Returns the center of a bounded visual tick bucket.
  double tickCenter(int index) => top + (index + 0.5) * height / tickCount;

  /// Maps the first message represented by a visual tick bucket.
  int bucketStart(int index) => index * entryCount ~/ tickCount;

  /// Maps the exclusive last message represented by a visual tick bucket.
  int bucketEnd(int index) => (index + 1) * entryCount ~/ tickCount;

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
