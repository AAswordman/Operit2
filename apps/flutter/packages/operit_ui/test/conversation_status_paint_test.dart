import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

void main() {
  testWidgets(
    'status ticks repaint locally without build, layout or semantics',
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        final animation = AnimationController(vsync: tester);
        addTearDown(animation.dispose);
        final counts = _RenderCounts();

        await tester.pumpWidget(_host(animation, counts));
        await tester.pump();
        final indicator = find.byType(ThemeConversationStatusHandle);
        final paintFinder = find.descendant(
          of: indicator,
          matching: find.byType(CustomPaint),
        );
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.descendant(
            of: indicator,
            matching: find.byType(RepaintBoundary),
          ),
        );
        final widget = tester.widget<CustomPaint>(paintFinder);
        final render = tester.renderObject<RenderCustomPaint>(paintFinder);
        final baseline = counts.snapshot;
        boundary.debugResetMetrics();

        for (final value in <double>[0.2, 0.4, 0.6, 0.8]) {
          animation.value = value;
          expect(render.debugNeedsPaint, isTrue);
          await tester.pump();
          expect(tester.widget<CustomPaint>(paintFinder), same(widget));
          expect(render.painter, same(widget.painter));
          expect(counts.snapshot, baseline);
        }
        expect(boundary.debugAsymmetricPaintCount, 4);
        expect(boundary.debugSymmetricPaintCount, 0);

        await tester.pumpWidget(_host(animation, counts, color: Colors.green));
        expect(
          tester.widget<CustomPaint>(paintFinder).painter,
          isNot(widget.painter),
        );
        expect(counts.layouts, baseline.$2);
        expect(counts.paints, baseline.$3);
        expect(
          render.painter!.shouldRebuildSemantics(widget.painter!),
          isFalse,
        );
        expect(
          tester
              .getSemantics(find.byTooltip('正在运行'))
              .getSemanticsData()
              .tooltip,
          '正在运行',
        );
        await tester.pumpWidget(const SizedBox.shrink());
      } finally {
        semantics.dispose();
      }
    },
  );

  testWidgets(
    'replacing the animation detaches its previous repaint listener',
    (tester) async {
      final previous = AnimationController(vsync: tester);
      final next = AnimationController(vsync: tester);
      addTearDown(previous.dispose);
      addTearDown(next.dispose);
      final counts = _RenderCounts();
      await tester.pumpWidget(_host(previous, counts));
      await tester.pumpWidget(_host(next, counts));
      final render = tester.renderObject<RenderCustomPaint>(
        find.descendant(
          of: find.byType(ThemeConversationStatusHandle),
          matching: find.byType(CustomPaint),
        ),
      );
      previous.value = 0.5;
      expect(render.debugNeedsPaint, isFalse);
      next.value = 0.5;
      expect(render.debugNeedsPaint, isTrue);
      await tester.pump();
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}

Widget _host(
  Animation<double> animation,
  _RenderCounts counts, {
  Color color = Colors.blue,
}) => MaterialApp(
  home: Center(
    child: _BuildProbe(
      counts: counts,
      child: ThemeConversationStatusHandle(
        isRunning: true,
        selected: false,
        hovered: false,
        animation: animation,
        appearance: ThemeDrawerPalette(
          containerColor: Colors.white,
          titleColor: Colors.black,
          statusAvailableColor: color,
          itemColor: Colors.black87,
          buttonContainerColor: Colors.white70,
          selectedContainerColor: Colors.blue,
          selectedContentColor: Colors.white,
          dividerColor: Colors.black12,
          transparentSurfaceEnabled: false,
        ),
      ),
    ),
  ),
);

class _RenderCounts {
  int builds = 0;
  int layouts = 0;
  int paints = 0;
  int semantics = 0;

  (int, int, int, int) get snapshot => (builds, layouts, paints, semantics);
}

class _BuildProbe extends StatelessWidget {
  const _BuildProbe({required this.counts, required this.child});
  final _RenderCounts counts;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    counts.builds++;
    return _RenderProbe(counts: counts, child: child);
  }
}

class _RenderProbe extends SingleChildRenderObjectWidget {
  const _RenderProbe({required this.counts, required super.child});
  final _RenderCounts counts;

  @override
  RenderObject createRenderObject(BuildContext context) => _ProbeBox(counts);
}

class _ProbeBox extends RenderProxyBox {
  _ProbeBox(this.counts);
  final _RenderCounts counts;

  @override
  void performLayout() {
    counts.layouts++;
    super.performLayout();
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    counts.paints++;
    super.paint(context, offset);
  }

  @override
  void describeSemanticsConfiguration(SemanticsConfiguration config) {
    super.describeSemanticsConfiguration(config);
    config.isSemanticBoundary = true;
  }

  @override
  void assembleSemanticsNode(
    SemanticsNode node,
    SemanticsConfiguration config,
    Iterable<SemanticsNode> children,
  ) {
    counts.semantics++;
    super.assembleSemanticsNode(node, config, children);
  }
}
