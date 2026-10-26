/// Helpers for recovering readable content out of protocol blocks that the XML renderer could not
/// classify.
///
/// Why this exists: a chat part is rendered through an XML block renderer, and that renderer used to
/// fall back to "show the raw text" whenever the tag name was unknown. When the block is really
/// protocol markup (a stream boundary cut the opening tag in half, or the tag name is malformed),
/// that fallback printed `<tool …>`, `<param …>`, `</tool_result>` straight into the transcript.
/// Users reported it as a leak; on device it looked like pages of raw XML in the chat.
///
/// The rule these helpers implement: never show the markup itself. Show the `<content>` payload if
/// there is one, otherwise show nothing.
library;

/// Tag names that only ever appear in protocol markup, never in prose the user should read.
const List<String> _protocolTags = <String>[
  'tool',
  'tool_call',
  'tool_calls',
  'tool_result',
  'param',
  'parameter',
  'content',
  'invoke',
  'result',
  'status',
  // 真机截图里漏过的：文件改动块。渲染层解析不出来时它整块进了正文。
  'file-diff',
  'file_diff',
  'diff',
];

final RegExp _protocolMarkupPattern = RegExp(
  '</?(?:${_protocolTags.join('|')})\\b',
  caseSensitive: false,
);

/// 任何"带属性的开标签"都当作协议块。
///
/// 只靠标签白名单会一直漏（`file-diff` 就是这么漏的）：渲染层每加一种语义标签，
/// 白名单就得跟着改。这里补一条与标签名无关的判据 —— 普通正文几乎不会出现
/// `<名字 属性="…">` 这种形状，而协议块一定会有。
final RegExp _attributedTagPattern = RegExp(
  r'<[a-z][a-z0-9_-]*\s+[a-z][a-z0-9_-]*\s*=',
  caseSensitive: false,
);

/// 可读负载：`<content>…</content>`，或协议块里常见的 CDATA（例如 file-diff 里的 diff 内容）。
final RegExp _contentPattern = RegExp(
  r'<content>([\s\S]*?)</content>',
  caseSensitive: false,
);

final RegExp _cdataPattern = RegExp(r'<!\[CDATA\[([\s\S]*?)\]\]>');

/// True when [value] is protocol markup rather than ordinary text.
bool looksLikeProtocolMarkup(String value) =>
    _protocolMarkupPattern.hasMatch(value) || _attributedTagPattern.hasMatch(value);

/// Returns the readable payload of a protocol block, or null when it has none.
///
/// The payload is entity-decoded because protocol payloads are XML-escaped.
String? protocolMarkupPayload(String value) {
  if (!looksLikeProtocolMarkup(value)) {
    return null;
  }
  final content = _contentPattern.firstMatch(value);
  final rawContent = content?.group(1)?.trim();
  if (rawContent != null && rawContent.isNotEmpty) {
    return unescapeProtocolEntities(rawContent);
  }
  final cdata = _cdataPattern.firstMatch(value);
  final rawCdata = cdata?.group(1)?.trim();
  if (rawCdata != null && rawCdata.isNotEmpty) {
    return unescapeProtocolEntities(rawCdata);
  }
  return null;
}

/// Minimal XML entity decoding for recovered payloads.
String unescapeProtocolEntities(String value) {
  return value
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .replaceAll('&quot;', '"')
      .replaceAll('&apos;', "'")
      .replaceAll('&amp;', '&');
}
