// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

@immutable
class SidebarMenuAction<T> {
  const SidebarMenuAction({
    required this.value,
    required this.icon,
    required this.label,
    this.dividerBefore = false,
    this.danger = false,
    this.enabled = true,
  });

  final T value;
  final IconData icon;
  final String label;
  final bool dividerBefore;
  final bool danger;
  final bool enabled;
}

/// Shares the conversation row's menu presentation with sidebar directory rows.
class SidebarMoreMenuButton<T> extends StatelessWidget {
  const SidebarMoreMenuButton({
    super.key,
    required this.actions,
    required this.onSelected,
    required this.color,
    this.tooltip = '更多操作',
    this.onMenuOpenChanged,
    this.compact = false,
  });

  final List<SidebarMenuAction<T>> actions;
  final ValueChanged<T> onSelected;
  final Color color;
  final String tooltip;
  final ValueChanged<bool>? onMenuOpenChanged;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final side = compact ? 20.0 : 24.0;
    return SizedBox(
      width: side,
      height: side,
      child: PopupMenuButton<T>(
        tooltip: tooltip,
        padding: EdgeInsets.zero,
        borderRadius: BorderRadius.circular(6),
        color: Color.alphaBlend(
          colorScheme.surfaceContainerHighest.withValues(alpha: 0.75),
          colorScheme.surface,
        ),
        elevation: 6,
        shadowColor: Colors.black.withValues(alpha: 0.45),
        offset: const Offset(0, 4),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: BorderSide(
            color: colorScheme.outlineVariant.withValues(alpha: 0.4),
          ),
        ),
        constraints: const BoxConstraints(minWidth: 118, maxWidth: 156),
        menuPadding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
        onOpened: () => onMenuOpenChanged?.call(true),
        onCanceled: () => onMenuOpenChanged?.call(false),
        onSelected: (action) {
          onMenuOpenChanged?.call(false);
          onSelected(action);
        },
        child: ThemeHistoryMoreGlyph(
          color: color,
          side: side,
          compact: compact,
        ),
        itemBuilder: (context) => [
          for (final action in actions) ...<PopupMenuEntry<T>>[
            if (action.dividerBefore) const PopupMenuDivider(height: 8),
            PopupMenuItem<T>(
              value: action.value,
              enabled: action.enabled,
              height: 32,
              padding: const EdgeInsets.symmetric(horizontal: 10),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(
                    action.icon,
                    size: 14.5,
                    color: !action.enabled
                        ? colorScheme.onSurface.withValues(alpha: 0.38)
                        : action.danger
                        ? colorScheme.error.withValues(alpha: 0.90)
                        : colorScheme.onSurfaceVariant.withValues(alpha: 0.85),
                  ),
                  const SizedBox(width: 9),
                  Flexible(
                    child: Text(
                      action.label,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12.5,
                        fontWeight: FontWeight.w500,
                        color: !action.enabled
                            ? colorScheme.onSurface.withValues(alpha: 0.38)
                            : action.danger
                            ? colorScheme.error.withValues(alpha: 0.90)
                            : colorScheme.onSurface.withValues(alpha: 0.92),
                        letterSpacing: -0.1,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}
