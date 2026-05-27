// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'ParamVisualizer.dart';
import 'ToolDisplayComponents.dart';

class ContentDetailDialog extends StatelessWidget {
  const ContentDetailDialog({
    super.key,
    required this.title,
    required this.content,
    required this.icon,
    required this.onDismiss,
    this.isDiffContent = false,
  });

  final String title;
  final String content;
  final IconData icon;
  final VoidCallback onDismiss;
  final bool isDiffContent;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isXmlContent = content.trim().startsWith('<');
    return Dialog(
      insetPadding: const EdgeInsets.all(16),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720),
        child: Card(
          margin: EdgeInsets.zero,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
          color: theme.colorScheme.surface,
          elevation: 6,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Row(
                  children: <Widget>[
                    Icon(icon, size: 20, color: theme.colorScheme.primary),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        title,
                        style: theme.textTheme.titleMedium?.copyWith(
                          color: theme.colorScheme.onSurface,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                Divider(color: theme.colorScheme.outlineVariant, height: 1),
                const SizedBox(height: 16),
                Container(
                  width: double.infinity,
                  constraints: const BoxConstraints(
                    minHeight: 50,
                    maxHeight: 400,
                  ),
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerHighest.withValues(
                      alpha: 0.5,
                    ),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: SingleChildScrollView(
                    child: isDiffContent
                        ? _DiffContent(lines: content.lines())
                        : isXmlContent
                        ? ParamVisualizer(xmlContent: content)
                        : CodeContentWithLineNumbers(
                            lines: content.lines(),
                            textColor: theme.colorScheme.onSurface,
                          ),
                  ),
                ),
                const SizedBox(height: 16),
                Align(
                  alignment: Alignment.centerRight,
                  child: FilledButton(
                    onPressed: onDismiss,
                    child: const Text('Close'),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _DiffContent extends StatelessWidget {
  const _DiffContent({required this.lines});

  final List<String> lines;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final line in lines)
          Container(
            width: double.infinity,
            color: _diffBackground(theme, line),
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Text(
              line.trimRight(),
              softWrap: true,
              style: theme.textTheme.bodySmall?.copyWith(
                color: _diffTextColor(theme, line),
                fontFamily: 'monospace',
                fontSize: 11,
              ),
            ),
          ),
      ],
    );
  }
}

Color _diffBackground(ThemeData theme, String line) {
  if (line.startsWith('+')) {
    return theme.colorScheme.primaryContainer.withValues(alpha: 0.2);
  }
  if (line.startsWith('-')) {
    return theme.colorScheme.errorContainer.withValues(alpha: 0.2);
  }
  if (line.startsWith('@@')) {
    return theme.colorScheme.secondaryContainer.withValues(alpha: 0.2);
  }
  return Colors.transparent;
}

Color _diffTextColor(ThemeData theme, String line) {
  if (line.startsWith('+')) {
    return theme.colorScheme.primary;
  }
  if (line.startsWith('-')) {
    return theme.colorScheme.error;
  }
  if (line.startsWith('@@')) {
    return theme.colorScheme.secondary;
  }
  return theme.colorScheme.onSurfaceVariant;
}

extension _DialogStringLines on String {
  List<String> lines() => split('\n');
}
