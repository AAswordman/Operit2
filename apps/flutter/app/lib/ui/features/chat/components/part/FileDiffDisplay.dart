// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../workspace/WorkspaceFileIcon.dart';
import '../workspace/WorkspaceInlineDiff.dart';
import 'ToolDisplayComponents.dart' show cleanToolPath;

class FileDiff {
  const FileDiff({
    required this.path,
    required this.diffContent,
    required this.details,
  });

  final String path;
  final String diffContent;
  final String details;
}

/// One file diff rendered as an integrated code card (Cursor-style).
class FileDiffDisplay extends StatelessWidget {
  const FileDiffDisplay({
    super.key,
    required this.diff,
    this.showHeader = true,
    this.actionTitle,
    this.isExpanded = true,
    this.onToggleExpand,
  });

  final FileDiff diff;
  final bool showHeader;
  final String? actionTitle;
  final bool isExpanded;
  final VoidCallback? onToggleExpand;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final stats = unifiedDiffStats(diff.diffContent);
    final (fileName, directory) = _splitPath(diff.path);
    return SelectionContainer.disabled(
      child: Container(
        width: double.infinity,
        margin: const EdgeInsets.only(top: 2, bottom: 6),
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
            if (showHeader)
              Material(
                color: Colors.transparent,
                child: InkWell(
                  onTap: onToggleExpand,
                  borderRadius: isExpanded
                      ? const BorderRadius.vertical(top: Radius.circular(8))
                      : BorderRadius.circular(8),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 7,
                    ),
                    child: Row(
                      children: <Widget>[
                        buildWorkspaceFileIcon(fileName, size: 15),
                        const SizedBox(width: 8),
                        if (actionTitle != null && actionTitle!.isNotEmpty) ...<Widget>[
                          Text(
                            actionTitle!,
                            style: theme.textTheme.labelMedium?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant
                                  .withValues(alpha: 0.85),
                              fontWeight: FontWeight.w500,
                              fontSize: 12,
                            ),
                          ),
                          const SizedBox(width: 8),
                        ],
                        Expanded(
                          child: Text.rich(
                            TextSpan(
                              children: <InlineSpan>[
                                TextSpan(
                                  text: fileName,
                                  style: theme.textTheme.labelMedium?.copyWith(
                                    color: theme.colorScheme.onSurface,
                                    fontWeight: FontWeight.w600,
                                    fontFamily: 'monospace',
                                    fontSize: 12.5,
                                  ),
                                ),
                                if (directory.isNotEmpty)
                                  TextSpan(
                                    text: '  $directory',
                                    style: theme.textTheme.bodySmall?.copyWith(
                                      color: theme.colorScheme.onSurfaceVariant
                                          .withValues(alpha: 0.5),
                                      fontFamily: 'monospace',
                                      fontSize: 11,
                                    ),
                                  ),
                              ],
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 8),
                        if (stats.additions > 0) ...<Widget>[
                          Text(
                            '+${stats.additions}',
                            style: theme.textTheme.labelSmall?.copyWith(
                              color: const Color(0xFF4ADE80),
                              fontWeight: FontWeight.w600,
                              fontFamily: 'monospace',
                              fontSize: 12,
                            ),
                          ),
                          const SizedBox(width: 6),
                        ],
                        if (stats.deletions > 0) ...<Widget>[
                          Text(
                            '-${stats.deletions}',
                            style: theme.textTheme.labelSmall?.copyWith(
                              color: const Color(0xFFF87171),
                              fontWeight: FontWeight.w600,
                              fontFamily: 'monospace',
                              fontSize: 12,
                            ),
                          ),
                          const SizedBox(width: 6),
                        ] else if (stats.additions > 0) ...<Widget>[
                          Text(
                            '-0',
                            style: theme.textTheme.labelSmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant
                                  .withValues(alpha: 0.45),
                              fontWeight: FontWeight.w600,
                              fontFamily: 'monospace',
                              fontSize: 12,
                            ),
                          ),
                          const SizedBox(width: 6),
                        ],
                        if (onToggleExpand != null)
                          Icon(
                            isExpanded
                                ? Icons.unfold_less
                                : Icons.unfold_more,
                            size: 14,
                            color: theme.colorScheme.onSurfaceVariant
                                .withValues(alpha: 0.7),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
            if (isExpanded) ...<Widget>[
              if (showHeader)
                Divider(
                  height: 1,
                  thickness: 1,
                  color: theme.colorScheme.outlineVariant
                      .withValues(alpha: 0.25),
                ),
              WorkspaceInlineDiff(
                diff: diff.diffContent,
                filePath: diff.path,
                embedded: true,
                allowImplicitHunks: true,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

(String, String) _splitPath(String rawPath) {
  final normalized = cleanToolPath(rawPath).replaceAll('\\', '/');
  final slash = normalized.lastIndexOf('/');
  if (slash == -1) {
    return (normalized, '');
  }
  return (
    normalized.substring(slash + 1),
    normalized.substring(0, slash),
  );
}

List<FileDiff> extractFileDiffs(String resultContent) {
  return RegExp(
    r'<file-diff\b([^>]*)>([\s\S]*?)<\/file-diff>',
    caseSensitive: false,
  ).allMatches(resultContent).map((match) {
    final attrs = match.group(1) ?? '';
    final body = match.group(2) ?? '';
    final path =
        RegExp(
          r'\bpath="([^"]*)"',
          caseSensitive: false,
        ).firstMatch(attrs)?.group(1) ??
        '';
    final details =
        RegExp(
          r'\bdetails="([^"]*)"',
          caseSensitive: false,
        ).firstMatch(attrs)?.group(1) ??
        '';
    final cdata = RegExp(
      r'<!\[CDATA\[([\s\S]*?)\]\]>',
      caseSensitive: false,
    ).firstMatch(body)?.group(1);
    return FileDiff(
      path: _decodeXmlText(path),
      details: _decodeXmlText(details),
      diffContent: _decodeXmlText((cdata ?? body).trim()),
    );
  }).toList();
}

String _decodeXmlText(String input) {
  return input
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .replaceAll('&amp;', '&')
      .replaceAll('&quot;', '"')
      .replaceAll('&apos;', "'");
}
