import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';

void main() {
  group('ThinkToolsXmlNodeGrouper.group', () {
    test(
      'all mode collapses a thinking block with at least two tool calls',
      () {
        final items =
            const ThinkToolsXmlNodeGrouper(
              showThinkingProcess: true,
              toolCollapseMode: ToolCollapseMode.all,
            ).group(<MarkdownNodeStable>[
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
      },
    );

    test('all mode keeps a single tool call expanded', () {
      final items =
          const ThinkToolsXmlNodeGrouper(
            showThinkingProcess: true,
            toolCollapseMode: ToolCollapseMode.all,
          ).group(<MarkdownNodeStable>[
            _xml('<think>plan</think>'),
            _xml('<tool name="read_file"></tool>'),
            _xml('<tool_result name="read_file"></tool_result>'),
          ], 'renderer');

      expect(items, hasLength(3));
      expect(items, everyElement(isA<MarkdownSingleItem>()));
    });

    test('full mode collapses even one tool call', () {
      final items =
          const ThinkToolsXmlNodeGrouper(
            showThinkingProcess: true,
            toolCollapseMode: ToolCollapseMode.full,
          ).group(<MarkdownNodeStable>[
            _xml('<think>plan</think>'),
            _xml('<tool name="edit_file"></tool>'),
            _xml('<tool_result name="edit_file"></tool_result>'),
          ], 'renderer');

      expect(items, hasLength(1));
      final group = items.single as MarkdownGroupItem;
      expect(group.startIndex, 0);
      expect(group.endIndexInclusive, 2);
    });

    test('readOnly mode groups readonly tools but stops at write tools', () {
      final readonlyItems =
          const ThinkToolsXmlNodeGrouper(
            showThinkingProcess: true,
            toolCollapseMode: ToolCollapseMode.readOnly,
          ).group(<MarkdownNodeStable>[
            _xml('<think>plan</think>'),
            _xml('<tool name="read_file"></tool>'),
            _xml('<tool_result name="read_file"></tool_result>'),
            _xml('<tool name="grep_code"></tool>'),
            _xml('<tool_result name="grep_code"></tool_result>'),
          ], 'renderer');
      expect(readonlyItems.single, isA<MarkdownGroupItem>());

      final writeItems =
          const ThinkToolsXmlNodeGrouper(
            showThinkingProcess: true,
            toolCollapseMode: ToolCollapseMode.readOnly,
          ).group(<MarkdownNodeStable>[
            _xml('<think>plan</think>'),
            _xml('<tool name="edit_file"></tool>'),
            _xml('<tool_result name="edit_file"></tool_result>'),
          ], 'renderer');
      expect(writeItems, hasLength(3));
      expect(writeItems, everyElement(isA<MarkdownSingleItem>()));
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
                toolCollapseMode: ToolCollapseMode.full,
              ),
              showThinkingProcess: true,
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
        );
      await tester.pump(const Duration(milliseconds: 250));

      expect(find.textContaining('read_file'), findsWidgets);

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
                toolCollapseMode: ToolCollapseMode.full,
              ),
              showThinkingProcess: true,
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
                toolCollapseMode: ToolCollapseMode.all,
              ),
              showThinkingProcess: true,
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
  return MarkdownNodeStable(
    type: MarkdownNodeType.xmlBlock,
    content: content,
    isStreaming: isStreaming,
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
    ..add(_markdownCompleted(parentBlockId: 1));
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
