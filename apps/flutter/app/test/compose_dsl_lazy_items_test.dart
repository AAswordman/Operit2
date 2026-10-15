import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

typedef _Range = ({int firstIndex, int lastIndex});

/// Builds a real renderer whose supplied node window changes through the range action.
Widget _harness({
  required ValueNotifier<_Range> range,
  required ValueNotifier<int> count,
  required Future<Object?> Function(String, [Object?]) onAction,
  Axis axis = Axis.vertical,
  bool reverse = false,
  double textScale = 1,
}) => MaterialApp(
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(
      context,
    ).copyWith(textScaler: TextScaler.linear(textScale)),
    child: child!,
  ),
  home: Scaffold(
    body: SizedBox(
      width: 320,
      height: 240,
      child: ListenableBuilder(
        listenable: Listenable.merge([range, count]),
        builder: (context, _) => buildComposeDslLayoutForTest(
          node: {
            'type': axis == Axis.vertical ? 'LazyColumn' : 'LazyRow',
            'props': {
              'reverseLayout': reverse,
              'viewport': {
                'firstIndex': range.value.firstIndex,
                'itemCount': count.value,
                'itemExtent': 40,
                'onRangeChanged': {'__actionId': 'range'},
              },
            },
            'children': [
              for (
                var index = range.value.firstIndex;
                index < range.value.lastIndex;
                index++
              )
                {
                  'type': 'Box',
                  'props': {'key': 'item:$index', 'height': 40, 'width': 40},
                  'children': [
                    {
                      'type': 'Text',
                      'props': {'text': '$index'},
                    },
                  ],
                },
            ],
          },
          onTextInput: (id, text) async => null,
          hostContext: ComposeDslWebViewHostContext(
            packageName: 'lazy-items-test',
            routeInstanceId: 'lazy-items-test',
            executionContextKey: 'lazy-items-test',
            dispatchAction: onAction,
            runtimeOptionsProvider: () => {},
          ),
          splitMarkdownContent: (_) async => [],
        ),
      ),
    ),
  ),
);

/// Exercises fixed extents, native Flutter scrolling, action ordering and catalog replacement.
void main() {
  for (final axis in Axis.values) {
    for (final reverse in [false, true]) {
      testWidgets(
        'windowed $axis list scrolls to its final real item with reverse=$reverse',
        (tester) async {
          final range = ValueNotifier<_Range>((firstIndex: 0, lastIndex: 12));
          final count = ValueNotifier(500);
          addTearDown(range.dispose);
          addTearDown(count.dispose);
          final requests = <_Range>[];
          await tester.pumpWidget(
            _harness(
              range: range,
              count: count,
              axis: axis,
              reverse: reverse,
              onAction: (id, [payload]) async {
                expect(id, 'range');
                final values = payload! as Map;
                final next = (
                  firstIndex: values['firstIndex'] as int,
                  lastIndex: values['lastIndex'] as int,
                );
                requests.add(next);
                range.value = next;
                return null;
              },
            ),
          );
          await tester.pumpAndSettle();
          expect(find.text('0'), findsOneWidget);
          expect(requests, isEmpty);
          final position = tester
              .state<ScrollableState>(find.byType(Scrollable))
              .position;
          position.jumpTo(2000);
          await tester.pumpAndSettle();
          expect(find.text('50'), findsOneWidget);
          expect(
            range.value.lastIndex - range.value.firstIndex,
            lessThanOrEqualTo(12),
          );
          position.jumpTo(position.maxScrollExtent);
          await tester.pumpAndSettle();
          expect(find.text('499'), findsOneWidget);
          expect(range.value.lastIndex, 500);
          expect(tester.takeException(), isNull);
          count.value = 3;
          range.value = (firstIndex: 0, lastIndex: 3);
          await tester.pumpAndSettle();
          expect(find.text('2'), findsOneWidget);
          expect(tester.takeException(), isNull);
        },
      );
    }
  }

  testWidgets(
    'windowed extents and range requests follow enlarged Flutter text',
    (tester) async {
      final range = ValueNotifier<_Range>((firstIndex: 0, lastIndex: 12));
      final count = ValueNotifier(500);
      addTearDown(range.dispose);
      addTearDown(count.dispose);
      await tester.pumpWidget(
        _harness(
          range: range,
          count: count,
          textScale: 2,
          onAction: (id, [payload]) async {
            final values = payload! as Map;
            range.value = (
              firstIndex: values['firstIndex'] as int,
              lastIndex: values['lastIndex'] as int,
            );
            return null;
          },
        ),
      );
      await tester.pumpAndSettle();
      final position = tester
          .state<ScrollableState>(find.byType(Scrollable))
          .position;
      expect(position.maxScrollExtent, 500 * 80 - 240);
      position.jumpTo(2000);
      await tester.pumpAndSettle();
      expect(find.text('25'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'rapid viewport changes keep one range action in flight and converge to the latest offset',
    (tester) async {
      final range = ValueNotifier<_Range>((firstIndex: 0, lastIndex: 12));
      final count = ValueNotifier(500);
      addTearDown(range.dispose);
      addTearDown(count.dispose);
      final firstReply = Completer<void>();
      final requests = <_Range>[];
      var active = 0;
      await tester.pumpWidget(
        _harness(
          range: range,
          count: count,
          onAction: (id, [payload]) async {
            active++;
            expect(active, 1);
            final values = payload! as Map;
            final next = (
              firstIndex: values['firstIndex'] as int,
              lastIndex: values['lastIndex'] as int,
            );
            requests.add(next);
            if (requests.length == 1) await firstReply.future;
            range.value = next;
            active--;
            return null;
          },
        ),
      );
      await tester.pumpAndSettle();
      final position = tester
          .state<ScrollableState>(find.byType(Scrollable))
          .position;
      position.jumpTo(2000);
      await tester.pump();
      expect(requests, hasLength(1));
      position.jumpTo(8000);
      await tester.pump();
      expect(requests, hasLength(1));
      firstReply.complete();
      await tester.pumpAndSettle();
      expect(requests, hasLength(2));
      expect(find.text('200'), findsOneWidget);
      expect(active, 0);
      expect(tester.takeException(), isNull);
    },
  );
}
