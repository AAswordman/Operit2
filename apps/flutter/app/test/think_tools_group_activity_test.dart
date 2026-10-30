import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';

const _tool = MarkdownNodeStable(
  type: MarkdownNodeType.xmlBlock,
  content: '<tool name="custom:run"/>',
  isStreaming: true,
  xmlTagName: 'tool',
  xmlAttributes: {'name': 'custom:run'},
  xmlIsClosed: true,
);
const _think = MarkdownNodeStable(
  type: MarkdownNodeType.xmlBlock,
  content: '<think>plan</think>',
  isStreaming: false,
  xmlTagName: 'think',
  xmlIsClosed: true,
);
const _lineBreak = MarkdownNodeStable(
  type: MarkdownNodeType.htmlBreak,
  content: '\n',
  isStreaming: false,
);
const _whitespace = MarkdownNodeStable(
  type: MarkdownNodeType.plainText,
  content: ' \n ',
  isStreaming: false,
);
const _answer = MarkdownNodeStable(
  type: MarkdownNodeType.plainText,
  content: 'Answer',
  isStreaming: true,
);

/// Supplies group inputs without depending on tool presentation or generated runtime clients.
Widget _renderGroup(
  List<MarkdownNodeStable> nodes, {
  bool live = true,
  List<Widget> body = const [Text('Tool body')],
}) {
  const grouper = ThinkToolsXmlNodeGrouper(showThinkingProcess: true);
  final group =
      grouper.group(nodes, 'activity-test').first as MarkdownGroupItem;
  final lastVisibleIndex = nodes.lastIndexWhere(
    (node) => node.content.trim().isNotEmpty,
  );
  return MaterialApp(
    home: Scaffold(
      body: grouper.renderGroup(
        group: group,
        nodes: nodes,
        rendererId: 'activity-test',
        isVisible: true,
        isLastNode: group.endIndexInclusive == lastVisibleIndex,
        textColor: Colors.black,
        xmlRenderer:
            ({
              required xmlContent,
              required isStreaming,
              required textColor,
              xmlStream,
              xmlMarkdownEventStream,
              renderInstanceKey,
            }) => const SizedBox.shrink(),
        mergeRender:
            ({
              required startIndex,
              required endIndexInclusive,
              required renderInstanceKeyPrefix,
              required shouldRenderNode,
            }) => body,
        xmlStreamResolver: (index) =>
            live && index == 0 ? const Stream<String>.empty() : null,
        xmlMarkdownEventStreamResolver: (_) => null,
        onLinkClick: null,
        fillMaxWidth: true,
        textStyle: const TextStyle(),
      ),
    ),
  );
}

void main() {
  Future<ScrollPosition> pumpScrollableGroup(WidgetTester tester) async {
    await tester.pumpWidget(
      _renderGroup([_tool], body: const [SizedBox(height: 1600)]),
    );
    await tester.pump();
    final position = tester
        .state<ScrollableState>(find.byType(Scrollable))
        .position;
    expect(position.extentAfter, 0);
    return position;
  }

  Future<void> appendOutput(WidgetTester tester) async {
    await tester.pumpWidget(
      _renderGroup([_tool], body: const [SizedBox(height: 1800)]),
    );
    await tester.pump();
  }

  testWidgets('wheel scrolling up pauses live group following until bottom', (
    tester,
  ) async {
    final position = await pumpScrollableGroup(tester);
    final bottom = position.pixels;
    final center = tester.getCenter(find.byType(ListView));
    await tester.sendEventToBinding(
      PointerScrollEvent(position: center, scrollDelta: const Offset(0, -20)),
    );
    await tester.pump();
    expect(position.pixels, closeTo(bottom - 20, 0.1));

    await appendOutput(tester);
    expect(position.pixels, closeTo(bottom - 20, 0.1));
    await tester.pump(const Duration(seconds: 1));
    expect(position.pixels, closeTo(bottom - 20, 0.1));

    await tester.sendEventToBinding(
      PointerScrollEvent(position: center, scrollDelta: const Offset(0, 10000)),
    );
    await tester.pump();
    expect(position.extentAfter, 0);
    await tester.pumpWidget(
      _renderGroup([_tool], body: const [SizedBox(height: 2000)]),
    );
    await tester.pump();
    expect(position.extentAfter, 0);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('a slow upward drag yields to the user before leaving bottom', (
    tester,
  ) async {
    final position = await pumpScrollableGroup(tester);
    final bottom = position.pixels;
    final gesture = await tester.startGesture(
      tester.getCenter(find.byType(ListView)),
    );
    await gesture.moveBy(const Offset(0, 30));
    await tester.pump();
    await gesture.moveBy(const Offset(0, 10));
    await tester.pump();
    expect(position.pixels, lessThan(bottom));
    expect(position.extentAfter, lessThan(80));
    final readingOffset = position.pixels;

    await appendOutput(tester);
    expect(position.pixels, closeTo(readingOffset, 0.1));
    await gesture.up();
    await tester.pump(const Duration(seconds: 1));
    expect(position.pixels, lessThan(bottom));
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('an untouched live group keeps following appended output', (
    tester,
  ) async {
    final position = await pumpScrollableGroup(tester);
    final bottom = position.pixels;
    await appendOutput(tester);
    expect(position.pixels, greaterThan(bottom));
    expect(position.extentAfter, 0);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  for (final kind in [PointerDeviceKind.touch, PointerDeviceKind.trackpad]) {
    testWidgets('a $kind contact holds off group jumps before dragging', (
      tester,
    ) async {
      final position = await pumpScrollableGroup(tester);
      final bottom = position.pixels;
      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(ListView)),
        kind: kind,
      );
      await appendOutput(tester);
      expect(position.pixels, bottom);
      await tester.pump(const Duration(seconds: 1));
      expect(position.pixels, bottom);

      // A stationary contact only pauses following for the gesture lifetime.
      await gesture.up();
      await tester.pump();
      expect(position.extentAfter, 0);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  testWidgets('nested content scrolling does not change group follow intent', (
    tester,
  ) async {
    final innerScroll = ScrollController(initialScrollOffset: 100);
    addTearDown(innerScroll.dispose);
    Widget group(double height) => _renderGroup(
      [_tool],
      body: [
        SizedBox(
          height: height,
          child: Column(
            children: [
              const Spacer(),
              SizedBox(
                height: 300,
                child: ListView(
                  controller: innerScroll,
                  children: const [SizedBox(height: 1000)],
                ),
              ),
            ],
          ),
        ),
      ],
    );
    await tester.pumpWidget(group(1300));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    await tester.pump();
    final position = tester
        .state<ScrollableState>(find.byType(Scrollable).first)
        .position;
    final bottom = position.pixels;
    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(find.byType(ListView).last),
        scrollDelta: const Offset(0, -20),
      ),
    );
    await tester.pump();
    expect(innerScroll.offset, 80);
    expect(position.pixels, bottom);

    await tester.pumpWidget(group(1500));
    await tester.pump();
    expect(position.pixels, greaterThan(bottom));
    expect(position.extentAfter, 0);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  for (final (description, trailing) in [
    ('no separator', <MarkdownNodeStable>[]),
    ('a line break', [_lineBreak]),
    ('whitespace', [_whitespace]),
    ('mixed separators', [_lineBreak, _whitespace]),
  ]) {
    testWidgets('keeps a live group expanded with $description', (
      tester,
    ) async {
      await tester.pumpWidget(_renderGroup([_tool, ...trailing]));
      expect(find.textContaining('Executing'), findsOneWidget);
      expect(find.textContaining('Completed'), findsNothing);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(find.text('Tool body'), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  testWidgets('appending separators does not complete an active group', (
    tester,
  ) async {
    await tester.pumpWidget(_renderGroup([_tool]));
    await tester.pumpWidget(_renderGroup([_tool, _lineBreak, _whitespace]));
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.textContaining('Executing'), findsOneWidget);
    expect(find.text('Tool body'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('folds a group when its stream finishes', (tester) async {
    await tester.pumpWidget(_renderGroup([_tool, _lineBreak]));
    expect(find.text('Tool body'), findsOneWidget);
    await tester.pumpWidget(_renderGroup([_tool, _lineBreak], live: false));
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.textContaining('Completed'), findsOneWidget);
    expect(find.text('Tool body'), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  for (final (description, trailing) in [
    ('answer text', [_answer]),
    ('a later thinking and tool group', [_think, _tool]),
  ]) {
    testWidgets('folds an earlier group when $description arrives', (
      tester,
    ) async {
      await tester.pumpWidget(_renderGroup([_tool]));
      await tester.pumpWidget(_renderGroup([_tool, _lineBreak, ...trailing]));
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.textContaining('Completed'), findsOneWidget);
      expect(find.text('Tool body'), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  testWidgets('preserves manual expansion after the stream finishes', (
    tester,
  ) async {
    await tester.pumpWidget(_renderGroup([_tool, _lineBreak], live: false));
    await tester.tap(find.textContaining('Completed'));
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pumpWidget(_renderGroup([_tool, _lineBreak]));
    await tester.pumpWidget(_renderGroup([_tool, _lineBreak], live: false));
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.textContaining('Completed'), findsOneWidget);
    expect(find.text('Tool body'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}
