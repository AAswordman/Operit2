import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/RetainedPage.dart';

/// Builds a retained page under the live application overlay.
Widget _app({required bool active, required Widget child}) => MaterialApp(
  theme: ThemeData(
    tooltipTheme: const TooltipThemeData(
      waitDuration: Duration(milliseconds: 500),
    ),
  ),
  home: Scaffold(
    body: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: 360,
        height: 500,
        child: RetainedPage(active: active, child: child),
      ),
    ),
  ),
);

/// Verifies that retained pages own the render lifecycle of their overlays.
void main() {
  testWidgets('visible tooltip sleeps with its retained anchor', (
    tester,
  ) async {
    final tooltipKey = GlobalKey<TooltipState>();
    final child = Center(
      child: Tooltip(
        key: tooltipKey,
        message: 'Mount new folder',
        child: IconButton(onPressed: () {}, icon: const Icon(Icons.add)),
      ),
    );
    await tester.pumpWidget(_app(active: true, child: child));
    expect(tooltipKey.currentState!.ensureTooltipVisible(), isTrue);
    await tester.pumpAndSettle();
    expect(find.text('Mount new folder'), findsOneWidget);
    final state = tooltipKey.currentState!;
    final anchor = tester.renderObject(find.byIcon(Icons.add));
    final tooltip = tester.renderObject(find.text('Mount new folder'));

    await tester.pumpWidget(_app(active: false, child: child));
    await tester.pump();
    expect(anchor.attached, isFalse);
    expect(tooltip.attached, isFalse);
    tester.view.physicalSize = const Size(900, 700);
    addTearDown(tester.view.resetPhysicalSize);
    await tester.pump(const Duration(seconds: 1));
    expect(find.text('Mount new folder'), findsNothing);
    expect(tester.takeException(), isNull);

    await tester.pumpWidget(_app(active: true, child: child));
    await tester.pumpAndSettle();
    expect(tooltipKey.currentState, same(state));
    expect(anchor.attached, isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('pending tooltip hover cannot escape a sleeping page', (
    tester,
  ) async {
    final child = Center(
      child: IconButton(
        tooltip: 'Mount new folder',
        onPressed: () {},
        icon: const Icon(Icons.add),
      ),
    );
    await tester.pumpWidget(_app(active: true, child: child));
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: const Offset(700, 550));
    await mouse.moveTo(tester.getCenter(find.byIcon(Icons.add)));
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.text('Mount new folder'), findsNothing);

    await tester.pumpWidget(_app(active: false, child: child));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    await tester.pump();
    expect(find.text('Mount new folder'), findsNothing);
    expect(tester.takeException(), isNull);
    await mouse.removePointer();
    await tester.pumpWidget(_app(active: true, child: child));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });
}
