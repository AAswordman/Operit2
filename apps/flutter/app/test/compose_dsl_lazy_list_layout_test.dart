import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Builds a Compose DSL tree using the production Flutter renderer.
Widget _render(Map<String, Object?> node) => buildComposeDslLayoutForTest(
  node: node,
  onTextInput: (id, text) async => null,
  hostContext: ComposeDslWebViewHostContext(
    packageName: 'com.operit.worldbook',
    routeInstanceId: 'worldbook-layout-test',
    executionContextKey: 'worldbook-layout-test',
    dispatchAction: (id, [payload]) async => null,
    runtimeOptionsProvider: () => <String, Object?>{},
  ),
  splitMarkdownContent: (_) async => [],
);

/// Creates a control with a natural height inside a horizontal lazy list.
Map<String, Object?> _control(String key, double height) => {
  'type': 'Box',
  'props': {'key': key, 'width': 56, 'height': height},
};

/// Builds the shared create/edit form structure that exposed the regression.
Map<String, Object?> _worldbookForm() => {
  'type': 'Column',
  'props': {'fillMaxWidth': true, 'spacing': 12},
  'children': [
    {
      'type': 'Card',
      'props': {'fillMaxWidth': true},
      'children': [
        {
          'type': 'Column',
          'props': {'fillMaxWidth': true, 'spacing': 12, 'padding': 16},
          'children': [
            {
              'type': 'Text',
              'props': {'text': '基础信息'},
            },
            {
              'type': 'TextField',
              'props': {'value': '', 'singleLine': true, 'fillMaxWidth': true},
            },
            {
              'type': 'LazyRow',
              'props': {'fillMaxWidth': true, 'spacing': 8},
              'children': [
                _control('target-system', 34),
                _control('target-user', 42),
                _control('target-assistant', 30),
              ],
            },
          ],
        },
      ],
    },
    {
      'type': 'Card',
      'props': {'fillMaxWidth': true},
      'children': [
        {
          'type': 'Column',
          'props': {'fillMaxWidth': true, 'padding': 16},
          'children': [
            {
              'type': 'Text',
              'props': {'text': '匹配与启用'},
            },
            {
              'type': 'TextField',
              'props': {'value': '', 'singleLine': true, 'fillMaxWidth': true},
            },
          ],
        },
      ],
    },
    {
      'type': 'Card',
      'props': {'fillMaxWidth': true},
      'children': [
        {
          'type': 'Column',
          'props': {'fillMaxWidth': true, 'padding': 16},
          'children': [
            {
              'type': 'Text',
              'props': {'text': '注入策略'},
            },
            {
              'type': 'LazyRow',
              'props': {'fillMaxWidth': true, 'spacing': 8},
              'children': [
                _control('position-prepend', 28),
                _control('position-append', 38),
                _control('position-depth', 32),
              ],
            },
          ],
        },
      ],
    },
  ],
};

/// Verifies the actual worldbook form hierarchy remains vertically laid out.
void main() {
  for (final view in ['create', 'edit']) {
    testWidgets('$view worldbook form keeps cards and lazy rows laid out', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(width: 360, child: _render(_worldbookForm())),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.byType(Card), findsNWidgets(3));
      final cards = find.byType(Card);
      for (var index = 0; index < cards.evaluate().length - 1; index++) {
        expect(
          tester.getRect(cards.at(index)).bottom,
          lessThan(tester.getRect(cards.at(index + 1)).top),
        );
      }
      final rows = find.byWidgetPredicate(
        (widget) =>
            widget is ListView && widget.scrollDirection == Axis.horizontal,
      );
      expect(rows, findsNWidgets(2));
      for (var index = 0; index < 2; index++) {
        final size = tester.getSize(rows.at(index));
        expect(size.isFinite, isTrue);
        expect(size.height, greaterThan(0));
      }
    });
  }
}
