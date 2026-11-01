// ignore_for_file: file_names

import 'dart:ui';

/// Shape-preserving cubic interpolation. Nulls break the curve, flat intervals
/// stay flat, and control points remain between each pair of actual values.
Path buildUsageTimelineCurve(List<Offset?> points) {
  final path = Path();
  var run = <Offset>[];
  void appendRun() {
    if (run.isEmpty) return;
    path.moveTo(run.first.dx, run.first.dy);
    if (run.length > 1) {
      final widths = <double>[];
      final slopes = <double>[];
      for (var i = 0; i < run.length - 1; i++) {
        final width = run[i + 1].dx - run[i].dx;
        widths.add(width);
        slopes.add((run[i + 1].dy - run[i].dy) / width);
      }
      final tangents = List<double>.filled(run.length, 0);
      tangents[0] = slopes.first;
      tangents[tangents.length - 1] = slopes.last;
      for (var i = 1; i < run.length - 1; i++) {
        final before = slopes[i - 1];
        final after = slopes[i];
        if (before * after <= 0) continue;
        final w1 = 2 * widths[i] + widths[i - 1];
        final w2 = widths[i] + 2 * widths[i - 1];
        tangents[i] = (w1 + w2) / (w1 / before + w2 / after);
      }
      for (var i = 0; i < run.length - 1; i++) {
        final from = run[i];
        final to = run[i + 1];
        final third = widths[i] / 3;
        final low = from.dy < to.dy ? from.dy : to.dy;
        final high = from.dy > to.dy ? from.dy : to.dy;
        path.cubicTo(
          from.dx + third,
          (from.dy + tangents[i] * third).clamp(low, high),
          to.dx - third,
          (to.dy - tangents[i + 1] * third).clamp(low, high),
          to.dx,
          to.dy,
        );
      }
    }
    run = <Offset>[];
  }

  for (final point in points) {
    if (point == null) {
      appendRun();
    } else {
      run.add(point);
    }
  }
  appendRun();
  return path;
}
