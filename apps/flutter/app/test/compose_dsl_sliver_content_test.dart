import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Creates mixed-height keyed items resembling role headings and conversation rows.
Map<String, Object?> _tree({int count = 1000, bool dialog = false}) => {
  'type': 'LazyColumn',
  'props': {'fillMaxWidth': true, 'paddingBottom': 16, 'spacing': 2},
  'children': [
    for (var index = 0; index < count; index++)
      {
        'type': 'Row',
        'props': {
          'key': 'chat-$index',
          'height': index % 10 == 0 ? 46 : 34,
          'fillMaxWidth': true,
        },
        'children': [
          {
            'type': 'Text',
            'props': {'text': 'Chat $index'},
          },
        ],
      },
    if (dialog)
      {
        'type': 'Dialog',
        'props': {'key': 'edit-chat'},
        'children': [
          {
            'type': 'Text',
            'props': {'text': 'Edit conversation'},
          },
        ],
      },
  ],
};

/// Uses the production renderer inside the same single viewport as host chrome.
Widget _surface(
  Map<String, Object?> tree,
  ScrollController controller, {
  String? error,
  double height = 400,
  List<String>? actions,
  Future<Object?> Function(String, [Object?])? onAction,
}) => MaterialApp(
  home: Scaffold(
    body: Center(
      child: SizedBox(
        width: 320,
        height: height,
        child: CustomScrollView(
          controller: controller,
          slivers: [
            const SliverToBoxAdapter(child: Text('Sidebar header')),
            buildComposeDslHostForTest(
              node: tree,
              contentBuilder: (context, content) => content.sliver,
              error: error,
              hostContext: ComposeDslWebViewHostContext(
                packageName: 'fixture.sliver',
                routeInstanceId: 'sliver-test',
                executionContextKey: 'sliver-test',
                dispatchAction:
                    onAction ??
                    (id, [payload]) async {
                      actions?.add(id);
                      return null;
                    },
                runtimeOptionsProvider: () => {},
              ),
            ),
            const SliverToBoxAdapter(child: Text('Sidebar footer')),
          ],
        ),
      ),
    ),
  ),
);

/// Counts mounted conversation labels, including the viewport's nearby cache.
int _mountedChats() => find
    .byWidgetPredicate(
      (widget) =>
          widget is Text && RegExp(r'^Chat \d+$').hasMatch(widget.data ?? ''),
    )
    .evaluate()
    .length;

/// Verifies viewport ownership, item virtualization, and independent modal lifetime.
void main() {
  setUpAll(ClientLogger.initialize);

  testWidgets(
    'variable sliver windows measure cached rows instead of dividing by an estimated height',
    (tester) async {
      final controller = ScrollController();
      final range = ValueNotifier((firstIndex: 0, lastIndex: 24));
      addTearDown(controller.dispose);
      addTearDown(range.dispose);
      var requests = 0;
      await tester.pumpWidget(
        ValueListenableBuilder(
          valueListenable: range,
          builder: (context, value, _) => _surface(
            {
              'type': 'LazyColumn',
              'props': {
                'viewport': {
                  'firstIndex': value.firstIndex,
                  'itemCount': 1000,
                  'itemExtent': 22,
                  'variableExtent': true,
                  'contentRevision': 0,
                  'onRangeChanged': {'__actionId': 'range'},
                },
              },
              'children': [
                for (
                  var index = value.firstIndex;
                  index < value.lastIndex;
                  index++
                )
                  {
                    'type': 'Text',
                    'props': {
                      'key': 'row-$index',
                      'text': 'Chat $index',
                      'height': index % 7 == 0 ? 96 : 22,
                    },
                  },
              ],
            },
            controller,
            onAction: (id, [payload]) async {
              requests++;
              final values = payload! as Map;
              range.value = (
                firstIndex: values['firstIndex'] as int,
                lastIndex: values['lastIndex'] as int,
              );
              return null;
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Chat 0'), findsOneWidget);
      controller.jumpTo(8000);
      await tester.pumpAndSettle();
      expect(range.value.firstIndex, greaterThan(100));
      expect(range.value.lastIndex - range.value.firstIndex, lessThan(60));
      for (var pass = 0; pass < 4; pass++) {
        controller.jumpTo(controller.position.maxScrollExtent);
        await tester.pumpAndSettle();
      }
      expect(find.text('Chat 999'), findsOneWidget);
      controller.jumpTo(0);
      await tester.pumpAndSettle();
      expect(find.text('Chat 0'), findsOneWidget);
      expect(requests, lessThan(20));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'the sliver cache is not buffered twice before requesting more JS rows',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final ranges = <Map>[];
      final tree = <String, Object?>{
        'type': 'LazyColumn',
        'props': {
          'viewport': {
            'firstIndex': 0,
            'itemCount': 1000,
            'itemExtent': 34,
            'staticItemCount': 2,
            'onRangeChanged': {'__actionId': 'range'},
          },
        },
        'children': [
          for (var index = 0; index < 2; index++)
            {
              'type': 'Text',
              'props': {'text': 'Toolbar $index', 'height': 34},
            },
          for (var index = 0; index < 24; index++)
            {
              'type': 'Text',
              'props': {'key': 'row-$index', 'text': 'Chat $index'},
            },
        ],
      };
      await tester.pumpWidget(
        _surface(
          tree,
          controller,
          height: 600,
          onAction: (id, [payload]) async {
            ranges.add(payload! as Map);
            return null;
          },
        ),
      );
      await tester.pumpAndSettle();
      expect(
        ranges,
        isEmpty,
        reason: 'The initial rows already cover the viewport and Flutter cache',
      );
      expect(find.text('Chat 13'), findsOneWidget);
      controller.jumpTo(200);
      await tester.pumpAndSettle();
      expect(
        ranges,
        hasLength(1),
        reason: 'Scrolling must still refill before the visible rows run out',
      );
      expect(ranges.single['lastIndex'], greaterThan(24));
      expect(
        ranges.single['lastIndex'],
        lessThanOrEqualTo(32),
        reason: 'The requested cache range needs only a small refill margin',
      );
      expect(find.text('Chat 18'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'expansion retains the static toolbar and rows during async work',
    (tester) async {
      final controller = ScrollController();
      final expanded = ValueNotifier(false);
      final reply = Completer<void>();
      addTearDown(controller.dispose);
      addTearDown(expanded.dispose);
      final ranges = <Map>[];
      Map<String, Object?> tree() => {
        'type': 'LazyColumn',
        'props': {
          'key': 'sidebar',
          'viewport': {
            'firstIndex': 0,
            'itemCount': expanded.value ? 203 : 7,
            'itemExtent': 34,
            'staticItemCount': 1,
            'onRangeChanged': {'__actionId': 'range'},
          },
        },
        'children': [
          {
            'type': 'Text',
            'props': {'key': 'toolbar', 'text': '新建对话'},
          },
          for (var index = 0; index < (expanded.value ? 24 : 7); index++)
            {
              'type': 'Row',
              'props': {
                'key': 'chat-$index',
                'height': 34,
                if (index == 6) 'onClick': {'__actionId': 'expand'},
              },
              'children': [
                {
                  'type': 'Text',
                  'props': {'text': index == 6 ? '展开更多' : 'Chat $index'},
                },
              ],
            },
        ],
      };
      await tester.pumpWidget(
        ValueListenableBuilder<bool>(
          valueListenable: expanded,
          builder: (context, value, child) => _surface(
            tree(),
            controller,
            onAction: (id, [payload]) async {
              if (id == 'expand') {
                await reply.future;
                expanded.value = true;
              } else {
                ranges.add(payload! as Map);
              }
              return null;
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      final toolbar = tester.element(find.text('新建对话'));
      final firstRow = tester.element(find.text('Chat 0'));
      final position = controller.position;
      await tester.tap(find.text('展开更多'));
      await tester.pump(const Duration(milliseconds: 200));
      expect(tester.element(find.text('Chat 0')), same(firstRow));
      expect(tester.element(find.text('新建对话')), same(toolbar));
      expect(ranges, isEmpty);
      reply.complete();
      await tester.pumpAndSettle();
      expect(find.text('Chat 8'), findsOneWidget);
      expect(tester.element(find.text('Chat 0')), same(firstRow));
      expect(tester.element(find.text('新建对话')), same(toolbar));
      expect(controller.position, same(position));
      expect(
        ranges,
        isEmpty,
        reason: 'The expansion commit already covers the viewport',
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'sliver presentation preserves rows and root loading across action errors',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final actions = <String>[];
      final tree = _tree(count: 20);
      (tree['props'] as Map)['onLoad'] = {'__actionId': 'root-load'};
      await tester.pumpWidget(_surface(tree, controller, actions: actions));
      await tester.pumpAndSettle();
      final row = tester.element(find.text('Chat 0'));
      final position = controller.position;
      expect(actions, ['root-load']);

      await tester.pumpWidget(
        _surface(tree, controller, error: 'Action failed', actions: actions),
      );
      await tester.pumpAndSettle();
      expect(find.text('Action failed', skipOffstage: false), findsOneWidget);
      expect(tester.element(find.text('Chat 0')), same(row));
      expect(controller.position, same(position));

      await tester.pumpWidget(_surface(tree, controller, actions: actions));
      await tester.pumpAndSettle();
      expect(find.text('Action failed', skipOffstage: false), findsNothing);
      expect(tester.element(find.text('Chat 0')), same(row));
      expect(actions, ['root-load']);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('a thousand mixed-height rows mount only the visible cache', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(_surface(_tree(), controller));
    await tester.pumpAndSettle();
    expect(find.byType(Scrollable), findsOneWidget);
    expect(find.text('Chat 0'), findsOneWidget);
    expect(find.text('Chat 999'), findsNothing);
    expect(_mountedChats(), inInclusiveRange(1, 40));
    expect(
      tester.state(
        find.ancestor(
          of: find.text('Sidebar header'),
          matching: find.byType(Scrollable),
        ),
      ),
      same(
        tester.state(
          find.ancestor(
            of: find.text('Sidebar footer', skipOffstage: false),
            matching: find.byType(Scrollable),
          ),
        ),
      ),
    );

    await tester.scrollUntilVisible(
      find.text('Chat 999'),
      2500,
      scrollable: find.byType(Scrollable),
      maxScrolls: 30,
    );
    await tester.pumpAndSettle();
    expect(find.text('Chat 999'), findsOneWidget);
    expect(find.text('Chat 0'), findsNothing);
    expect(_mountedChats(), inInclusiveRange(1, 40));

    controller.jumpTo(0);
    await tester.pumpAndSettle();
    expect(find.text('Chat 0'), findsOneWidget);
    expect(find.text('Chat 999'), findsNothing);
    expect(_mountedChats(), inInclusiveRange(1, 40));
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'a trailing modal mounts without scrolling to its list position',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      await tester.pumpWidget(_surface(_tree(dialog: true), controller));
      await tester.pumpAndSettle();
      expect(find.text('Edit conversation'), findsOneWidget);
      expect(find.text('Chat 999'), findsNothing);
      expect(_mountedChats(), lessThan(40));

      controller.jumpTo(12000);
      await tester.pumpAndSettle();
      expect(find.text('Edit conversation'), findsOneWidget);
      await tester.pumpWidget(_surface(_tree(), controller));
      await tester.pumpAndSettle();
      expect(find.text('Edit conversation'), findsNothing);
      expect(controller.offset, 12000);
      expect(tester.takeException(), isNull);
    },
  );
}
