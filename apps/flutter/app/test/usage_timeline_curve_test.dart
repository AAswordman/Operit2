import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/settings/data/UsageTimelineCurve.dart';

void main() {
  test('smooth spikes stay within local values and keep idle spans flat', () {
    const points = [
      Offset(0, 0),
      Offset(7, 40),
      Offset(27, 5),
      Offset(40, 5),
      Offset(50, 100),
      Offset(53, 0),
    ];
    final metric = buildUsageTimelineCurve(points).computeMetrics().single;
    for (var i = 0; i <= 1000; i++) {
      final at = metric.getTangentForOffset(metric.length * i / 1000)!.position;
      final segment = points
          .indexWhere((point) => point.dx >= at.dx)
          .clamp(1, points.length - 1);
      final from = points[segment - 1].dy;
      final to = points[segment].dy;
      expect(
        at.dy,
        inInclusiveRange(
          (from < to ? from : to) - 0.001,
          (from > to ? from : to) + 0.001,
        ),
      );
      if (at.dx > 7.1 && at.dx < 26.9) {
        // The interpolation must curve rather than simply draw the old chords.
        final chord = 40 + (at.dx - 7) / 20 * (5 - 40);
        if ((at.dx - 12).abs() < 0.5) {
          expect((at.dy - chord).abs(), greaterThan(1));
        }
      }
    }
  });

  test('unknown and idle cache buckets remain separate curve segments', () {
    final path = buildUsageTimelineCurve(const [
      Offset(0, 10),
      Offset(10, 20),
      null,
      Offset(30, 80),
      Offset(40, 90),
    ]);
    final segments = path.computeMetrics().toList();
    expect(segments.length, 2);
    expect(
      segments[0].getTangentForOffset(segments[0].length)!.position.dx,
      closeTo(10, 0.001),
    );
    expect(segments[1].getTangentForOffset(0)!.position.dx, closeTo(30, 0.001));
  });

  test('empty and isolated observations do not create fabricated lines', () {
    expect(buildUsageTimelineCurve([]).computeMetrics(), isEmpty);
    expect(
      buildUsageTimelineCurve([const Offset(10, 20)]).computeMetrics(),
      isEmpty,
    );
  });
}
