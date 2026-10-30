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
import 'package:operit2/ui/features/chat/components/part/ToolCallResultMergeRender.dart';

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

    test('groups a single tool call with its result', () {
      final items = const ThinkToolsXmlNodeGrouper(showThinkingProcess: true)
          .group(<MarkdownNodeStable>[
            _xml('<tool name="read_file"></tool>'),
            _xml('<tool_result name="read_file"></tool_result>'),
          ], 'renderer');

      expect(items, hasLength(1));
      final group = items.single as MarkdownGroupItem;
      expect(group.startIndex, 0);
      expect(group.endIndexInclusive, 1);
      expect(group.stableKey, 'tools-only-0');
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
              mergeRender: const ToolCallResultMergeRender(),
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

      expect(find.text('read_file'), findsOneWidget);
      expect(find.byType(StreamingCursor), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(ToolCallRow),
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
              mergeRender: const ToolCallResultMergeRender(),
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

      expect(find.text('read_file'), findsOneWidget);

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
              mergeRender: const ToolCallResultMergeRender(),
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
      expect(find.text('read_file'), findsOneWidget);

      _emitRsXmlBlock(
        controller,
        blockId: 3,
        xml: '<tool_result name="read_file">ok</tool_result>',
      );
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.text('read_file'), findsOneWidget);

      _emitRsXmlBlock(
        controller,
        blockId: 4,
        xml: '<tool name="grep_code"><param name="pattern">TODO</param></tool>',
      );
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.text('grep_code'), findsOneWidget);

      _emitRsXmlBlock(
        controller,
        blockId: 5,
        xml: '<tool_result name="grep_code">none</tool_result>',
      );
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.text('grep_code'), findsOneWidget);

      await controller.close();
    });

    testWidgets(
      'keeps streamed Markdown visible when later tool groups arrive',
      (tester) async {
        final controller = StreamController<Object>();
        await tester.pumpWidget(
          MaterialApp(
            locale: const Locale('zh'),
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: Scaffold(
              body: StreamMarkdownRenderer(
                mergeRender: const ToolCallResultMergeRender(),
                content: '',
                isStreaming: true,
                contentStream: controller.stream,
                textColor: Colors.black,
                backgroundColor: Colors.white,
                nodeGrouper: const ThinkToolsXmlNodeGrouper(
                  showThinkingProcess: true,
                ),
                showThinkingProcess: true,
                splitMarkdownContent: (_) async =>
                    const <MarkdownStreamEvent>[],
              ),
            ),
          ),
        );

        _emitRsXmlBlock(
          controller,
          blockId: 1,
          xml: '<think>first plan</think>',
          complete: true,
        );
        _emitRsXmlBlock(
          controller,
          blockId: 2,
          xml: '<tool name="read_file"></tool>',
          complete: true,
        );
        await tester.pump(const Duration(milliseconds: 250));
        expect(find.textContaining('正在执行 · 1 次思考 · 1 次调用'), findsOneWidget);
        expect(
          find.descendant(
            of: find.byType(ToolCallRow),
            matching: find.byType(StreamingCursor),
          ),
          findsOneWidget,
        );
        expect(find.text('read_file'), findsOneWidget);

        _emitRsXmlBlock(
          controller,
          blockId: 3,
          xml: '<tool_result name="read_file">first result</tool_result>',
          complete: true,
        );
        await tester.pump(const Duration(milliseconds: 250));
        expect(find.textContaining('正在执行 · 1 次思考 · 1 次调用'), findsOneWidget);
        expect(
          find.descendant(
            of: find.byType(ToolCallRow),
            matching: find.byType(StreamingCursor),
          ),
          findsNothing,
        );

        _emitRsTextBlock(
          controller,
          blockId: 4,
          text: 'Streaming intermediate A',
        );
        await tester.pump(const Duration(milliseconds: 900));
        await tester.pump(const Duration(milliseconds: 250));

        final markdownA = find.text(
          'Streaming intermediate A',
          findRichText: true,
        );
        final firstGroupHeader = find.textContaining('1 次思考 · 1 次调用');
        expect(markdownA, findsOneWidget);
        expect(firstGroupHeader, findsOneWidget);
        expect(find.text('已完成 · 1 次思考 · 1 次调用'), findsOneWidget);
        expect(find.textContaining('正在执行'), findsNothing);
        // Finished phases fold away automatically once body text arrives.
        expect(find.text('read_file'), findsNothing);

        await tester.tap(firstGroupHeader);
        await tester.pump(const Duration(milliseconds: 250));
        // Manual expansion survives and is not auto-collapsed again.
        expect(find.text('read_file'), findsOneWidget);
        final originalMarkdownOffset = tester.getTopLeft(markdownA);

        _emitRsXmlBlock(
          controller,
          blockId: 5,
          xml: '<think>second plan</think>',
          complete: true,
        );
        _emitRsXmlBlock(
          controller,
          blockId: 6,
          xml: '<tool name="grep_code"></tool>',
          complete: true,
        );
        await tester.pump(const Duration(milliseconds: 250));

        expect(markdownA, findsOneWidget);
        expect(firstGroupHeader, findsNWidgets(2));
        expect(find.text('已完成 · 1 次思考 · 1 次调用'), findsOneWidget);
        expect(
          find.descendant(
            of: find.byType(ToolCallRow),
            matching: find.byType(StreamingCursor),
          ),
          findsOneWidget,
        );
        expect(find.text('grep_code'), findsOneWidget);
        expect(find.textContaining('2 次思考 · 2 次调用'), findsNothing);
        expect(tester.getTopLeft(markdownA), originalMarkdownOffset);

        _emitRsXmlBlock(
          controller,
          blockId: 7,
          xml: '<tool_result name="grep_code">second result</tool_result>',
          complete: true,
        );
        _emitRsTextBlock(
          controller,
          blockId: 8,
          text: 'Streaming intermediate B',
        );
        controller.add(_markdownCompleted());
        await tester.pump(const Duration(milliseconds: 900));
        await tester.pump(const Duration(milliseconds: 250));

        expect(markdownA, findsOneWidget);
        expect(
          find.text('Streaming intermediate B', findRichText: true),
          findsOneWidget,
        );
        expect(firstGroupHeader, findsNWidgets(2));
        expect(find.textContaining('正在执行'), findsNothing);
        // The second phase auto-collapses on completion; the manually opened
        // first phase stays expanded.
        expect(find.text('grep_code'), findsNothing);
        expect(find.text('read_file'), findsOneWidget);
        expect(tester.getTopLeft(markdownA), originalMarkdownOffset);

        await controller.close();
      },
    );
  });

  group('formatExecutionDuration', () {
    test('formats seconds, minutes and hours with carryover correctly', () {
      expect(formatExecutionDuration(Duration.zero), '0s');
      expect(formatExecutionDuration(const Duration(seconds: 45)), '45s');
      expect(formatExecutionDuration(const Duration(seconds: 59)), '59s');
      expect(formatExecutionDuration(const Duration(seconds: 60)), '1min');
      expect(formatExecutionDuration(const Duration(seconds: 61)), '1min 1s');
      expect(formatExecutionDuration(const Duration(seconds: 90)), '1min 30s');
      expect(formatExecutionDuration(const Duration(seconds: 120)), '2min');
      expect(
        formatExecutionDuration(const Duration(seconds: 3599)),
        '59min 59s',
      );
      expect(formatExecutionDuration(const Duration(seconds: 3600)), '1h');
      expect(formatExecutionDuration(const Duration(seconds: 3605)), '1h 5s');
      expect(formatExecutionDuration(const Duration(seconds: 3660)), '1h 1min');
      expect(
        formatExecutionDuration(const Duration(seconds: 3665)),
        '1h 1min 5s',
      );
      expect(formatExecutionDuration(const Duration(seconds: 7320)), '2h 2min');
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
  String? nodeType = 'XmlBlock',
}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownBlockChunk',
    value: value,
    id: null,
    blockId: blockId,
    inlineId: null,
    parentBlockId: parentBlockId,
    nodeType: nodeType,
    headerLevel: null,
    xml: _xmlEvent(value),
  );
}

MarkdownStreamEvent _markdownBlockEnd(
  int blockId, {
  String? nodeType,
  String? xmlContent,
}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownBlockEnd',
    value: null,
    id: null,
    blockId: blockId,
    inlineId: null,
    parentBlockId: null,
    nodeType: nodeType,
    headerLevel: null,
    xml: xmlContent == null ? null : _xmlEvent(xmlContent),
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
  bool complete = false,
}) {
  controller
    ..add(_markdownChunk(xml))
    ..add(_markdownBlockStart(blockId))
    ..add(_markdownBlockChunk(blockId, xml));
  if (complete) {
    controller.add(
      _markdownBlockEnd(blockId, nodeType: 'XmlBlock', xmlContent: xml),
    );
  }
}

void _emitRsTextBlock(
  StreamController<Object> controller, {
  required int blockId,
  required String text,
}) {
  controller
    ..add(_markdownChunk(text))
    ..add(_markdownBlockStart(blockId, nodeType: null))
    ..add(_markdownBlockChunk(blockId, text, nodeType: null))
    ..add(_markdownBlockEnd(blockId));
}
