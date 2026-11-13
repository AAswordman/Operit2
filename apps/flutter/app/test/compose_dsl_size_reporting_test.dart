import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Verifies that viewport callbacks follow layout changes without JS recomposition.
void main() {
  testWidgets('size observer follows wide and narrow parent constraints', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final extent = ValueNotifier(const Size(1100, 720));
    addTearDown(extent.dispose);
    final sizes = <Size>[];
    final content = buildComposeDslLayoutForTest(
      node: {
        'type': 'LazyColumn',
        'props': {
          'fillMaxSize': true,
          'modifier': {
            '__modifierOps': [
              {
                'name': 'onSizeChanged',
                'args': [
                  {'__actionId': 'viewport-size'},
                ],
              },
            ],
          },
        },
        'children': [
          {
            'type': 'Text',
            'props': {'text': 'Retained plugin page'},
          },
        ],
      },
      onTextInput: (_, _) async => null,
      hostContext: ComposeDslWebViewHostContext(
        packageName: 'test.responsive',
        routeInstanceId: 'viewport-test',
        executionContextKey: 'viewport-test',
        dispatchAction: (id, [payload]) async {
          if (id == 'viewport-size') {
            final value = payload! as Map;
            sizes.add(
              Size(
                (value['width'] as num).toDouble(),
                (value['height'] as num).toDouble(),
              ),
            );
          }
          return null;
        },
        runtimeOptionsProvider: () => {},
      ),
      splitMarkdownContent: (_) async => [],
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: ValueListenableBuilder<Size>(
              valueListenable: extent,
              child: content,
              builder: (_, size, child) =>
                  SizedBox.fromSize(size: size, child: child),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(sizes, [const Size(1100, 720)]);
    extent.value = const Size(360, 720);
    await tester.pump();
    await tester.pump();
    expect(sizes.last, const Size(360, 720));
    extent.value = const Size(1100, 720);
    await tester.pump();
    await tester.pump();
    expect(sizes, [
      const Size(1100, 720),
      const Size(360, 720),
      const Size(1100, 720),
    ]);
    await tester.pump();
    expect(sizes.length, 3);
    expect(tester.takeException(), isNull);
  });
}
