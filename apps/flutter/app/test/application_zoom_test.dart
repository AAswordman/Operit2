import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/AdaptiveSidePanel.dart';
import 'package:operit2/ui/common/layout/ApplicationZoom.dart';
import 'package:operit2/ui/common/layout/OverlayGeometry.dart';

/// Verifies explicit interface zoom, input preservation, and local geometry.
void main() {
  test('zoom uses bounded discrete levels and rejects unknown values', () {
    expect(ApplicationZoom.step(1, 1), 1.1);
    expect(ApplicationZoom.step(1, -1), 0.9);
    expect(ApplicationZoom.step(0.7, -1), 0.7);
    expect(ApplicationZoom.step(1.5, 1), 1.5);
    expect(() => ApplicationZoom.step(0.95, 1), throwsArgumentError);
  });

  testWidgets('zoom controls and shortcuts share the effective zoom', (
    tester,
  ) async {
    final zoom = ValueNotifier<double>(1.2);
    addTearDown(zoom.dispose);
    await tester.pumpWidget(
      _zoomApp(
        zoom,
        Builder(
          builder: (context) {
            final scope = ApplicationZoomScope.of(context);
            return TextButton(
              style: TextButton.styleFrom(
                splashFactory: NoSplash.splashFactory,
              ),
              onPressed: () => scope.onZoomChanged(1.3),
              child: Text('${(scope.zoom * 100).round()}%'),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('120%'), findsOneWidget);

    await tester.tap(find.byType(TextButton));
    await tester.pumpAndSettle();
    expect(zoom.value, 1.3);
    expect(find.text('130%'), findsOneWidget);

    await _shortcut(tester, LogicalKeyboardKey.minus);
    expect(zoom.value, 1.2);
    expect(find.text('120%'), findsOneWidget);

    await _shortcut(tester, LogicalKeyboardKey.digit0);
    expect(zoom.value, 1);
    expect(find.text('100%'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('zoom shortcuts preserve draft, focus, and input state', (
    tester,
  ) async {
    final zoom = ValueNotifier<double>(1);
    final input = TextEditingController(text: 'unsent draft');
    final focus = FocusNode();
    addTearDown(zoom.dispose);
    addTearDown(input.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      _zoomApp(zoom, TextField(controller: input, focusNode: focus)),
    );
    focus.requestFocus();
    await tester.pumpAndSettle();
    final inputState = tester.state(find.byType(TextField));
    await _shortcut(tester, LogicalKeyboardKey.equal);
    expect(zoom.value, 1.1);
    await _shortcut(tester, LogicalKeyboardKey.equal, shift: true);
    expect(zoom.value, 1.2);
    await _shortcut(tester, LogicalKeyboardKey.minus);
    expect(zoom.value, 1.1);
    await _shortcut(tester, LogicalKeyboardKey.numpadAdd);
    expect(zoom.value, 1.2);
    await _shortcut(tester, LogicalKeyboardKey.numpadSubtract);
    expect(zoom.value, 1.1);
    await _shortcut(tester, LogicalKeyboardKey.digit0);
    expect(zoom.value, 1);
    await _shortcut(tester, LogicalKeyboardKey.add, meta: true, shift: true);
    expect(zoom.value, 1.1);
    await _shortcut(tester, LogicalKeyboardKey.numpad0, meta: true);
    expect(zoom.value, 1);
    expect(tester.state(find.byType(TextField)), same(inputState));
    expect(input.text, 'unsent draft');
    expect(focus.hasFocus, isTrue);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('shortcuts work without an editable control owning focus', (
    tester,
  ) async {
    final zoom = ValueNotifier<double>(1);
    addTearDown(zoom.dispose);
    await tester.pumpWidget(_zoomApp(zoom, const SizedBox.expand()));
    await tester.pumpAndSettle();
    await _shortcut(tester, LogicalKeyboardKey.minus);
    expect(zoom.value, 0.9);
    await tester.sendKeyEvent(LogicalKeyboardKey.equal);
    await tester.pump();
    expect(zoom.value, 0.9);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('nested browser shortcuts retain their independent zoom', (
    tester,
  ) async {
    final zoom = ValueNotifier<double>(1);
    var browserZooms = 0;
    addTearDown(zoom.dispose);
    await tester.pumpWidget(
      _zoomApp(
        zoom,
        ApplicationZoomExclusion(
          child: CallbackShortcuts(
            bindings: <ShortcutActivator, VoidCallback>{
              const SingleActivator(
                LogicalKeyboardKey.equal,
                control: true,
              ): () =>
                  browserZooms++,
            },
            child: const Focus(autofocus: true, child: SizedBox.expand()),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await _shortcut(tester, LogicalKeyboardKey.equal);
    expect(browserZooms, 1);
    expect(zoom.value, 1);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('disabled shortcuts do not change startup viewport zoom', (
    tester,
  ) async {
    final zoom = ValueNotifier<double>(1);
    addTearDown(zoom.dispose);
    await tester.pumpWidget(
      _zoomApp(zoom, const SizedBox.expand(), shortcutsEnabled: false),
    );
    await tester.pumpAndSettle();
    await _shortcut(tester, LogicalKeyboardKey.equal);
    expect(zoom.value, 1);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets(
    'viewport geometry scales while host DPR and text scaling remain',
    (tester) async {
      final zoom = ValueNotifier<double>(1.5);
      addTearDown(zoom.dispose);
      late MediaQueryData data;
      await tester.pumpWidget(
        MaterialApp(
          builder: (context, child) => MediaQuery(
            data: const MediaQueryData(
              size: Size(900, 600),
              devicePixelRatio: 2,
              padding: EdgeInsets.all(15),
              viewPadding: EdgeInsets.all(30),
              viewInsets: EdgeInsets.only(bottom: 150),
              systemGestureInsets: EdgeInsets.all(12),
              textScaler: TextScaler.linear(1.2),
            ),
            child: ApplicationZoomHost(
              zoom: zoom.value,
              onZoomChanged: (value) => zoom.value = value,
              child: child!,
            ),
          ),
          home: Builder(
            builder: (context) {
              data = MediaQuery.of(context);
              return const SizedBox.expand();
            },
          ),
        ),
      );
      expect(data.size, const Size(600, 400));
      expect(data.padding, const EdgeInsets.all(10));
      expect(data.viewPadding, const EdgeInsets.all(20));
      expect(data.viewInsets.bottom, 100);
      expect(data.systemGestureInsets, const EdgeInsets.all(8));
      expect(data.devicePixelRatio, 2);
      expect(data.textScaler.scale(10), 12);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets(
    'scaled popup anchors and pointer hits share overlay coordinates',
    (tester) async {
      final zoom = ValueNotifier<double>(1.5);
      final anchorKey = GlobalKey();
      var taps = 0;
      addTearDown(zoom.dispose);
      late BuildContext anchorContext;
      await tester.pumpWidget(
        _zoomApp(
          zoom,
          Builder(
            builder: (context) {
              anchorContext = context;
              return Stack(
                children: <Widget>[
                  Positioned(
                    left: 40,
                    top: 30,
                    width: 80,
                    height: 40,
                    child: GestureDetector(
                      onTap: () => taps++,
                      child: ColoredBox(key: anchorKey, color: Colors.blue),
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      final target = anchorKey.currentContext!.findRenderObject()! as RenderBox;
      expect(
        overlayTargetRectOf(anchorContext, target),
        const Rect.fromLTWH(40, 30, 80, 40),
      );
      await tester.tapAt(const Offset(120, 75));
      expect(taps, 1);
      final entry = OverlayEntry(
        builder: (context) {
          final rect = overlayTargetRectOf(anchorContext, target);
          return Positioned(
            left: rect.left,
            top: rect.bottom + 8,
            width: 80,
            height: 40,
            child: const ColoredBox(key: ValueKey('popup'), color: Colors.red),
          );
        },
      );
      Overlay.of(anchorContext).insert(entry);
      await tester.pump();
      expect(
        tester.getTopLeft(find.byKey(const ValueKey('popup'))),
        const Offset(60, 117),
      );
      entry.remove();
      entry.dispose();
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets('side panel drag distance follows the zoomed coordinate space', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final zoom = ValueNotifier<double>(1.5);
    addTearDown(zoom.dispose);
    await tester.pumpWidget(
      _zoomApp(
        zoom,
        AdaptiveSidePanel(
          open: true,
          animate: false,
          onOpenChanged: (_) {},
          panel: const ColoredBox(key: ValueKey('panel'), color: Colors.blue),
          child: const SizedBox.expand(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final handle = find.byWidgetPredicate(
      (widget) =>
          widget is GestureDetector && widget.onHorizontalDragUpdate != null,
    );
    expect(handle, findsOneWidget);
    final panel = find.byKey(const ValueKey('panel'));
    final initialWidth = tester.getSize(panel).width;
    final gesture = await tester.startGesture(
      tester.getCenter(handle) + const Offset(8, 0),
    );
    await gesture.moveBy(const Offset(-40, 0));
    await tester.pump();
    final startedWidth = tester.getSize(panel).width;
    await gesture.moveBy(const Offset(-60, 0));
    await tester.pump();
    expect(tester.getSize(panel).width - startedWidth, closeTo(40, 0.01));
    expect(tester.getSize(panel).width, greaterThan(initialWidth));
    await gesture.up();
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

/// Builds a stable Navigator below an explicitly controlled zoom host.
Widget _zoomApp(
  ValueNotifier<double> zoom,
  Widget child, {
  bool shortcutsEnabled = true,
}) {
  return MaterialApp(
    builder: (context, navigator) => ValueListenableBuilder<double>(
      valueListenable: zoom,
      child: navigator,
      builder: (context, value, navigator) => ApplicationZoomHost(
        zoom: value,
        shortcutsEnabled: shortcutsEnabled,
        onZoomChanged: (value) => zoom.value = value,
        child: navigator!,
      ),
    ),
    home: Scaffold(body: child),
  );
}

/// Sends one complete Ctrl or Cmd shortcut including optional Shift.
Future<void> _shortcut(
  WidgetTester tester,
  LogicalKeyboardKey key, {
  bool meta = false,
  bool shift = false,
}) async {
  final modifier = meta
      ? LogicalKeyboardKey.metaLeft
      : LogicalKeyboardKey.controlLeft;
  await tester.sendKeyDownEvent(modifier);
  if (shift) {
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  }
  await tester.sendKeyEvent(
    key,
    physicalKey: key == LogicalKeyboardKey.add
        ? PhysicalKeyboardKey.equal
        : null,
  );
  if (shift) {
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  }
  await tester.sendKeyUpEvent(modifier);
  await tester.pump();
}
