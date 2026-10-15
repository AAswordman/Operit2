import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Verifies action errors are visible without disposing and remounting the successful UI.
void main() {
  setUpAll(ClientLogger.initialize);
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
  testWidgets(
    'management child dialog loads once per mount while the page stays mounted',
    (tester) async {
      final actions = <String>[];
      final host = ComposeDslWebViewHostContext(
        packageName: 'regression',
        routeInstanceId: 'management',
        executionContextKey: 'management',
        dispatchAction: (id, [payload]) async {
          actions.add(id);
          return null;
        },
        runtimeOptionsProvider: () => {},
      );

      Future<void> show({
        bool dialog = false,
        bool loaded = false,
        String? error,
      }) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: buildComposeDslHostForTest(
                node: {
                  'type': 'Box',
                  'props': {
                    'onLoad': {'__actionId': 'load-management'},
                  },
                  'children': [
                    {
                      'type': 'Text',
                      'props': {'key': 'management', 'text': 'Management list'},
                    },
                    if (dialog)
                      {
                        'type': 'Dialog',
                        'props': {
                          'key': 'entity-dialog',
                          'onLoad': {'__actionId': 'load-entity'},
                        },
                        'children': [
                          {
                            'type': 'Text',
                            'props': {
                              'text': loaded
                                  ? 'Edit character'
                                  : 'Loading entity',
                            },
                          },
                        ],
                      },
                  ],
                },
                hostContext: host,
                error: error,
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
      }

      await show();
      final management = tester.element(find.text('Management list'));
      expect(actions, ['load-management']);

      await show(dialog: true);
      expect(actions, ['load-management', 'load-entity']);
      expect(find.text('Loading entity'), findsOneWidget);
      await show(dialog: true, loaded: true);
      expect(find.text('Edit character'), findsOneWidget);
      await show(
        dialog: true,
        loaded: true,
        error: 'A recoverable action error',
      );
      await show(dialog: true, loaded: true);
      expect(actions, ['load-management', 'load-entity']);

      await show();
      expect(tester.element(find.text('Management list')), same(management));
      await show(dialog: true);
      expect(actions, ['load-management', 'load-entity', 'load-entity']);
      await show();
      expect(tester.takeException(), isNull);
    },
  );
}
