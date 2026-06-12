// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../theme/OperitGlassSurface.dart';
import '../../../../theme/OperitTheme.dart';
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

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final useTermuxTabs =
        defaultTargetPlatform == TargetPlatform.android &&
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
    final transparentSurface = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.transparentSurfaceEnabled;
    return OperitGlassSurface(
      color: transparentSurface
          ? theme.colorScheme.surface.withValues(alpha: 0.04)
          : theme.colorScheme.surface,
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.03,
      borderRadius: BorderRadius.zero,
      clip: false,
      child: SizedBox(
        height: 42,
        child: Stack(
          children: <Widget>[
            PositionedDirectional(
              start: 0,
              end: 0,
              bottom: 0,
              child: Divider(
                height: 1,
                thickness: 1,
                color: theme.colorScheme.outlineVariant.withValues(alpha: 0.45),
              ),
            ),
            ListView.separated(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsetsDirectional.fromSTEB(10, 5, 10, 0),
              itemCount: tabs.length,
              separatorBuilder: (context, index) => const SizedBox(width: 4),
              itemBuilder: (context, index) {
                final tab = tabs[index];
                final selected = index == selectedIndex;
                return _WorkspaceTabButton(
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
              },
            ),
          ],
        ),
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
              return _TermuxWorkspaceTabButton(
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
            },
          ),
        ),
      ),
    );
  }
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
    final backgroundColor = selected
        ? OperitTheme.of(
                context,
              ).themePreferenceSnapshot.transparentSurfaceEnabled
              ? theme.colorScheme.surface.withValues(alpha: 0.18)
              : theme.colorScheme.surface
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
          height: 36,
          padding: const EdgeInsetsDirectional.only(start: 10, end: 6),
          decoration: selected
              ? BoxDecoration(
                  color: backgroundColor,
                  borderRadius: const BorderRadius.vertical(
                    top: Radius.circular(8),
                  ),
                  border: Border(
                    top: BorderSide(
                      color: theme.colorScheme.outlineVariant.withValues(
                        alpha: 0.45,
                      ),
                    ),
                    left: BorderSide(
                      color: theme.colorScheme.outlineVariant.withValues(
                        alpha: 0.45,
                      ),
                    ),
                    right: BorderSide(
                      color: theme.colorScheme.outlineVariant.withValues(
                        alpha: 0.45,
                      ),
                    ),
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
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelMedium?.copyWith(
                    color: contentColor,
                    fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
                  ),
                ),
              ),
              if (onClose == null)
                const SizedBox(width: 8)
              else ...<Widget>[
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

String _tabTitle(AppLocalizations l10n, WorkspaceTab tab) {
  if (tab.title.isNotEmpty) {
    return tab.title;
  }
  switch (tab.kind) {
    case WorkspaceTabKind.home:
      return l10n.home;
    case WorkspaceTabKind.setup:
      return l10n.workspaceSetupTitle;
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
  }
}
