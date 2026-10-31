import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

// This suite also runs in a real browser: flutter test --platform chrome
// test/application_windows_test.dart. It needs no native method-channel mocks.
void main() {
  testWidgets(
    'drag uses pointer-down anchors across sparse moves, rebuilds and reversals',
    (tester) async {
      var taps = 0;
      const pet = ValueKey('drag-pet');
      await tester.pumpWidget(
        FloatingWindowsHost(
          windowBuilder: (_) => GestureDetector(
            onTap: () => taps++,
            behavior: HitTestBehavior.opaque,
            child: const SizedBox.expand(key: pet),
          ),
          child: const SizedBox.expand(),
        ),
      );
      final window = await FloatingWindows.open(
        payload: {},
        options: const FloatingWindowOptions(
          x: 40,
          y: 60,
          width: 100,
          height: 100,
          draggable: true,
        ),
      );
      await tester.pump();
      final pointer = await tester.startGesture(const Offset(70, 90));
      await pointer.moveTo(const Offset(270, 190));
      await tester.pump();
      expect(tester.getTopLeft(find.byKey(pet)), const Offset(240, 160));
      await pointer.moveTo(const Offset(470, 290));
      await tester.pump();
      expect(tester.getTopLeft(find.byKey(pet)), const Offset(440, 260));
      // One large reverse move must use the original down point, not the last frame.
      await pointer.moveTo(const Offset(170, 140));
      await tester.pump();
      expect(tester.getTopLeft(find.byKey(pet)), const Offset(140, 110));
      await pointer.up();
      await tester.pump();
      expect(taps, 0);
      await tester.tapAt(const Offset(160, 130));
      await tester.pump();
      expect(taps, 1);
      // A new drag gets a new anchor at the current window position.
      final next = await tester.startGesture(const Offset(160, 130));
      await next.moveTo(const Offset(260, 180));
      await tester.pump();
      expect(tester.getTopLeft(find.byKey(pet)), const Offset(240, 160));
      await next.up();
      expect((await window.getBounds()).x, 240);
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.iOS),
  );

  testWidgets(
    'live payload and size updates retain the surface identity and dragged position',
    (tester) async {
      final seen = <FloatingWindowLaunch>[];
      const pet = ValueKey('updated-pet');
      await tester.pumpWidget(
        FloatingWindowsHost(
          windowBuilder: (launch) {
            seen.add(launch);
            return Text(launch.payload['text'] as String, key: pet);
          },
          child: const SizedBox.expand(),
        ),
      );
      final window = await FloatingWindows.open(
        payload: {'text': 'animated', 'keep': 42},
        options: const FloatingWindowOptions(
          x: 120,
          y: 80,
          width: 100,
          height: 100,
        ),
      );
      await tester.pump();
      final element = find.byKey(pet).evaluate().single;
      await window.update(
        const FloatingWindowOptions(width: 200, clickThrough: true),
        payload: {'text': 'still'},
      );
      await tester.pump();
      expect(find.text('animated'), findsNothing);
      expect(find.text('still'), findsOneWidget);
      expect(identical(find.byKey(pet).evaluate().single, element), true);
      expect(tester.getTopLeft(find.byKey(pet)), const Offset(120, 80));
      expect(tester.getSize(find.byKey(pet)).width, 200);
      expect(seen.last.payload['keep'], 42);
      expect(await FloatingWindows.liveIds(), {window.id});
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.iOS),
  );

  testWidgets(
    'a handle from a disposed host cannot control a new host window',
    (tester) async {
      Widget host(String text) => FloatingWindowsHost(
        windowBuilder: (_) => Text(text),
        child: const SizedBox.expand(),
      );
      await tester.pumpWidget(host('old'));
      final stale = await FloatingWindows.open(payload: {});
      await tester.pump();
      expect(find.text('old'), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(host('new'));
      final current = await FloatingWindows.open(payload: {});
      await tester.pump();
      expect(current.id, isNot(stale.id));
      await expectLater(stale.hide(), throwsStateError);
      await expectLater(stale.close(), throwsStateError);
      expect(find.text('new'), findsOneWidget);
      expect(await FloatingWindows.liveIds(), {current.id});
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.iOS),
  );

  testWidgets(
    'input passes through an application surface and survives hide/show',
    (tester) async {
      var applicationTaps = 0;
      var surfaceTaps = 0;
      await tester.pumpWidget(
        FloatingWindowsHost(
          windowBuilder: (_) => GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () => surfaceTaps++,
            child: const Text('surface'),
          ),
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () => applicationTaps++,
            child: const SizedBox.expand(),
          ),
        ),
      );
      expect((await FloatingWindows.capabilities())['scope'], 'application');
      final window = await FloatingWindows.open(
        payload: {},
        options: const FloatingWindowOptions(
          x: 20,
          y: 20,
          width: 100,
          height: 100,
        ),
      );
      await tester.pump();
      await tester.tapAt(const Offset(40, 40));
      expect(surfaceTaps, 1);
      expect(applicationTaps, 0);
      await window.update(const FloatingWindowOptions(clickThrough: true));
      await tester.pump();
      await tester.tapAt(const Offset(40, 40));
      expect(surfaceTaps, 1);
      expect(applicationTaps, 1);
      await window.hide();
      await tester.pump();
      expect(find.text('surface'), findsNothing);
      await tester.tapAt(const Offset(40, 40));
      expect(applicationTaps, 2);
      await window.show();
      await window.update(const FloatingWindowOptions(clickThrough: false));
      await tester.pump();
      await tester.tapAt(const Offset(40, 40));
      expect(surfaceTaps, 2);
      await window.close();
      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      expect(await FloatingWindows.liveIds(), isEmpty);
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.iOS),
  );
}
