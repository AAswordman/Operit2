// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import 'DialogComponents.dart';
import 'XmlCanvasSummaryComponents.dart';

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
    return _ToolDetailLauncher(
      displayToolName: display.toolName,
      displayParams: display.params,
      childBuilder: (openDialog) => Row(
        children: <Widget>[
          Expanded(
            child: CanvasToolSummaryRow(
              toolName: display.toolName,
              summary: summary,
              semanticDescription: buildToolSemanticDescription(
                display.toolName,
                display.params,
                useByteSummary: false,
              ),
              leadingIcon: getToolIcon(display.toolName),
              titleColor: theme.colorScheme.primary,
              summaryColor: textColor.withValues(alpha: 0.7),
              onClick: display.params.trim().isEmpty ? null : openDialog,
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
    return _ToolDetailLauncher(
      displayToolName: display.toolName,
      displayParams: display.params,
      childBuilder: (openDialog) => Row(
        children: <Widget>[
          Expanded(
            child: CanvasToolSummaryRow(
              toolName: display.toolName,
              summary: buildToolParamsSizeLabel(display.params),
              semanticDescription: buildToolSemanticDescription(
                display.toolName,
                display.params,
                useByteSummary: true,
              ),
              leadingIcon: getToolIcon(display.toolName),
              titleColor: theme.colorScheme.primary,
              summaryColor: textColor.withValues(alpha: 0.7),
              onClick: display.params.trim().isEmpty ? null : openDialog,
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

class CodeContentWithLineNumbers extends StatelessWidget {
  const CodeContentWithLineNumbers({
    super.key,
    required this.lines,
    required this.textColor,
  });

  final List<String> lines;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (var index = 0; index < lines.length; index++)
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              SizedBox(
                width: 40,
                child: Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: Text(
                    '${index + 1}',
                    textAlign: TextAlign.end,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(
                        context,
                      ).colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
                      fontFamily: 'monospace',
                      fontSize: 10,
                    ),
                  ),
                ),
              ),
              Expanded(
                child: Text(
                  lines[index],
                  softWrap: true,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: textColor.withValues(alpha: 0.8),
                    fontFamily: 'monospace',
                    fontSize: 11,
                  ),
                ),
              ),
            ],
          ),
      ],
    );
  }
}

class _ToolDetailLauncher extends StatefulWidget {
  const _ToolDetailLauncher({
    required this.displayToolName,
    required this.displayParams,
    required this.childBuilder,
  });

  final String displayToolName;
  final String displayParams;
  final Widget Function(VoidCallback openDialog) childBuilder;

  @override
  State<_ToolDetailLauncher> createState() => _ToolDetailLauncherState();
}

class _ToolDetailLauncherState extends State<_ToolDetailLauncher> {
  @override
  Widget build(BuildContext context) {
    return widget.childBuilder(() {
      showDialog<void>(
        context: context,
        builder: (dialogContext) {
          return ContentDetailDialog(
            title: '${widget.displayToolName} Tool call parameters',
            content: widget.displayParams,
            icon: getToolIcon(widget.displayToolName),
            onDismiss: () {
              Navigator.of(dialogContext).pop();
            },
          );
        },
      );
    });
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

String buildToolParamsSizeLabel(String params) {
  return '${calculateToolParamsBytes(params)} B';
}

String buildToolSemanticDescription(
  String toolName,
  String params, {
  required bool useByteSummary,
}) {
  final summary = useByteSummary
      ? buildToolParamsSizeLabel(params)
      : buildParamsHeadPreview(params);
  return 'Tool operation: $toolName, $summary';
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
