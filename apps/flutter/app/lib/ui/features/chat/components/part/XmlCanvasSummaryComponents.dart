// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../common/interactions/MessagePressShield.dart';

class CanvasToolSummaryRow extends StatelessWidget {
  const CanvasToolSummaryRow({
    super.key,
    required this.toolName,
    required this.summary,
    required this.semanticDescription,
    required this.leadingIcon,
    required this.titleColor,
    required this.summaryColor,
    this.onClick,
  });

  final String toolName;
  final String summary;
  final String semanticDescription;
  final IconData leadingIcon;
  final Color titleColor;
  final Color summaryColor;
  final VoidCallback? onClick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final row = Semantics(
      button: onClick != null,
      label: semanticDescription,
      child: Padding(
        padding: const EdgeInsets.only(top: 4),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            Icon(
              leadingIcon,
              size: 16,
              color: titleColor.withValues(alpha: 0.7),
            ),
            const SizedBox(width: 8),
            ConstrainedBox(
              constraints: const BoxConstraints(minWidth: 80, maxWidth: 120),
              child: Text(
                toolName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.labelMedium?.copyWith(color: titleColor),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                _toCanvasSingleLineText(summary),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(color: summaryColor),
              ),
            ),
          ],
        ),
      ),
    );

    if (onClick == null) {
      return SizedBox(width: double.infinity, child: row);
    }
    return MessagePressShieldRegion(
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onClick,
          child: SizedBox(width: double.infinity, child: row),
        ),
      ),
    );
  }
}

class CanvasToolResultRow extends StatelessWidget {
  const CanvasToolResultRow({
    super.key,
    required this.summary,
    required this.isSuccess,
    required this.semanticDescription,
    this.emphasizeSummary = false,
    this.onClick,
    this.onCopyClick,
  });

  final String summary;
  final bool isSuccess;
  final String semanticDescription;
  final bool emphasizeSummary;
  final VoidCallback? onClick;
  final VoidCallback? onCopyClick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final leadingTint = isSuccess
        ? theme.colorScheme.primary.withValues(alpha: 0.7)
        : theme.colorScheme.error.withValues(alpha: 0.7);
    final statusTint = isSuccess
        ? theme.colorScheme.primary
        : theme.colorScheme.error;
    final summaryColor = isSuccess
        ? theme.colorScheme.onSurface.withValues(alpha: 0.8)
        : theme.colorScheme.error.withValues(alpha: 0.8);

    final row = Semantics(
      button: onClick != null,
      label: semanticDescription,
      child: Padding(
        padding: const EdgeInsets.only(left: 24, right: 16, top: 2, bottom: 2),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            Icon(Icons.subdirectory_arrow_right, size: 18, color: leadingTint),
            const SizedBox(width: 8),
            Icon(
              isSuccess ? Icons.check : Icons.close,
              size: 14,
              color: statusTint,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                _toCanvasSingleLineText(summary),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: summaryColor,
                  fontWeight: emphasizeSummary ? FontWeight.w600 : null,
                ),
              ),
            ),
            if (onCopyClick != null)
              MessagePressShieldRegion(
                child: Semantics(
                  button: true,
                  label: '$semanticDescription, copy',
                  child: InkResponse(
                    onTap: onCopyClick,
                    radius: 16,
                    child: Padding(
                      padding: const EdgeInsets.all(5),
                      child: Icon(
                        Icons.content_copy,
                        size: 14,
                        color: theme.colorScheme.primary.withValues(alpha: 0.6),
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );

    if (onClick == null) {
      return SizedBox(width: double.infinity, child: row);
    }
    return MessagePressShieldRegion(
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onClick,
          child: SizedBox(width: double.infinity, child: row),
        ),
      ),
    );
  }
}

String _toCanvasSingleLineText(String value) {
  final normalized = value.replaceAll('\r', ' ').replaceAll('\n', ' ').trim();
  if (normalized.length <= 160) {
    return normalized;
  }
  return '${normalized.substring(0, 160)}...';
}
