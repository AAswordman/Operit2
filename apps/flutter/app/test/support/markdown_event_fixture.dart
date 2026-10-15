import 'package:xml/xml.dart' as xml;

import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';

/// Models Core's semantic block classification instead of treating XML as text.
Future<List<MarkdownStreamEvent>> splitMarkdownEventFixture(
  String content,
) async {
  final events = <MarkdownStreamEvent>[];
  final xmlBlocks = RegExp(
    r'<(think|thinking|tool|tool_result|status)\b[^>]*>[\s\S]*?</\1>|<status\b[^>]*/>',
  );
  var offset = 0;
  var blockId = 0;
  void append(String text, String? type) {
    if (text.isEmpty) return;
    final id = ++blockId;
    final element = type == 'XmlBlock'
        ? xml.XmlDocument.parse(text).rootElement
        : null;
    final metadata = element == null
        ? null
        : MarkdownXmlStreamEvent(
            tagName: element.name.local,
            attributes: {
              for (final attr in element.attributes)
                attr.name.local: attr.value,
            },
            bodyChunk: element.children
                .map((child) => child.toXmlString())
                .join(),
            children: [],
            isClosed: true,
          );
    for (final eventType in [
      'markdownBlockStart',
      'markdownBlockChunk',
      'markdownBlockEnd',
    ]) {
      events.add(
        MarkdownStreamEvent(
          chatId: 'test',
          eventType: eventType,
          value: eventType == 'markdownBlockChunk' ? text : null,
          id: null,
          blockId: id,
          inlineId: null,
          parentBlockId: null,
          nodeType: type,
          headerLevel: null,
          xml: metadata,
        ),
      );
    }
  }

  for (final match in xmlBlocks.allMatches(content)) {
    append(content.substring(offset, match.start), null);
    append(match.group(0)!, 'XmlBlock');
    offset = match.end;
  }
  append(content.substring(offset), null);
  events.add(
    const MarkdownStreamEvent(
      chatId: 'test',
      eventType: 'completed',
      value: null,
      id: null,
      blockId: null,
      inlineId: null,
      parentBlockId: null,
      nodeType: null,
      headerLevel: null,
      xml: null,
    ),
  );
  return events;
}
