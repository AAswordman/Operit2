// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'WorkspaceTabModels.dart';

class WorkspaceTabStrip extends StatelessWidget {
  const WorkspaceTabStrip({
    super.key,
    required this.tabs,
    required this.selectedIndex,
    required this.onSelected,
    required this.onClosed,
  });

  final List<WorkspaceTab> tabs;
  final int selectedIndex;
  final ValueChanged<int> onSelected;
  final ValueChanged<int> onClosed;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ColoredBox(
      color: theme.colorScheme.surfaceContainerLow,
      child: SizedBox(
        height: 44,
        child: Stack(
          children: <Widget>[
            PositionedDirectional(
              start: 0,
              end: 0,
              bottom: 0,
              child: Divider(
                height: 1,
                thickness: 1,
                color: theme.colorScheme.outlineVariant,
              ),
            ),
            ListView.separated(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsetsDirectional.fromSTEB(8, 6, 8, 0),
              itemCount: tabs.length,
              separatorBuilder: (context, index) => const SizedBox(width: 4),
              itemBuilder: (context, index) {
                final tab = tabs[index];
                final selected = index == selectedIndex;
                return _WorkspaceTabButton(
                  tab: tab,
                  selected: selected,
                  onTap: () {
                    onSelected(index);
                  },
                  onClose: tab.closable
                      ? () {
                          onClosed(index);
                        }
                      : null,
                );
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceTabButton extends StatelessWidget {
  const _WorkspaceTabButton({
    required this.tab,
    required this.selected,
    required this.onTap,
    required this.onClose,
  });

  final WorkspaceTab tab;
  final bool selected;
  final VoidCallback onTap;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final backgroundColor = selected
        ? theme.colorScheme.surface
        : Colors.transparent;
    final contentColor = selected
        ? theme.colorScheme.onSurface
        : theme.colorScheme.onSurfaceVariant;
    return Material(
      color: backgroundColor,
      borderRadius: const BorderRadius.vertical(top: Radius.circular(8)),
      child: InkWell(
        borderRadius: const BorderRadius.vertical(top: Radius.circular(8)),
        onTap: onTap,
        child: Container(
          height: 38,
          padding: const EdgeInsetsDirectional.only(start: 10, end: 6),
          decoration: selected
              ? BoxDecoration(
                  color: backgroundColor,
                  borderRadius: const BorderRadius.vertical(
                    top: Radius.circular(8),
                  ),
                  border: Border(
                    top: BorderSide(color: theme.colorScheme.outlineVariant),
                    left: BorderSide(color: theme.colorScheme.outlineVariant),
                    right: BorderSide(color: theme.colorScheme.outlineVariant),
                  ),
                )
              : null,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(tab.icon, size: 17, color: contentColor),
              const SizedBox(width: 6),
              Flexible(
                child: Text(
                  tab.title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelMedium?.copyWith(
                    color: contentColor,
                    fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
                  ),
                ),
              ),
              if (onClose != null) ...<Widget>[
                const SizedBox(width: 4),
                SizedBox(
                  width: 22,
                  height: 22,
                  child: IconButton(
                    padding: EdgeInsets.zero,
                    iconSize: 15,
                    tooltip: MaterialLocalizations.of(
                      context,
                    ).closeButtonTooltip,
                    onPressed: onClose,
                    icon: Icon(Icons.close, color: contentColor),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
