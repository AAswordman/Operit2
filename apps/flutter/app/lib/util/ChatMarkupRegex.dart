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

  static final memoryTag = RegExp(
    r'<memory>.*?</memory>',
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
    r'<attachment\s+id="([^"]+)"\s+filename="([^"]+)"\s+type="([^"]+)"(?:\s+size="([^"]+)")?\s*>([\s\S]*?)</attachment>',
  );
  static final attachmentDataSelfClosingTag = RegExp(
    r'<attachment\s+id="([^"]+)"\s+filename="([^"]+)"\s+type="([^"]+)"(?:\s+size="([^"]+)")?(?:\s+content="(.*?)")?\s*/>',
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
}
