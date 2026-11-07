import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/AdaptiveSidePanel.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceLayoutMetrics.dart';

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

  for (final width in <double>[402, 600, 768, 900, 1023, 1024, 1440]) {
    testWidgets('workspace adapts to host width $width in a wide window', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1600, 1200);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final open = ValueNotifier<bool>(true);
      addTearDown(open.dispose);
      const contentKey = ValueKey('chat');
      const panelKey = ValueKey('workspace');
      const toolbarKey = ValueKey('original-toolbar');
      const toggleKey = ValueKey('original-workspace-toggle');
      const toolbarHeight = 64.0;
      final overlay = width < workspaceSideBySideBreakpoint;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: width,
                child: ValueListenableBuilder<bool>(
                  valueListenable: open,
                  builder: (context, value, _) => AdaptiveSidePanel(
                    open: value,
                    onOpenChanged: (value) => open.value = value,
                    animate: false,
                    breakpoint: workspaceSideBySideBreakpoint,
                    defaultWidth: workspaceDefaultTabletWidth,
                    defaultWidthFraction: workspaceDefaultWidthFraction,
                    minWidth: workspaceMinWidth,
                    minContentWidth: workspaceMinTabletChatWidth,
                    widePanelPadding: EdgeInsets.zero,
                    overlayTopInset: toolbarHeight,
                    panel: const ColoredBox(key: panelKey, color: Colors.blue),
                    child: Column(
                      key: contentKey,
                      children: [
                        SizedBox(
                          key: toolbarKey,
                          height: toolbarHeight,
                          child: Row(
                            children: [
                              const Expanded(child: Text('Conversation title')),
                              IconButton(
                                key: toggleKey,
                                onPressed: () => open.value = !open.value,
                                icon: const Icon(Icons.view_sidebar_outlined),
                              ),
                            ],
                          ),
                        ),
                        const Expanded(child: SizedBox.expand()),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final host = tester.getRect(find.byType(AdaptiveSidePanel));
      final content = tester.getRect(find.byKey(contentKey));
      final panel = tester.getRect(find.byKey(panelKey));
      expect(host.width, width);
      expect(panel.right, host.right);
      expect(panel.bottom, host.bottom);
      expect(content.width, overlay ? width : width - panel.width);
      if (overlay) {
        expect(panel.width, width);
        expect(panel.top, tester.getRect(find.byKey(toolbarKey)).bottom);
        expect(find.byKey(toolbarKey), findsOneWidget);
        expect(find.byKey(toggleKey).hitTestable(), findsOneWidget);
        // The original toolbar stays visible above both panel and barrier.
        await tester.tap(find.byKey(toggleKey));
        await tester.pumpAndSettle();
        expect(open.value, isFalse);
        expect(tester.getRect(find.byKey(contentKey)).width, width);
        await tester.tap(find.byKey(toggleKey));
        await tester.pumpAndSettle();
        expect(open.value, isTrue);
        expect(tester.getRect(find.byKey(panelKey)).top, toolbarHeight);
      } else {
        expect(
          panel.width,
          closeTo(width * workspaceDefaultWidthFraction, 0.001),
        );
        expect(panel.top, host.top);
        expect(find.byKey(toolbarKey), findsOneWidget);
        // The desktop divider remains draggable.
        final gesture = await tester.startGesture(
          Offset(panel.left, panel.center.dy),
        );
        await gesture.moveBy(const Offset(-80, 0));
        await tester.pump();
        await gesture.up();
        await tester.pumpAndSettle();
        expect(
          tester.getRect(find.byKey(panelKey)).width,
          greaterThan(panel.width),
        );
      }
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  testWidgets('flush panel uses its full width fraction without an inset', (
    tester,
  ) async {
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
            widePanelPadding: EdgeInsets.zero,
            panel: const SizedBox(key: panelKey),
            child: const SizedBox.expand(),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final panelRect = tester.getRect(find.byKey(panelKey));
    expect(panelRect.width, closeTo(screenWidth * 0.45, 0.001));
    expect(panelRect.top, 0);
    expect(panelRect.bottom, 800);
    expect(panelRect.right, screenWidth);
  });
}
