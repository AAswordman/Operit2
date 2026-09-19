import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/TerminalPinchZoom.dart';

void main() {
  testWidgets('pinch scales text, clamps size, and resets after cancellation', (tester) async {
    double scale = 1;
    await tester.pumpWidget(MaterialApp(home: TerminalPinchZoom(
      builder: (_, value) {
        scale = value;
        return const SizedBox.expand();
      },
    )));
    final first = await tester.startGesture(const Offset(100, 100), pointer: 1);
    await first.moveTo(const Offset(100, 120));
    await tester.pump();
    expect(scale, 1);
    final second = await tester.startGesture(const Offset(200, 120), pointer: 2);
    await second.moveTo(const Offset(300, 120));
    await tester.pump();
    expect(scale, closeTo(2, 0.01));
    await second.moveTo(const Offset(700, 120));
    await tester.pump();
    expect(scale, 2.5);
    await second.cancel();
    await first.moveTo(const Offset(120, 120));
    await tester.pump();
    expect(scale, 2.5);
    final third = await tester.startGesture(const Offset(320, 120), pointer: 3);
    await third.moveTo(const Offset(130, 120));
    await tester.pump();
    expect(scale, 0.6);
    await first.up();
    await third.up();
  });

  testWidgets('single finger still scrolls the child', (tester) async {
    final controller = ScrollController();
    await tester.pumpWidget(MaterialApp(home: TerminalPinchZoom(
      builder: (_, scale) => ListView(
        controller: controller,
        children: const [SizedBox(height: 3000)],
      ),
    )));
    await tester.drag(find.byType(ListView), const Offset(0, -200));
    await tester.pumpAndSettle();
    expect(controller.offset, greaterThan(0));
    await tester.pumpWidget(const SizedBox());
    controller.dispose();
  });
}
