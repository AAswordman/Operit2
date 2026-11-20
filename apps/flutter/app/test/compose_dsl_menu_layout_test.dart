import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Mounts the real Compose renderer under the same constraints as a workspace tab.
Future<void> mountMenu(
  WidgetTester tester,
  Map<String, Object?> tree,
  List<(String, Object?)> actions, {
  ValueChanged<String>? onAction,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: buildComposeDslLayoutForTest(
          node: tree,
          hostContext: ComposeDslWebViewHostContext(
            packageName: 'com.operit.change_review',
            routeInstanceId: 'menu-regression',
            executionContextKey: 'menu-regression',
            dispatchAction: (id, [payload]) async {
              actions.add((id, payload));
              onAction?.call(id);
              return null;
            },
            runtimeOptionsProvider: () => {},
          ),
          splitMarkdownContent: (_) async => [],
          onTextInput: (_, _) async => null,
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
}

/// Reproduces the market plugin's own button and separately controlled popup.
Map<String, Object?> controlledMenu(bool expanded) => {
  'type': 'LazyColumn',
  'props': {'padding': 16},
  'children': [
    {
      'type': 'Box',
      'props': {'fillMaxWidth': true},
      'children': [
        {
          'type': 'OutlinedButton',
          'props': {
            'text': '全局默认',
            'fillMaxWidth': true,
            'onClick': {'__actionId': 'toggle'},
          },
        },
        {
          'type': 'DropdownMenu',
          'props': {
            'expanded': expanded,
            'onDismissRequest': {'__actionId': 'dismiss'},
            'properties': {'focusable': true},
          },
          'children': [
            for (var index = 0; index < 30; index++)
              {
                'type': 'Box',
                'props': {
                  'fillMaxWidth': true,
                  'modifier': {
                    '__modifierOps': [
                      {
                        'name': 'clickable',
                        'args': [
                          {'__actionId': 'choose-$index'},
                        ],
                      },
                      {
                        'name': 'padding',
                        'args': [16, 12],
                      },
                    ],
                  },
                },
                'children': [
                  {
                    'type': 'Text',
                    'props': {'text': '角色 $index'},
                  },
                ],
              },
          ],
        },
      ],
    },
    {
      'type': 'Text',
      'props': {'text': '表情包路径'},
    },
  ],
};

/// Covers intrinsic measurement used by Material popup menus and unbounded rows.
void main() {
  testWidgets('controlled dropdown uses only the plugin trigger', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(390, 640));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final actions = <(String, Object?)>[];
    var expanded = false;

    /// Applies the plugin's toggle, selection, and dismissal state transitions.
    Future<void> render() => mountMenu(
      tester,
      controlledMenu(expanded),
      actions,
      onAction: (id) => expanded = id == 'toggle',
    );

    await render();
    final trigger = find.byType(OutlinedButton);
    final nextSection = find.text('表情包路径');
    final triggerBounds = tester.getRect(trigger);
    final sectionPosition = tester.getTopLeft(nextSection);
    expect(find.byType(InputDecorator), findsNothing);
    expect(find.text('角色 0'), findsNothing);

    await tester.tap(trigger);
    expect(actions.single.$1, 'toggle');
    await render();
    expect(tester.getRect(trigger), triggerBounds);
    expect(tester.getTopLeft(nextSection), sectionPosition);
    expect(find.byType(InputDecorator), findsNothing);
    final menuBounds = tester.getRect(find.byType(SingleChildScrollView));
    expect(menuBounds.left, triggerBounds.left);
    expect(menuBounds.width, triggerBounds.width);
    expect(menuBounds.top, greaterThan(triggerBounds.bottom));
    expect(menuBounds.bottom, lessThanOrEqualTo(640));

    await tester.drag(
      find.byType(SingleChildScrollView),
      const Offset(0, -500),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('角色 17'));
    expect(actions.last.$1, 'choose-17');
    await render();
    expect(find.byType(SingleChildScrollView), findsNothing);

    await tester.tap(trigger);
    await render();
    await tester.tapAt(const Offset(8, 200));
    expect(actions.last.$1, 'dismiss');
    await render();

    await tester.tap(trigger);
    await render();
    await tester.binding.handlePopRoute();
    expect(actions.last.$1, 'dismiss');
    await render();
    expect(trigger, findsOneWidget);
    expect(find.text('角色 0'), findsNothing);
  });

  testWidgets(
    'indexed dropdown keeps its own trigger and dispatches a choice',
    (tester) async {
      final actions = <(String, Object?)>[];
      await mountMenu(tester, {
        'type': 'DropdownMenu',
        'props': {
          'label': 'Profile',
          'text': 'Global',
          'onClick': {'__actionId': 'select'},
        },
        'children': [
          {
            'type': 'Text',
            'props': {'text': 'Character'},
          },
        ],
      }, actions);
      expect(find.byType(InputDecorator), findsOneWidget);
      await tester.tap(find.text('Global'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Character'));
      await tester.pumpAndSettle();
      expect(actions, [('select', 0)]);
    },
  );

  final workspace = Map<String, Object?>.from(
    (jsonDecode(
              File(
                'test/fixtures/change_review_workspace.json',
              ).readAsStringSync(),
            )
            as Map)['light-unified']
        as Map,
  );
  for (final menu in [('切换变更对比源', '分支变更', 1), ('选择打开方式', '终端 (Terminal)', 3)]) {
    testWidgets('workspace popup measures composed rows: ${menu.$1}', (
      tester,
    ) async {
      final actions = <(String, Object?)>[];
      await mountMenu(tester, workspace, actions);
      await tester.tap(find.byTooltip(menu.$1));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text(menu.$2), findsOneWidget);
      await tester.tap(find.text(menu.$2));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(actions.where((entry) => entry.$2 == menu.$3), hasLength(1));
      expect(find.text(menu.$2), findsNothing);
    });
  }

  testWidgets('nested composed rows support intrinsic and unbounded width', (
    tester,
  ) async {
    final tree = <String, Object?>{
      'type': 'Row',
      'children': [
        {
          'type': 'Icon',
          'props': {'name': 'folder', 'size': 14},
        },
        {
          'type': 'Spacer',
          'props': {'width': 8},
        },
        {
          'type': 'Row',
          'children': [
            {
              'type': 'Text',
              'props': {'text': 'Natural width'},
            },
            {
              'type': 'Text',
              'props': {'text': ' nested text'},
            },
          ],
        },
      ],
    };
    for (final intrinsic in [true, false]) {
      final child = buildComposeDslLayoutForTest(
        node: tree,
        hostContext: ComposeDslWebViewHostContext(
          packageName: 'row-regression',
          routeInstanceId: 'row',
          executionContextKey: 'row',
          dispatchAction: (_, [payload]) async => null,
          runtimeOptionsProvider: () => {},
        ),
        splitMarkdownContent: (_) async => [],
        onTextInput: (_, _) async => null,
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Align(
              alignment: Alignment.topLeft,
              child: intrinsic
                  ? IntrinsicWidth(child: child)
                  : SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: child,
                    ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('Natural width'), findsOneWidget);
      expect(tester.getSize(find.text('Natural width')).width, greaterThan(0));
    }
  });
}
