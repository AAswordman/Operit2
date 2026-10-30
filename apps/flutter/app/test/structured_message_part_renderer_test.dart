import 'support/markdown_event_fixture.dart';

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';
import 'package:operit2/ui/features/chat/components/part/CustomXmlRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/FileDiffDisplay.dart';
import 'package:operit2/ui/features/chat/components/part/StructuredMessagePartRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';

/// Verifies semantic message parts keep the established XML render behavior.
void main() {
  testWidgets('renders semantic thinking with the shared thinking panel', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: StreamingStructuredMessageRenderer(
            parts: const <MessagePart>[
              MessagePart(
                partId: 'thinking',
                sequence: 0,
                kind: MessagePartKind.thinking,
                content: 'reasoning',
                toolCallId: null,
                toolName: null,
                attributes: <String, String>{},
              ),
            ],
            textColor: Colors.black,
            backgroundColor: Colors.white,
            showThinkingProcess: true,
            contentStream: null,
            streamState: StreamMarkdownRendererState(),
            splitMarkdownContent: _splitMarkdownContent,
            nodeGrouper: const NoopMarkdownNodeGrouper(),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byType(CustomXmlRenderer), findsOneWidget);
    expect(find.byType(ExpansionTile), findsNothing);
  });

  testWidgets('keeps streamed thinking body mounted when height toggles', (
    tester,
  ) async {
    var listenCount = 0;
    final markdownController = StreamController<Object>(
      onListen: () {
        listenCount += 1;
      },
    );
    addTearDown(markdownController.close);

    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: SizedBox(
            width: 360,
            child: CustomXmlRenderer(
              xmlContent: '<think>streaming body',
              isStreaming: true,
              textColor: Colors.black,
              xmlStream: const Stream<String>.empty(),
              xmlMarkdownEventStream: markdownController.stream,
              showThinkingProcess: true,
              initialThinkingExpanded: true,
              splitMarkdownContent: _splitMarkdownContent,
            ),
          ),
        ),
      ),
    );

    markdownController
      ..add(_markdownEvent('markdownBlockStart', blockId: 1))
      ..add(
        _markdownEvent(
          'markdownBlockChunk',
          blockId: 1,
          value: 'streaming body',
        ),
      );
    await tester.pump(const Duration(milliseconds: 240));
    await tester.pump();

    final bodyToggle = find.byKey(
      const ValueKey<String>('thinking-body-toggle'),
    );
    expect(bodyToggle, findsOneWidget);

    tester.widget<GestureDetector>(bodyToggle).onTap!();
    await tester.pump(const Duration(milliseconds: 120));
    tester.widget<GestureDetector>(bodyToggle).onTap!();
    await tester.pump(const Duration(milliseconds: 120));

    expect(listenCount, 1);
  });

  testWidgets('renders completion status with the established status card', (
    tester,
  ) async {
    await tester.pumpWidget(
      _messagePartApp(const <MessagePart>[
        MessagePart(
          partId: 'status',
          sequence: 0,
          kind: MessagePartKind.status,
          content: 'ignored completion body',
          toolCallId: null,
          toolName: null,
          attributes: <String, String>{'type': 'completion'},
        ),
      ], nodeGrouper: const NoopMarkdownNodeGrouper()),
    );
    await tester.pumpAndSettle();

    expect(find.text('✓ Task completed'), findsOneWidget);
  });

  testWidgets('uses the shared tool row for completed structured tool calls', (
    tester,
  ) async {
    await tester.pumpWidget(
      _messagePartApp(const <MessagePart>[
        MessagePart(
          partId: 'tool-call',
          sequence: 0,
          kind: MessagePartKind.toolCall,
          content: '',
          toolCallId: 'call-1',
          toolName: 'read_file',
          attributes: <String, String>{'path': 'README.md'},
        ),
      ], nodeGrouper: const NoopMarkdownNodeGrouper()),
    );
    await tester.pumpAndSettle();

    expect(find.byType(ToolCallRow), findsOneWidget);
    expect(find.text('read_file'), findsOneWidget);
    await tester.tap(find.text('read_file'));
    await tester.pumpAndSettle();
    expect(find.textContaining('path: README.md'), findsOneWidget);
  });

  testWidgets('preserves structured tool-result error and file-diff rendering', (
    tester,
  ) async {
    await tester.pumpWidget(
      _messagePartApp(const <MessagePart>[
        MessagePart(
          partId: 'tool-result-error',
          sequence: 0,
          kind: MessagePartKind.toolResult,
          content: '<error>permission denied</error>',
          toolCallId: 'call-1',
          toolName: 'read_file',
          attributes: <String, String>{'status': 'FAILED'},
        ),
        MessagePart(
          partId: 'tool-result-diff',
          sequence: 1,
          kind: MessagePartKind.toolResult,
          content:
              '<file-diff path="lib/main.dart" details="updated">+line</file-diff>',
          toolCallId: 'call-2',
          toolName: 'apply_file',
          attributes: <String, String>{
            'status': 'SUCCESS',
            'display': '{"label":{"en":"Edit"},"resultRenderer":"fileDiff"}',
          },
        ),
      ], nodeGrouper: const NoopMarkdownNodeGrouper()),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('read_file'));
    await tester.pumpAndSettle();

    expect(find.text('permission denied'), findsOneWidget);
    expect(find.byType(FileDiffDisplay), findsOneWidget);
    expect(
      tester.widget<FileDiffDisplay>(find.byType(FileDiffDisplay)).diff.path,
      'lib/main.dart',
    );
    expect(find.textContaining('line'), findsWidgets);
  });

  testWidgets(
    'replays persisted display and typed results without a tool registry',
    (tester) async {
      await tester.pumpWidget(
        _messagePartApp([
          MessagePart(
            partId: 'persisted-result',
            sequence: 0,
            kind: MessagePartKind.toolResult,
            content: 'remote prose',
            toolCallId: 'call-42',
            toolName: 'example:z42',
            attributes: {
              'status': 'success',
              'display': jsonEncode({
                'label': {'en': 'Inspect & verify'},
                'icon': 'folder',
                'resultRenderer': 'directory',
              }),
              'result_data': jsonEncode({
                '__type': 'DirectoryListingData',
                'path': '/remote/workspace',
                'entries': [
                  {'name': 'kept.txt', 'isDirectory': false, 'size': 12},
                ],
              }),
            },
          ),
        ], nodeGrouper: const NoopMarkdownNodeGrouper()),
      );
      await tester.pumpAndSettle();
      expect(find.text('Inspect & verify'), findsOneWidget);
      await tester.tap(find.text('Inspect & verify'));
      await tester.pumpAndSettle();
      expect(find.text('kept.txt'), findsOneWidget);
      expect(find.text('/remote/workspace'), findsOneWidget);
    },
  );

  testWidgets('groups persisted thinking and one tool call like live XML', (
    tester,
  ) async {
    await tester.pumpWidget(
      _messagePartApp(
        const <MessagePart>[
          MessagePart(
            partId: 'thinking-1',
            sequence: 0,
            kind: MessagePartKind.thinking,
            content: 'need current time',
            toolCallId: null,
            toolName: null,
            attributes: <String, String>{},
          ),
          MessagePart(
            partId: 'tool-call-1',
            sequence: 1,
            kind: MessagePartKind.toolCall,
            content: '',
            toolCallId: 'call-1',
            toolName: 'daily_life:get_current_time',
            attributes: <String, String>{},
          ),
          MessagePart(
            partId: 'tool-result-1',
            sequence: 2,
            kind: MessagePartKind.toolResult,
            content: '03:08 AM',
            toolCallId: 'call-1',
            toolName: 'daily_life:get_current_time',
            attributes: <String, String>{
              'status': 'SUCCESS',
              'display': '{"label":{"en":"Edit"},"resultRenderer":"fileDiff"}',
            },
          ),
          MessagePart(
            partId: 'thinking-2',
            sequence: 3,
            kind: MessagePartKind.thinking,
            content: 'answer in Chinese',
            toolCallId: null,
            toolName: null,
            attributes: <String, String>{},
          ),
        ],
        locale: const Locale('zh'),
        nodeGrouper: const ThinkToolsXmlNodeGrouper(showThinkingProcess: true),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('已完成 · 2 次思考 · 1 次调用'), findsOneWidget);
    expect(find.text('思考过程'), findsNothing);
    expect(find.byType(CompactToolDisplay), findsNothing);
  });

  testWidgets('keeps persisted Markdown between execution groups in order', (
    tester,
  ) async {
    await tester.pumpWidget(
      _messagePartApp(
        const <MessagePart>[
          MessagePart(
            partId: 'thinking-1',
            sequence: 0,
            kind: MessagePartKind.thinking,
            content: 'first plan',
            toolCallId: null,
            toolName: null,
            attributes: <String, String>{},
          ),
          MessagePart(
            partId: 'tool-call-1',
            sequence: 1,
            kind: MessagePartKind.toolCall,
            content: '',
            toolCallId: 'call-1',
            toolName: 'read_file',
            attributes: <String, String>{},
          ),
          MessagePart(
            partId: 'tool-result-1',
            sequence: 2,
            kind: MessagePartKind.toolResult,
            content: 'first result',
            toolCallId: 'call-1',
            toolName: 'read_file',
            attributes: <String, String>{
              'status': 'SUCCESS',
              'display': '{"label":{"en":"Edit"},"resultRenderer":"fileDiff"}',
            },
          ),
          MessagePart(
            partId: 'markdown-1',
            sequence: 3,
            kind: MessagePartKind.markdown,
            content: 'Persisted intermediate A',
            toolCallId: null,
            toolName: null,
            attributes: <String, String>{},
          ),
          MessagePart(
            partId: 'thinking-2',
            sequence: 4,
            kind: MessagePartKind.thinking,
            content: 'second plan',
            toolCallId: null,
            toolName: null,
            attributes: <String, String>{},
          ),
          MessagePart(
            partId: 'tool-call-2',
            sequence: 5,
            kind: MessagePartKind.toolCall,
            content: '',
            toolCallId: 'call-2',
            toolName: 'grep_code',
            attributes: <String, String>{},
          ),
          MessagePart(
            partId: 'tool-result-2',
            sequence: 6,
            kind: MessagePartKind.toolResult,
            content: 'second result',
            toolCallId: 'call-2',
            toolName: 'grep_code',
            attributes: <String, String>{
              'status': 'SUCCESS',
              'display': '{"label":{"en":"Edit"},"resultRenderer":"fileDiff"}',
            },
          ),
          MessagePart(
            partId: 'markdown-2',
            sequence: 7,
            kind: MessagePartKind.markdown,
            content: 'Persisted intermediate B',
            toolCallId: null,
            toolName: null,
            attributes: <String, String>{},
          ),
        ],
        locale: const Locale('zh'),
        nodeGrouper: const ThinkToolsXmlNodeGrouper(showThinkingProcess: true),
      ),
    );
    await tester.pumpAndSettle();

    final groupHeaders = find.text('已完成 · 1 次思考 · 1 次调用');
    final markdownA = find.text('Persisted intermediate A', findRichText: true);
    final markdownB = find.text('Persisted intermediate B', findRichText: true);
    expect(groupHeaders, findsNWidgets(2));
    expect(markdownA, findsOneWidget);
    expect(markdownB, findsOneWidget);
    expect(
      tester.getTopLeft(groupHeaders.at(0)).dy,
      lessThan(tester.getTopLeft(markdownA).dy),
    );
    expect(
      tester.getTopLeft(markdownA).dy,
      lessThan(tester.getTopLeft(groupHeaders.at(1)).dy),
    );
    expect(
      tester.getTopLeft(groupHeaders.at(1)).dy,
      lessThan(tester.getTopLeft(markdownB).dy),
    );
  });

  testWidgets(
    'restores persisted execution duration on historical message header',
    (tester) async {
      await tester.pumpWidget(
        _messagePartApp(
          <MessagePart>[
            MessagePart(
              partId: 'thinking-1',
              sequence: 0,
              kind: MessagePartKind.thinking,
              content: 'plan time lookup',
              toolCallId: null,
              toolName: null,
              attributes: <String, String>{},
            ),
            MessagePart(
              partId: 'call-1',
              sequence: 1,
              kind: MessagePartKind.toolCall,
              content: '',
              toolCallId: 'call-1',
              toolName: 'daily_life:get_current_time',
              attributes: <String, String>{},
            ),
          ],
          locale: const Locale('zh'),
          nodeGrouper: const ThinkToolsXmlNodeGrouper(
            showThinkingProcess: true,
            defaultCompletedDuration: Duration(seconds: 75),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('已完成 (1min 15s) · 1 次思考 · 1 次调用'), findsOneWidget);
    },
  );
}

/// Wraps structured parts in the localized Material test surface.
Widget _messagePartApp(
  List<MessagePart> parts, {
  Locale? locale,
  required MarkdownNodeGrouper nodeGrouper,
}) {
  return MaterialApp(
    locale: locale,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(
      body: StreamingStructuredMessageRenderer(
        parts: parts,
        contentStream: null,
        textColor: Colors.black,
        backgroundColor: Colors.white,
        showThinkingProcess: true,
        streamState: StreamMarkdownRendererState(),
        splitMarkdownContent: _splitMarkdownContent,
        nodeGrouper: nodeGrouper,
      ),
    ),
  );
}

/// Produces a complete plain Markdown event sequence for renderer tests.
Future<List<MarkdownStreamEvent>> _splitMarkdownContent(String content) =>
    splitMarkdownEventFixture(content);

/// Creates a generated Markdown stream event for renderer tests.
MarkdownStreamEvent _markdownEvent(String type, {String? value, int? blockId}) {
  return MarkdownStreamEvent(
    chatId: 'test',
    eventType: type,
    value: value,
    id: null,
    blockId: blockId,
    inlineId: null,
    parentBlockId: null,
    nodeType: null,
    headerLevel: null,
    xml: null,
  );
}
