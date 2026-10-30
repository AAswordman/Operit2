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
  List<(String, Object?)> actions,
) async {
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

/// Covers intrinsic measurement used by Material popup menus and unbounded rows.
void main() {
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
