import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Builds the real renderer without initializing a runtime or loading a plugin.
Widget renderNode(Map<String, Object?> node) => buildComposeDslLayoutForTest(
  node: node,
  onTextInput: (id, text) async => null,
  hostContext: ComposeDslWebViewHostContext(
    packageName: 'test.package',
    routeInstanceId: 'layout-test',
    executionContextKey: 'layout-test',
    dispatchAction: (id, [payload]) async => null,
    runtimeOptionsProvider: () => <String, Object?>{},
  ),
  splitMarkdownContent: (_) async => [],
);

/// Checks actual Card and Row nesting through both DSL content encodings.
void main() {
  for (final type in ['Card', 'ElevatedCard', 'OutlinedCard']) {
    for (final useSlot in [false, true]) {
      testWidgets('$type in Row with content slot=$useSlot', (tester) async {
        final content = <Map<String, Object?>>[
          {
            'type': 'Box',
            'props': {'padding': 8},
            'children': [
              {
                'type': 'Text',
                'props': {'text': 'Nested content'},
              },
            ],
          },
        ];
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: Center(
                child: SizedBox(
                  width: 668,
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      renderNode({
                        'type': 'Row',
                        'children': [
                          {
                            'type': type,
                            if (useSlot) 'slots': {'content': content},
                            if (!useSlot) 'children': content,
                          },
                        ],
                      }),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );
        expect(tester.takeException(), isNull);
        expect(find.text('Nested content'), findsOneWidget);
        final size = tester.getSize(find.byType(Card));
        expect(size.width, greaterThan(0));
        expect(size.width, lessThan(668));
        expect(size.height, greaterThan(0));
        expect(size.isFinite, isTrue);
      });
    }
  }

  for (final useSlot in [false, true]) {
    testWidgets(
      'Card content keeps natural width under bounded parent, slot=$useSlot',
      (tester) async {
        final content = <Map<String, Object?>>[
          {
            'type': 'Box',
            'props': {'width': 40, 'height': 30, 'key': 'natural'},
          },
          {
            'type': 'Box',
            'props': {'fillMaxWidth': true, 'height': 20, 'key': 'filled'},
          },
        ];
        await tester.pumpWidget(
          MaterialApp(
            home: Center(
              child: SizedBox(
                width: 300,
                child: renderNode({
                  'type': 'Card',
                  if (useSlot) 'slots': {'content': content},
                  if (!useSlot) 'children': content,
                }),
              ),
            ),
          ),
        );
        expect(tester.takeException(), isNull);
        expect(
          tester.getSize(find.byKey(const ValueKey<Object?>('natural'))),
          const Size(40, 30),
        );
        expect(
          tester.getSize(find.byKey(const ValueKey<Object?>('filled'))),
          const Size(300, 20),
        );
      },
    );
  }

  for (final type in ['Card', 'ElevatedCard', 'OutlinedCard']) {
    for (final useSlot in [false, true]) {
      for (final fill in ['fillMaxSize', 'fillMaxHeight']) {
        for (final modifier in [false, true]) {
          testWidgets(
            '$type details viewport stays bounded: slot=$useSlot fill=$fill modifier=$modifier',
            (tester) async {
              final content = <Map<String, Object?>>[
                {
                  'type': 'Column',
                  'props': {
                    'fillMaxWidth': true,
                    if (!modifier) fill: true,
                    if (modifier)
                      'modifier': {
                        '__modifierOps': [
                          {'name': fill, 'args': <Object?>[]},
                        ],
                      },
                    'padding': 16,
                    'spacing': 10,
                  },
                  'children': [
                    {
                      'type': 'Row',
                      'props': {'fillMaxWidth': true},
                      'children': [
                        {
                          'type': 'Text',
                          'props': {'text': 'Task details', 'weight': 1},
                        },
                        {
                          'type': 'TextButton',
                          'props': {'text': 'Close'},
                        },
                      ],
                    },
                    {
                      'type': 'ScrollArea',
                      'props': {
                        'key': 'viewport',
                        'weight': 1,
                        'fillMaxWidth': true,
                      },
                      'children': [
                        {
                          'type': 'Box',
                          'props': {
                            'key': 'scroll-content',
                            'height': 1000,
                            'fillMaxWidth': true,
                          },
                        },
                      ],
                    },
                    {
                      'type': 'TextButton',
                      'props': {'text': 'Hide pet'},
                    },
                  ],
                },
              ];
              await tester.pumpWidget(
                MaterialApp(
                  home: Scaffold(
                    body: Center(
                      child: SizedBox(
                        width: 360,
                        height: 460,
                        child: renderNode({
                          'type': type,
                          'props': {'fillMaxSize': true, 'contentPadding': 8},
                          if (useSlot) 'slots': {'content': content},
                          if (!useSlot) 'children': content,
                        }),
                      ),
                    ),
                  ),
                ),
              );
              expect(tester.takeException(), isNull);
              expect(tester.getSize(find.byType(Card)), const Size(360, 460));
              final viewport = tester.getRect(
                find.byKey(const ValueKey<Object?>('viewport')),
              );
              final footer = tester.getRect(find.text('Hide pet'));
              expect(viewport.height, greaterThan(0));
              expect(viewport.height, lessThan(460));
              expect(
                viewport.top,
                greaterThan(tester.getRect(find.text('Task details')).bottom),
              );
              expect(viewport.bottom, lessThanOrEqualTo(footer.top));
              final scrollContent = find.byKey(
                const ValueKey<Object?>('scroll-content'),
              );
              final contentTop = tester.getTopLeft(scrollContent).dy;
              await tester.drag(
                find.byType(SingleChildScrollView),
                const Offset(0, -100),
              );
              await tester.pumpAndSettle();
              expect(tester.takeException(), isNull);
              expect(tester.getTopLeft(scrollContent).dy, lessThan(contentTop));
              expect(tester.getRect(find.text('Hide pet')), footer);
            },
          );
        }
      }
    }
  }
}
