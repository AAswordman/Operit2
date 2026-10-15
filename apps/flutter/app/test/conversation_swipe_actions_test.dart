import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/SwipeActions.dart';

/// Builds a fixed-width row with observable action and tap callbacks.
Widget _buildRow({
  required VoidCallback onStartAction,
  required VoidCallback onEndAction,
  VoidCallback? onTap,
  VoidCallback? onLongPress,
  TextDirection textDirection = TextDirection.ltr,
  Color childColor = Colors.white,
}) => MaterialApp(
  home: Directionality(
    textDirection: textDirection,
    child: Scaffold(
      body: Center(
        child: SizedBox(
          width: 240,
          height: 34,
          child: RepaintBoundary(
            key: const ValueKey('swipe-row-paint'),
            child: SwipeActions(
              onStartAction: onStartAction,
              onEndAction: onEndAction,
              background: const ColoredBox(color: Colors.blue),
              secondaryBackground: const ColoredBox(color: Colors.red),
              child: Material(
                color: childColor,
                child: InkWell(
                  onTap: onTap,
                  onLongPress: onLongPress,
                  child: const Center(child: Text('Conversation')),
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  ),
);

/// Samples the revealed strip and covered content from the actual row painting.
Future<List<Color>> _rowPixelColors(
  WidgetTester tester,
  List<Offset> positions,
) async {
  final boundary = tester.renderObject<RenderRepaintBoundary>(
    find.byKey(const ValueKey('swipe-row-paint')),
  );
  return (await tester.runAsync(() async {
    final image = await boundary.toImage(pixelRatio: 1);
    final bytes = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
    final colors = positions.map((position) {
      final index =
          (position.dy.toInt() * image.width + position.dx.toInt()) * 4;
      return Color.fromARGB(
        bytes.getUint8(index + 3),
        bytes.getUint8(index),
        bytes.getUint8(index + 1),
        bytes.getUint8(index + 2),
      );
    }).toList();
    image.dispose();
    return colors;
  }))!;
}

/// Checks bounded swipes, gesture intent, cancellation, and scroll arbitration.
void main() {
  for (final direction in TextDirection.values) {
    testWidgets('transparent rows reveal only the swipe strip in $direction', (
      tester,
    ) async {
      var actions = 0;
      await tester.pumpWidget(
        _buildRow(
          onStartAction: () => actions++,
          onEndAction: () => actions++,
          textDirection: direction,
          childColor: Colors.transparent,
        ),
      );
      final bounds = tester.getRect(find.byType(SwipeActions));
      for (final distance in <double>[70, -70]) {
        final gesture = await tester.startGesture(bounds.center);
        await gesture.moveBy(Offset(distance, 0));
        await tester.pump();
        final startSide = direction == TextDirection.ltr
            ? distance > 0
            : distance < 0;
        final pixels = await _rowPixelColors(tester, <Offset>[
          Offset(distance > 0 ? 35 : 205, 4),
          Offset(distance > 0 ? 205 : 35, 4),
        ]);
        expect(
          pixels.first.toARGB32(),
          (startSide ? Colors.blue : Colors.red).toARGB32(),
        );
        expect(pixels.last.toARGB32(), Colors.transparent.toARGB32());
        await gesture.up();
        await tester.pumpAndSettle();
      }
      expect(actions, 0);
    });
  }

  testWidgets(
    'half-width swipes from the center invoke an action at either edge',
    (tester) async {
      var renames = 0;
      var deletes = 0;
      await tester.pumpWidget(
        _buildRow(onStartAction: () => renames++, onEndAction: () => deletes++),
      );
      final bounds = tester.getRect(find.byType(SwipeActions));
      await tester.dragFrom(bounds.center, const Offset(120, 0));
      await tester.pumpAndSettle();
      expect(renames, 1);
      expect(deletes, 0);
      await tester.dragFrom(bounds.center, const Offset(-120, 0));
      await tester.pumpAndSettle();
      expect(renames, 1);
      expect(deletes, 1);
    },
  );

  testWidgets('less than half the row width does not invoke either action', (
    tester,
  ) async {
    var actions = 0;
    await tester.pumpWidget(
      _buildRow(onStartAction: () => actions++, onEndAction: () => actions++),
    );
    final bounds = tester.getRect(find.byType(SwipeActions));
    for (final distance in <double>[119, -119]) {
      await tester.dragFrom(bounds.center, Offset(distance, 0));
      await tester.pumpAndSettle();
      expect(actions, 0);
      expect(tester.getCenter(find.text('Conversation')), bounds.center);
    }
  });

  testWidgets('reversing below the action distance does not invoke an action', (
    tester,
  ) async {
    var actions = 0;
    await tester.pumpWidget(
      _buildRow(onStartAction: () => actions++, onEndAction: () => actions++),
    );
    final bounds = tester.getRect(find.byType(SwipeActions));
    final gesture = await tester.startGesture(bounds.center);
    await gesture.moveBy(const Offset(120, 0));
    await tester.pump();
    await gesture.moveBy(const Offset(-60, 0));
    await tester.pump();
    await gesture.up();
    await tester.pumpAndSettle();
    expect(actions, 0);
    expect(tester.getCenter(find.text('Conversation')), bounds.center);
  });

  testWidgets('long presses remain available to the row', (tester) async {
    var longPresses = 0;
    var actions = 0;
    await tester.pumpWidget(
      _buildRow(
        onStartAction: () => actions++,
        onEndAction: () => actions++,
        onLongPress: () => longPresses++,
      ),
    );
    await tester.longPress(find.text('Conversation'));
    await tester.pumpAndSettle();
    expect(longPresses, 1);
    expect(actions, 0);
  });

  testWidgets('deliberate swipes invoke the matching action exactly once', (
    tester,
  ) async {
    var renames = 0;
    var deletes = 0;
    await tester.pumpWidget(
      _buildRow(onStartAction: () => renames++, onEndAction: () => deletes++),
    );
    final bounds = tester.getRect(find.byType(SwipeActions));
    await tester.dragFrom(
      Offset(bounds.left + 40, bounds.center.dy),
      const Offset(150, 0),
    );
    await tester.pumpAndSettle();
    expect(renames, 1);
    expect(deletes, 0);
    expect(tester.getCenter(find.text('Conversation')), bounds.center);
    await tester.dragFrom(
      Offset(bounds.right - 40, bounds.center.dy),
      const Offset(-150, 0),
    );
    await tester.pumpAndSettle();
    expect(renames, 1);
    expect(deletes, 1);
  });

  testWidgets('short fast flings in either direction do not invoke actions', (
    tester,
  ) async {
    var actions = 0;
    await tester.pumpWidget(
      _buildRow(onStartAction: () => actions++, onEndAction: () => actions++),
    );
    final bounds = tester.getRect(find.byType(SwipeActions));
    for (final distance in <double>[70, -70]) {
      await tester.flingFrom(bounds.center, Offset(distance, 0), 2000);
      await tester.pumpAndSettle();
    }
    expect(actions, 0);
    expect(tester.getCenter(find.text('Conversation')), bounds.center);
  });

  testWidgets(
    'small incidental horizontal movement does not translate the row',
    (tester) async {
      var actions = 0;
      await tester.pumpWidget(
        _buildRow(onStartAction: () => actions++, onEndAction: () => actions++),
      );
      final bounds = tester.getRect(find.byType(SwipeActions));
      final gesture = await tester.startGesture(bounds.center);
      await gesture.moveBy(const Offset(24, 0));
      await tester.pump();
      expect(tester.getCenter(find.text('Conversation')), bounds.center);
      await gesture.up();
      await tester.pumpAndSettle();
      expect(actions, 0);
    },
  );

  testWidgets(
    'leaving the row cancels permanently even after reaching action distance',
    (tester) async {
      var actions = 0;
      await tester.pumpWidget(
        _buildRow(onStartAction: () => actions++, onEndAction: () => actions++),
      );
      final bounds = tester.getRect(find.byType(SwipeActions));
      for (final outside in <Offset>[
        Offset(bounds.left + 190, bounds.bottom + 30),
        Offset(bounds.right + 30, bounds.center.dy),
      ]) {
        final gesture = await tester.startGesture(
          Offset(bounds.left + 40, bounds.center.dy),
        );
        await gesture.moveBy(const Offset(150, 0));
        await tester.pump();
        await gesture.moveTo(outside);
        await tester.pumpAndSettle();
        expect(actions, 0);
        expect(tester.getCenter(find.text('Conversation')), bounds.center);
        await gesture.moveTo(Offset(bounds.left + 190, bounds.center.dy));
        await gesture.up();
        await tester.pumpAndSettle();
        expect(actions, 0);
      }
    },
  );

  testWidgets('pointer cancellation does not invoke an action', (tester) async {
    var actions = 0;
    await tester.pumpWidget(
      _buildRow(onStartAction: () => actions++, onEndAction: () => actions++),
    );
    final bounds = tester.getRect(find.byType(SwipeActions));
    final gesture = await tester.startGesture(
      Offset(bounds.left + 40, bounds.center.dy),
    );
    await gesture.moveBy(const Offset(150, 0));
    await tester.pump();
    await gesture.cancel();
    await tester.pumpAndSettle();
    expect(actions, 0);
    expect(tester.getCenter(find.text('Conversation')), bounds.center);
  });

  testWidgets('tapping still activates the conversation', (tester) async {
    var taps = 0;
    var actions = 0;
    await tester.pumpWidget(
      _buildRow(
        onStartAction: () => actions++,
        onEndAction: () => actions++,
        onTap: () => taps++,
      ),
    );
    await tester.tap(find.text('Conversation'));
    await tester.pumpAndSettle();
    expect(taps, 1);
    expect(actions, 0);
  });

  testWidgets('RTL swipes preserve logical rename and delete directions', (
    tester,
  ) async {
    var renames = 0;
    var deletes = 0;
    await tester.pumpWidget(
      _buildRow(
        onStartAction: () => renames++,
        onEndAction: () => deletes++,
        textDirection: TextDirection.rtl,
      ),
    );
    final bounds = tester.getRect(find.byType(SwipeActions));
    await tester.dragFrom(
      Offset(bounds.right - 40, bounds.center.dy),
      const Offset(-150, 0),
    );
    await tester.pumpAndSettle();
    expect(renames, 1);
    expect(deletes, 0);
    await tester.dragFrom(
      Offset(bounds.left + 40, bounds.center.dy),
      const Offset(150, 0),
    );
    await tester.pumpAndSettle();
    expect(renames, 1);
    expect(deletes, 1);
  });

  testWidgets(
    'vertical and diagonal drags scroll without conversation actions',
    (tester) async {
      var actions = 0;
      final scrollController = ScrollController();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 240,
                height: 200,
                child: ListView.builder(
                  controller: scrollController,
                  itemCount: 30,
                  itemExtent: 34,
                  itemBuilder: (context, index) => SwipeActions(
                    onStartAction: () => actions++,
                    onEndAction: () => actions++,
                    background: const ColoredBox(color: Colors.blue),
                    secondaryBackground: const ColoredBox(color: Colors.red),
                    child: Text('Conversation $index'),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      for (final drag in <Offset>[
        const Offset(0, -100),
        const Offset(60, -100),
      ]) {
        scrollController.jumpTo(0);
        await tester.pump();
        await tester.drag(find.byType(SwipeActions).at(3), drag);
        await tester.pumpAndSettle();
        expect(scrollController.offset, greaterThan(0));
        expect(actions, 0);
      }
      await tester.pumpWidget(const SizedBox.shrink());
      scrollController.dispose();
    },
  );

  testWidgets('mouse drags use the same bounded gesture rules', (tester) async {
    var actions = 0;
    await tester.pumpWidget(
      _buildRow(onStartAction: () => actions++, onEndAction: () => actions++),
    );
    final bounds = tester.getRect(find.byType(SwipeActions));
    final gesture = await tester.startGesture(
      Offset(bounds.left + 40, bounds.center.dy),
      kind: PointerDeviceKind.mouse,
    );
    await gesture.moveBy(const Offset(150, 0));
    await tester.pump();
    await gesture.moveBy(const Offset(0, 40));
    await gesture.up();
    await tester.pumpAndSettle();
    expect(actions, 0);
  });
}
