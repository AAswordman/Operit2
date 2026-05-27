// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'DialogComponents.dart';

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

class FileDiffDisplay extends StatefulWidget {
  const FileDiffDisplay({super.key, required this.diff});

  final FileDiff diff;

  @override
  State<FileDiffDisplay> createState() => _FileDiffDisplayState();
}

class _FileDiffDisplayState extends State<FileDiffDisplay> {
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final fileName = _extractFileName(widget.diff.path);
    final summary = _buildDiffSummary(widget.diff.diffContent);
    return Padding(
      padding: const EdgeInsets.only(left: 24, bottom: 8),
      child: InkWell(
        onTap: () {
          showDialog<void>(
            context: context,
            builder: (context) => ContentDetailDialog(
              title: 'File Changes: $fileName',
              content: widget.diff.diffContent,
              icon: Icons.difference,
              isDiffContent: true,
              onDismiss: () {
                Navigator.of(context).pop();
              },
            ),
          );
        },
        borderRadius: BorderRadius.circular(4),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Row(
            children: <Widget>[
              Icon(
                Icons.subdirectory_arrow_right,
                size: 18,
                color: theme.colorScheme.primary.withValues(alpha: 0.7),
              ),
              const SizedBox(width: 8),
              Icon(
                Icons.difference,
                size: 16,
                color: theme.colorScheme.primary.withValues(alpha: 0.7),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      fileName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.labelMedium?.copyWith(
                        fontWeight: FontWeight.w500,
                        color: theme.colorScheme.primary,
                      ),
                    ),
                    Text(
                      summary,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant.withValues(
                          alpha: 0.7,
                        ),
                      ),
                    ),
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

String _extractFileName(String path) {
  final normalized = path.replaceAll('\\', '/');
  final segments = normalized.split('/');
  return segments.isEmpty || segments.last.isEmpty ? path : segments.last;
}

String _buildDiffSummary(String diffContent) {
  final lines = diffContent.trimRight().split('\n');
  final additions = lines.where((line) => line.startsWith('+')).length;
  final deletions = lines.where((line) => line.startsWith('-')).length;
  if (additions > 0 && deletions > 0) {
    return '$additions insertions(+), $deletions deletions(-)';
  }
  if (additions > 0) {
    return '$additions insertions(+)';
  }
  if (deletions > 0) {
    return '$deletions deletions(-)';
  }
  return 'No changes detected';
}
