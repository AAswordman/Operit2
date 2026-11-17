import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

Map<String, Object?> node(
  String type,
  Map<String, Object?> props, [
  List<Map<String, Object?>> children = const [],
]) => {'type': type, 'props': props, 'children': children};

Future<void> mount(WidgetTester tester, Map<String, Object?> tree) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 320,
            child: buildComposeDslLayoutForTest(
              node: tree,
              hostContext: ComposeDslWebViewHostContext(
                packageName: 'test.legacy.row',
                routeInstanceId: 'layout-test',
                executionContextKey: 'layout-test',
                dispatchAction: (id, [payload]) async => null,
                runtimeOptionsProvider: () => {},
              ),
              onTextInput: (_, _) async => null,
              splitMarkdownContent: (_) async => [],
            ),
          ),
        ),
      ),
    ),
  );
  expect(tester.takeException(), isNull);
}

void main() {
  for (final height in [48.0, 80.0]) {
    for (final useSlot in [false, true]) {
      testWidgets(
        'single card row centers vertically and fills remaining width: height=$height slot=$useSlot',
        (tester) async {
          final row = node(
            'Row',
            {
              'mainAxisSize': 'max',
              'verticalAlignment': 'center',
              'spacing': 8,
              'padding': {'horizontal': 12, 'vertical': 6},
            },
            [
              node('Icon', {'name': 'play_arrow', 'size': 16}),
              node(
                'Row',
                {
                  'key': 'wave',
                  'mainAxisSize': 'max',
                  'mainAxisAlignment': 'spaceBetween',
                  'verticalAlignment': 'center',
                },
                [
                  for (var i = 0; i < 6; i++)
                    node('Box', {'key': 'bar-$i', 'width': 2, 'height': 12}),
                ],
              ),
              node('Text', {'text': "1' 32\"", 'fontSize': 11}),
            ],
          );
          await mount(tester, {
            'type': 'Card',
            'props': {'width': 260, 'height': height},
            if (useSlot)
              'slots': {
                'content': [row],
              },
            if (!useSlot) 'children': [row],
          });
          final card = tester.getRect(find.byType(Card));
          final icon = tester.getRect(find.byType(Icon));
          final text = tester.getRect(find.text("1' 32\""));
          final wave = tester.getRect(
            find.byKey(const ValueKey<Object?>('wave')),
          );
          expect(icon.center.dy, closeTo(card.center.dy, .01));
          expect(text.center.dy, closeTo(card.center.dy, .01));
          expect(icon.left - card.left, closeTo(12, .01));
          expect(card.right - text.right, closeTo(12, .01));
          expect(wave.left - icon.right, closeTo(8, .01));
          expect(text.left - wave.right, closeTo(8, .01));
          final first = tester.getRect(
            find.byKey(const ValueKey<Object?>('bar-0')),
          );
          final last = tester.getRect(
            find.byKey(const ValueKey<Object?>('bar-5')),
          );
          expect(first.left, closeTo(wave.left, .01));
          expect(last.right, closeTo(wave.right, .01));
        },
      );
    }
  }

  testWidgets('ordinary single card content keeps its declared size', (
    tester,
  ) async {
    await mount(
      tester,
      node(
        'Card',
        {'width': 200, 'height': 80},
        [
          node('Box', {'key': 'ordinary', 'width': 40, 'height': 20}),
        ],
      ),
    );
    expect(
      tester.getSize(find.byKey(const ValueKey<Object?>('ordinary'))),
      const Size(40, 20),
    );
  });

  testWidgets('a weighted legacy row keeps its column flex scope', (
    tester,
  ) async {
    await mount(
      tester,
      node(
        'Card',
        {'width': 200, 'height': 80},
        [
          node(
            'Row',
            {
              'key': 'weighted',
              'mainAxisSize': 'max',
              'weight': 1,
              'verticalAlignment': 'center',
            },
            [
              node('Icon', {'name': 'play_arrow', 'size': 16}),
            ],
          ),
        ],
      ),
    );
    final card = tester.getRect(find.byType(Card));
    final row = tester.getRect(find.byKey(const ValueKey<Object?>('weighted')));
    final icon = tester.getRect(find.byType(Icon));
    expect(row.height, closeTo(card.height, .01));
    expect(icon.center.dy, closeTo(card.center.dy, .01));
  });

  for (final canonical in [false, true]) {
    testWidgets(
      'row alignment aliases respect canonical precedence=$canonical',
      (tester) async {
        await mount(
          tester,
          node(
            'Row',
            {
              'width': 240,
              'height': 40,
              'mainAxisSize': 'max',
              'mainAxisAlignment': 'center',
              if (canonical) 'horizontalArrangement': 'end',
              'verticalAlignment': 'center',
              'spacing': 6,
            },
            [
              node('Box', {'key': 'first', 'width': 20, 'height': 10}),
              node('Box', {'key': 'last', 'width': 40, 'height': 10}),
            ],
          ),
        );
        final first = tester.getRect(
          find.byKey(const ValueKey<Object?>('first')),
        );
        final last = tester.getRect(
          find.byKey(const ValueKey<Object?>('last')),
        );
        expect(last.left - first.right, closeTo(6, .01));
        final row = tester.getRect(
          find.byWidgetPredicate((widget) => widget is Flex),
        );
        final contentWidth = first.width + 6 + last.width;
        expect(
          first.left - row.left,
          closeTo((row.width - contentWidth) / (canonical ? 1 : 2), .01),
        );
        expect(first.center.dy, closeTo(20, .01));
      },
    );
  }
}
