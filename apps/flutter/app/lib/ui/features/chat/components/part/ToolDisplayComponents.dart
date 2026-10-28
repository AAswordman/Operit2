// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:path/path.dart' as p;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../common/interactions/MessagePressShield.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../workspace/WorkspaceDiffContent.dart';
import '../workspace/WorkspaceInlineDiff.dart';
import 'FileDiffDisplay.dart';
import '../workspace/WorkspaceTabModels.dart';
import 'XmlCanvasBlockComponents.dart';

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
    return ToolCallRow(
      presentation: ToolPresentation.fromCall(toolName, params),
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
  });

  final String toolName;
  final String params;
  final Color textColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    return ToolCallRow(
      presentation: ToolPresentation.fromCall(toolName, params),
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
  });

  factory ToolPresentation.fromCall(
    String toolName,
    String params, {
    String? callId,
    String result = '',
    bool? isSuccess,
  }) {
    final display = normalizeToolDisplayForStrictProxy(toolName, params);
    return ToolPresentation(
      toolName: display.toolName,
      callId: callId,
      icon: getToolIcon(display.toolName),
      rawParams: display.params,
      parameterSummary: _parameterSummaryLine(display.toolName, display.params),
      result: _ordinaryResultText(result),
      isSuccess: isSuccess,
      fileDiffs: extractFileDiffs(result),
    );
  }

  factory ToolPresentation.fromResult(
    String toolName,
    String result, {
    required bool isSuccess,
  }) {
    return ToolPresentation.fromCall(
      toolName,
      '',
      result: result,
      isSuccess: isSuccess,
    );
  }

  final String toolName;
  final String? callId;
  final IconData icon;
  final String rawParams;
  final String parameterSummary;
  final String result;
  final bool? isSuccess;
  final List<FileDiff> fileDiffs;

  /// Namespace prefix of plugin-scoped tools, e.g. `daily_life` in
  /// `daily_life:get_current_time`.
  String? get namespace {
    final colon = toolName.lastIndexOf(':');
    return colon > 0 ? toolName.substring(0, colon) : null;
  }

  bool get hasDetails {
    final lower = toolName.toLowerCase();
    // Web visit tools present their target URL directly on the single tool row;
    // do not expand into a second line unless execution failed:
    if ((lower == 'visit_web' || lower == 'web_visit') && isSuccess != false) {
      return false;
    }
    return parameterSummary.isNotEmpty ||
        result.isNotEmpty ||
        fileDiffs.isNotEmpty;
  }
}

String buildToolActionName(String toolName) {
  final action = toolName.contains(':')
      ? toolName.substring(toolName.lastIndexOf(':') + 1)
      : toolName;
  final words = action
      .replaceAllMapped(
        RegExp(r'([a-z0-9])([A-Z])'),
        (match) => '${match.group(1)} ${match.group(2)}',
      )
      .split(RegExp(r'[_\-\s]+'))
      .where((word) => word.isNotEmpty)
      .map(
        (word) => '${word[0].toUpperCase()}${word.substring(1).toLowerCase()}',
      )
      .join(' ');
  return words.isEmpty ? toolName : words;
}

String formatToolParameters(String rawParams) {
  final trimmed = rawParams.trim();
  if (trimmed.isEmpty) return '';
  final paramMap = extractToolParamMap(trimmed);
  if (paramMap.isNotEmpty) {
    return paramMap.entries
        .map((entry) => '${entry.key}: ${entry.value}')
        .join('\n');
  }
  return unescapeXmlForDisplay(
    trimmed,
  ).replaceAll(RegExp(r'<[^>]+>'), ' ').replaceAll(RegExp(r'\s+'), ' ').trim();
}

/// One dense line describing secondary call parameters, omitting parameters
/// already presented in the action title/target (e.g. path, package_name).
String _parameterSummaryLine(String toolName, String rawParams) {
  final paramMap = extractToolParamMap(rawParams);
  if (paramMap.isEmpty) return '';
  final lowerTool = toolName.toLowerCase();
  final entries = <String>[];
  for (final entry in paramMap.entries) {
    final key = entry.key;
    if (key == 'path' &&
        (lowerTool.contains('file') ||
            lowerTool.contains('dir') ||
            lowerTool == 'list_files' ||
            lowerTool == 'find_files')) {
      continue;
    }
    if (key == 'package_name' &&
        (lowerTool.contains('package') || lowerTool.contains('toolpkg'))) {
      continue;
    }
    if (key == 'url' &&
        (lowerTool.contains('browser') ||
            lowerTool.contains('web') ||
            lowerTool == 'visit_web')) {
      continue;
    }
    if (lowerTool == 'visit_web' || lowerTool == 'web_visit') {
      continue;
    }
    if (key == 'include_image_links' ||
        key == 'visit_key' ||
        key == 'link_number' ||
        key == 'image_number' ||
        key == 'headers' ||
        key == 'user_agent' ||
        key == 'user_agent_preset') {
      continue;
    }
    if (key == 'create_parents' ||
        (key == 'overwrite' && entry.value == 'false')) {
      continue;
    }
    var value = unescapeXmlForDisplay(entry.value)
        .replaceAll(RegExp(r'\s+'), ' ')
        .trim();
    if (key.contains('path') ||
        key == 'source' ||
        key == 'destination' ||
        key == 'target' ||
        key == 'cwd') {
      value = cleanToolPath(value);
    }
    final short = value.length > 60 ? '${value.substring(0, 57)}…' : value;
    entries.add('$key: $short');
  }
  return entries.join('  ·  ');
}

/// Short visible target for the collapsed row: file name without directories,
/// line range kept, URLs reduced to host, long text truncated.
String shortToolTargetLabel(String rawSummary) {
  var text = rawSummary.trim();
  if (text.isEmpty) return '';
  if (text.startsWith('http://') || text.startsWith('https://')) {
    final uri = Uri.tryParse(text);
    if (uri != null && uri.host.isNotEmpty) {
      final path = uri.path.isEmpty ? '' : uri.path;
      final host = '${uri.host}$path';
      return host.length > 40 ? '${host.substring(0, 37)}…' : host;
    }
  }
  final lineMatch = RegExp(
    r'\s*\((L\d+(?:-L\d+)?)\)$',
  ).firstMatch(text);
  final suffix = lineMatch != null ? ' ${lineMatch.group(1)}' : '';
  if (lineMatch != null) {
    text = text.substring(0, lineMatch.start).trim();
  }
  final normalized = text.replaceAll('\\', '/');
  // Preserve the relative path if within reasonable length so project and
  // directory structure is visible (e.g. pelican_flutter/lib/geometry):
  if (normalized.length <= 45) {
    return '$normalized$suffix';
  }
  final segments = normalized.split('/');
  if (segments.length > 2) {
    final candidate = '…/${segments.sublist(segments.length - 2).join('/')}';
    if (candidate.length <= 40) {
      return '$candidate$suffix';
    }
  }
  final slash = normalized.lastIndexOf('/');
  final base = slash >= 0 && slash < normalized.length - 1
      ? normalized.substring(slash + 1)
      : normalized;
  final label = (base.isEmpty ? normalized : base) + suffix;
  return label.length > 40 ? '${label.substring(0, 37)}…' : label;
}

String _ordinaryResultText(String result) {
  final withoutDiff = result
      .replaceAll(
        RegExp(r'<file-diff[\s\S]*<\/file-diff>', caseSensitive: false),
        '',
      )
      .trim();
  final error = RegExp(
    r'^<error\b[^>]*>([\s\S]*?)<\/error>$',
    caseSensitive: false,
  ).firstMatch(withoutDiff);
  var text = (error?.group(1) ?? withoutDiff).trim();
  text = text.replaceAllMapped(
    RegExp(r'''(?:/mnt/local/folders/|/app/workspaces/)[0-9a-zA-Z_-]{8,64}(?:/[^\s\n"'<>]+)?'''),
    (match) => cleanToolPath(match.group(0)!),
  );
  if (text.startsWith('Directory created: ')) {
    final p = text.substring('Directory created: '.length).trim();
    text = '已创建目录 $p';
  } else if (text.startsWith('File created: ')) {
    final p = text.substring('File created: '.length).trim();
    text = '已创建文件 $p';
  } else if (text.startsWith('File deleted: ')) {
    final p = text.substring('File deleted: '.length).trim();
    text = '已删除文件 $p';
  }
  return text;
}

/// Basename of a path for picking the language file icon.
String _pathBasename(String path) {
  final normalized = path.replaceAll('\\', '/');
  final slash = normalized.lastIndexOf('/');
  return slash >= 0 && slash < normalized.length - 1
      ? normalized.substring(slash + 1)
      : normalized;
}

String resolveWebVisitUrl(ToolPresentation presentation) {
  final paramMap = extractToolParamMap(presentation.rawParams);
  var url = paramMap['url'] ??
      paramMap['uri'] ??
      paramMap['link'] ??
      paramMap['href'] ??
      paramMap['target'] ??
      '';
  if (url.isEmpty) {
    final m = RegExp(r'''https?://[^\s<>"'\]]+''').firstMatch(presentation.rawParams);
    if (m != null) url = m.group(0)!;
  }
  if (url.isEmpty && presentation.result.isNotEmpty) {
    final m = RegExp(r'''https?://[^\s<>"'\]]+''').firstMatch(presentation.result);
    if (m != null) url = m.group(0)!;
  }
  return url;
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
    // Edit/write rows lead with the diff itself; everything else starts closed.
    _expanded = widget.presentation.fileDiffs.isNotEmpty;
  }

  @override
  void didUpdateWidget(covariant ToolCallRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    // A diff arriving with the result opens the row unless the user chose.
    if (!_userToggled &&
        oldWidget.presentation.fileDiffs.isEmpty &&
        widget.presentation.fileDiffs.isNotEmpty) {
      _expanded = true;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final presentation = widget.presentation;
    final isZh = Localizations.localeOf(context).languageCode == 'zh';
    final semantic = buildSemanticToolDisplay(
      presentation.toolName,
      presentation.rawParams,
      isZh: isZh,
    );
    final title = semantic.actionName == presentation.toolName
        ? buildToolActionName(presentation.toolName)
        : semantic.actionName;
    var targetLabel = shortToolTargetLabel(semantic.targetSummary);
    final isWebVisit = presentation.toolName == 'visit_web' ||
        presentation.toolName == 'web_visit';
    final webUrl = isWebVisit ? resolveWebVisitUrl(presentation) : '';
    if (isWebVisit && webUrl.isNotEmpty) {
      targetLabel = shortToolTargetLabel(webUrl);
    }
    final dirEntries = (presentation.toolName == 'list_files' && presentation.result.isNotEmpty)
        ? tryParseDirectoryListing(presentation.result)
        : null;
    if (dirEntries != null && dirEntries.isNotEmpty) {
      if (dirEntries.length == 1) {
        targetLabel = targetLabel.isNotEmpty
            ? '$targetLabel · ${dirEntries.first.name}'
            : dirEntries.first.name;
      } else {
        final count = isZh ? '${dirEntries.length} 个项目' : '${dirEntries.length} items';
        targetLabel = targetLabel.isNotEmpty ? '$targetLabel · $count' : count;
      }
    }
    final namespace = presentation.namespace;
    final hasDetails = presentation.hasDetails;
    final status = widget.isStreaming
        ? (isZh ? '正在执行' : 'Running')
        : presentation.isSuccess == false
        ? (isZh ? '执行失败' : 'Failed')
        : (isZh ? '已完成' : 'Completed');
    final iconColor = presentation.isSuccess == false
        ? theme.colorScheme.error
        : theme.colorScheme.onSurfaceVariant;

    if (presentation.fileDiffs.isNotEmpty) {
      return _buildDiffToolRow(
        theme,
        presentation,
        title,
        status,
        iconColor,
        isZh,
      );
    }

    return SelectionContainer.disabled(
      child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Semantics(
          button: hasDetails,
          label: '$title, $status',
          child: MessagePressShieldRegion(
            child: Material(
              color: Colors.transparent,
              child: InkWell(
                onTap: hasDetails
                    ? () => setState(() {
                        _expanded = !_expanded;
                        _userToggled = true;
                      })
                    : null,
                borderRadius: BorderRadius.circular(4),
                child: SizedBox(
                  height: 28,
                  child: Row(
                    children: <Widget>[
                      Icon(presentation.icon, size: 16, color: iconColor),
                      const SizedBox(width: 8),
                      if (namespace != null) ...<Widget>[
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 4,
                            vertical: 1,
                          ),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.surfaceContainerHighest
                                .withValues(alpha: 0.5),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text(
                            namespace,
                            style: theme.textTheme.labelSmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                              fontSize: 10,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ),
                        const SizedBox(width: 6),
                      ],
                      Expanded(
                        child: Text.rich(
                          TextSpan(
                            children: <InlineSpan>[
                              TextSpan(
                                text: title,
                                style: theme.textTheme.labelMedium?.copyWith(
                                  color: presentation.isSuccess == false
                                      ? theme.colorScheme.error
                                      : theme.colorScheme.onSurface,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                              if (targetLabel.isNotEmpty) ...<InlineSpan>[
                                const WidgetSpan(
                                  child: SizedBox(width: 8),
                                ),
                                TextSpan(
                                  text: targetLabel,
                                  style: theme.textTheme.bodySmall?.copyWith(
                                    color: theme.colorScheme.onSurfaceVariant
                                        .withValues(alpha: 0.8),
                                    fontFamily: 'monospace',
                                    fontSize: 11,
                                  ),
                                ),
                              ],
                            ],
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (isWebVisit && webUrl.isNotEmpty)
                        MessagePressShieldRegion(
                          child: Tooltip(
                            message: isZh ? '复制网址' : 'Copy URL',
                            child: InkResponse(
                              onTap: () => unawaited(
                                Clipboard.setData(ClipboardData(text: webUrl)),
                              ),
                              radius: 12,
                              child: Padding(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 4,
                                  vertical: 4,
                                ),
                                child: Icon(
                                  Icons.content_copy,
                                  size: 13,
                                  color: theme.colorScheme.onSurfaceVariant
                                      .withValues(alpha: 0.6),
                                ),
                              ),
                            ),
                          ),
                        ),
                      if (widget.isStreaming)
                        const StreamingCursor()
                      else if (presentation.isSuccess == false)
                        Icon(
                          Icons.error_outline,
                          size: 15,
                          color: theme.colorScheme.error,
                        ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
        if (_expanded && hasDetails)
          CanvasIndentedGuide(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                if (presentation.parameterSummary.isNotEmpty && dirEntries == null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 4),
                    child: Text(
                      presentation.parameterSummary,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant.withValues(
                          alpha: 0.75,
                        ),
                        fontFamily: 'monospace',
                        fontSize: 11,
                      ),
                    ),
                  ),
                if (presentation.result.isNotEmpty) ..._resultSection(
                  theme,
                  presentation.result,
                  presentation.isSuccess,
                  isZh,
                ),
              ],
            ),
          ),
      ],
      ),
    );
  }
  List<Widget> _resultSection(
    ThemeData theme,
    String result,
    bool? isSuccess,
    bool isZh,
  ) {
    final failed = isSuccess == false;
    final compact = !result.contains('\n') && result.length <= 80;
    if (compact) {
      return <Widget>[
        Padding(
          padding: const EdgeInsets.only(bottom: 4),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Icon(
                failed ? Icons.error_outline : Icons.check_outlined,
                size: 13,
                color: failed
                    ? theme.colorScheme.error
                    : theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
              ),
              const SizedBox(width: 5),
              Expanded(
                child: Text(
                  result,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: failed
                        ? theme.colorScheme.error
                        : theme.colorScheme.onSurfaceVariant.withValues(
                            alpha: 0.85,
                          ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ];
    }
    final dirEntries = tryParseDirectoryListing(result);
    if (dirEntries != null) {
      final paramMap = extractToolParamMap(widget.presentation.rawParams);
      final rawPath = paramMap['path'] ?? tryParseDirectoryListingPath(result) ?? '';
      return <Widget>[
        DirectoryListingDisplay(
          rawPath: rawPath,
          entries: dirEntries,
          rawText: result,
        ),
      ];
    }
    final lowerTool = widget.presentation.toolName.toLowerCase();
    if (lowerTool == 'visit_web' || lowerTool == 'web_visit') {
      if (isSuccess == false) {
        return <Widget>[
          ToolInlineDetailSurface(
            title: isZh ? '访问失败' : 'Failed',
            text: result,
            isSuccess: false,
          ),
        ];
      }
      final paramMap = extractToolParamMap(widget.presentation.rawParams);
      var rawUrl = paramMap['url'] ??
          paramMap['uri'] ??
          paramMap['link'] ??
          paramMap['href'] ??
          '';
      if (rawUrl.isEmpty) {
        final m = RegExp(r'''https?://[^\s<>"'\]]+''').firstMatch(widget.presentation.rawParams);
        rawUrl = m?.group(0) ?? '';
      }
      if (rawUrl.isEmpty) {
        final m = RegExp(r'''https?://[^\s<>"'\]]+''').firstMatch(result);
        rawUrl = m?.group(0) ?? '';
      }
      return <Widget>[
        WebVisitDetailDisplay(
          url: rawUrl,
          isSuccess: isSuccess,
        ),
      ];
    }
    final pkgListing = tryParsePackageListing(result);
    if (pkgListing != null) {
      return <Widget>[
        PackageListingDisplay(
          data: pkgListing,
          rawText: result,
        ),
      ];
    }
    return <Widget>[
      ToolInlineDetailSurface(
        title: isZh ? '执行结果' : 'Result',
        text: result,
        isSuccess: isSuccess,
      ),
    ];
  }

  /// Edit/write layout: action + syntax badge + path + stats on one line,
  /// the diff itself directly below. No parameter or result text.
  Widget _buildDiffToolRow(
    ThemeData theme,
    ToolPresentation presentation,
    String title,
    String status,
    Color iconColor,
    bool isZh,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (final fileDiff in presentation.fileDiffs)
          FileDiffDisplay(
            key: ValueKey<String>(
              '${presentation.callId ?? presentation.toolName}-${fileDiff.path}',
            ),
            diff: fileDiff,
            actionTitle: title,
            isExpanded: _expanded,
            onToggleExpand: () => setState(() {
              _expanded = !_expanded;
              _userToggled = true;
            }),
          ),
      ],
    );
  }
}


class DirectoryEntryItem {
  const DirectoryEntryItem({
    required this.name,
    required this.isDirectory,
    required this.size,
  });

  final String name;
  final bool isDirectory;
  final int size;
}

List<DirectoryEntryItem>? tryParseDirectoryListing(String text) {
  if (!text.contains('Directory listing for')) {
    return null;
  }
  final entries = <DirectoryEntryItem>[];
  final lines = text.split('\n');
  final entryRegex = RegExp(r'^([d-])[rwx-]*\s+(\d+)\s+.*?name="([^"]+)"');

  for (final line in lines) {
    final trimmed = line.trim();
    final match = entryRegex.firstMatch(trimmed);
    if (match != null) {
      final isDir = match.group(1) == 'd';
      final size = int.tryParse(match.group(2) ?? '0') ?? 0;
      final name = match.group(3) ?? '';
      if (name.isNotEmpty) {
        entries.add(DirectoryEntryItem(
          name: name,
          isDirectory: isDir,
          size: size,
        ));
      }
    }
  }
  return entries.isNotEmpty ? entries : null;
}

String? tryParseDirectoryListingPath(String text) {
  final match = RegExp(r'Directory listing for (.*?):').firstMatch(text);
  return match?.group(1)?.trim();
}

class PackageListingData {
  const PackageListingData({
    required this.packageName,
    required this.description,
    required this.tools,
  });
  final String packageName;
  final String description;
  final List<PackageToolItem> tools;
}

class PackageToolItem {
  const PackageToolItem({
    required this.name,
    required this.description,
  });
  final String name;
  final String description;
}

PackageListingData? tryParsePackageListing(String text) {
  if (!text.contains('Available tools in this package:') &&
      !text.contains('Using package:')) {
    return null;
  }
  final pkgMatch = RegExp(r'Using package:\s*(\S+)').firstMatch(text);
  final descMatch = RegExp(
    r'Description:\s*(.*?)(?=\n\s*\n|\nAvailable|\Z)',
    dotAll: true,
  ).firstMatch(text);
  final pkgName = pkgMatch?.group(1) ?? '';
  final description =
      descMatch?.group(1)?.replaceAll(RegExp(r'\s+'), ' ').trim() ?? '';

  final tools = <PackageToolItem>[];
  final toolMatches =
      RegExp(r'-\s*([a-zA-Z0-9_:]+):\s*([^\n]+)').allMatches(text);
  for (final m in toolMatches) {
    var toolName = m.group(1) ?? '';
    if (toolName.contains(':')) {
      toolName = toolName.substring(toolName.indexOf(':') + 1);
    }
    final desc = (m.group(2) ?? '').trim();
    if (toolName.isNotEmpty &&
        toolName != 'path' &&
        toolName != 'source' &&
        toolName != 'destination') {
      tools.add(PackageToolItem(name: toolName, description: desc));
    }
  }
  if (description.isNotEmpty || tools.isNotEmpty) {
    return PackageListingData(
      packageName: pkgName,
      description: description,
      tools: tools,
    );
  }
  return null;
}

class PackageListingDisplay extends StatelessWidget {
  const PackageListingDisplay({
    super.key,
    required this.data,
    required this.rawText,
  });

  final PackageListingData data;
  final String rawText;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isZh = Localizations.localeOf(context).languageCode == 'zh';

    return SelectionContainer.disabled(
      child: Container(
        width: double.infinity,
        margin: const EdgeInsets.only(bottom: 6),
        decoration: BoxDecoration(
          color:
              theme.colorScheme.surfaceContainerLowest.withValues(alpha: 0.85),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.35),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
              child: Row(
                children: <Widget>[
                  Icon(
                    Icons.extension_outlined,
                    size: 15,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      data.packageName.isNotEmpty
                          ? data.packageName
                          : (isZh ? '插件工具包' : 'Tool Package'),
                      style: theme.textTheme.labelMedium?.copyWith(
                        color: theme.colorScheme.onSurface,
                        fontWeight: FontWeight.w600,
                        fontFamily: 'monospace',
                        fontSize: 12,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  MessagePressShieldRegion(
                    child: Tooltip(
                      message: isZh ? '复制原始信息' : 'Copy package info',
                      child: InkResponse(
                        onTap: () => unawaited(
                          Clipboard.setData(ClipboardData(text: rawText)),
                        ),
                        radius: 12,
                        child: Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 6,
                            vertical: 4,
                          ),
                          child: Icon(
                            Icons.content_copy,
                            size: 12,
                            color: theme.colorScheme.onSurfaceVariant
                                .withValues(alpha: 0.7),
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            if (data.description.isNotEmpty) ...<Widget>[
              Divider(
                height: 1,
                thickness: 1,
                color: theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(10, 6, 10, 6),
                child: Text(
                  data.description,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant
                        .withValues(alpha: 0.9),
                    fontSize: 11,
                    height: 1.4,
                  ),
                ),
              ),
            ],
            if (data.tools.isNotEmpty) ...<Widget>[
              Divider(
                height: 1,
                thickness: 1,
                color: theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
              ),
              Padding(
                padding: const EdgeInsets.all(8),
                child: Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: <Widget>[
                    for (final tool in data.tools)
                      Tooltip(
                        message: tool.description,
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 7,
                            vertical: 3,
                          ),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.surfaceContainerHighest
                                .withValues(alpha: 0.4),
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(
                              color: theme.colorScheme.outlineVariant
                                  .withValues(alpha: 0.25),
                            ),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: <Widget>[
                              Icon(
                                Icons.build_circle_outlined,
                                size: 12,
                                color: theme.colorScheme.primary
                                    .withValues(alpha: 0.8),
                              ),
                              const SizedBox(width: 4),
                              Text(
                                tool.name,
                                style: theme.textTheme.labelSmall?.copyWith(
                                  color: theme.colorScheme.onSurface,
                                  fontFamily: 'monospace',
                                  fontSize: 11,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class WebVisitDetailDisplay extends StatelessWidget {
  const WebVisitDetailDisplay({
    super.key,
    required this.url,
    required this.isSuccess,
  });

  final String url;
  final bool? isSuccess;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isZh = Localizations.localeOf(context).languageCode == 'zh';

    return SelectionContainer.disabled(
      child: Container(
        width: double.infinity,
        margin: const EdgeInsets.only(bottom: 6),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
        decoration: BoxDecoration(
          color:
              theme.colorScheme.surfaceContainerLowest.withValues(alpha: 0.85),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.35),
          ),
        ),
        child: Row(
          children: <Widget>[
            Icon(
              Icons.language,
              size: 15,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                url.isNotEmpty
                    ? url
                    : (isZh ? '网页内容已获取' : 'Webpage content fetched'),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurface,
                  fontFamily: 'monospace',
                  fontSize: 11,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            if (url.isNotEmpty) ...<Widget>[
              const SizedBox(width: 8),
              MessagePressShieldRegion(
                child: Tooltip(
                  message: isZh ? '复制链接' : 'Copy URL',
                  child: InkResponse(
                    onTap: () => unawaited(
                      Clipboard.setData(ClipboardData(text: url)),
                    ),
                    radius: 12,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 6,
                        vertical: 4,
                      ),
                      child: Icon(
                        Icons.content_copy,
                        size: 12,
                        color: theme.colorScheme.onSurfaceVariant
                            .withValues(alpha: 0.7),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

String _filePreviewKindLabel(String fileName, bool isDirectory, bool isZh) {
  if (isDirectory) {
    return isZh ? '文件夹' : 'Folder';
  }
  final kind = workspacePreviewKindForPath(fileName);
  return switch (kind) {
    WorkspaceFilePreviewKind.html => isZh ? '网页预览' : 'Web preview',
    WorkspaceFilePreviewKind.image => isZh ? '图片' : 'Image',
    WorkspaceFilePreviewKind.markdown => 'Markdown',
    WorkspaceFilePreviewKind.pdf => isZh ? 'PDF 文档' : 'PDF',
    WorkspaceFilePreviewKind.word => isZh ? 'Word 文档' : 'Word',
    WorkspaceFilePreviewKind.spreadsheet => isZh ? '表格' : 'Sheet',
    WorkspaceFilePreviewKind.presentation => isZh ? '幻灯片' : 'Slides',
    WorkspaceFilePreviewKind.audio => isZh ? '音频' : 'Audio',
    WorkspaceFilePreviewKind.video => isZh ? '视频' : 'Video',
    WorkspaceFilePreviewKind.text => isZh ? '文本文件' : 'Text',
    WorkspaceFilePreviewKind.binary => isZh ? '文件' : 'File',
  };
}

String _formatByteSize(int bytes) {
  if (bytes <= 0) return '0 B';
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
}

class DirectoryListingDisplay extends StatelessWidget {
  const DirectoryListingDisplay({
    super.key,
    required this.rawPath,
    required this.entries,
    required this.rawText,
  });

  final String rawPath;
  final List<DirectoryEntryItem> entries;
  final String rawText;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isZh = Localizations.localeOf(context).languageCode == 'zh';
    final displayPath = cleanToolPath(rawPath, isZh: isZh);
    final countLabel = isZh ? '${entries.length} 个项目' : '${entries.length} items';

    return SelectionContainer.disabled(
      child: Container(
        width: double.infinity,
        margin: const EdgeInsets.only(bottom: 6),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerLowest.withValues(alpha: 0.85),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.35),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
              child: Row(
                children: <Widget>[
                  Icon(
                    Icons.folder_open,
                    size: 15,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text.rich(
                      TextSpan(
                        children: <InlineSpan>[
                          TextSpan(
                            text: displayPath.endsWith('/') ? displayPath : '$displayPath/',
                            style: theme.textTheme.labelMedium?.copyWith(
                              color: theme.colorScheme.onSurface,
                              fontWeight: FontWeight.w600,
                              fontFamily: 'monospace',
                              fontSize: 12,
                            ),
                          ),
                          TextSpan(
                            text: '  ($countLabel)',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant
                                  .withValues(alpha: 0.6),
                              fontSize: 11,
                            ),
                          ),
                        ],
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  MessagePressShieldRegion(
                    child: Tooltip(
                      message: isZh ? '复制原始输出' : 'Copy listing',
                      child: InkResponse(
                        onTap: () => unawaited(Clipboard.setData(ClipboardData(text: rawText))),
                        radius: 12,
                        child: Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
                          child: Icon(
                            Icons.content_copy,
                            size: 12,
                            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            Divider(
              height: 1,
              thickness: 1,
              color: theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 3),
              child: Column(
                children: <Widget>[
                  for (final entry in entries)
                    Material(
                      color: Colors.transparent,
                      child: Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                        child: Row(
                          children: <Widget>[
                            entry.isDirectory
                                ? const Icon(
                                    Icons.folder_outlined,
                                    size: 16,
                                    color: Color(0xFFF59E0B),
                                  )
                                : buildWorkspaceFileIcon(entry.name, size: 16),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                entry.name,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: theme.textTheme.bodySmall?.copyWith(
                                  color: theme.colorScheme.onSurface,
                                  fontWeight: FontWeight.w500,
                                  fontFamily: 'monospace',
                                  fontSize: 12,
                                ),
                              ),
                            ),
                            const SizedBox(width: 8),
                            Text(
                              _filePreviewKindLabel(entry.name, entry.isDirectory, isZh),
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.65),
                                fontSize: 11,
                              ),
                            ),
                            if (!entry.isDirectory && entry.size > 0) ...<Widget>[
                              Text(
                                '  ·  ${_formatByteSize(entry.size)}',
                                style: theme.textTheme.bodySmall?.copyWith(
                                  color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
                                  fontFamily: 'monospace',
                                  fontSize: 11,
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class ToolInlineDetailSurface extends StatefulWidget {
  const ToolInlineDetailSurface({
    super.key,
    required this.title,
    required this.text,
    this.isSuccess,
  });

  final String title;
  final String text;
  final bool? isSuccess;

  @override
  State<ToolInlineDetailSurface> createState() =>
      _ToolInlineDetailSurfaceState();
}

class _ToolInlineDetailSurfaceState extends State<ToolInlineDetailSurface> {
  final ScrollController _scrollController = ScrollController();

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isZh = Localizations.localeOf(context).languageCode == 'zh';
    final failed = widget.isSuccess == false;
    final borderColor = failed
        ? theme.colorScheme.error.withValues(alpha: 0.35)
        : theme.colorScheme.outlineVariant.withValues(alpha: 0.35);
    return SelectionContainer.disabled(
      child: Container(
        width: double.infinity,
        margin: const EdgeInsets.only(bottom: 6),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerLowest.withValues(alpha: 0.6),
          borderRadius: BorderRadius.circular(6),
          border: Border.all(color: borderColor),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            SizedBox(
              height: 28,
              child: Row(
                children: <Widget>[
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      widget.title,
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: failed
                            ? theme.colorScheme.error
                            : theme.colorScheme.onSurfaceVariant,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ),
                  MessagePressShieldRegion(
                    child: Tooltip(
                      message: isZh ? '复制' : 'Copy',
                      child: InkResponse(
                        onTap: () => unawaited(_copyToolParam(widget.text)),
                        radius: 12,
                        child: Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 6,
                          ),
                          child: Icon(
                            Icons.content_copy,
                            size: 12,
                            color: theme.colorScheme.onSurfaceVariant
                                .withValues(alpha: 0.7),
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 200),
              child: Scrollbar(
                controller: _scrollController,
                child: SingleChildScrollView(
                  controller: _scrollController,
                  padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
                  child: SizedBox(
                    width: double.infinity,
                    child: Text(
                      widget.text,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurface,
                        fontFamily: 'monospace',
                        height: 1.4,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

Future<void> _copyToolParam(String text) async {
  try {
    await Clipboard.setData(ClipboardData(text: text));
  } on PlatformException catch (error, stackTrace) {
    debugPrint('Failed to copy tool parameters: $error\n$stackTrace');
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

  var rawTargetToolName = toolNameMatch?.group(1)?.trim() ?? '';
  final rawProxiedParams = paramsMatch?.group(1)?.trim() ?? '';

  if (rawTargetToolName.isEmpty && rawProxiedParams.isNotEmpty) {
    try {
      final unescaped = normalizeEscapedTextForDisplay(rawProxiedParams);
      final decoded = jsonDecode(unescaped);
      if (decoded is Map<String, dynamic>) {
        final candidate =
            decoded['tool_name'] ?? decoded['tool'] ?? decoded['name'];
        if (candidate is String && candidate.trim().isNotEmpty) {
          rawTargetToolName = candidate.trim();
        } else if (decoded.containsKey('input') ||
            decoded.containsKey('control')) {
          rawTargetToolName = 'input';
        }
      }
    } catch (_) {}
  }

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

String cleanTerminalCommand(String rawCmd) {
  var cmd = unescapeXmlForDisplay(rawCmd).trim();
  cmd = cmd.replaceAll(
    RegExp(r'^(?:export\s+[A-Za-z0-9_]+=[^\s;]+;?\s*)+'),
    '',
  );
  cmd = cmd.replaceAll(
    RegExp(r'''^cd\s+['"][^'"]+['"]\s*(?:&&|;)\s*'''),
    '',
  );
  cmd = cmd.replaceAll(
    RegExp(r'''^cd\s+[^\s;&]+(?:\s*(?:&&|;)\s*)'''),
    '',
  );
  return cmd.trim();
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

IconData getToolIcon(String toolName) {
  final lower = toolName.toLowerCase();
  final action = lower.contains(':')
      ? lower.substring(lower.lastIndexOf(':') + 1)
      : lower;
  const fileTools = <String>{
    'list_files', 'read_file', 'read_file_part', 'read_file_full',
    'read_file_binary', 'write_file', 'write_file_binary', 'delete_file',
    'file_exists', 'move_file', 'copy_file', 'make_directory', 'find_files',
    'grep_code', 'grep_context', 'file_info', 'zip_files', 'unzip_files',
    'open_file', 'share_file', 'download_file', 'apply_file', 'create_file',
    'edit_file',
  };
  const terminalTools = <String>{
    'execute_cli_command', 'run_command', 'terminal', 'bash', 'sh', 'cmd',
    'exec', 'input', 'get_terminal_info', 'execute_in_terminal_session',
    'execute_in_terminal_session_streaming', 'terminal_observe',
    'execute_hidden_terminal_command', 'create_terminal_session',
    'close_terminal_session', 'input_in_terminal_session',
    'get_terminal_session_screen', 'terminal_input', 'terminal_get_screen',
  };
  const deviceTools = <String>{
    'device_info', 'get_notifications', 'get_app_usage_time',
    'get_device_location', 'capture_screenshot', 'install_app',
    'uninstall_app', 'list_installed_apps', 'start_app', 'stop_app',
  };
  const chatTools = <String>{
    'start_chat_service', 'stop_chat_service', 'create_new_chat',
    'list_chats', 'find_chat', 'agent_status', 'switch_chat',
    'update_chat_title', 'update_chat_group', 'update_chat_pinned',
    'update_chat_locked', 'reorder_chats', 'delete_chat',
    'send_message_to_ai', 'send_message_to_ai_streaming',
    'chat_send_message', 'chat_observe', 'chat_cancel',
    'chat_read_extension', 'chat_write_extension', 'chat_delete_extension',
    'get_chat_messages', 'get_chat_messages_range',
  };

  if (fileTools.contains(action)) {
    if (action.startsWith('grep_') || action == 'find_files') {
      return Icons.search;
    }
    if (action == 'list_files' || action == 'make_directory') {
      return Icons.folder_outlined;
    }
    if (action == 'delete_file') return Icons.delete_outline;
    if (action == 'create_file') return Icons.note_add_outlined;
    if (action == 'edit_file' || action == 'apply_file') {
      return Icons.edit_outlined;
    }
    return Icons.description_outlined;
  }
  if (terminalTools.contains(action)) return Icons.terminal;
  if (action.startsWith('browser_')) return Icons.language_outlined;
  if (action.startsWith('bluetooth_') ||
      action == 'request_bluetooth_permission' ||
      action == 'get_bluetooth_state' ||
      action == 'request_enable_bluetooth' ||
      action == 'list_bluetooth_bonded_devices' ||
      action == 'scan_bluetooth_devices') {
    return Icons.bluetooth_outlined;
  }
  if (chatTools.contains(action)) return Icons.chat_bubble_outline;
  if (action.startsWith('music_') ||
      action.startsWith('tts_') ||
      action == 'stt_transcribe' ||
      action.startsWith('media_')) {
    return Icons.audio_file_outlined;
  }
  if (action.startsWith('theme_')) return Icons.palette_outlined;
  if (deviceTools.contains(action)) return Icons.devices_outlined;
  if (action == 'list_core_nodes' || action == 'switch_core') {
    return Icons.hub_outlined;
  }
  if (action == 'use_package' ||
      action == 'package_proxy' ||
      action == 'proxy' ||
      action == 'catalog_read_tool_sources') {
    return Icons.extension_outlined;
  }
  if (action == 'http_request' ||
      action == 'web_socket' ||
      action == 'multipart_request' ||
      action == 'manage_cookies' ||
      action == 'visit_web') {
    return Icons.language_outlined;
  }
  if (action == 'models_list_summaries' || action == 'call_chat_model') {
    return Icons.psychology_outlined;
  }
  if (action == 'get_system_setting' ||
      action == 'modify_system_setting' ||
      action == 'read_environment_variable' ||
      action == 'write_environment_variable') {
    return Icons.tune_outlined;
  }
  if (action == 'sleep') return Icons.schedule_outlined;
  if (action == 'toast' ||
      action == 'send_notification' ||
      action == 'set_clipboard_text') {
    return Icons.notifications_outlined;
  }
  if (action == 'storage_request') return Icons.storage_outlined;
  if (action == 'edge_execute' || action == 'io_execute') {
    return Icons.hub_outlined;
  }
  return Icons.widgets_outlined;
}

class SemanticToolDisplayInfo {
  const SemanticToolDisplayInfo({
    required this.actionName,
    required this.targetSummary,
    required this.icon,
  });

  final String actionName;
  final String targetSummary;
  final IconData icon;
}


class _VfsMountEntry {
  const _VfsMountEntry({required this.id, required this.name, required this.root});
  final String id;
  final String name;
  final String root;
}

Map<String, _VfsMountEntry>? _cachedVfsMounts;
DateTime _lastVfsMountCheck = DateTime.fromMillisecondsSinceEpoch(0);

Map<String, _VfsMountEntry> _getVfsMounts() {
  final now = DateTime.now();
  if (_cachedVfsMounts != null && now.difference(_lastVfsMountCheck).inSeconds < 5) {
    return _cachedVfsMounts!;
  }
  _lastVfsMountCheck = now;
  final mounts = <String, _VfsMountEntry>{};

  try {
    final catalogDirs = <String>[];
    if (Platform.isWindows) {
      final appData = Platform.environment['APPDATA'];
      if (appData != null && appData.isNotEmpty) {
        catalogDirs.add(p.join(appData, 'Operit2', 'runtime', 'identities'));
      }
      final localAppData = Platform.environment['LOCALAPPDATA'];
      if (localAppData != null && localAppData.isNotEmpty) {
        catalogDirs.add(p.join(localAppData, 'Operit2', 'runtime', 'identities'));
      }
    } else {
      final home = Platform.environment['HOME'];
      if (home != null && home.isNotEmpty) {
        if (Platform.isMacOS) {
          catalogDirs.add(p.join(home, 'Library', 'Application Support', 'Operit2', 'runtime', 'identities'));
        } else {
          catalogDirs.add(p.join(home, '.config', 'Operit2', 'runtime', 'identities'));
          catalogDirs.add(p.join(home, '.local', 'share', 'Operit2', 'runtime', 'identities'));
        }
      }
    }
    for (final identitiesDir in catalogDirs) {
      final dir = Directory(identitiesDir);
      if (!dir.existsSync()) continue;
      for (final identityEntity in dir.listSync()) {
        if (identityEntity is! Directory) continue;
        final catalogFile = File(p.join(identityEntity.path, 'config', 'vfs_mounts.json'));
        if (!catalogFile.existsSync()) continue;
        try {
          final content = catalogFile.readAsStringSync();
          final parsed = jsonDecode(content);
          if (parsed is Map<String, dynamic> && parsed['mounts'] is List) {
            for (final item in parsed['mounts'] as List) {
              if (item is Map) {
                final id = item['id']?.toString() ?? '';
                final name = item['name']?.toString() ?? '';
                final root = item['root']?.toString() ?? '';
                if (id.isNotEmpty) {
                  mounts[id] = _VfsMountEntry(
                    id: id,
                    name: name.isNotEmpty ? name : (root.isNotEmpty ? p.basename(root) : id),
                    root: root,
                  );
                }
              }
            }
          }
        } catch (_) {}
      }
    }
  } catch (_) {}
  _cachedVfsMounts = mounts;
  return mounts;
}

String cleanToolPath(String rawPath, {bool isZh = true}) {
  var path = rawPath.trim().replaceAll('\\', '/');
  if (path.isEmpty ||
      path == '.' ||
      path == './' ||
      path == '/app/workspaces' ||
      path == '/mnt/local/folders' ||
      path == '/app/workspaces/' ||
      path == '/mnt/local/folders/') {
    return isZh ? '工作区根目录' : 'Workspace root';
  }

  // Strip container mount prefixes and resolve mount name:
  final mountMatch = RegExp(
    r'^(?:/mnt/local/folders/|/app/workspaces/)([0-9a-zA-Z_-]{8,64})(?:/(.*))?$',
  ).firstMatch(path);
  if (mountMatch != null) {
    final mountId = mountMatch.group(1)!;
    final sub = mountMatch.group(2)?.trim();
    final vfsMounts = _getVfsMounts();
    final mountName = vfsMounts[mountId]?.name ?? '';

    if (sub != null && sub.isNotEmpty) {
      if (mountName.isNotEmpty) {
        if (sub == mountName) {
          return mountName;
        }
        if (sub.startsWith('$mountName/')) {
          return sub;
        }
      }
      return mountName.isNotEmpty ? '$mountName/$sub' : sub;
    } else {
      return mountName.isNotEmpty
          ? mountName
          : (isZh ? '工作区根目录' : 'Workspace root');
    }
  }

  // Strip Android storage prefixes:
  final androidRegex = RegExp(
    r'^/mnt/android/sdcard/(?:workspaces/[^/]+/)?(.*)$',
  );
  final androidMatch = androidRegex.firstMatch(path);
  if (androidMatch != null) {
    final sub = androidMatch.group(1)?.trim();
    if (sub != null && sub.isNotEmpty) {
      return sub;
    }
  }

  return path;
}

Map<String, String> extractToolParamMap(String params) {
  final result = <String, String>{};
  final paramRegex = RegExp(r'''<param\s+name=['"]([^'"]+)['"]>([\s\S]*?)<\/param>''');
  for (final match in paramRegex.allMatches(params)) {
    final name = match.group(1)?.trim();
    final value = match.group(2)?.trim();
    if (name != null && value != null) {
      result[name] = unescapeXmlForDisplay(value);
    }
  }
  final directTagRegex = RegExp(r'<([a-zA-Z0-9_]+)>([^<]+)<\/\1>');
  for (final match in directTagRegex.allMatches(params)) {
    final name = match.group(1)!.trim();
    if (!result.containsKey(name) && name != 'param' && name != 'tool' && name != 'tool_result') {
      result[name] = unescapeXmlForDisplay(match.group(2)!.trim());
    }
  }
  final trimmed = params.trim();
  if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
    try {
      final decoded = jsonDecode(trimmed);
      if (decoded is Map<String, dynamic>) {
        for (final entry in decoded.entries) {
          if (!result.containsKey(entry.key) && entry.value != null) {
            result[entry.key] = entry.value.toString();
          }
        }
      }
    } catch (_) {}
  }
  return result;
}

SemanticToolDisplayInfo buildSemanticToolDisplay(
  String toolName,
  String params, {
  bool isZh = true,
}) {
  final paramMap = extractToolParamMap(params);
  final rawPath = paramMap['path'] ?? '';
  final cleanedPath = cleanToolPath(rawPath, isZh: isZh);
  final lowerTool = toolName.toLowerCase();

  if (lowerTool == 'read_file' ||
      lowerTool == 'read_file_part' ||
      lowerTool == 'read_file_full' ||
      lowerTool == 'read_file_binary') {
    final start = paramMap['start_line'];
    final end = paramMap['end_line'];
    var target = cleanedPath;
    if (start != null && start.isNotEmpty) {
      target = end != null && end.isNotEmpty
          ? '$cleanedPath (L$start-L$end)'
          : '$cleanedPath (L$start)';
    }
    return SemanticToolDisplayInfo(
      actionName: isZh ? '读取' : 'Read',
      targetSummary: target.isNotEmpty ? target : (isZh ? '文件' : 'file'),
      icon: Icons.file_open,
    );
  }

  if (lowerTool == 'edit_file') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '编辑' : 'Edit',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '文件' : 'file'),
      icon: Icons.edit_note,
    );
  }

  if (lowerTool == 'apply_file') {
    final type = paramMap['type']?.toLowerCase();
    final action = switch (type) {
      'create' => isZh ? '创建' : 'Create',
      'delete' => isZh ? '删除' : 'Delete',
      _ => isZh ? '编辑' : 'Edit',
    };
    return SemanticToolDisplayInfo(
      actionName: action,
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '文件' : 'file'),
      icon: Icons.edit_note,
    );
  }

  if (lowerTool == 'write_file' || lowerTool == 'write_file_binary') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '写入' : 'Write',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '文件' : 'file'),
      icon: Icons.edit_document,
    );
  }

  if (lowerTool == 'create_file') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '创建' : 'Create',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '文件' : 'file'),
      icon: Icons.note_add,
    );
  }

  if (lowerTool == 'delete_file') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '删除' : 'Delete',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '文件' : 'file'),
      icon: Icons.delete_outline,
    );
  }

  if (lowerTool == 'list_files') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '浏览' : 'List',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '目录' : 'directory'),
      icon: Icons.folder_open,
    );
  }

  if (lowerTool == 'find_files') {
    final pattern = paramMap['pattern'] ?? '';
    final target = pattern.isNotEmpty
        ? (cleanedPath.isNotEmpty ? '$pattern ($cleanedPath)' : pattern)
        : cleanedPath;
    return SemanticToolDisplayInfo(
      actionName: isZh ? '查找' : 'Find',
      targetSummary: target,
      icon: Icons.search,
    );
  }

  if (lowerTool == 'grep_code' ||
      lowerTool == 'grep' ||
      lowerTool == 'grep_context') {
    final pattern = paramMap['pattern'] ?? paramMap['intent'] ?? '';
    return SemanticToolDisplayInfo(
      actionName: isZh ? '搜索' : 'Grep',
      targetSummary: pattern.isNotEmpty ? pattern : cleanedPath,
      icon: Icons.search,
    );
  }

  if (lowerTool == 'use_package' || lowerTool == 'package_use') {
    final pkg = paramMap['package_name'] ??
        paramMap['name'] ??
        paramMap['package'] ??
        buildParamsHeadPreview(params);
    return SemanticToolDisplayInfo(
      actionName: isZh ? '加载工具包' : 'Package',
      targetSummary: pkg,
      icon: Icons.extension,
    );
  }

  if (lowerTool == 'execute_cli_command' ||
      lowerTool == 'run_command' ||
      lowerTool == 'terminal' ||
      lowerTool == 'bash' ||
      lowerTool == 'sh' ||
      lowerTool.endsWith(':bash') ||
      lowerTool.endsWith(':sh') ||
      lowerTool.endsWith(':cmd') ||
      lowerTool.endsWith(':exec')) {
    var cmd = paramMap['command'] ?? paramMap['cmd'] ?? '';
    if (cmd.isEmpty) {
      cmd = buildParamsHeadPreview(params);
    }
    cmd = cleanTerminalCommand(cmd);
    return SemanticToolDisplayInfo(
      actionName: isZh ? '终端' : 'Terminal',
      targetSummary: cmd,
      icon: Icons.terminal,
    );
  }

  if (lowerTool == 'terminal_input' ||
      lowerTool == 'input' ||
      lowerTool.endsWith(':input')) {
    var input = paramMap['input'] ??
        paramMap['text'] ??
        paramMap['control'] ??
        '';
    if (input.isEmpty) {
      input = buildParamsHeadPreview(params);
    }
    input = unescapeXmlForDisplay(input).trim();
    return SemanticToolDisplayInfo(
      actionName: isZh ? '输入' : 'Input',
      targetSummary: input,
      icon: Icons.keyboard,
    );
  }

  if (lowerTool == 'terminal_get_screen' ||
      lowerTool == 'get_screen' ||
      lowerTool.endsWith(':get_screen') ||
      lowerTool.endsWith(':screen')) {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '终端屏幕' : 'Screen',
      targetSummary: isZh ? '捕获控制台输出' : 'Capture console output',
      icon: Icons.screenshot_monitor,
    );
  }

  if (lowerTool == 'package_proxy' || lowerTool == 'proxy') {
    final target = paramMap['tool_name'] ??
        paramMap['command'] ??
        paramMap['input'] ??
        buildParamsHeadPreview(params);
    return SemanticToolDisplayInfo(
      actionName: isZh ? '代理调用' : 'Proxy',
      targetSummary: target,
      icon: Icons.alt_route,
    );
  }

  if (lowerTool == 'list_core_nodes') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '设备' : 'Nodes',
      targetSummary: isZh ? '获取可用计算节点' : 'List available devices',
      icon: Icons.hub,
    );
  }

  if (lowerTool == 'switch_core') {
    final targetNode = paramMap['node_id'] ?? paramMap['node'] ?? '';
    return SemanticToolDisplayInfo(
      actionName: isZh ? '流转' : 'Switch',
      targetSummary: targetNode.isNotEmpty
          ? (isZh ? '切换至 $targetNode' : 'Switch to $targetNode')
          : (isZh ? '切换节点' : 'Switch device'),
      icon: Icons.swap_horiz,
    );
  }

  if (lowerTool == 'visit_web' || lowerTool == 'web_visit') {
    var url = paramMap['url'] ??
        paramMap['uri'] ??
        paramMap['link'] ??
        paramMap['href'] ??
        paramMap['target'] ??
        '';
    if (url.isEmpty) {
      final urlMatch = RegExp(r'''https?://[^\s<>"']+''').firstMatch(params);
      if (urlMatch != null) {
        url = urlMatch.group(0)!;
      }
    }
    if (url.isEmpty && paramMap.containsKey('link_number')) {
      url = '链接 #${paramMap['link_number']}';
    }
    return SemanticToolDisplayInfo(
      actionName: isZh ? '访问' : 'Visit',
      targetSummary: url,
      icon: Icons.web,
    );
  }

  if (lowerTool == 'file_exists') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '检查' : 'Check',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '文件' : 'file'),
      icon: Icons.help_outline,
    );
  }

  if (lowerTool == 'copy_file') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '复制' : 'Copy',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '文件' : 'file'),
      icon: Icons.copy_all,
    );
  }

  if (lowerTool == 'move_file') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '移动' : 'Move',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '文件' : 'file'),
      icon: Icons.drive_file_move_outlined,
    );
  }

  if (lowerTool == 'make_directory') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '新建目录' : 'Mkdir',
      targetSummary: cleanedPath.isNotEmpty ? cleanedPath : (isZh ? '目录' : 'directory'),
      icon: Icons.create_new_folder_outlined,
    );
  }

  if (lowerTool == 'browser_navigate' || lowerTool == 'browser_navigate_back') {
    final url = paramMap['url'] ?? '';
    return SemanticToolDisplayInfo(
      actionName: isZh ? '网页导航' : 'Navigate',
      targetSummary: url,
      icon: Icons.language,
    );
  }

  if (lowerTool == 'browser_click') {
    final target = paramMap['selector'] ?? paramMap['text'] ?? paramMap['target'] ?? '';
    return SemanticToolDisplayInfo(
      actionName: isZh ? '点击元素' : 'Click',
      targetSummary: target,
      icon: Icons.touch_app,
    );
  }

  if (lowerTool == 'browser_type' || lowerTool == 'browser_fill_form') {
    final target = paramMap['text'] ?? paramMap['value'] ?? '';
    return SemanticToolDisplayInfo(
      actionName: isZh ? '网页输入' : 'Type',
      targetSummary: target,
      icon: Icons.keyboard,
    );
  }

  if (lowerTool == 'browser_take_screenshot' || lowerTool == 'browser_snapshot') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '网页截图' : 'Screenshot',
      targetSummary: isZh ? '捕获网页画面' : 'Capture webpage',
      icon: Icons.camera_alt_outlined,
    );
  }

  if (lowerTool == 'capture_screenshot') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '屏幕截图' : 'Screenshot',
      targetSummary: isZh ? '捕获屏幕画面' : 'Capture screen',
      icon: Icons.screenshot_outlined,
    );
  }

  if (lowerTool == 'set_clipboard_text') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '剪贴板' : 'Clipboard',
      targetSummary: isZh ? '写入文本' : 'Copy text',
      icon: Icons.content_paste_outlined,
    );
  }

  if (lowerTool == 'toast' || lowerTool == 'send_notification') {
    final msg = paramMap['message'] ?? paramMap['title'] ?? paramMap['text'] ?? '';
    return SemanticToolDisplayInfo(
      actionName: isZh ? '发送通知' : 'Notify',
      targetSummary: msg,
      icon: Icons.notifications_outlined,
    );
  }

  if (lowerTool == 'music_play') {
    final track = paramMap['title'] ?? paramMap['name'] ?? paramMap['song'] ?? paramMap['path'] ?? '';
    return SemanticToolDisplayInfo(
      actionName: isZh ? '播放音乐' : 'Play',
      targetSummary: cleanToolPath(track, isZh: isZh),
      icon: Icons.music_note,
    );
  }

  if (lowerTool == 'music_pause' || lowerTool == 'music_stop' || lowerTool == 'music_resume') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '音乐控制' : 'Music',
      targetSummary: '',
      icon: Icons.music_note,
    );
  }

  if (lowerTool == 'tts_synthesize') {
    final text = paramMap['text'] ?? '';
    return SemanticToolDisplayInfo(
      actionName: isZh ? '语音合成' : 'TTS',
      targetSummary: text.length > 30 ? '${text.substring(0, 27)}...' : text,
      icon: Icons.record_voice_over_outlined,
    );
  }

  if (lowerTool == 'stt_transcribe') {
    return SemanticToolDisplayInfo(
      actionName: isZh ? '语音识别' : 'STT',
      targetSummary: isZh ? '转录音频' : 'Transcribe',
      icon: Icons.mic_outlined,
    );
  }

  // Generic plugin tool heuristic for ToolPkg and custom plugins:
  final colon = toolName.indexOf(':');
  final shortName = colon > 0 ? toolName.substring(colon + 1) : toolName;
  final pluginSemantic = _inferPluginSemantic(shortName, paramMap, isZh: isZh);
  if (pluginSemantic != null) {
    return SemanticToolDisplayInfo(
      actionName: pluginSemantic.actionName,
      targetSummary: pluginSemantic.targetSummary,
      icon: pluginSemantic.icon,
    );
  }

  final defaultSummary = buildParamsHeadPreview(params);
  return SemanticToolDisplayInfo(
    actionName: toolName,
    targetSummary: defaultSummary,
    icon: getToolIcon(toolName),
  );
}

/// Heuristically derives a user-friendly action name and target for custom
/// plugins and ToolPkg tools that do not have dedicated built-in rules.
SemanticToolDisplayInfo? _inferPluginSemantic(
  String shortToolName,
  Map<String, String> paramMap, {
  required bool isZh,
}) {
  final lower = shortToolName.toLowerCase();
  String? verb;
  IconData icon = Icons.extension_outlined;

  if (lower.startsWith('get_') || lower.startsWith('fetch_') || lower.startsWith('query_')) {
    verb = isZh ? '查询' : 'Query';
    icon = Icons.search;
  } else if (lower.startsWith('list_')) {
    verb = isZh ? '列出' : 'List';
    icon = Icons.format_list_bulleted;
  } else if (lower.startsWith('search_') || lower.startsWith('find_')) {
    verb = isZh ? '搜索' : 'Search';
    icon = Icons.search;
  } else if (lower.startsWith('create_') || lower.startsWith('add_') || lower.startsWith('new_')) {
    verb = isZh ? '创建' : 'Create';
    icon = Icons.add_circle_outline;
  } else if (lower.startsWith('update_') || lower.startsWith('edit_') || lower.startsWith('set_') || lower.startsWith('modify_')) {
    verb = isZh ? '修改' : 'Update';
    icon = Icons.edit_outlined;
  } else if (lower.startsWith('delete_') || lower.startsWith('remove_')) {
    verb = isZh ? '删除' : 'Delete';
    icon = Icons.delete_outline;
  } else if (lower.startsWith('send_') || lower.startsWith('post_') || lower.startsWith('push_')) {
    verb = isZh ? '发送' : 'Send';
    icon = Icons.send_outlined;
  } else if (lower.startsWith('open_') || lower.startsWith('load_')) {
    verb = isZh ? '打开' : 'Open';
    icon = Icons.folder_open;
  } else if (lower.startsWith('run_') || lower.startsWith('exec_') || lower.startsWith('start_')) {
    verb = isZh ? '执行' : 'Execute';
    icon = Icons.play_arrow_outlined;
  } else if (lower.startsWith('stop_') || lower.startsWith('close_')) {
    verb = isZh ? '停止' : 'Stop';
    icon = Icons.stop_circle_outlined;
  } else if (lower.startsWith('check_') || lower.startsWith('verify_') || lower.startsWith('validate_')) {
    verb = isZh ? '检查' : 'Check';
    icon = Icons.verified_outlined;
  } else if (lower.startsWith('sync_')) {
    verb = isZh ? '同步' : 'Sync';
    icon = Icons.sync;
  }

  const priorityKeys = <String>[
    'path', 'file', 'target', 'name', 'title', 'query', 'pattern', 'url',
    'location', 'city', 'command', 'cmd', 'key', 'id', 'message', 'text',
  ];
  String target = '';
  for (final key in priorityKeys) {
    final val = paramMap[key];
    if (val != null && val.isNotEmpty) {
      target = cleanToolPath(val, isZh: isZh);
      break;
    }
  }

  if (verb == null && target.isEmpty) return null;
  return SemanticToolDisplayInfo(
    actionName: verb ?? buildToolActionName(shortToolName),
    targetSummary: target,
    icon: icon,
  );
}
