// ignore_for_file: file_names

import 'package:flutter/material.dart';

class ChatMultiSelectBar extends StatelessWidget {
  const ChatMultiSelectBar({
    super.key,
    required this.selectedCount,
    required this.allSelected,
    required this.onClose,
    required this.onToggleSelectAll,
    required this.onCopy,
    required this.onShareImage,
    required this.onDelete,
  });

  final int selectedCount;
  final bool allSelected;
  final VoidCallback onClose;
  final VoidCallback onToggleSelectAll;
  final VoidCallback? onCopy;
  final VoidCallback? onShareImage;
  final VoidCallback? onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return SafeArea(
      top: false,
      minimum: const EdgeInsets.fromLTRB(12, 6, 12, 12),
      child: Align(
        alignment: Alignment.bottomCenter,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: Material(
            color: colorScheme.surface.withValues(alpha: 0.98),
            elevation: 8,
            shadowColor: Colors.black.withValues(alpha: 0.16),
            borderRadius: BorderRadius.circular(14),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
              child: Row(
                children: <Widget>[
                  IconButton(
                    onPressed: onClose,
                    icon: const Icon(Icons.close),
                    tooltip: '退出多选',
                    visualDensity: VisualDensity.compact,
                  ),
                  Expanded(
                    child: Text(
                      selectedCount == 0 ? '多选' : '已选 $selectedCount 条',
                      style: theme.textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  IconButton(
                    onPressed: onToggleSelectAll,
                    icon: Icon(allSelected ? Icons.deselect : Icons.select_all),
                    tooltip: allSelected ? '清空选择' : '全选',
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton(
                    onPressed: onCopy,
                    icon: const Icon(Icons.content_copy),
                    tooltip: '复制所选',
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton.filledTonal(
                    onPressed: onShareImage,
                    icon: const Icon(Icons.ios_share),
                    tooltip: '生成长图',
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton.filledTonal(
                    onPressed: onDelete,
                    icon: const Icon(Icons.delete_outline),
                    tooltip: '删除所选',
                    color: colorScheme.onErrorContainer,
                    style: IconButton.styleFrom(
                      backgroundColor: colorScheme.errorContainer,
                    ),
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
