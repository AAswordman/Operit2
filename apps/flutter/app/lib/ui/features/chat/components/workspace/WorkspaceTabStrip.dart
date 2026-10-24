// ignore_for_file: file_names

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../theme/OperitGlassSurface.dart';
import '../../../../theme/OperitTheme.dart';
import '../../../../main/layout/SidebarDockController.dart';
import 'WorkspaceLayoutMetrics.dart';
import 'WorkspaceTabModels.dart';

/// Caps a single tab's width so long titles ellipsize instead of pushing the
/// following tabs and the close button outside the strip's viewport.
const double _workspaceTabMaxWidth = 220;

class WorkspaceTabStrip extends StatefulWidget {
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
  State<WorkspaceTabStrip> createState() => _WorkspaceTabStripState();
}

class _WorkspaceTabStripState extends State<WorkspaceTabStrip> {
  final ScrollController _scrollController = ScrollController();
  final Map<int, GlobalKey> _tabKeys = <int, GlobalKey>{};

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _revealTab(widget.selectedIndex);
    });
  }

  @override
  void didUpdateWidget(WorkspaceTabStrip oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selectedIndex != widget.selectedIndex ||
        oldWidget.tabs.length != widget.tabs.length) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _revealTab(widget.selectedIndex);
      });
    }
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  /// Scrolls just far enough to bring the selected tab on screen, so its
  /// title and close button stay reachable when the strip overflows.
  void _revealTab(int index) {
    if (!mounted || !_scrollController.hasClients) {
      return;
    }
    final RenderObject? renderObject = _tabKeys[index]?.currentContext
        ?.findRenderObject();
    if (renderObject is! RenderBox) {
      return;
    }
    final RenderAbstractViewport? viewport = RenderAbstractViewport.maybeOf(
      renderObject,
    );
    if (viewport == null) {
      return;
    }
    final ScrollPosition position = _scrollController.position;
    final double startOffset = viewport
        .getOffsetToReveal(renderObject, 0.0)
        .offset
        .clamp(position.minScrollExtent, position.maxScrollExtent)
        .toDouble();
    final double endOffset = viewport
        .getOffsetToReveal(renderObject, 1.0)
        .offset
        .clamp(position.minScrollExtent, position.maxScrollExtent)
        .toDouble();
    final double current = position.pixels;
    double target = current;
    if (startOffset < current) {
      target = startOffset;
    } else if (endOffset > current) {
      target = endOffset;
    }
    if ((target - current).abs() < 0.5) {
      return;
    }
    _scrollController.animateTo(
      target,
      duration: const Duration(milliseconds: 160),
      curve: Curves.easeOutCubic,
    );
  }

  GlobalKey _tabKeyFor(int index) => _tabKeys.putIfAbsent(index, GlobalKey.new);

  @override
  Widget build(BuildContext context) {
    _tabKeys.removeWhere((index, _) => index >= widget.tabs.length);
    final l10n = AppLocalizations.of(context)!;
    final useTermuxTabs =
        MediaQuery.sizeOf(context).width < workspaceTabletBreakpoint;
    return useTermuxTabs
        ? _buildTermuxStrip(context)
        : _buildDesktopStrip(context, l10n);
  }

  Widget _buildDesktopStrip(BuildContext context, AppLocalizations l10n) {
    final theme = Theme.of(context);
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
            _scrollableStrip(
              padding: const EdgeInsetsDirectional.fromSTEB(10, 5, 10, 0),
              separatorExtent: 4,
              buildTab: (context, tab, index) {
                final button = _WorkspaceTabButton(
                  tab: tab,
                  title: _tabTitle(l10n, tab),
                  selected: index == widget.selectedIndex,
                  onTap: () {
                    widget.onSelected(index);
                  },
                  onClose: tab.closable
                      ? () {
                          widget.onClosed(index);
                        }
                      : null,
                );
                return _buildTabDragSource(context, tab, button);
              },
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildTermuxStrip(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
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
          child: _scrollableStrip(
            padding: const EdgeInsetsDirectional.fromSTEB(8, 5, 8, 5),
            separatorExtent: 6,
            buildTab: (context, tab, index) {
              final button = _TermuxWorkspaceTabButton(
                title: _tabTitle(l10n, tab),
                selected: index == widget.selectedIndex,
                closable: tab.closable,
                backgroundColor: Colors.transparent,
                selectedBackgroundColor: selectedBackgroundColor,
                borderColor: tabBorderColor,
                primaryTextColor: colorScheme.onSurface,
                secondaryTextColor: colorScheme.onSurfaceVariant,
                onTap: () {
                  widget.onSelected(index);
                },
                onClose: () {
                  widget.onClosed(index);
                },
              );
              return _buildTabDragSource(context, tab, button);
            },
          ),
        ),
      ),
    );
  }

  /// Horizontal strip with mouse dragging and a scrollbar so tabs pushed past
  /// the viewport stay reachable; all tabs stay mounted for [_revealTab].
  Widget _scrollableStrip({
    required EdgeInsetsDirectional padding,
    required double separatorExtent,
    required Widget Function(BuildContext context, WorkspaceTab tab, int index)
    buildTab,
  }) {
    final children = <Widget>[];
    for (var index = 0; index < widget.tabs.length; index++) {
      if (children.isNotEmpty) {
        children.add(SizedBox(width: separatorExtent));
      }
      final tab = widget.tabs[index];
      children.add(
        KeyedSubtree(
          key: _tabKeyFor(index),
          child: buildTab(context, tab, index),
        ),
      );
    }
    // SingleChildScrollView shrink-wraps under loose constraints, which the
    // hosting Column centers; force the viewport to span the strip's width.
    return SizedBox(
      width: double.infinity,
      child: ScrollConfiguration(
        behavior: ScrollConfiguration.of(context).copyWith(
          dragDevices: const <PointerDeviceKind>{
            PointerDeviceKind.touch,
            PointerDeviceKind.mouse,
            PointerDeviceKind.trackpad,
            PointerDeviceKind.stylus,
          },
        ),
        child: Scrollbar(
          controller: _scrollController,
          thumbVisibility: false,
          thickness: 6,
          radius: const Radius.circular(999),
          child: SingleChildScrollView(
            controller: _scrollController,
            scrollDirection: Axis.horizontal,
            padding: padding,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: children,
            ),
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
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: _workspaceTabMaxWidth),
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
