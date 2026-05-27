// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../common/markdown/StreamMarkdownRenderer.dart';

class ToolResultDisplay extends StatelessWidget {
  const ToolResultDisplay({
    super.key,
    required this.toolName,
    required this.result,
    required this.isSuccess,
    required this.isStreaming,
  });

  final String toolName;
  final String result;
  final bool isSuccess;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final statusColor = isSuccess
        ? theme.colorScheme.primary
        : theme.colorScheme.error;
    final trimmedResult = result.trim();
    final hasContent = trimmedResult.isNotEmpty;
    final summary = hasContent
        ? trimmedResult.substring(
            0,
            trimmedResult.length > 200 ? 200 : trimmedResult.length,
          )
        : (isSuccess ? 'Execution success' : 'Execution failed');

    return Padding(
      padding: const EdgeInsets.only(left: 24, bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          Icon(
            Icons.subdirectory_arrow_right,
            size: 16,
            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
          ),
          const SizedBox(width: 6),
          Icon(
            isSuccess ? Icons.check : Icons.close,
            size: 15,
            color: statusColor,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              summary,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
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
