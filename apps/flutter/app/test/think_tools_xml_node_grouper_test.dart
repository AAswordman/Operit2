import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';

void main() {
  group('MarkdownEventNodeBuilder streaming cursor', () {
    test(
      'keeps trailing empty and line-break nodes from owning the cursor',
      () {
        final state = StreamMarkdownRendererState();
        state.eventBuilder.startBlock(
          blockId: 1,
          type: MarkdownNodeType.xmlBlock,
          headerLevel: null,
        );
        state.eventBuilder.appendBlock(
          blockId: 1,
          content: '<tool name="read_file"></tool>',
        );
        state.eventBuilder.startBlock(
          blockId: 2,
          type: MarkdownNodeType.htmlBreak,
          headerLevel: null,
        );
        state.eventBuilder.startBlock(
          blockId: 3,
          type: MarkdownNodeType.plainText,
          headerLevel: null,
        );

        final nodes = state.eventBuilder.toStableNodes(isStreaming: true);

        expect(nodes, hasLength(3));
        expect(nodes.first.isStreaming, isTrue);
        expect(nodes[1].content, '\n');
        expect(nodes[1].isStreaming, isFalse);
        expect(nodes.last.content, isEmpty);
        expect(nodes.last.isStreaming, isFalse);
        state.reset();
      },
    );
  });

  group('ThinkToolsXmlNodeGrouper.group', () {
    test('collapses a thinking block with two tool calls', () {
      final items = const ThinkToolsXmlNodeGrouper(showThinkingProcess: true)
          .group(<MarkdownNodeStable>[
            _xml('<think>plan</think>'),
            _xml('<tool name="read_file"></tool>'),
            _xml('<tool_result name="read_file"></tool_result>'),
            _xml('<tool name="grep_code"></tool>'),
            _xml('<tool_result name="grep_code"></tool_result>'),
          ], 'renderer');

      expect(items, hasLength(1));
      final group = items.single as MarkdownGroupItem;
      expect(group.startIndex, 0);
      expect(group.endIndexInclusive, 4);
      expect(group.stableKey, 'think-tools-0');
    });

    test('collapses four calls followed by four results into one group', () {
      final items = const ThinkToolsXmlNodeGrouper(showThinkingProcess: true)
          .group(<MarkdownNodeStable>[
            _xml('<tool name="daily_life:get_current_time" />'),
            _xml('<tool name="daily_life:device_status" />'),
            _xml('<tool name="visit_web" />'),
            _xml('<tool name="list_core_nodes" />'),
            _xml(
              '<tool_result name="daily_life:get_current_time">12:00</tool_result>',
            ),
            _xml(
              '<tool_result name="daily_life:device_status">ready</tool_result>',
            ),
            _xml('<tool_result name="visit_web">blocked</tool_result>'),
            _xml('<tool_result name="list_core_nodes">core-1</tool_result>'),
          ], 'renderer');

      expect(items, hasLength(1));
      final group = items.single as MarkdownGroupItem;
      expect(group.startIndex, 0);
      expect(group.endIndexInclusive, 7);
      expect(group.stableKey, 'tools-only-0');
    });

    test('keeps line-break separators inside one four-call group', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="daily_life:get_current_time" />'),
        _break(),
        _xml('<tool name="daily_life:device_status" />'),
        _break(),
        _xml('<tool name="visit_web" />'),
        _break(),
        _xml('<tool name="list_core_nodes" />'),
        _break(),
        _xml(
          '<tool_result name="daily_life:get_current_time">12:00</tool_result>',
        ),
        _break(),
        _xml(
          '<tool_result name="daily_life:device_status">ready</tool_result>',
        ),
        _break(),
        _xml('<tool_result name="visit_web">blocked</tool_result>'),
        _break(),
        _xml('<tool_result name="list_core_nodes">core-1</tool_result>'),
      ];

      final items = const ThinkToolsXmlNodeGrouper(
        showThinkingProcess: true,
      ).group(nodes, 'renderer');

      expect(items, hasLength(1));
      final group = items.single as MarkdownGroupItem;
      expect(group.startIndex, 0);
      expect(group.endIndexInclusive, nodes.length - 1);
    });

    test('collapses a thinking block with one tool call', () {
      final items = const ThinkToolsXmlNodeGrouper(showThinkingProcess: true)
          .group(<MarkdownNodeStable>[
            _xml('<think>plan</think>'),
            _xml('<tool name="read_file"></tool>'),
            _xml('<tool_result name="read_file"></tool_result>'),
          ], 'renderer');

      expect(items, hasLength(1));
      final group = items.single as MarkdownGroupItem;
      expect(group.startIndex, 0);
      expect(group.endIndexInclusive, 2);
      expect(group.stableKey, 'think-tools-0');
    });

    test('keeps a single complete tool-only call expanded', () {
      final items = const ThinkToolsXmlNodeGrouper(showThinkingProcess: true)
          .group(<MarkdownNodeStable>[
            _xml('<tool name="read_file"></tool>'),
            _xml('<tool_result name="read_file"></tool_result>'),
          ], 'renderer');

      expect(items, hasLength(2));
      expect(items, everyElement(isA<MarkdownSingleItem>()));
    });

    test('collapses a thinking block with one write tool call', () {
      final items = const ThinkToolsXmlNodeGrouper(showThinkingProcess: true)
          .group(<MarkdownNodeStable>[
            _xml('<think>plan</think>'),
            _xml('<tool name="edit_file"></tool>'),
            _xml('<tool_result name="edit_file"></tool_result>'),
          ], 'renderer');

      expect(items, hasLength(1));
      final group = items.single as MarkdownGroupItem;
      expect(group.startIndex, 0);
      expect(group.endIndexInclusive, 2);
      expect(group.stableKey, 'think-tools-0');
    });
  });

  group('ThinkToolsXmlNodeGrouper streaming render', () {
    testWidgets('keeps tool visible after a closed thinking block', (
      tester,
    ) async {
      final controller = StreamController<Object>();
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: StreamMarkdownRenderer(
              content: '',
              isStreaming: true,
              contentStream: controller.stream,
              textColor: Colors.black,
              backgroundColor: Colors.white,
              nodeGrouper: const ThinkToolsXmlNodeGrouper(
                showThinkingProcess: true,
              ),
              showThinkingProcess: true,
              splitMarkdownContent: (_) async => const <MarkdownStreamEvent>[],
            ),
          ),
        ),
      );

      controller
        ..add(_markdownBlockStart(1))
        ..add(_markdownBlockChunk(1, '<think>plan</think>'))
        ..add(_markdownBlockStart(2))
        ..add(
          _markdownBlockChunk(
            2,
            '<tool name="read_file"><param name="path">README.md</param></tool>',
          ),
        )
        ..add(_markdownBlockStart(3, nodeType: 'HtmlBreak'));
      await tester.pump(const Duration(milliseconds: 250));

      expect(find.textContaining('read_file'), findsWidgets);
      expect(find.byType(StreamingCursor), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(CompactToolDisplay),
          matching: find.byType(StreamingCursor),
        ),
        findsOneWidget,
      );

      await controller.close();
    });

    testWidgets('keeps tool visible after a live thinking body closes', (
      tester,
    ) async {
      final controller = StreamController<Object>();
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: StreamMarkdownRenderer(
              content: '',
              isStreaming: true,
              contentStream: controller.stream,
              textColor: Colors.black,
              backgroundColor: Colors.white,
              nodeGrouper: const ThinkToolsXmlNodeGrouper(
                showThinkingProcess: true,
              ),
              showThinkingProcess: true,
              splitMarkdownContent: (_) async => const <MarkdownStreamEvent>[],
            ),
          ),
        ),
      );

      controller
        ..add(_markdownBlockStart(1))
        ..add(_markdownBlockChunk(1, '<think>'));
      await tester.pump(const Duration(milliseconds: 250));

      controller
        ..add(_markdownBlockChunk(1, 'plan'))
        ..add(_markdownChunk('plan', parentBlockId: 1))
        ..add(_markdownBlockStart(1, parentBlockId: 1, nodeType: null))
        ..add(_markdownInlineStart(1, 1, parentBlockId: 1))
        ..add(_markdownInlineChunk(1, 1, 'plan', parentBlockId: 1));
      await tester.pump(const Duration(milliseconds: 250));

      controller
        ..add(_markdownBlockChunk(1, '</think>'))
        ..add(_markdownCompleted(parentBlockId: 1))
        ..add(_markdownBlockStart(2))
        ..add(
          _markdownBlockChunk(
            2,
            '<tool name="read_file"><param name="path">README.md</param></tool>',
          ),
        );
      await tester.pump(const Duration(milliseconds: 250));

      expect(find.textContaining('read_file'), findsWidgets);

      await controller.close();
    });

    testWidgets('renders a live RS think and tools event sequence', (
      tester,
    ) async {
      final controller = StreamController<Object>();
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: StreamMarkdownRenderer(
              content: '',
              isStreaming: true,
              contentStream: controller.stream,
              textColor: Colors.black,
              backgroundColor: Colors.white,
              nodeGrouper: const ThinkToolsXmlNodeGrouper(
                showThinkingProcess: true,
              ),
              showThinkingProcess: true,
              splitMarkdownContent: (_) async => const <MarkdownStreamEvent>[],
            ),
          ),
        ),
      );

      _emitRsThinkOpen(controller);
      await tester.pump(const Duration(milliseconds: 250));

      _emitRsThinkBody(controller, 'plan');
      await tester.pump(const Duration(milliseconds: 250));

      _emitRsThinkClose(controller);
      await tester.pump(const Duration(milliseconds: 250));

      _emitRsXmlBlock(
        controller,
        blockId: 2,
        xml:
            '<tool name="read_file"><param name="path">README.md</param></tool>',
      );
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.textContaining('read_file'), findsWidgets);

      _emitRsXmlBlock(
        controller,
        blockId: 3,
        xml: '<tool_result name="read_file">ok</tool_result>',
      );
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.textContaining('read_file'), findsWidgets);

      _emitRsXmlBlock(
        controller,
        blockId: 4,
        xml: '<tool name="grep_code"><param name="pattern">TODO</param></tool>',
      );
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.textContaining('grep_code'), findsWidgets);

      _emitRsXmlBlock(
        controller,
        blockId: 5,
        xml: '<tool_result name="grep_code">none</tool_result>',
      );
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.textContaining('grep_code'), findsWidgets);

      await controller.close();
    });
  });
}

MarkdownNodeStable _xml(String content, {bool isStreaming = false}) {
  final trimmed = content.trim();
  final opening = RegExp(
    r'^<([A-Za-z][A-Za-z0-9_]*)([^>]*)>',
  ).firstMatch(trimmed);
  final tagName = opening?.group(1);
  final attributes = <String, String>{
    for (final match in RegExp(
      r'([A-Za-z][A-Za-z0-9_-]*)\s*=\s*"([^"]*)"',
    ).allMatches(opening?.group(2) ?? ''))
      match.group(1)!: match.group(2)!,
  };
  final closingStart = tagName == null
      ? -1
      : trimmed.toLowerCase().lastIndexOf('</${tagName.toLowerCase()}>');
  final bodyStart = opening?.end ?? 0;
  final bodyEnd = closingStart >= bodyStart ? closingStart : trimmed.length;
  return MarkdownNodeStable(
    type: MarkdownNodeType.xmlBlock,
    content: content,
    isStreaming: isStreaming,
    xmlTagName: tagName,
    xmlAttributes: attributes,
    xmlBody: bodyStart <= bodyEnd ? trimmed.substring(bodyStart, bodyEnd) : '',
    xmlIsClosed: trimmed.endsWith('/>') || closingStart >= bodyStart,
  );
}

/// Creates one rendered Markdown line-break separator.
MarkdownNodeStable _break() {
  return const MarkdownNodeStable(
    type: MarkdownNodeType.htmlBreak,
    content: '\n',
    isStreaming: false,
  );
}

MarkdownStreamEvent _markdownBlockStart(
  int blockId, {
  int? parentBlockId,
  String? nodeType = 'XmlBlock',
}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownBlockStart',
    value: null,
    id: null,
    blockId: blockId,
    inlineId: null,
    parentBlockId: parentBlockId,
    nodeType: nodeType,
    headerLevel: null,
    xml: null,
  );
}

MarkdownStreamEvent _markdownBlockChunk(
  int blockId,
  String value, {
  int? parentBlockId,
}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownBlockChunk',
    value: value,
    id: null,
    blockId: blockId,
    inlineId: null,
    parentBlockId: parentBlockId,
    nodeType: 'XmlBlock',
    headerLevel: null,
    xml: _xmlEvent(value),
  );
}

MarkdownStreamEvent _markdownChunk(String value, {int? parentBlockId}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'chunk',
    value: value,
    id: null,
    blockId: null,
    inlineId: null,
    parentBlockId: parentBlockId,
    nodeType: null,
    headerLevel: null,
    xml: null,
  );
}

MarkdownStreamEvent _markdownInlineStart(
  int blockId,
  int inlineId, {
  int? parentBlockId,
}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownInlineStart',
    value: null,
    id: null,
    blockId: blockId,
    inlineId: inlineId,
    parentBlockId: parentBlockId,
    nodeType: null,
    headerLevel: null,
    xml: null,
  );
}

MarkdownStreamEvent _markdownInlineChunk(
  int blockId,
  int inlineId,
  String value, {
  int? parentBlockId,
}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownInlineChunk',
    value: value,
    id: null,
    blockId: blockId,
    inlineId: inlineId,
    parentBlockId: parentBlockId,
    nodeType: null,
    headerLevel: null,
    xml: null,
  );
}

MarkdownStreamEvent _markdownCompleted({int? parentBlockId}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'completed',
    value: null,
    id: null,
    blockId: null,
    inlineId: null,
    parentBlockId: parentBlockId,
    nodeType: null,
    headerLevel: null,
    xml: null,
  );
}

void _emitRsThinkOpen(StreamController<Object> controller) {
  controller
    ..add(_markdownChunk('<think>'))
    ..add(_markdownBlockStart(1))
    ..add(_markdownBlockChunk(1, '<think>'));
}

void _emitRsThinkBody(StreamController<Object> controller, String text) {
  controller
    ..add(_markdownChunk(text))
    ..add(_markdownBlockChunk(1, text))
    ..add(_markdownChunk(text, parentBlockId: 1))
    ..add(_markdownBlockStart(1, parentBlockId: 1, nodeType: null))
    ..add(_markdownInlineStart(1, 1, parentBlockId: 1))
    ..add(_markdownInlineChunk(1, 1, text, parentBlockId: 1));
}

void _emitRsThinkClose(StreamController<Object> controller) {
  controller
    ..add(_markdownChunk('</think>'))
    ..add(_markdownBlockChunk(1, '</think>'))
    ..add(
      MarkdownStreamEvent(
        chatId: 'chat',
        eventType: 'markdownBlockEnd',
        value: null,
        id: null,
        blockId: 1,
        inlineId: null,
        parentBlockId: null,
        nodeType: 'XmlBlock',
        headerLevel: null,
        xml: const MarkdownXmlStreamEvent(
          tagName: 'think',
          attributes: <String, String>{},
          bodyChunk: '',
          children: <MarkdownXmlChildStreamEvent>[],
          isClosed: true,
        ),
      ),
    )
    ..add(_markdownCompleted(parentBlockId: 1));
}

/// Builds the structured XML metadata used by the RS event test helpers.
MarkdownXmlStreamEvent? _xmlEvent(String value) {
  final opening = RegExp(r'<([A-Za-z][A-Za-z0-9_]*)([^>]*)>').firstMatch(value);
  if (opening == null) {
    return null;
  }
  final tagName = opening.group(1)!;
  final attributes = <String, String>{
    for (final match in RegExp(
      r'([A-Za-z][A-Za-z0-9_-]*)\s*=\s*"([^"]*)"',
    ).allMatches(opening.group(2)!))
      match.group(1)!: match.group(2)!,
  };
  return MarkdownXmlStreamEvent(
    tagName: tagName,
    attributes: attributes,
    bodyChunk: '',
    children: const <MarkdownXmlChildStreamEvent>[],
    isClosed: value.trim().endsWith('/>') || value.contains('</'),
  );
}

void _emitRsXmlBlock(
  StreamController<Object> controller, {
  required int blockId,
  required String xml,
}) {
  controller
    ..add(_markdownChunk(xml))
    ..add(_markdownBlockStart(blockId))
    ..add(_markdownBlockChunk(blockId, xml));
}
