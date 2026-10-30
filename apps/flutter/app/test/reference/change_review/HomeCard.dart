import 'package:flutter/material.dart';

// Frozen pre-migration widget used only for pixel comparison.
class ChangeReviewHomeReference extends StatelessWidget {
  const ChangeReviewHomeReference({required this.onOpenDiff});

  final VoidCallback onOpenDiff;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final accentColor = colorScheme.primary;

    return Card.outlined(
      margin: EdgeInsets.zero,
      color: colorScheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(10),
        side: BorderSide(color: colorScheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: <Widget>[
            Container(
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: accentColor.withValues(alpha: 0.15),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                  color: accentColor.withValues(alpha: 0.3),
                ),
              ),
              child: Icon(
                Icons.difference_outlined,
                size: 20,
                color: accentColor,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    '代码变更审查',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    '查看工作区代码改动、行级对比与文件审查',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant.withValues(alpha: 0.8),
                      fontSize: 11.5,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            FilledButton.tonal(
              onPressed: onOpenDiff,
              style: FilledButton.styleFrom(
                backgroundColor: colorScheme.surfaceContainerHigh,
                foregroundColor: colorScheme.onSurface,
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(7),
                  side: BorderSide(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.4),
                  ),
                ),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    '查看变更',
                    style: TextStyle(
                      fontWeight: FontWeight.w600,
                      fontSize: 12,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  const SizedBox(width: 4),
                  Icon(
                    Icons.arrow_forward_rounded,
                    size: 14,
                    color: colorScheme.onSurfaceVariant,
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

