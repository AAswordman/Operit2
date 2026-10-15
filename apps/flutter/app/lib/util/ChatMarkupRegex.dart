// ignore_for_file: file_names

class ChatMarkupRegex {
  static const toolTagNameRegexSource =
      r'tool(?:_(?!result(?:_|$))[A-Za-z0-9_]+)?';
  static const toolResultTagNameRegexSource = r'tool_result(?:_[A-Za-z0-9_]+)?';

  static final _toolTagNameRegex = RegExp(
    '^$toolTagNameRegexSource\$',
    caseSensitive: false,
  );
  static final _toolResultTagNameRegex = RegExp(
    '^$toolResultTagNameRegexSource\$',
    caseSensitive: false,
  );
  static final _openingTagNameRegex = RegExp(r'<([A-Za-z][A-Za-z0-9_]*)');

  static final xmlBlockStartTag = RegExp(
    '<(think|thinking|search|status|$toolResultTagNameRegexSource|'
    '$toolTagNameRegexSource|html|mood|font|details|detail|meta)\\b[^>]*>',
    caseSensitive: false,
    dotAll: true,
  );

  static final proxySenderTag = RegExp(
    r'<proxy_sender\s+name="([^"]+)"\s*/>',
    caseSensitive: false,
  );
  static final nameAttr = RegExp(r'name\s*=\s*"([^"]+)"', caseSensitive: false);
  static final replyToTag = RegExp(
    r'<reply_to\s+sender="([^"]+)"\s+timestamp="([^"]+)">([^<]*)</reply_to>',
  );
  static final attachmentDataTag = RegExp(
    r'<attachment\s+id="([^"]+)"\s+filename="([^"]+)"\s+type="([^"]+)"'
    r'(?:\s+(?:node_id|path)="[^"]*")*'
    r'(?:\s+size="([^"]+)")?'
    r'(?:\s+(?:node_id|path)="[^"]*")*'
    r'\s*>([\s\S]*?)</attachment>',
  );
  static final attachmentDataSelfClosingTag = RegExp(
    r'<attachment\s+id="([^"]+)"\s+filename="([^"]+)"\s+type="([^"]+)"'
    r'(?:\s+(?:node_id|path)="[^"]*")*'
    r'(?:\s+size="([^"]+)")?'
    r'(?:\s+(?:node_id|path)="[^"]*")*'
    r'(?:\s+content="(.*?)")?\s*/>',
    dotAll: true,
  );
  static final attachmentTag = RegExp(
    r'<attachment\b[\s\S]*?</attachment>',
    caseSensitive: false,
    dotAll: true,
  );
  static final attachmentSelfClosingTag = RegExp(
    r'<attachment\b[\s\S]*?/>',
    caseSensitive: false,
    dotAll: true,
  );
  static final workspaceAttachmentTag = RegExp(
    r'<workspace_attachment\b[\s\S]*?</workspace_attachment>',
    caseSensitive: false,
    dotAll: true,
  );

  static final _mediaLinkTag = RegExp(
    r'''<link\b[^>]*(?:/\s*>|>[\s\S]*?</link\s*>)''',
    caseSensitive: false,
    dotAll: true,
  );
  static final _mediaLinkAttribute = <String, RegExp>{
    'type': RegExp(
      r'''\btype\s*=\s*\\*["']?([^"'\\\s>]+)\\*["']?''',
      caseSensitive: false,
    ),
    'id': RegExp(
      r'''\bid\s*=\s*\\*["']?([^"'\\\s>]+)\\*["']?''',
      caseSensitive: false,
    ),
    'filename': RegExp(
      r'''\bfilename\s*=\s*\\*["']?([^"'\\\s>]+)\\*["']?''',
      caseSensitive: false,
    ),
  };

  static bool isToolTagName(String? tagName) {
    return tagName != null && _toolTagNameRegex.hasMatch(tagName);
  }

  static bool isToolResultTagName(String? tagName) {
    return tagName != null && _toolResultTagNameRegex.hasMatch(tagName);
  }

  static String? normalizeToolLikeTagName(String? tagName) {
    if (isToolTagName(tagName)) {
      return 'tool';
    }
    if (isToolResultTagName(tagName)) {
      return 'tool_result';
    }
    return tagName;
  }

  static String? extractOpeningTagName(String xml) {
    return _openingTagNameRegex.firstMatch(xml.trim())?.group(1);
  }

  /// Extracts canonical image, audio, and video pool links in source order.
  static List<ChatMediaLink> extractMediaLinks(String message) {
    final links = <ChatMediaLink>[];
    final seen = <String>{};
    for (final match in _mediaLinkTag.allMatches(message)) {
      final source = match.group(0)!;
      final type = _mediaLinkAttribute['type']
          ?.firstMatch(source)
          ?.group(1)
          ?.toLowerCase();
      final id = _mediaLinkAttribute['id']?.firstMatch(source)?.group(1);
      if (type == null || id == null || id == 'error') {
        continue;
      }
      if (!const <String>{'image', 'audio', 'video', 'file'}.contains(type)) {
        continue;
      }
      final fileNameMatch = type == 'file'
          ? _mediaLinkAttribute['filename']?.firstMatch(source)
          : null;
      final fileName = fileNameMatch == null
          ? null
          : decodeChatXmlText(fileNameMatch.group(1)!);
      if (type == 'file' && (fileName == null || fileName.isEmpty)) {
        continue;
      }
      if (seen.add('$type:$id')) {
        links.add(ChatMediaLink(type: type, id: id, fileName: fileName));
      }
    }
    return links;
  }

  /// Removes canonical image, audio, and video pool links from message text.
  static String removeMediaLinks(String message) {
    return message.replaceAllMapped(_mediaLinkTag, (match) {
      final source = match.group(0)!;
      final type = _mediaLinkAttribute['type']
          ?.firstMatch(source)
          ?.group(1)
          ?.toLowerCase();
      return const <String>{'image', 'audio', 'video', 'file'}.contains(type)
          ? ''
          : source;
    });
  }
}

/// Identifies one canonical media-pool link in chat markup.
class ChatMediaLink {
  const ChatMediaLink({required this.type, required this.id, this.fileName});

  final String type;
  final String id;
  final String? fileName;
}

/// Decodes XML entities from attachment text after markup parsing.
String decodeChatXmlText(String input) {
  return input
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .replaceAll('&quot;', '"')
      .replaceAll('&apos;', "'")
      .replaceAll('&amp;', '&');
}
