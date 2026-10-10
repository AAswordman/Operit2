import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/common/components/SwipeActions.dart';

Map<String, Object?> action(String id) => {'__actionId': id};
Map<String, Object?> node(
  String type,
  Map<String, Object?> props, [
  List<Map<String, Object?>> children = const [],
  Map<String, Object?> slots = const {},
]) => {'type': type, 'props': props, 'children': children, 'slots': slots};
Map<String, Object?> text(String label) => node('Text', {'text': label});
Map<String, Object?> surface(String label) => node(
  'Box',
  {'width': 240, 'height': 40, 'background': '#dddddd'},
  [text(label)],
);

Future<void> mount(
  WidgetTester tester,
  Map<String, Object?> tree,
  List<(String, Object?)> events,
) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 260,
            child: buildComposeDslLayoutForTest(
              node: tree,
              hostContext: ComposeDslWebViewHostContext(
                packageName: 'generic.interactions',
                routeInstanceId: 'test',
                executionContextKey: 'test',
                dispatchAction: (id, [payload]) async {
                  events.add((id, payload));
                  return null;
                },
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
  await tester.pump();
  expect(tester.takeException(), isNull);
}

Future<void> swipe(WidgetTester tester, bool right, double fraction) async {
  final rect = tester.getRect(find.byType(SwipeActions));
  final start = Offset(
    rect.left + rect.width * (right ? .1 : .9),
    rect.center.dy,
  );
  final gesture = await tester.startGesture(start);
  await gesture.moveBy(Offset(rect.width * fraction * (right ? 1 : -1), 0));
  await gesture.up();
  await tester.pump(const Duration(milliseconds: 200));
}

Future<void> drag(
  WidgetTester tester,
  String source,
  String target, {
  bool hold = false,
}) async {
  final gesture = await tester.startGesture(
    tester.getCenter(find.text(source)),
  );
  if (hold) await tester.pump(const Duration(milliseconds: 600));
  await gesture.moveBy(const Offset(0, 20));
  await tester.pump();
  await gesture.moveTo(tester.getCenter(find.text(target)));
  await tester.pump();
  await gesture.up();
  await tester.pump();
}

Map<String, Object?> draggable(
  String label,
  Object? payload, {
  String type = 'generic',
  bool hold = false,
}) => node(
  'Draggable',
  {
    'data': payload,
    'dragType': type,
    'longPress': hold,
    'onDragStarted': action('started'),
    'onDragEnd': action('ended'),
  },
  [surface(label)],
  {
    'feedback': [surface('Feedback')],
    'childWhenDragging': [surface('Dragging')],
  },
);

void main() {
  testWidgets(
    'generic pill Surface uses native Material stadium geometry and clipping',
    (tester) async {
      final events = <(String, Object?)>[];
      await mount(
        tester,
        node(
          'Surface',
          {
            'key': 'generic-pill',
            'shape': {'type': 'pill'},
            'containerColor': '#004b6f',
            'width': 210,
            'height': 34,
          },
          [
            node(
              'Row',
              {
                'fillMaxSize': true,
                'horizontalArrangement': 'center',
                'verticalAlignment': 'center',
                'spacing': 6,
              },
              [
                node('Icon', {'name': 'AddRounded', 'size': 17}),
                text('新建对话'),
              ],
            ),
          ],
        ),
        events,
      );
      final material = tester
          .widgetList<Material>(find.byType(Material))
          .singleWhere((value) => value.color == const Color(0xff004b6f));
      expect(material.shape, isA<StadiumBorder>());
      expect(material.clipBehavior, Clip.antiAlias);
      expect(tester.getSize(find.byWidget(material)), const Size(260, 34));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'generic Image remembers its provider across unrelated recompositions',
    (tester) async {
      final bytes = File('assets/images/operit_avatar.png').readAsBytesSync();
      final source = 'data:image/png;base64,${base64Encode(bytes)}';
      final events = <(String, Object?)>[];
      Map<String, Object?> tree(String title, String uri) =>
          node('Column', {}, [
            node('Image', {
              'key': 'stable-avatar',
              'uri': uri,
              'width': 32,
              'height': 32,
            }),
            text(title),
          ]);
      await mount(tester, tree('before', source), events);
      await tester.pumpAndSettle();
      final provider = tester.widget<Image>(find.byType(Image)).image;
      final imageElement = tester.element(find.byType(Image));
      for (var update = 0; update < 5; update++) {
        await mount(tester, tree('updated $update', source), events);
        expect(
          identical(tester.widget<Image>(find.byType(Image)).image, provider),
          isTrue,
        );
        expect(tester.element(find.byType(Image)), same(imageElement));
        expect(tester.takeException(), isNull);
      }
      await mount(
        tester,
        tree(
          'changed avatar',
          source.replaceFirst('image/png;', 'image/png;charset=utf-8;'),
        ),
        events,
      );
      expect(
        identical(tester.widget<Image>(find.byType(Image)).image, provider),
        isFalse,
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('centered Compose Row preserves explicit icon-label spacing', (
    tester,
  ) async {
    final events = <(String, Object?)>[];
    await mount(
      tester,
      node(
        'Row',
        {
          'fillMaxWidth': true,
          'height': 34,
          'horizontalArrangement': 'center',
          'verticalAlignment': 'center',
          'spacing': 6,
        },
        [
          node('Icon', {'name': 'AddRounded', 'size': 17}),
          text('新建对话'),
        ],
      ),
      events,
    );
    final icon = tester.getRect(find.byType(Icon));
    final label = tester.getRect(find.text('新建对话'));
    expect(label.left - icon.right, closeTo(6, .01));
  });

  setUpAll(ClientLogger.initialize);
  testWidgets(
    'default drag feedback constrains fill-width rows and Expanded children in the overlay',
    (tester) async {
      final events = <(String, Object?)>[];
      await mount(
        tester,
        node('Column', {}, [
          node(
            'Draggable',
            {'data': 'generic-payload', 'onDragEnd': action('ended')},
            [
              node(
                'Row',
                {'fillMaxWidth': true, 'height': 40},
                [
                  node('Text', {'text': 'Auto feedback', 'weight': 1}),
                ],
              ),
            ],
          ),
          node('DragTarget', {'onDrop': action('drop')}, [surface('Target')]),
        ]),
        events,
      );
      await drag(tester, 'Auto feedback', 'Target');
      expect(tester.takeException(), isNull);
      expect(
        events.where((event) => event.$1 == 'drop').single.$2,
        'generic-payload',
      );
    },
  );

  testWidgets(
    'combinedClickable exposes independent tap and long press callbacks on arbitrary content',
    (tester) async {
      final events = <(String, Object?)>[];
      final tree = surface('Generic button');
      (tree['props'] as Map)['modifier'] = {
        '__modifierOps': [
          {
            'name': 'combinedClickable',
            'args': [
              {'onClick': action('tap'), 'onLongClick': action('long')},
            ],
          },
        ],
      };
      await mount(tester, tree, events);
      await tester.tap(find.text('Generic button'));
      await tester.pump();
      await tester.longPress(find.text('Generic button'));
      await tester.pump();
      expect(events.map((event) => event.$1), ['tap', 'long']);
    },
  );
  testWidgets(
    'generic swipe composes arbitrary start/end backgrounds and preserves original threshold',
    (tester) async {
      final events = <(String, Object?)>[];
      await mount(
        tester,
        node(
          'SwipeActions',
          {
            'onStartAction': action('start'),
            'onEndAction': action('end'),
            'actionThreshold': .4,
          },
          [surface('Swipe anything')],
          {
            'startBackground': [text('Start')],
            'endBackground': [text('End')],
          },
        ),
        events,
      );
      await swipe(tester, true, .2);
      expect(events, isEmpty);
      await swipe(tester, true, .5);
      expect(events, [('start', null)]);
      await swipe(tester, false, .5);
      expect(events, [('start', null), ('end', null)]);
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'drag and drop transports arbitrary nested JSON without any chat host or schema',
    (tester) async {
      final events = <(String, Object?)>[];
      final payload = {
        'arbitrary': [
          1,
          true,
          {'value': 'anything'},
        ],
      };
      await mount(
        tester,
        node('Column', {}, [
          draggable('Source', payload),
          node('DragTarget', {'onDrop': action('drop')}, [surface('Target')]),
        ]),
        events,
      );
      await drag(tester, 'Source', 'Target');
      expect(events.where((event) => event.$1 == 'drop').single.$2, payload);
      final end = events.where((event) => event.$1 == 'ended').single.$2 as Map;
      expect(end['wasAccepted'], isTrue);
      expect(end['data'], payload);
      expect(end['dragType'], 'generic');
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'declarative type and deep-value filters accept matching data and reject other payloads',
    (tester) async {
      final events = <(String, Object?)>[];
      await mount(
        tester,
        node('Column', {}, [
          draggable('Matching', {'id': 'a'}),
          draggable('Wrong type', {'id': 'a'}, type: 'other'),
          draggable('Wrong data', {'id': 'b'}),
          node(
            'DragTarget',
            {
              'acceptedTypes': ['generic'],
              'acceptedData': [
                {'id': 'a'},
              ],
              'onDrop': action('drop'),
            },
            [surface('Target')],
          ),
        ]),
        events,
      );
      await drag(tester, 'Matching', 'Target');
      await drag(tester, 'Wrong type', 'Target');
      await drag(tester, 'Wrong data', 'Target');
      expect(events.where((event) => event.$1 == 'drop'), hasLength(1));
      expect(
        events
            .where((event) => event.$1 == 'ended')
            .map((event) => (event.$2 as Map)['wasAccepted']),
        [true, false, false],
      );
    },
  );
  testWidgets(
    'long press dragging and disabled drop targets follow the same generic contract',
    (tester) async {
      final events = <(String, Object?)>[];
      await mount(
        tester,
        node('Column', {}, [
          draggable('Hold source', 'opaque', hold: true),
          node(
            'DragTarget',
            {'enabled': false, 'onDrop': action('disabled')},
            [surface('Disabled')],
          ),
          node('DragTarget', {'onDrop': action('drop')}, [surface('Target')]),
        ]),
        events,
      );
      await drag(tester, 'Hold source', 'Disabled', hold: true);
      expect(events.where((event) => event.$1 == 'disabled'), isEmpty);
      await drag(tester, 'Hold source', 'Target', hold: true);
      expect(events.where((event) => event.$1 == 'drop').single.$2, 'opaque');
    },
  );
}
