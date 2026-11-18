import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

Map<String, Object?> node(
  String type,
  Map<String, Object?> props, [
  List<Map<String, Object?>> children = const [],
]) => {'type': type, 'props': props, 'children': children};

Widget render(Map<String, Object?> tree) => Material(
  child: buildComposeDslLayoutForTest(
    node: tree,
    onTextInput: (_, _) async => null,
    hostContext: ComposeDslWebViewHostContext(
      packageName: 'test.profile',
      routeInstanceId: 'profile',
      executionContextKey: 'profile',
      dispatchAction: (_, [payload]) async => null,
      runtimeOptionsProvider: () => {},
    ),
    splitMarkdownContent: (_) async => [],
  ),
);

void main() {
  for (final width in [340.0, 740.0]) {
    testWidgets('multiline profile fills remaining dialog space at $width', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Center(
            child: SizedBox(
              width: width,
              height: 500,
              child: render(
                node(
                  'Column',
                  {'fillMaxSize': true},
                  [
                    node('Text', {'text': '用户资料'}),
                    node('TextField', {
                      'key': 'profile',
                      'value': '用户资料正文',
                      'singleLine': false,
                      'fillMaxSize': true,
                      'weight': 1,
                      'onValueChange': {'__actionId': 'edit'},
                    }),
                    node('Text', {'text': '保存'}),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      expect(tester.takeException(), isNull);
      final field = tester.widget<TextField>(find.byType(TextField));
      expect(field.expands, isTrue);
      expect(field.maxLines, isNull);
      expect(field.minLines, isNull);
      final rect = tester.getRect(find.byType(TextField));
      expect(rect.width, width);
      expect(rect.height, greaterThan(440));
      expect(
        rect.bottom,
        lessThanOrEqualTo(tester.getRect(find.text('保存')).top),
      );
    });
  }

  testWidgets('fill multiline field remains finite inside a scrolling form', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: SingleChildScrollView(
          child: render(
            node('TextField', {
              'value': '资料',
              'fillMaxSize': true,
              'minLines': 3,
              'onValueChange': {'__actionId': 'edit'},
            }),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(tester.takeException(), isNull);
    expect(tester.widget<TextField>(find.byType(TextField)).expands, isFalse);
    expect(tester.getSize(find.byType(TextField)).height.isFinite, isTrue);
  });
}
