import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Verifies action errors are visible without disposing and remounting the successful UI.
void main() {
  testWidgets(
    'action error preserves mounted renderer and does not rerun onLoad',
    (tester) async {
      final actions = <String>[];
      final host = ComposeDslWebViewHostContext(
        packageName: 'regression',
        routeInstanceId: 'page',
        executionContextKey: 'page',
        dispatchAction: (id, [payload]) async {
          actions.add(id);
          return null;
        },
        runtimeOptionsProvider: () => {},
      );
      const tree = <String, Object?>{
        'type': 'Box',
        'props': {
          'onLoad': {'__actionId': 'load'},
        },
        'children': [
          {
            'type': 'Text',
            'props': {'text': 'Mounted content'},
            'children': [],
          },
        ],
      };

      /// Updates only the host error while keeping the same page element mounted.
      Future<void> show(String? error) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: buildComposeDslHostForTest(
                node: tree,
                hostContext: host,
                error: error,
              ),
            ),
          ),
        );
        await tester.pump();
      }

      await show(null);
      final content = tester.element(find.text('Mounted content'));
      expect(actions, ['load']);
      for (var index = 0; index < 3; index++) {
        await show('compose action not found: __action_3');
        expect(
          find.text('compose action not found: __action_3'),
          findsOneWidget,
        );
        expect(tester.element(find.text('Mounted content')), same(content));
        await show(null);
        expect(tester.element(find.text('Mounted content')), same(content));
      }
      expect(actions, ['load']);
      expect(tester.takeException(), isNull);
    },
  );
}
