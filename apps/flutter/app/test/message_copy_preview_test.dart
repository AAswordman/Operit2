import 'package:flutter_test/flutter_test.dart';

import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/ui/features/chat/components/MessageCopyPreview.dart';

void main() {
  /// Creates a runtime event with only the fields used by the Markdown builder.
  core_proxy.MarkdownStreamEvent event({
    required String type,
    String? value,
    int? blockId,
    int? inlineId,
    String? nodeType,
  }) {
    return core_proxy.MarkdownStreamEvent(
      chatId: 'test',
      eventType: type,
      value: value,
      id: null,
      blockId: blockId,
      inlineId: inlineId,
      parentBlockId: null,
      nodeType: nodeType,
      headerLevel: null,
    );
  }

  /// Converts a fixture into the event stream consumed by the copy renderer.
  Future<List<core_proxy.MarkdownStreamEvent>> split(
    List<core_proxy.MarkdownStreamEvent> events,
  ) async {
    return events;
  }

  test('converts Markdown AST nodes into readable copy text', () async {
    final events = <core_proxy.MarkdownStreamEvent>[
      event(type: 'markdownBlockStart', blockId: 1, nodeType: null),
      event(
        type: 'markdownInlineStart',
        blockId: 1,
        inlineId: 1,
        nodeType: 'Bold',
      ),
      event(
        type: 'markdownInlineChunk',
        blockId: 1,
        inlineId: 1,
        value: '标题',
        nodeType: 'Bold',
      ),
      event(type: 'markdownInlineStart', blockId: 1, inlineId: 2),
      event(type: 'markdownInlineChunk', blockId: 1, inlineId: 2, value: ' '),
      event(
        type: 'markdownInlineStart',
        blockId: 1,
        inlineId: 3,
        nodeType: 'Link',
      ),
      event(
        type: 'markdownInlineChunk',
        blockId: 1,
        inlineId: 3,
        value: '[链接](https://example.com)',
        nodeType: 'Link',
      ),
      event(type: 'markdownBlockStart', blockId: 2, nodeType: 'CodeBlock'),
      event(
        type: 'markdownBlockChunk',
        blockId: 2,
        nodeType: 'CodeBlock',
        value: '```dart\nfinal value = 1;\n```',
      ),
      event(type: 'completed'),
    ];

    final result = await markdownToPlainTextForCopy(
      'ignored',
      splitMarkdownContent: (_) => split(events),
    );

    expect(
      result,
      '标题链接 (https://example.com)\n\n----dart-----\nfinal value = 1;',
    );
  });

  test('converts Markdown tables into tab-separated text', () async {
    final events = <core_proxy.MarkdownStreamEvent>[
      event(type: 'markdownBlockStart', blockId: 1, nodeType: 'Table'),
      event(
        type: 'markdownBlockChunk',
        blockId: 1,
        nodeType: 'Table',
        value: '| 名称 | 数量 |\n| --- | ---: |\n| 苹果 | **2** |',
      ),
      event(type: 'completed'),
    ];

    final result = await markdownToPlainTextForCopy(
      'ignored',
      splitMarkdownContent: (_) => split(events),
    );

    expect(result, '名称\t数量\n苹果\t2');
  });
}
