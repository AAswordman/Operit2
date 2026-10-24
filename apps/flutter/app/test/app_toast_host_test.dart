import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/core/host/AppToastController.dart';
import 'package:operit2/ui/common/AppToastHost.dart';
import 'package:operit2/ui/common/MacNoticeScaffoldMessenger.dart';

/// Verifies corner notices and snack-bar redirection.
void main() {
  setUp(AppToastController.instance.reset);

  tearDown(AppToastController.instance.reset);

  for (final initiallyVisible in [false, true]) {
    testWidgets(
      'notice ${initiallyVisible ? "exit" : "entry"} retains the application background and input',
      (tester) async {
        final controller = AppToastController.instance;
        final int? initialNotice = initiallyVisible
            ? controller.show('Already queued')
            : null;
        const mediaKey = ValueKey('background-media');
        await tester.pumpWidget(
          MaterialApp(
            builder: (context, child) => AppToastHost(
              child: LiquidGlassBackdrop(
                background: StatefulBuilder(
                  key: mediaKey,
                  builder: (context, setState) =>
                      const ColoredBox(color: Colors.teal),
                ),
                child: child!,
              ),
            ),
            home: const Scaffold(
              backgroundColor: Colors.transparent,
              body: TextField(),
            ),
          ),
        );
        await _pumpNotice(tester);
        await tester.enterText(find.byType(TextField), 'Keep this draft');
        final backdrop = tester.state(find.byType(LiquidGlassBackdrop));
        final media = tester.state(find.byKey(mediaKey));
        final input = tester.state(find.byType(EditableText));
        final int? queuedNotice;
        if (initialNotice == null) {
          queuedNotice = controller.show('Added to queue');
        } else {
          queuedNotice = null;
          controller.dismiss(initialNotice);
        }
        await _pumpNotice(tester);
        expect(tester.state(find.byType(LiquidGlassBackdrop)), same(backdrop));
        expect(tester.state(find.byKey(mediaKey)), same(media));
        expect(tester.state(find.byType(EditableText)), same(input));
        expect(find.text('Keep this draft'), findsOneWidget);
        final extra = controller.show('Another queued message');
        await _pumpNotice(tester);
        controller.dismiss(extra);
        if (queuedNotice != null) controller.dismiss(queuedNotice);
        await _pumpNotice(tester);
        expect(tester.state(find.byType(LiquidGlassBackdrop)), same(backdrop));
        expect(tester.state(find.byKey(mediaKey)), same(media));
        expect(tester.state(find.byType(EditableText)), same(input));
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }

  testWidgets('shows a host notice at the top-right of the window', (
    tester,
  ) async {
    final controller = AppToastController.instance;
    await tester.pumpWidget(
      const MaterialApp(
        home: AppToastHost(child: ColoredBox(color: Colors.white)),
      ),
    );

    controller.show('Frontend toast');
    await _pumpNotice(tester);

    expect(find.text('Frontend toast'), findsOneWidget);
    expect(find.text('Operit'), findsOneWidget);
    expect(find.byIcon(Icons.close), findsOneWidget);
    _expectTopRightNotice(tester, 'Frontend toast');

    await tester.tap(find.byIcon(Icons.close));
    await _pumpNotice(tester);

    expect(find.text('Frontend toast'), findsNothing);
  });

  testWidgets(
    'snack bars become top-right notices and leave the bottom empty',
    (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: AppToastHost(
            child: MacNoticeScaffoldMessenger(
              child: Scaffold(
                floatingActionButton: FloatingActionButton(
                  onPressed: () {},
                  child: const Icon(Icons.add),
                ),
                body: Builder(
                  builder: (context) {
                    return TextButton(
                      onPressed: () {
                        ScaffoldMessenger.of(
                          context,
                        ).showSnackBar(const SnackBar(content: Text('包已删除')));
                      },
                      child: const Text('notify'),
                    );
                  },
                ),
              ),
            ),
          ),
        ),
      );

      await tester.tap(find.text('notify'));
      await _pumpNotice(tester);

      expect(tester.takeException(), isNull);
      expect(find.byType(SnackBar), findsNothing);
      expect(find.text('包已删除'), findsOneWidget);
      _expectTopRightNotice(tester, '包已删除');

      ScaffoldMessenger.of(
        tester.element(find.text('notify')),
      ).hideCurrentSnackBar();
      await _pumpNotice(tester);

      expect(find.text('包已删除'), findsNothing);
    },
  );

  testWidgets('close tooltip works when the host sits above the navigator', (
    tester,
  ) async {
    var taps = 0;
    await tester.pumpWidget(
      MaterialApp(
        builder: (context, child) => AppToastHost(child: child!),
        home: TextButton(
          onPressed: () => taps += 1,
          child: const Text('under'),
        ),
      ),
    );

    AppToastController.instance.show('Frontend toast');
    await _pumpNotice(tester);

    expect(tester.takeException(), isNull);
    expect(find.text('Frontend toast'), findsOneWidget);

    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await gesture.moveTo(tester.getCenter(find.byIcon(Icons.close)));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 500));

    expect(tester.takeException(), isNull);
    expect(find.text('Close'), findsOneWidget);

    await tester.tap(find.text('under'));
    await tester.pump();
    expect(taps, 1);

    await tester.tap(find.byIcon(Icons.close));
    await _pumpNotice(tester);
    expect(find.text('Frontend toast'), findsNothing);
    expect(tester.takeException(), isNull);
  });
}

/// Advances a notice through its slide animation.
Future<void> _pumpNotice(WidgetTester tester) async {
  await tester.pump();
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 240));
}

/// Checks that a notice sits inside the top-right of the window.
void _expectTopRightNotice(WidgetTester tester, String text) {
  final rect = tester.getRect(find.text(text));
  final window = tester.getSize(find.byType(MaterialApp));
  expect(rect.right, lessThanOrEqualTo(window.width));
  expect(rect.right, greaterThan(window.width - 360));
  expect(rect.top, lessThan(120));
}
