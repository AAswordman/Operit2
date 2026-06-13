// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../common/components/OperitDialog.dart';

import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import 'XmlCanvasSummaryComponents.dart';

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
    final trimmedResult = result.trim();
    final hasContent = trimmedResult.isNotEmpty;
    final summary = hasContent
        ? trimmedResult.substring(
            0,
            trimmedResult.length > 200 ? 200 : trimmedResult.length,
          )
        : (isSuccess ? 'Execution success' : 'Execution failed');

    return Row(
      children: <Widget>[
        Expanded(
          child: CanvasToolResultRow(
            summary: summary,
            isSuccess: isSuccess,
            semanticDescription: _semanticDescription(
              toolName,
              summary,
              trimmedResult,
              isSuccess,
              hasContent,
            ),
            emphasizeSummary: !hasContent,
            onClick: hasContent
                ? () {
                    showDialog<void>(
                      context: context,
                      builder: (dialogContext) {
                        return _ToolResultDetailDialog(
                          toolName: toolName,
                          result: result,
                          isSuccess: isSuccess,
                          onDismiss: () {
                            Navigator.of(dialogContext).pop();
                          },
                          onCopy: () {
                            Clipboard.setData(ClipboardData(text: result));
                          },
                        );
                      },
                    );
                  }
                : null,
            onCopyClick: hasContent
                ? () {
                    Clipboard.setData(ClipboardData(text: result));
                  }
                : null,
          ),
        ),
        if (isStreaming)
          const Padding(
            padding: EdgeInsets.only(left: 6),
            child: StreamingCursor(),
          ),
      ],
    );
  }
}

String _semanticDescription(
  String toolName,
  String summary,
  String result,
  bool isSuccess,
  bool hasContent,
) {
  final status = isSuccess ? 'Success' : 'Failed';
  if (!hasContent) {
    return 'Tool execution result: $toolName, $status, $summary';
  }
  final normalized = result.replaceAll(RegExp(r'\s+'), ' ').trim();
  final resultText = normalized.length <= 20
      ? normalized
      : '${normalized.substring(0, 20)}...';
  return 'Tool execution result: $toolName, $status, $resultText';
}

class _ToolResultDetailDialog extends StatelessWidget {
  const _ToolResultDetailDialog({
    required this.toolName,
    required this.result,
    required this.isSuccess,
    required this.onDismiss,
    required this.onCopy,
  });

  final String toolName;
  final String result;
  final bool isSuccess;
  final VoidCallback onDismiss;
  final VoidCallback onCopy;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return OperitDialogScaffold(
      title:
          '$toolName ${isSuccess ? 'Execution success' : 'Execution failed'}',
      icon: Icon(
        isSuccess ? Icons.check : Icons.close,
        size: 20,
        color: isSuccess ? theme.colorScheme.primary : theme.colorScheme.error,
      ),
      maxWidth: 720,
      titleActions: <Widget>[
        IconButton(
          onPressed: onCopy,
          icon: Icon(Icons.content_copy, color: theme.colorScheme.primary),
        ),
      ],
      actions: <Widget>[
        FilledButton(onPressed: onDismiss, child: const Text('Close')),
      ],
      child: Container(
        width: double.infinity,
        constraints: const BoxConstraints(minHeight: 50, maxHeight: 300),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: isSuccess
              ? theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5)
              : theme.colorScheme.errorContainer.withValues(alpha: 0.2),
          borderRadius: BorderRadius.circular(8),
        ),
        child: SingleChildScrollView(
          child: SelectableText(
            result,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurface,
            ),
          ),
        ),
      ),
    );
  }
}
