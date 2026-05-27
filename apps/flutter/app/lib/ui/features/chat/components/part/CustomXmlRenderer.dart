// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import 'ToolDisplayComponents.dart';
import 'ToolResultDisplay.dart';

class CustomXmlRenderer extends StatelessWidget {
  const CustomXmlRenderer({
    super.key,
    required this.xmlContent,
    required this.isStreaming,
    required this.textColor,
  });

  final String xmlContent;
  final bool isStreaming;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final parsed = _ParsedXml.from(xmlContent);
    if (_shouldHideGeminiThoughtSignatureMeta(xmlContent, parsed.tagName)) {
      return const SizedBox.shrink();
    }
    if (!_isXmlFullyClosed(xmlContent) &&
        _builtInTags.contains(parsed.tagName) &&
        !const {
          'tool',
          'think',
          'thinking',
          'search',
        }.contains(parsed.tagName)) {
      return const SizedBox.shrink();
    }
    switch (parsed.tagName) {
      case 'think':
      case 'thinking':
        return _ThinkPanel(
          text: parsed.body,
          textColor: textColor,
          isStreaming: isStreaming,
        );
      case 'search':
        return _LabeledPanel(
          label: 'Search',
          text: parsed.body,
          color: Theme.of(context).colorScheme.tertiary,
          isStreaming: isStreaming,
        );
      case 'status':
        return _StatusChip(text: parsed.body, isStreaming: isStreaming);
      case 'tool':
        return _ToolRequestRenderer(
          xmlContent: xmlContent,
          parsed: parsed,
          textColor: textColor,
          isStreaming: isStreaming,
        );
      case 'tool_result':
        final result = _extractToolResult(parsed, xmlContent);
        return ToolResultDisplay(
          toolName: result.toolName,
          result: result.resultContent,
          isSuccess: result.isSuccess,
          isStreaming: isStreaming,
        );
      case 'html':
      case 'details':
      case 'detail':
      case 'font':
      case 'mood':
        return StreamMarkdownRenderer(
          content: parsed.body,
          isStreaming: isStreaming,
          textColor: textColor,
          backgroundColor: Theme.of(context).colorScheme.surface,
        );
    }
    return SelectableText(
      parsed.body.isEmpty ? xmlContent : parsed.body,
      style: Theme.of(
        context,
      ).textTheme.bodyMedium?.copyWith(color: textColor, height: 1.45),
    );
  }
}

const _toolParamTokenThreshold = 50;

const _builtInTags = <String>{
  'think',
  'thinking',
  'search',
  'tool',
  'status',
  'tool_result',
  'html',
  'mood',
  'font',
  'details',
  'detail',
  'meta',
};

class _ToolRequestRenderer extends StatelessWidget {
  const _ToolRequestRenderer({
    required this.xmlContent,
    required this.parsed,
    required this.textColor,
    required this.isStreaming,
  });

  final String xmlContent;
  final _ParsedXml parsed;
  final Color textColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final rawToolName = parsed.attr('name') ?? 'Unknown tool';
    final params = _extractParamsFromTool(xmlContent);
    final paramText = _extractContentFromXml(
      xmlContent,
      tagName: 'tool',
    ).trim();
    final displayToolName = _resolveToolDisplayNameForRender(
      rawToolName,
      params,
    );
    final isClosed = _isXmlFullyClosed(xmlContent);
    final paramTokenEstimate = _estimateTokenCount(paramText);

    if (displayToolName == 'apply_file' ||
        displayToolName == 'create_file' ||
        displayToolName == 'edit_file') {
      if (isClosed) {
        return CompactToolDisplay(
          toolName: rawToolName,
          params: paramText,
          textColor: textColor,
          isStreaming: isStreaming,
        );
      }
      return DetailedToolDisplay(
        toolName: rawToolName,
        params: paramText,
        textColor: textColor,
        isStreaming: isStreaming,
      );
    }

    if (!isClosed && paramTokenEstimate > _toolParamTokenThreshold) {
      return DetailedToolDisplay(
        toolName: rawToolName,
        params: paramText,
        textColor: textColor,
        isStreaming: isStreaming,
      );
    }

    return CompactToolDisplay(
      toolName: rawToolName,
      params: paramText,
      textColor: textColor,
      isStreaming: isStreaming,
    );
  }
}

class _ToolResultRenderState {
  const _ToolResultRenderState({
    required this.toolName,
    required this.isSuccess,
    required this.resultContent,
  });

  final String toolName;
  final bool isSuccess;
  final String resultContent;
}

class _ThinkPanel extends StatelessWidget {
  const _ThinkPanel({
    required this.text,
    required this.textColor,
    required this.isStreaming,
  });

  final String text;
  final Color textColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    return _LabeledPanel(
      label: 'Thinking',
      text: text,
      color: Theme.of(context).colorScheme.secondary,
      isStreaming: isStreaming,
    );
  }
}

class _LabeledPanel extends StatelessWidget {
  const _LabeledPanel({
    required this.label,
    required this.text,
    required this.color,
    required this.isStreaming,
  });

  final String label;
  final String text;
  final Color color;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.fromLTRB(10, 8, 10, 8),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(8),
        border: Border(
          left: BorderSide(color: color.withValues(alpha: 0.55), width: 3),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            label,
            style: theme.textTheme.labelSmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 4),
          SelectableText(
            text,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              height: 1.35,
            ),
          ),
          if (isStreaming)
            const Padding(
              padding: EdgeInsets.only(top: 4),
              child: StreamingCursor(),
            ),
        ],
      ),
    );
  }
}

class _StatusChip extends StatelessWidget {
  const _StatusChip({required this.text, required this.isStreaming});

  final String text;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(
          alpha: 0.55,
        ),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(
            Icons.info_outline,
            size: 14,
            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.72),
          ),
          const SizedBox(width: 6),
          Flexible(
            child: Text(
              text,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          if (isStreaming)
            const Padding(
              padding: EdgeInsets.only(left: 6),
              child: StreamingCursor(),
            ),
        ],
      ),
    );
  }
}

class _ParsedXml {
  const _ParsedXml({
    required this.tagName,
    required this.attributes,
    required this.body,
  });

  factory _ParsedXml.from(String xml) {
    final open = RegExp(
      r'^<([a-zA-Z_][\w:-]*)\b([^>]*)>',
      dotAll: true,
    ).firstMatch(xml.trim());
    if (open == null) {
      return _ParsedXml(tagName: '', attributes: const {}, body: xml);
    }
    final rawTagName = open.group(1)!;
    final tagName = _normalizeToolLikeTagName(rawTagName);
    final attributes = _parseAttributes(open.group(2) ?? '');
    final closeTag = '</${rawTagName.toLowerCase()}>';
    final lowerXml = xml.toLowerCase();
    final closeIndex = lowerXml.lastIndexOf(closeTag);
    final bodyEnd = closeIndex > open.end ? closeIndex : xml.length;
    return _ParsedXml(
      tagName: tagName,
      attributes: attributes,
      body: xml.substring(open.end, bodyEnd).trim(),
    );
  }

  final String tagName;
  final Map<String, String> attributes;
  final String body;

  String? attr(String name) {
    return attributes[name.toLowerCase()];
  }

  bool get success {
    final value = attr('success') ?? attr('status') ?? attr('ok');
    if (value == null) {
      return true;
    }
    return !const {
      'false',
      'failed',
      'error',
      '0',
    }.contains(value.toLowerCase());
  }
}

String _normalizeToolLikeTagName(String tagName) {
  final lower = tagName.toLowerCase();
  if (lower == 'tool-result') {
    return 'tool_result';
  }
  return lower;
}

Map<String, String> _parseAttributes(String source) {
  final result = <String, String>{};
  final pattern = RegExp(
    r'''([\w:-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+))''',
  );
  for (final match in pattern.allMatches(source)) {
    result[match.group(1)!.toLowerCase()] =
        match.group(2) ?? match.group(3) ?? match.group(4) ?? '';
  }
  return result;
}

bool _shouldHideGeminiThoughtSignatureMeta(String content, String tagName) {
  return tagName == 'meta' &&
      RegExp(
        r'''\bprovider\s*=\s*["']gemini:thought_signature["']''',
        caseSensitive: false,
      ).hasMatch(content);
}

bool _isXmlFullyClosed(String xml) {
  final trimmed = xml.trim();
  final rawTagName = _extractRawXmlTagName(trimmed);
  if (rawTagName == null) {
    return false;
  }
  if (trimmed.endsWith('/>')) {
    return true;
  }
  return trimmed.contains('</$rawTagName>');
}

String? _extractRawXmlTagName(String xml) {
  return RegExp(r'^<([a-zA-Z_][\w:-]*)\b').firstMatch(xml.trim())?.group(1);
}

String _extractContentFromXml(String content, {String? tagName}) {
  final rawTagName = _extractRawXmlTagName(content);
  if (rawTagName == null) {
    return content;
  }
  final effectiveTagName = tagName ?? rawTagName;
  final startTag = '<$effectiveTagName';
  final startTagIndex = content.indexOf(startTag);
  if (startTagIndex < 0) {
    return content;
  }
  final startTagEnd = content.indexOf('>', startTagIndex);
  if (startTagEnd < 0) {
    return content;
  }
  final endTag = '</$effectiveTagName>';
  final endIndex = content.lastIndexOf(endTag);
  final contentEndExclusive = endIndex > startTagEnd
      ? endIndex
      : content.length;
  return content.substring(startTagEnd + 1, contentEndExclusive).trim();
}

Map<String, String> _extractParamsFromTool(String content) {
  final params = <String, String>{};
  final pattern = RegExp(
    r'''<param\b[^>]*name=["']([^"']+)["'][^>]*>([\s\S]*?)<\/param>''',
    caseSensitive: false,
  );
  for (final match in pattern.allMatches(content)) {
    params[match.group(1)!] = match.group(2)!.trim();
  }
  return params;
}

String _resolveToolDisplayNameForRender(
  String toolName,
  Map<String, String> params,
) {
  if (toolName != 'package_proxy' && toolName != 'proxy') {
    return toolName;
  }
  final targetToolName = params['tool_name'] == null
      ? ''
      : normalizeEscapedTextForDisplay(params['tool_name']!).trim();
  return targetToolName.isNotEmpty ? targetToolName : toolName;
}

int _estimateTokenCount(String text) {
  var chineseCharCount = 0;
  var otherCharCount = 0;
  for (final codePoint in text.runes) {
    if (codePoint >= 0x4E00 && codePoint <= 0x9FFF) {
      chineseCharCount++;
    } else {
      otherCharCount++;
    }
  }
  return (chineseCharCount * 1.5 + otherCharCount * 0.25).toInt();
}

_ToolResultRenderState _extractToolResult(
  _ParsedXml parsed,
  String xmlContent,
) {
  final toolName = (parsed.attr('name') ?? '').trim();
  final status = (parsed.attr('status') ?? 'success').trim().toLowerCase();
  final contentMatch = RegExp(
    r'<content\b[^>]*>([\s\S]*?)<\/content>',
    caseSensitive: false,
  ).firstMatch(xmlContent);
  final resultContent = (contentMatch?.group(1) ?? '').trim();
  final isSuccess = status == 'success';

  if (!isSuccess) {
    final errorMatch = RegExp(
      r'<error\b[^>]*>([\s\S]*?)<\/error>',
      caseSensitive: false,
    ).firstMatch(resultContent);
    return _ToolResultRenderState(
      toolName: toolName.isEmpty ? 'Unknown tool' : toolName,
      isSuccess: false,
      resultContent: (errorMatch?.group(1) ?? resultContent).trim(),
    );
  }

  final withoutFileDiff = resultContent
      .replaceAll(
        RegExp(r'<file-diff[\s\S]*<\/file-diff>', caseSensitive: false),
        '',
      )
      .trim();
  return _ToolResultRenderState(
    toolName: toolName.isEmpty ? 'Unknown tool' : toolName,
    isSuccess: true,
    resultContent: withoutFileDiff,
  );
}
