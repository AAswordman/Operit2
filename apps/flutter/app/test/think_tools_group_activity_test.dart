import 'dart:async';

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
Widget _renderGroup(List<MarkdownNodeStable> nodes, {bool live = true}) {
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
            }) => [const Text('Tool body')],
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
