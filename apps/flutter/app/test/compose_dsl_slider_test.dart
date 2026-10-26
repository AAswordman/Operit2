import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Verifies the actual Material slider and numeric action payloads used by plugin settings.
void main() {
  testWidgets('native slider emits changed and completed numeric values', (
    tester,
  ) async {
    final events = <(String, Object?)>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 320,
            child: buildComposeDslLayoutForTest(
              node: {
                'type': 'Slider',
                'props': {
                  'value': 250,
                  'min': 200,
                  'max': 1000,
                  'divisions': 40,
                  'onValueChange': {'__actionId': 'changed'},
                  'onValueChangeFinished': {'__actionId': 'finished'},
                },
                'children': [],
                'slots': {},
              },
              hostContext: ComposeDslWebViewHostContext(
                packageName: 'generic.slider',
                routeInstanceId: 'test',
                executionContextKey: 'test',
                dispatchAction: (id, [payload]) async {
                  events.add((id, payload));
                  return null;
                },
                runtimeOptionsProvider: () => {},
              ),
              onTextInput: (_, _) async => null,
              splitMarkdownContent: (_) async => [],
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.byType(Slider), findsOneWidget);
    await tester.tapAt(tester.getCenter(find.byType(Slider)));
    await tester.pump();
    expect(events.where((event) => event.$1 == 'changed'), isNotEmpty);
    expect(events.where((event) => event.$1 == 'finished'), isNotEmpty);
    for (final event in events) {
      expect(event.$2, isA<num>());
      expect((event.$2 as num), inInclusiveRange(200, 1000));
    }
    expect(tester.takeException(), isNull);
  });
}
