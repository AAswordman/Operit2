// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../common/markdown/StreamMarkdownRenderer.dart';

class CompactToolDisplay extends StatelessWidget {
  const CompactToolDisplay({
    super.key,
    required this.toolName,
    required this.params,
    required this.textColor,
    required this.isStreaming,
  });

  final String toolName;
  final String params;
  final Color textColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final display = normalizeToolDisplayForStrictProxy(toolName, params);
    final summary = buildParamsHeadPreview(display.params);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          Icon(
            getToolIcon(display.toolName),
            size: 17,
            color: theme.colorScheme.primary,
          ),
          const SizedBox(width: 8),
          Text(
            display.toolName,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
          if (summary.isNotEmpty) ...<Widget>[
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                summary,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: textColor.withValues(alpha: 0.68),
                ),
              ),
            ),
          ],
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

class DetailedToolDisplay extends StatelessWidget {
  const DetailedToolDisplay({
    super.key,
    required this.toolName,
    required this.params,
    required this.textColor,
    required this.isStreaming,
  });

  final String toolName;
  final String params;
  final Color textColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final display = normalizeToolDisplayForStrictProxy(toolName, params);
    final bytes = calculateToolParamsBytes(display.params);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          Icon(
            getToolIcon(display.toolName),
            size: 17,
            color: theme.colorScheme.primary,
          ),
          const SizedBox(width: 8),
          Text(
            display.toolName,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              '$bytes B',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: textColor.withValues(alpha: 0.68),
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

class ToolDisplayData {
  const ToolDisplayData({required this.toolName, required this.params});

  final String toolName;
  final String params;
}

ToolDisplayData normalizeToolDisplayForStrictProxy(
  String toolName,
  String params,
) {
  if (toolName != 'package_proxy' && toolName != 'proxy') {
    return ToolDisplayData(toolName: toolName, params: params);
  }

  final toolNameMatch = RegExp(
    r'<param\s+name="tool_name">([\s\S]*?)<\/param>',
  ).firstMatch(params);
  final paramsMatch = RegExp(
    r'<param\s+name="params">([\s\S]*?)<\/param>',
  ).firstMatch(params);

  final rawTargetToolName = toolNameMatch?.group(1)?.trim() ?? '';
  final rawProxiedParams = paramsMatch?.group(1)?.trim() ?? '';
  final displayToolName = normalizeEscapedTextForDisplay(
    rawTargetToolName,
  ).trim();
  final displayParams = rawProxiedParams.isNotEmpty
      ? parseProxyJsonParamsToXml(
          normalizeEscapedTextForDisplay(rawProxiedParams),
        )
      : params;

  return ToolDisplayData(
    toolName: displayToolName.isNotEmpty ? displayToolName : toolName,
    params: displayParams ?? params,
  );
}

String normalizeEscapedTextForDisplay(String input) {
  final unescaped = unescapeXmlForDisplay(input).replaceAll(r'\"', '"');
  final trimmed = unescaped.trim();
  if ((trimmed.startsWith('"{') && trimmed.endsWith('}"')) ||
      (trimmed.startsWith('"[') && trimmed.endsWith(']"'))) {
    return trimmed.substring(1, trimmed.length - 1).replaceAll(r'\"', '"');
  }
  return unescaped;
}

String unescapeXmlForDisplay(String input) {
  var result = input;
  if (result.startsWith('<![CDATA[') && result.endsWith(']]>')) {
    result = result.substring(9, result.length - 3);
  }
  if (result.endsWith(']]>')) {
    result = result.substring(0, result.length - 3);
  }
  if (result.startsWith('<![CDATA[')) {
    result = result.substring(9);
  }
  return result
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .replaceAll('&amp;', '&')
      .replaceAll('&quot;', '"')
      .replaceAll('&apos;', "'");
}

String? parseProxyJsonParamsToXml(String input) {
  final trimmed = input.trim();
  if (trimmed.isEmpty) {
    return '';
  }
  try {
    final parsed = jsonDecode(trimmed);
    if (parsed is Map<String, Object?>) {
      return parsed.entries
          .map(
            (entry) =>
                '<param name="${escapeXmlAttribute(entry.key)}">${escapeXmlText(jsonValueToParamText(entry.value))}</param>',
          )
          .join('\n');
    }
    if (parsed is List<Object?>) {
      return parsed
          .asMap()
          .entries
          .map(
            (entry) =>
                '<param name="${entry.key}">${escapeXmlText(jsonValueToParamText(entry.value))}</param>',
          )
          .join('\n');
    }
  } on FormatException {
    return null;
  }
  return null;
}

String jsonValueToParamText(Object? value) {
  if (value == null) {
    return 'null';
  }
  if (value is String) {
    return value;
  }
  return jsonEncode(value);
}

String escapeXmlAttribute(String input) {
  return input
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&apos;');
}

String escapeXmlText(String input) {
  return input
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
}

String buildParamsHeadPreview(String params, {int maxChars = 120}) {
  final match = RegExp(r'<param.*?>([^<]*)<\/param>').firstMatch(params);
  final matched = match?.group(1)?.trim();
  final cleaned = (matched != null && matched.isNotEmpty ? matched : params)
      .replaceAll('\n', ' ')
      .trim();
  return cleaned.length <= maxChars
      ? cleaned
      : '${cleaned.substring(0, maxChars)}...';
}

int calculateToolParamsBytes(String params) {
  if (params.isEmpty) {
    return 0;
  }
  final payloads = extractParamPayloadsForSize(params);
  return payloads.fold<int>(
    0,
    (total, payload) => total + utf8.encode(payload).length,
  );
}

List<String> extractParamPayloadsForSize(String params) {
  final tagRegex = RegExp(r'</?param\b[^>]*>');
  final payloads = <String>[];
  var insideParam = false;
  var valueStart = -1;

  for (final match in tagRegex.allMatches(params)) {
    final tagText = match.group(0)!;
    if (tagText.startsWith('</')) {
      if (insideParam) {
        final rawValue = params.substring(valueStart, match.start);
        payloads.add(normalizeEscapedTextForDisplay(rawValue));
        insideParam = false;
        valueStart = -1;
      }
      continue;
    }

    if (!insideParam) {
      insideParam = true;
      valueStart = match.end;
    }
  }

  if (insideParam && valueStart >= 0 && valueStart <= params.length) {
    payloads.add(normalizeEscapedTextForDisplay(params.substring(valueStart)));
  }

  return payloads.isNotEmpty
      ? payloads
      : <String>[normalizeEscapedTextForDisplay(params)];
}

IconData getToolIcon(String toolName) {
  final lower = toolName.toLowerCase();
  if (lower.contains('file') ||
      lower.contains('read') ||
      lower.contains('write')) {
    return Icons.file_open;
  }
  if (lower.contains('search') ||
      lower.contains('find') ||
      lower.contains('query')) {
    return Icons.search;
  }
  if (lower.contains('terminal') ||
      lower.contains('exec') ||
      lower.contains('command') ||
      lower.contains('shell')) {
    return Icons.terminal;
  }
  if (lower.contains('code') || lower.contains('ffmpeg')) {
    return Icons.code;
  }
  if (lower.contains('http') ||
      lower.contains('web') ||
      lower.contains('visit')) {
    return Icons.web;
  }
  return Icons.arrow_forward;
}
