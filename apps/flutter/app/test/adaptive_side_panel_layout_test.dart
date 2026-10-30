import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/AdaptiveSidePanel.dart';

void main() {
  for (final width in <double>[402, 900]) {
    for (final open in <bool>[false, true]) {
      testWidgets('fills available height at width $width, open: $open', (
        tester,
      ) async {
        tester.view.physicalSize = Size(width, 874);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        const contentKey = ValueKey('content');
        const inputKey = ValueKey('input');
        // Match the chat's positioned body plus a small non-positioned toast.
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: AdaptiveSidePanel(
                open: open,
                onOpenChanged: (_) {},
                animate: false,
                panel: const ColoredBox(color: Colors.blue),
                child: Stack(
                  key: contentKey,
                  children: [
                    Positioned.fill(
                      child: Column(
                        children: [
                          const Expanded(child: SizedBox()),
                          Container(key: inputKey, height: 112),
                        ],
                      ),
                    ),
                    const SizedBox(height: 172),
                  ],
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final host = tester.getRect(find.byType(AdaptiveSidePanel));
        final content = tester.getRect(find.byKey(contentKey));
        expect(content.top, host.top);
        expect(content.height, host.height);
        expect(tester.getRect(find.byKey(inputKey)).bottom, host.bottom);
        expect(tester.takeException(), isNull);
      });
    }
  }

  testWidgets('applies defaultWidthFraction on wide layout', (tester) async {
    const double screenWidth = 1000;
    tester.view.physicalSize = const Size(screenWidth, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    const panelKey = ValueKey('panel');
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AdaptiveSidePanel(
            open: true,
            onOpenChanged: (_) {},
            animate: false,
            defaultWidth: 360,
            defaultWidthFraction: 0.45,
            panel: const SizedBox(key: panelKey),
            child: const SizedBox.expand(),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final panelRect = tester.getRect(find.byKey(panelKey));
    expect(panelRect.width, closeTo(screenWidth * 0.45, 0.001));
  });
}
