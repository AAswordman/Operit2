import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

ToolPkgComposeDslNodeRecord record(
  String id,
  String type, {
  Map<String, Object?> props = const {},
  List<String> children = const [],
}) => ToolPkgComposeDslNodeRecord(
  id: id,
  nodeType: type,
  props: {'key': id, ...props},
  children: children,
  slots: const {},
);

void main() {
  setUpAll(ClientLogger.initialize);

  testWidgets(
    'long press then pin closes a dirty retained dialog without using disposed nodes',
    (tester) async {
      late void Function(ToolPkgComposeDslNodeUpdate) apply;
      late VoidCallback dispose;
      var revision = 0;
      var pinned = false;
      final actions = <String>[];
      final modal = [
        record('dialog', 'Dialog', children: ['dialog-content']),
        record(
          'dialog-content',
          'Column',
          props: {'width': 280, 'height': 120},
          children: ['pin'],
        ),
        record(
          'pin',
          'TextButton',
          props: {
            'text': '置顶',
            'onClick': {'__actionId': 'pin'},
          },
        ),
      ];
      ToolPkgComposeDslNodeRecord root({bool dialog = false}) =>
          record('root', 'Column', children: ['chat', if (dialog) 'dialog']);
      ToolPkgComposeDslNodeUpdate update(
        List<ToolPkgComposeDslNodeRecord> records, [
        List<String> removed = const [],
      ]) => ToolPkgComposeDslNodeUpdate(
        reset: revision == 0,
        revision: ++revision,
        rootId: 'root',
        upserts: records,
        removed: removed,
      );
      final layout = buildRetainedComposeDslLayoutForTest(
        update: update([
          root(),
          record(
            'chat',
            'Row',
            props: {
              'height': 34,
              'modifier': {
                '__modifierOps': [
                  {
                    'name': 'combinedClickable',
                    'args': [
                      {
                        'onLongClick': {'__actionId': 'open'},
                      },
                    ],
                  },
                ],
              },
            },
            children: ['title'],
          ),
          record('title', 'Text', props: {'text': 'Conversation'}),
        ]),
        hostContext: ComposeDslWebViewHostContext(
          packageName: 'regression',
          routeInstanceId: 'sidebar',
          executionContextKey: 'sidebar',
          dispatchAction: (id, [payload]) async {
            actions.add(id);
            if (id == 'open') {
              apply(update([root(dialog: true), ...modal]));
            } else if (id == 'pin') {
              // Busy-state update queues the modal content for a rebuild. The pin
              // completes and removes it before the next frame consumes that work.
              apply(
                update([
                  record(
                    'dialog-content',
                    'Column',
                    props: {'width': 280, 'height': 120},
                    children: ['pin'],
                  ),
                ]),
              );
              pinned = true;
              apply(update([root()], ['pin', 'dialog-content', 'dialog']));
            }
            return null;
          },
          runtimeOptionsProvider: () => {},
        ),
        registerDispose: (callback) => dispose = callback,
        registerApply: (callback) => apply = callback,
      );
      await tester.pumpWidget(MaterialApp(home: Scaffold(body: layout)));
      await tester.longPress(find.text('Conversation'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.text('置顶'), findsOneWidget);
      await tester.runAsync(ClientLogger.clear);
      await tester.tap(find.text('置顶'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(actions, ['open', 'pin']);
      expect(pinned, isTrue);
      expect(find.byType(Dialog), findsNothing);
      expect(find.text('Conversation'), findsOneWidget);
      expect(tester.takeException(), isNull);
      expect(
        await tester.runAsync(ClientLogger.readText),
        isNot(contains('used after being disposed')),
      );
      await tester.pumpWidget(const SizedBox.shrink());
      dispose();
    },
  );
}
