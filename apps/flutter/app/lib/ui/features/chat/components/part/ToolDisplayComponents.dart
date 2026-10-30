// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import 'ChatActivityDisclosure.dart';
import 'ChatActivityRowStyle.dart';
import 'FileDiffDisplay.dart';

class CompactToolDisplay extends StatelessWidget {
  const CompactToolDisplay({
    super.key,
    required this.toolName,
    required this.params,
    required this.textColor,
    required this.isStreaming,
    this.display,
  });

  final String toolName;
  final String params;
  final Color textColor;
  final bool isStreaming;
  final ToolDisplayDeclaration? display;

  @override
  Widget build(BuildContext context) {
    return ToolCallRow(
      presentation: ToolPresentation.fromCall(
        toolName,
        params,
        display: display,
      ),
      isStreaming: isStreaming,
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
    this.display,
  });

  final String toolName;
  final String params;
  final Color textColor;
  final bool isStreaming;
  final ToolDisplayDeclaration? display;

  @override
  Widget build(BuildContext context) {
    return ToolCallRow(
      presentation: ToolPresentation.fromCall(
        toolName,
        params,
        display: display,
      ),
      isStreaming: isStreaming,
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
                  ),
                ),
              ),
            ],
          ),
      ],
    );
  }
}

enum ToolResultRenderer { text, directory, fileDiff }

/// Immutable display metadata supplied by the registered tool owner.
class ToolDisplayDeclaration {
  const ToolDisplayDeclaration({
    required this.labels,
    this.icon = 'tool',
    this.targetParameter,
    this.resultRenderer = ToolResultRenderer.text,
  });

  factory ToolDisplayDeclaration.fromJson(Map<String, dynamic> json) =>
      ToolDisplayDeclaration(
        labels: (json['label'] as Map? ?? const {}).map(
          (key, value) => MapEntry(key.toString(), value.toString()),
        ),
        icon: json['icon'] as String? ?? 'tool',
        targetParameter: json['targetParameter'] as String?,
        resultRenderer: switch (json['resultRenderer']) {
          'directory' => ToolResultRenderer.directory,
          'fileDiff' => ToolResultRenderer.fileDiff,
          _ => ToolResultRenderer.text,
        },
      );

  final Map<String, String> labels;
  final String icon;
  final String? targetParameter;
  final ToolResultRenderer resultRenderer;

  String labelFor(Locale locale, String fallback) =>
      labels[locale.toLanguageTag()] ??
      labels[locale.languageCode] ??
      labels['default'] ??
      labels['en'] ??
      (labels.isNotEmpty ? labels.values.first : fallback);
}

/// Decodes only explicit renderer metadata, accepting raw events or XML attributes.
Map<String, dynamic>? decodeToolMetadata(String? value) {
  if (value == null || value.isEmpty) return null;
  for (final candidate in [value, unescapeXmlForDisplay(value)]) {
    try {
      final decoded = jsonDecode(candidate);
      if (decoded is Map<String, dynamic>) return decoded;
    } on FormatException {
      continue;
    }
  }
  return null;
}

ToolDisplayDeclaration? toolDisplayFromAttributes(
  Map<String, String>? attributes,
) {
  final json = decodeToolMetadata(attributes?['display']);
  if (json == null || json['label'] is! Map) return null;
  final labels = json['label'] as Map;
  if (labels.entries.any(
        (entry) => entry.key is! String || entry.value is! String,
      ) ||
      (json['icon'] != null && json['icon'] is! String) ||
      (json['targetParameter'] != null && json['targetParameter'] is! String) ||
      (json['resultRenderer'] != null && json['resultRenderer'] is! String)) {
    return null;
  }
  return ToolDisplayDeclaration.fromJson(json);
}

class ToolPresentation {
  const ToolPresentation({
    required this.toolName,
    required this.icon,
    required this.parameterSummary,
    required this.result,
    required this.isSuccess,
    required this.fileDiffs,
    this.rawParams = '',
    this.callId,
    this.display,
    this.resultData,
  });

  factory ToolPresentation.fromCall(
    String toolName,
    String params, {
    String? callId,
    String result = '',
    bool? isSuccess,
    ToolDisplayDeclaration? display,
    Map<String, dynamic>? resultData,
  }) {
    final call = normalizeToolDisplayForStrictProxy(toolName, params);
    return ToolPresentation(
      toolName: call.toolName,
      callId: callId,
      icon: toolDisplayIcon(display?.icon),
      rawParams: call.params,
      parameterSummary: formatToolParameters(call.params),
      result: result,
      isSuccess: isSuccess,
      display: display,
      resultData: resultData,
      fileDiffs: fileDiffsForResult(result, resultData, display: display),
    );
  }

  factory ToolPresentation.fromResult(
    String toolName,
    String result, {
    required bool isSuccess,
    ToolDisplayDeclaration? display,
    Map<String, dynamic>? resultData,
  }) => ToolPresentation.fromCall(
    toolName,
    '',
    result: result,
    isSuccess: isSuccess,
    display: display,
    resultData: resultData,
  );

  final String toolName;
  final String? callId;
  final IconData icon;
  final String rawParams;
  final String parameterSummary;
  final String result;
  final bool? isSuccess;
  final ToolDisplayDeclaration? display;
  final Map<String, dynamic>? resultData;
  final List<FileDiff> fileDiffs;

  bool get hasDetails =>
      parameterSummary.isNotEmpty ||
      result.isNotEmpty ||
      resultData != null ||
      fileDiffs.isNotEmpty;

  bool get hasDirectoryListing =>
      display?.resultRenderer == ToolResultRenderer.directory &&
      isDirectoryListingData(resultData);

  String get target {
    final parameter = display?.targetParameter;
    return parameter == null
        ? ''
        : extractToolParamMap(rawParams)[parameter] ?? '';
  }
}

/// Selects a visual for a declared semantic icon, independently of tool names.
IconData toolDisplayIcon(String? icon) => switch (icon) {
  'file' => Icons.description_outlined,
  'folder' => Icons.folder_outlined,
  'edit' => Icons.edit_outlined,
  'delete' => Icons.delete_outline,
  'search' => Icons.search,
  'terminal' => Icons.terminal,
  'web' => Icons.language,
  'download' => Icons.download_outlined,
  'package' => Icons.extension_outlined,
  'time' => Icons.schedule,
  'notification' => Icons.notifications_outlined,
  'clipboard' => Icons.content_paste,
  'audio' => Icons.volume_up_outlined,
  _ => Icons.build_outlined,
};

String formatToolParameters(String rawParams) {
  final parameters = extractToolParamMap(rawParams);
  return parameters.isEmpty
      ? rawParams
      : parameters.entries
            .map((entry) => '${entry.key}: ${entry.value}')
            .join('\n');
}

/// Reads parameter markup without selecting, suppressing, or interpreting parameter names.
Map<String, String> extractToolParamMap(String params) => {
  for (final match in RegExp(
    r'<param\s+name="([^"]+)">([\s\S]*?)</param>',
  ).allMatches(params))
    unescapeXmlForDisplay(match.group(1)!): unescapeXmlForDisplay(
      match.group(2)!,
    ),
};

/// Retains paths from the execution host exactly as supplied.
String cleanToolPath(String path) => path;

List<FileDiff> fileDiffsForResult(
  String text,
  Map<String, dynamic>? data, {
  ToolDisplayDeclaration? display,
}) {
  if (display?.resultRenderer != ToolResultRenderer.fileDiff) {
    return const [];
  }
  final operation = data?['operation'];
  final diff = data?['diffContent'];
  if (operation is Map &&
      operation['path'] is String &&
      (operation['details'] == null || operation['details'] is String) &&
      diff is String &&
      diff.isNotEmpty) {
    return [
      FileDiff(
        path: operation['path'] as String,
        diffContent: diff,
        details: operation['details'] as String? ?? '',
      ),
    ];
  }
  // Historical results may carry the explicit file-diff protocol instead of typed data.
  return extractFileDiffs(text);
}

/// Validates the directory payload after its renderer was explicitly declared.
bool isDirectoryListingData(Map<String, dynamic>? data) {
  if (data?['path'] is! String) return false;
  final entries = data?['entries'];
  return entries is List &&
      entries.every(
        (entry) =>
            entry is Map &&
            entry['name'] is String &&
            entry['isDirectory'] is bool &&
            entry['size'] is num,
      );
}

class ToolCallRow extends StatefulWidget {
  const ToolCallRow({
    super.key,
    required this.presentation,
    required this.isStreaming,
  });
  final ToolPresentation presentation;
  final bool isStreaming;
  @override
  State<ToolCallRow> createState() => _ToolCallRowState();
}

class _ToolCallRowState extends State<ToolCallRow> {
  bool _expanded = false;
  bool _userToggled = false;

  @override
  void initState() {
    super.initState();
    _expanded = widget.presentation.fileDiffs.isNotEmpty;
  }

  @override
  void didUpdateWidget(covariant ToolCallRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_userToggled &&
        oldWidget.presentation.fileDiffs.isEmpty &&
        widget.presentation.fileDiffs.isNotEmpty) {
      _expanded = true;
    }
  }

  @override
  Widget build(BuildContext context) {
    final presentation = widget.presentation;
    final theme = Theme.of(context);
    final l10n =
        AppLocalizations.of(context) ??
        lookupAppLocalizations(Localizations.localeOf(context));
    final title =
        presentation.display?.labelFor(
          Localizations.localeOf(context),
          presentation.toolName,
        ) ??
        presentation.toolName;
    final status = widget.isStreaming
        ? l10n.toolRunning
        : presentation.isSuccess == false
        ? l10n.failed
        : l10n.completed;
    final color = presentation.isSuccess == false
        ? theme.colorScheme.error
        : theme.colorScheme.onSurfaceVariant;
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        InkWell(
          onTap: presentation.hasDetails
              ? () => setState(() {
                  _expanded = !_expanded;
                  _userToggled = true;
                })
              : null,
          child: Padding(
            padding: ChatActivityRowStyle.headerPadding,
            child: Row(
              children: [
                Icon(
                  presentation.icon,
                  size: ChatActivityRowStyle.iconSize,
                  color: color,
                ),
                const SizedBox(width: ChatActivityRowStyle.iconGap),
                Text(title, style: ChatActivityRowStyle.textStyle(context)),
                if (presentation.target.isNotEmpty) ...[
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      presentation.target,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: ChatActivityRowStyle.textStyle(context),
                    ),
                  ),
                ] else
                  const Spacer(),
                if (widget.isStreaming)
                  const StreamingCursor()
                else
                  Tooltip(
                    message: status,
                    child: Icon(
                      presentation.isSuccess == false
                          ? Icons.error_outline
                          : Icons.check,
                      size: 14,
                      color: color,
                    ),
                  ),
                if (presentation.hasDetails)
                  AnimatedRotation(
                    turns: _expanded ? 0.25 : 0,
                    duration: ChatActivityDisclosure.duration,
                    curve: ChatActivityDisclosure.curve,
                    child: const Icon(Icons.chevron_right, size: 16),
                  ),
              ],
            ),
          ),
        ),
        ChatActivityDisclosure(
          child: _expanded && presentation.hasDetails
              ? Padding(
                  key: const ValueKey('tool-details'),
                  padding: const EdgeInsets.only(
                    left: ChatActivityRowStyle.contentInset,
                    bottom: 8,
                  ),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (presentation.parameterSummary.isNotEmpty)
                        ToolInlineDetailSurface(
                          title: l10n.toolParametersLabel,
                          text: presentation.parameterSummary,
                          initiallyCollapsed: true,
                        ),
                      for (final diff in presentation.fileDiffs)
                        FileDiffDisplay(diff: diff, actionTitle: title),
                      if (presentation.hasDirectoryListing)
                        DirectoryListingDisplay(
                          data: presentation.resultData!,
                          showPath:
                              presentation.resultData!['path'] !=
                              presentation.target,
                        ),
                      if (presentation.result.isNotEmpty)
                        ToolInlineDetailSurface(
                          title: l10n.toolResultLabel,
                          text: presentation.result,
                          isSuccess: presentation.isSuccess,
                          initiallyCollapsed:
                              presentation.fileDiffs.isNotEmpty ||
                              presentation.hasDirectoryListing,
                        ),
                      if (presentation.result.isEmpty &&
                          presentation.resultData != null)
                        ToolInlineDetailSurface(
                          title: l10n.toolResultLabel,
                          text: const JsonEncoder.withIndent(
                            '  ',
                          ).convert(presentation.resultData),
                          initiallyCollapsed:
                              presentation.fileDiffs.isNotEmpty ||
                              presentation.hasDirectoryListing,
                        ),
                    ],
                  ),
                )
              : null,
        ),
      ],
    );
  }
}

/// Renders the native directory result schema, including empty directories.
class DirectoryListingDisplay extends StatelessWidget {
  const DirectoryListingDisplay({
    super.key,
    required this.data,
    this.showPath = true,
  });

  final Map<String, dynamic> data;
  final bool showPath;

  /// Uses chat-sized rows instead of Material list tiles and bounds long listings.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = ChatActivityRowStyle.textStyle(context);
    final entries = data['entries'] as List;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxHeight: 240),
        child: SingleChildScrollView(
          primary: false,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (showPath)
                Padding(
                  padding: const EdgeInsets.only(bottom: 4),
                  child: SelectableText(
                    data['path'] as String,
                    style: style.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              if (entries.isEmpty)
                Text(
                  (AppLocalizations.of(context) ??
                          lookupAppLocalizations(
                            Localizations.localeOf(context),
                          ))
                      .emptyFolder,
                  style: style.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              for (final entry in entries)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(
                        entry['isDirectory'] as bool
                            ? Icons.folder_outlined
                            : Icons.description_outlined,
                        size: ChatActivityRowStyle.iconSize,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: ChatActivityRowStyle.iconGap),
                      Expanded(
                        child: SelectableText(
                          entry['name'] as String,
                          style: style,
                        ),
                      ),
                      if (entry['isDirectory'] == false) ...[
                        const SizedBox(width: 8),
                        Text(
                          '${entry['size']} B',
                          style: style.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Compact, independently expandable tool payload with complete copyable text.
class ToolInlineDetailSurface extends StatefulWidget {
  const ToolInlineDetailSurface({
    super.key,
    required this.title,
    required this.text,
    this.isSuccess,
    this.initiallyCollapsed = false,
  });
  final String title;
  final String text;
  final bool? isSuccess;
  final bool initiallyCollapsed;

  /// Keeps disclosure state local so streaming updates do not reopen details.
  @override
  State<ToolInlineDetailSurface> createState() =>
      _ToolInlineDetailSurfaceState();
}

class _ToolInlineDetailSurfaceState extends State<ToolInlineDetailSurface> {
  late bool _expanded;

  /// Applies the initial disclosure choice only when this payload first appears.
  @override
  void initState() {
    super.initState();
    _expanded = !widget.initiallyCollapsed;
  }

  /// Renders a small disclosure header and a bounded, selectable payload body.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = widget.isSuccess == false
        ? theme.colorScheme.error
        : theme.colorScheme.onSurfaceVariant;
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: InkWell(
                onTap: () => setState(() => _expanded = !_expanded),
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 5),
                  child: Row(
                    children: [
                      AnimatedRotation(
                        turns: _expanded ? 0.25 : 0,
                        duration: ChatActivityDisclosure.duration,
                        curve: ChatActivityDisclosure.curve,
                        child: Icon(
                          Icons.chevron_right,
                          size: 14,
                          color: color,
                        ),
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: Text(
                          widget.title,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: color,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            IconButton(
              tooltip: MaterialLocalizations.of(context).copyButtonLabel,
              constraints: const BoxConstraints.tightFor(width: 28, height: 28),
              padding: EdgeInsets.zero,
              visualDensity: VisualDensity.compact,
              icon: Icon(Icons.copy, size: 14, color: color),
              onPressed: () =>
                  Clipboard.setData(ClipboardData(text: widget.text)),
            ),
          ],
        ),
        ChatActivityDisclosure(
          child: _expanded
              ? Container(
                  key: const ValueKey('tool-payload'),
                  margin: const EdgeInsets.only(top: 2, bottom: 6),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(
                      color: theme.colorScheme.outlineVariant.withValues(
                        alpha: 0.35,
                      ),
                    ),
                  ),
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxHeight: 240),
                    child: SingleChildScrollView(
                      primary: false,
                      child: SelectableText(
                        widget.text,
                        style: ChatActivityRowStyle.textStyle(context).copyWith(
                          color: widget.isSuccess == false
                              ? theme.colorScheme.error
                              : theme.colorScheme.onSurface,
                        ),
                      ),
                    ),
                  ),
                )
              : null,
        ),
      ],
    );
  }
}

class ToolDisplayData {
  const ToolDisplayData({required this.toolName, required this.params});
  final String toolName;
  final String params;
}

/// Unwraps only the explicit proxy protocol; malformed envelopes remain readable as submitted.
ToolDisplayData normalizeToolDisplayForStrictProxy(
  String toolName,
  String params,
) {
  if (toolName != 'package_proxy' && toolName != 'proxy') {
    return ToolDisplayData(toolName: toolName, params: params);
  }
  final parameters = extractToolParamMap(params);
  final target = parameters['tool_name'];
  final payload = parameters['params'];
  if (target == null || target.trim().isEmpty || payload == null) {
    return ToolDisplayData(toolName: toolName, params: params);
  }
  final decoded = decodeToolMetadata(payload);
  if (decoded == null) {
    return ToolDisplayData(toolName: toolName, params: params);
  }
  return ToolDisplayData(
    toolName: target,
    params: decoded.entries
        .map(
          (entry) =>
              '<param name="${escapeXmlAttribute(entry.key)}">${escapeXmlText(jsonValueToParamText(entry.value))}</param>',
        )
        .join('\n'),
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
      .replaceAll('&quot;', '"')
      .replaceAll('&apos;', "'")
      .replaceAll('&amp;', '&');
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
  var cleaned = (matched != null && matched.isNotEmpty ? matched : params)
      .replaceAll('\n', ' ')
      .trim();
  cleaned = unescapeXmlForDisplay(cleaned);
  cleaned = cleaned
      .replaceAll(RegExp(r'<[^>]+>'), ' ')
      .replaceAll(RegExp(r'\s+'), ' ')
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
