import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

// This suite also runs in a real browser: flutter test --platform chrome
// test/application_windows_test.dart. It needs no native method-channel mocks.
void main() {
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
