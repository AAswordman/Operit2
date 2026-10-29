import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

void main() {
  for (final enabled in [false, true]) {
    for (final hasAction in [false, true]) {
      testWidgets('dropdown enabled=$enabled action=$hasAction', (
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
                    'type': 'Select',
                    'props': {
                      'enabled': enabled,
                      'value': 'a',
                      'options': [
                        {'value': 'a', 'label': 'Alpha'},
                        {'value': 'b', 'label': 'Beta'},
                      ],
                      if (hasAction) 'onValueChange': {'__actionId': 'select'},
                    },
                  },
                  hostContext: ComposeDslWebViewHostContext(
                    packageName: 'test.selection',
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
        await tester.pumpAndSettle();
        final dropdown = tester.widget<DropdownMenu<String>>(
          find.byType(DropdownMenu<String>),
        );
        expect(dropdown.enabled, enabled && hasAction);
        if (enabled && hasAction) {
          await tester.tap(find.byType(TextField));
          await tester.pumpAndSettle();
          await tester.tap(find.text('Beta').last);
          await tester.pumpAndSettle();
          expect(events, [('select', 'b')]);
        } else {
          expect(dropdown.onSelected, isNull);
          expect(
            tester.widget<TextField>(find.byType(TextField)).enabled,
            isFalse,
          );
          expect(events, isEmpty);
        }
        expect(tester.takeException(), isNull);
      });
    }
  }
}
