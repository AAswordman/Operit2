// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../theme/OperitGlassSurface.dart';
import '../../../../theme/OperitTheme.dart';
import '../../../../main/layout/SidebarDockController.dart';
import 'WorkspaceLayoutMetrics.dart';
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

  /// Builds a horizontally scrollable strip sized for the available width.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final useTermuxTabs =
        MediaQuery.sizeOf(context).width < workspaceTabletBreakpoint;
    if (useTermuxTabs) {
      return _TermuxWorkspaceTabStrip(
        tabs: tabs,
        selectedIndex: selectedIndex,
        onSelected: onSelected,
        onClosed: onClosed,
        tabTitle: (tab) => _tabTitle(l10n, tab),
      );
    }
    final isDark = theme.brightness == Brightness.dark;
    final dividerColor = isDark
        ? const Color(0xFF262626)
        : theme.colorScheme.outlineVariant.withValues(alpha: 0.35);

    return SizedBox(
      height: 38,
      child: Stack(
        children: <Widget>[
          PositionedDirectional(
            start: 0,
            end: 0,
            bottom: 0,
            child: Divider(height: 1, thickness: 1, color: dividerColor),
          ),
          Center(
            child: SizedBox(
              height: 28,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                padding: const EdgeInsetsDirectional.symmetric(horizontal: 10),
                itemCount: tabs.length,
                separatorBuilder: (context, index) => const SizedBox(width: 4),
                itemBuilder: (context, index) {
                  final tab = tabs[index];
                  final selected = index == selectedIndex;
                  final button = _WorkspaceTabButton(
                    tab: tab,
                    title: _tabTitle(l10n, tab),
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
                  return _buildTabDragSource(context, tab, button);
                },
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _TermuxWorkspaceTabStrip extends StatelessWidget {
  const _TermuxWorkspaceTabStrip({
    required this.tabs,
    required this.selectedIndex,
    required this.onSelected,
    required this.onClosed,
    required this.tabTitle,
  });

  final List<WorkspaceTab> tabs;
  final int selectedIndex;
  final ValueChanged<int> onSelected;
  final ValueChanged<int> onClosed;
  final String Function(WorkspaceTab tab) tabTitle;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final transparentSurface = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.transparentSurfaceEnabled;
    final backgroundColor = transparentSurface
        ? colorScheme.surface.withValues(alpha: 0.04)
        : colorScheme.surface;
    final selectedBackgroundColor = transparentSurface
        ? colorScheme.surfaceContainerHighest.withValues(alpha: 0.62)
        : colorScheme.surfaceContainerHighest;
    final tabBorderColor = transparentSurface
        ? colorScheme.outlineVariant.withValues(alpha: 0.34)
        : colorScheme.outlineVariant.withValues(alpha: 0.72);
    return OperitGlassSurface(
      color: backgroundColor,
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.03,
      borderRadius: BorderRadius.zero,
      clip: false,
      child: SizedBox(
        height: 44,
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(
                color: colorScheme.outlineVariant.withValues(alpha: 0.45),
              ),
            ),
          ),
          child: ListView.separated(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsetsDirectional.fromSTEB(8, 5, 8, 5),
            itemCount: tabs.length,
            separatorBuilder: (context, index) => const SizedBox(width: 6),
            itemBuilder: (context, index) {
              final tab = tabs[index];
              final selected = index == selectedIndex;
              final button = _TermuxWorkspaceTabButton(
                title: tabTitle(tab),
                selected: selected,
                closable: tab.closable,
                backgroundColor: Colors.transparent,
                selectedBackgroundColor: selectedBackgroundColor,
                borderColor: tabBorderColor,
                primaryTextColor: colorScheme.onSurface,
                secondaryTextColor: colorScheme.onSurfaceVariant,
                onTap: () {
                  onSelected(index);
                },
                onClose: () {
                  onClosed(index);
                },
              );
              return _buildTabDragSource(context, tab, button);
            },
          ),
        ),
      ),
    );
  }
}

/// Starts tab docking after a long press so swipes remain scroll gestures.
Widget _buildTabDragSource(
  BuildContext context,
  WorkspaceTab tab,
  Widget child,
) {
  if (tab.kind == WorkspaceTabKind.home) {
    return child;
  }
  if (tab.kind == WorkspaceTabKind.plugin && tab.pluginEntryId != null) {
    return LongPressDraggable<SidebarDockDragPayload>(
      maxSimultaneousDrags: 1,
      data: SidebarDockDragPayload(entryId: tab.pluginEntryId!),
      feedback: Material(
        elevation: 8,
        child: Opacity(opacity: 0.92, child: child),
      ),
      childWhenDragging: Opacity(opacity: 0.32, child: child),
      child: child,
    );
  }
  return LongPressDraggable<WorkspaceTabDragPayload>(
    maxSimultaneousDrags: 1,
    data: WorkspaceTabDragPayload(tab: tab),
    feedback: Material(
      elevation: 8,
      child: Opacity(opacity: 0.92, child: child),
    ),
    childWhenDragging: Opacity(opacity: 0.32, child: child),
    child: child,
  );
}

class _TermuxWorkspaceTabButton extends StatelessWidget {
  const _TermuxWorkspaceTabButton({
    required this.title,
    required this.selected,
    required this.closable,
    required this.backgroundColor,
    required this.selectedBackgroundColor,
    required this.borderColor,
    required this.primaryTextColor,
    required this.secondaryTextColor,
    required this.onTap,
    required this.onClose,
  });

  final String title;
  final bool selected;
  final bool closable;
  final Color backgroundColor;
  final Color selectedBackgroundColor;
  final Color borderColor;
  final Color primaryTextColor;
  final Color secondaryTextColor;
  final VoidCallback onTap;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final textColor = selected ? primaryTextColor : secondaryTextColor;
    return Material(
      color: selected ? selectedBackgroundColor : backgroundColor,
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: ConstrainedBox(
          constraints: const BoxConstraints(
            minWidth: 74,
            maxWidth: 156,
            minHeight: 34,
          ),
          child: DecoratedBox(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: borderColor),
            ),
            child: Padding(
              padding: EdgeInsetsDirectional.only(
                start: 12,
                end: closable ? 4 : 12,
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Flexible(
                    child: Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodyMedium!.copyWith(
                        height: 1.0,
                        letterSpacing: 0,
                        fontWeight: FontWeight.w600,
                        color: textColor,
                      ),
                    ),
                  ),
                  if (closable) ...<Widget>[
                    const SizedBox(width: 4),
                    SizedBox.square(
                      dimension: 28,
                      child: IconButton(
                        padding: EdgeInsets.zero,
                        iconSize: 17,
                        tooltip: MaterialLocalizations.of(
                          context,
                        ).closeButtonTooltip,
                        onPressed: onClose,
                        icon: Icon(Icons.close, color: textColor),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _WorkspaceTabButton extends StatelessWidget {
  const _WorkspaceTabButton({
    required this.tab,
    required this.title,
    required this.selected,
    required this.onTap,
    required this.onClose,
  });

  final WorkspaceTab tab;
  final String title;
  final bool selected;
  final VoidCallback onTap;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;
    final backgroundColor = selected
        ? (isDark ? const Color(0xFF282828) : const Color(0xFFE8E8E8))
        : Colors.transparent;
    final contentColor = selected
        ? theme.colorScheme.onSurface
        : theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.75);
    final borderColor = selected
        ? (isDark ? const Color(0xFF383838) : theme.colorScheme.outlineVariant)
        : Colors.transparent;

    return Material(
      color: backgroundColor,
      borderRadius: BorderRadius.circular(5),
      child: InkWell(
        borderRadius: BorderRadius.circular(5),
        onTap: onTap,
        child: Container(
          height: 26,
          padding: EdgeInsetsDirectional.only(
            start: 8,
            end: onClose == null ? 8 : 4,
          ),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(5),
            border: Border.all(color: borderColor, width: 0.8),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(tab.icon, size: 14, color: contentColor),
              const SizedBox(width: 5),
              Flexible(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelMedium?.copyWith(
                    color: contentColor,
                    fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
                    fontSize: 11.5,
                  ),
                ),
              ),
              if (onClose == null)
                const SizedBox(width: 2)
              else ...<Widget>[
                const SizedBox(width: 3),
                SizedBox.square(
                  dimension: 18,
                  child: IconButton(
                    padding: EdgeInsets.zero,
                    iconSize: 12,
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

String _tabTitle(AppLocalizations l10n, WorkspaceTab tab) {
  if (tab.title.isNotEmpty) {
    return tab.title;
  }
  switch (tab.kind) {
    case WorkspaceTabKind.home:
      return l10n.home;
    case WorkspaceTabKind.workspacePicker:
      return l10n.workspaceBindExistingTitle;
    case WorkspaceTabKind.files:
      return l10n.files;
    case WorkspaceTabKind.terminal:
      return l10n.terminal;
    case WorkspaceTabKind.browser:
      return l10n.browser;
    case WorkspaceTabKind.webVisit:
      return 'visit_web';
    case WorkspaceTabKind.filePreview:
      return l10n.filePreview;
    case WorkspaceTabKind.plugin:
      return '插件';
  }
}
