// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../common/OperitLogoMark.dart';
import '../../features/chat/components/NewChatIntro.dart';
import '../navigation/AppNavigationModels.dart';
import '../layout/SidebarDockController.dart';
import '../layout/NavigationLayoutMetrics.dart';
import '../screens/ScreenRouteRegistry.dart';
import '../../common/components/SwipeActions.dart';
import 'NavigationDrawerAppearance.dart';

class CollapsedDrawerContent extends StatelessWidget {
  /// Receives the real nullable chat source without decoding plugin configuration.
  const CollapsedDrawerContent({
    super.key,
    required this.navigationEntries,
    required this.pluginEntries,
    required this.selectedRouteId,
    required this.appearance,
    required this.onNavigationEntrySelected,
    required this.onConversationActivated,
    required this.currentChatId,
    this.bridge = const ProxyCoreRuntimeBridge(),
  });

  final List<NavigationEntrySpec> navigationEntries;
  final List<NavigationEntrySpec> pluginEntries;
  final String selectedRouteId;
  final NavigationDrawerAppearance appearance;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;
  final VoidCallback onConversationActivated;
  final String? currentChatId;
  final OperitRuntimeBridge bridge;
  static const double _topBarHeight = 64;
  static const EdgeInsets _collapsedItemPadding = EdgeInsets.symmetric(
    vertical: 2,
  );

  /// Creates through the generic hook without reading any role or grouping preference.
  Future<void> _createConversation(BuildContext context) async {
    final sourceChatId = currentChatId;
    newChatIntroArmed.value = true;
    try {
      await GeneratedCoreProxyClients(
        bridge,
      ).chatRuntimeHolderMain.createNewChat(
        setAsCurrentChat: true,
        sourceChatId: sourceChatId,
        parentChatId: null,
        input: null,
      );
      onConversationActivated();
    } catch (error) {
      newChatIntroArmed.value = false;
      if (context.mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    }
  }

  void _openPackageManager() {
    for (final entry in navigationEntries) {
      if (entry.entryId == 'main.package_manager') {
        onNavigationEntrySelected(entry);
        return;
      }
    }
    throw StateError('Unknown navigation entry: main.package_manager');
  }

  void _openSettings() {
    for (final entry in navigationEntries) {
      if (entry.entryId == 'main.settings') {
        onNavigationEntrySelected(entry);
        return;
      }
    }
    throw StateError('Unknown navigation entry: main.settings');
  }

  @override
  Widget build(BuildContext context) {
    final topPadding = MediaQuery.paddingOf(context).top;
    final bottomPadding = MediaQuery.paddingOf(context).bottom;
    return Column(
      children: <Widget>[
        Expanded(
          child: ListView(
            padding: EdgeInsets.zero,
            children: <Widget>[
              SizedBox(
                height: topPadding + _topBarHeight,
                child: Padding(
                  padding: EdgeInsets.only(top: topPadding),
                  child: Center(
                    child: OperitLogoMark(
                      size: 34,
                      color: appearance.statusAvailableColor,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 8),
              Padding(
                padding: _collapsedItemPadding,
                child: Center(
                  child: _RoundDrawerButton(
                    selected:
                        selectedRouteId == navigationEntries.first.routeId,
                    appearance: appearance,
                    icon: Icons.chat_bubble_outline,
                    onClick: onConversationActivated,
                  ),
                ),
              ),
              Padding(
                padding: _collapsedItemPadding,
                child: Center(
                  child: _RoundDrawerButton(
                    selected: false,
                    appearance: appearance,
                    icon: Icons.add_comment_outlined,
                    onClick: () => unawaited(_createConversation(context)),
                  ),
                ),
              ),
              if (pluginEntries.isNotEmpty) ...<Widget>[
                Divider(
                  height: 12,
                  indent: 14,
                  endIndent: 14,
                  color: appearance.dividerColor,
                ),
                for (var index = 0; index < pluginEntries.length; index++)
                  Padding(
                    padding: _collapsedItemPadding,
                    child: Center(
                      child: _DockedPluginRoundButton(
                        entry: pluginEntries[index],
                        insertionIndex: index,
                        selected:
                            selectedRouteId == pluginEntries[index].routeId,
                        appearance: appearance,
                        onClick: () =>
                            onNavigationEntrySelected(pluginEntries[index]),
                      ),
                    ),
                  ),
                if (pluginEntries.isNotEmpty)
                  SidebarDockEndDropTarget(
                    controller: SidebarDockScope.maybeOf(context),
                    location: SidebarDockLocation.primary,
                    height: 18,
                  ),
              ],
            ],
          ),
        ),
        Padding(
          padding: EdgeInsets.only(bottom: bottomPadding + 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Padding(
                padding: _collapsedItemPadding,
                child: Center(
                  child: _RoundDrawerButton(
                    selected: selectedRouteId == _packageManagerRouteId,
                    appearance: appearance,
                    icon: Icons.inventory_2_outlined,
                    onClick: _openPackageManager,
                  ),
                ),
              ),
              Padding(
                padding: _collapsedItemPadding,
                child: Center(
                  child: _RoundDrawerButton(
                    selected: selectedRouteId == _settingsRouteId,
                    appearance: appearance,
                    icon: Icons.settings_outlined,
                    onClick: _openSettings,
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  String get _packageManagerRouteId {
    for (final entry in navigationEntries) {
      if (entry.entryId == 'main.package_manager') {
        return entry.routeId;
      }
    }
    throw StateError('Unknown navigation entry: main.package_manager');
  }

  String get _settingsRouteId {
    return ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.settings);
  }
}

class SidebarInfoCard extends StatelessWidget {
  const SidebarInfoCard({
    super.key,
    required this.brandName,
    required this.appearance,
    this.trailing,
  });

  final String brandName;
  final NavigationDrawerAppearance appearance;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return ThemeSidebarInfoCard(
      brandName: brandName,
      appearance: appearance,
      trailing: trailing,
    );
  }
}

class ConversationSearchField extends StatelessWidget {
  const ConversationSearchField({
    super.key,
    required this.controller,
    required this.appearance,
  });

  final TextEditingController controller;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(14);
    return TextField(
      controller: controller,
      minLines: 1,
      maxLines: 1,
      style: Theme.of(
        context,
      ).textTheme.bodyMedium?.copyWith(color: appearance.titleColor),
      decoration: InputDecoration(
        isDense: true,
        hintText: '搜索对话',
        hintStyle: Theme.of(context).textTheme.bodyMedium?.copyWith(
          color: appearance.itemColor.withValues(alpha: 0.62),
        ),
        prefixIcon: Icon(
          Icons.search,
          size: 20,
          color: appearance.itemColor.withValues(alpha: 0.72),
        ),
        filled: true,
        fillColor: appearance.buttonContainerColor,
        border: OutlineInputBorder(
          borderRadius: shape,
          borderSide: BorderSide.none,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: shape,
          borderSide: BorderSide.none,
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: shape,
          borderSide: BorderSide(color: appearance.statusAvailableColor),
        ),
        contentPadding: const EdgeInsets.symmetric(
          horizontal: 12,
          vertical: 10,
        ),
      ),
    );
  }
}

class HistoryRail extends StatelessWidget {
  /// Creates the nested history rail.
  const HistoryRail({
    super.key,
    required this.height,
    required this.appearance,
    this.width = 20,
    this.thickness = 1,
    this.selected = false,
    this.hovered = false,
  });

  final double height;
  final NavigationDrawerAppearance appearance;
  final double width;
  final double thickness;
  final bool selected;
  final bool hovered;

  /// Builds the vertical rail for nested history rows.
  @override
  Widget build(BuildContext context) {
    return ThemeHistoryRail(
      height: height,
      appearance: appearance,
      width: width,
      thickness: thickness,
      selected: selected,
      hovered: hovered,
    );
  }
}

enum _ConversationQuickAction { rename, togglePinned, toggleLocked, delete }

class ConversationDrawerItem extends StatefulWidget {
  /// Creates a conversation row with separate menu and confirmed swipe deletions.
  const ConversationDrawerItem({
    super.key,
    required this.history,
    required this.title,
    required this.selected,
    required this.isRunning,
    required this.appearance,
    required this.onClick,
    required this.onRename,
    required this.onTogglePinned,
    required this.onToggleLocked,
    required this.onDelete,
    required this.onSwipeDelete,
    required this.onLongPress,
    required this.onMoveTo,
    required this.canAcceptDrop,
    required this.canDetach,
    required this.onDetach,
    this.nested = false,
    this.workspaceStyle = false,
    this.childrenToggle,
  });

  final core_proxy.ChatHistoryListItem history;
  final String title;
  final bool selected;
  final bool isRunning;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;
  final VoidCallback onRename;
  final VoidCallback onTogglePinned;
  final VoidCallback onToggleLocked;
  final VoidCallback onDelete;

  /// Requests confirmation for a conversation deletion initiated by a swipe.
  final VoidCallback onSwipeDelete;
  final VoidCallback onLongPress;
  final ValueChanged<core_proxy.ChatHistoryListItem> onMoveTo;
  final bool Function(core_proxy.ChatHistoryListItem) canAcceptDrop;
  final bool canDetach;
  final VoidCallback onDetach;
  final bool nested;
  final bool workspaceStyle;

  /// Expands native child conversations independently of activation and execution.
  final Widget? childrenToggle;

  @override
  State<ConversationDrawerItem> createState() => _ConversationDrawerItemState();
}

class _ConversationDrawerItemState extends State<ConversationDrawerItem>
    with SingleTickerProviderStateMixin {
  late final AnimationController _marqueeController;

  @override
  void initState() {
    super.initState();
    _marqueeController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 900),
    );
    if (widget.isRunning) {
      _marqueeController.repeat();
    }
  }

  @override
  void didUpdateWidget(covariant ConversationDrawerItem oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.isRunning != oldWidget.isRunning) {
      if (widget.isRunning) {
        _marqueeController.repeat();
      } else {
        _marqueeController.stop();
        _marqueeController.reset();
      }
    }
  }

  @override
  void dispose() {
    _marqueeController.dispose();
    super.dispose();
  }

  bool _hovered = false;
  bool _menuOpen = false;

  @override
  Widget build(BuildContext context) {
    final workspaceStyle = widget.workspaceStyle;
    final selected = widget.selected;
    final appearance = widget.appearance;
    final history = widget.history;
    final active = _hovered || _menuOpen;
    final windowSize = MediaQuery.sizeOf(context);
    final platform = Theme.of(context).platform;
    final touchPlatform =
        platform == TargetPlatform.android || platform == TargetPlatform.iOS;
    final showActions =
        active || selected || touchPlatform || widget.childrenToggle != null;

    return DragTarget<core_proxy.ChatHistoryListItem>(
      onWillAcceptWithDetails: (details) =>
          details.data.id != history.id && widget.canAcceptDrop(details.data),
      onAcceptWithDetails: (details) => widget.onMoveTo(details.data),
      builder: (context, candidateData, rejectedData) {
        return MouseRegion(
          onEnter: (_) => setState(() => _hovered = true),
          onExit: (_) => setState(() => _hovered = false),
          child: ThemeConversationRow(
            title: widget.title,
            selected: selected,
            active: active,
            appearance: appearance,
            workspaceStyle: workspaceStyle,
            nested: widget.nested,
            pinned: history.pinned,
            locked: history.locked,
            dragHovering: candidateData.isNotEmpty,
            showActions: showActions,
            onTap: widget.onClick,
            onLongPress: widget.onLongPress,
            surfaceWrapper: (surface) => SwipeActions(
              key: ValueKey<String>('conversation-${history.id}'),
              onStartAction: widget.onRename,
              onEndAction: widget.onSwipeDelete,
              background: _SwipeActionBackground(
                alignment: AlignmentDirectional.centerStart,
                color: Theme.of(context).colorScheme.primary,
                icon: Icons.edit,
                label: '重命名',
              ),
              secondaryBackground: _SwipeActionBackground(
                alignment: AlignmentDirectional.centerEnd,
                color: Theme.of(context).colorScheme.error,
                icon: Icons.delete,
                label: '删除',
              ),
              child: surface,
            ),
            leading: Draggable<core_proxy.ChatHistoryListItem>(
              data: history,
              dragAnchorStrategy: pointerDragAnchorStrategy,
              onDragEnd: (details) {
                if (details.wasAccepted) return;
                final offset = details.offset;
                final outsideWindow =
                    offset.dx < 0 ||
                    offset.dy < 0 ||
                    offset.dx > windowSize.width ||
                    offset.dy > windowSize.height;
                if (widget.canDetach && outsideWindow) widget.onDetach();
              },
              feedback: Material(
                color: Colors.transparent,
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 280),
                  child: _DraggingConversationItem(
                    history: history,
                    title: widget.title,
                    appearance: appearance,
                  ),
                ),
              ),
              childWhenDragging: Opacity(
                opacity: 0.35,
                child: _ConversationStatusHandle(
                  isRunning: widget.isRunning,
                  animation: _marqueeController,
                  selected: selected,
                  hovered: active,
                  appearance: appearance,
                  compact: workspaceStyle,
                ),
              ),
              child: AnimatedOpacity(
                duration: const Duration(milliseconds: 140),
                opacity: widget.isRunning
                    ? 1.0
                    : active
                    ? 0.72
                    : selected
                    ? 0.36
                    : 0.0,
                child: _ConversationStatusHandle(
                  isRunning: widget.isRunning,
                  animation: _marqueeController,
                  selected: selected,
                  hovered: active,
                  appearance: appearance,
                  compact: workspaceStyle,
                ),
              ),
            ),
            action: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (widget.childrenToggle != null) widget.childrenToggle!,
                _ConversationMoreMenuButton(
                  history: history,
                  selected: selected,
                  appearance: appearance,
                  compact: workspaceStyle,
                  onRename: widget.onRename,
                  onTogglePinned: widget.onTogglePinned,
                  onToggleLocked: widget.onToggleLocked,
                  onDelete: widget.onDelete,
                  onMenuOpenChanged: (open) {
                    if (mounted) setState(() => _menuOpen = open);
                  },
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class PluginNavigationDrawerItem extends StatelessWidget {
  const PluginNavigationDrawerItem({
    super.key,
    required this.entry,
    required this.selected,
    required this.appearance,
    required this.onClick,
  });

  final NavigationEntrySpec entry;
  final bool selected;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;

  static const double _endPadding = 12;

  @override
  Widget build(BuildContext context) {
    final dockController =
        MediaQuery.sizeOf(context).width >= navigationTabletBreakpoint
        ? SidebarDockScope.maybeOf(context)
        : null;
    final shape = BorderRadius.circular(12);
    final contentColor = selected
        ? appearance.selectedContentColor
        : appearance.itemColor;
    final item = Padding(
      padding: const EdgeInsetsDirectional.only(
        start: 12,
        end: _endPadding,
        bottom: 3,
      ),
      child: Material(
        color: selected
            ? appearance.selectedContainerColor
            : Colors.transparent,
        borderRadius: shape,
        child: InkWell(
          borderRadius: shape,
          onTap: onClick,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
            child: Row(
              children: <Widget>[
                Container(
                  width: 28,
                  height: 28,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: selected
                        ? appearance.selectedContentColor.withValues(
                            alpha: 0.14,
                          )
                        : appearance.buttonContainerColor,
                  ),
                  alignment: Alignment.center,
                  child: Icon(entry.icon, size: 17, color: contentColor),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    entry.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: contentColor,
                      fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    if (dockController == null) {
      return item;
    }
    return _PrimarySidebarDragSource(entry: entry, child: item);
  }
}

class _PrimarySidebarDragSource extends StatelessWidget {
  const _PrimarySidebarDragSource({required this.entry, required this.child});

  final NavigationEntrySpec entry;
  final Widget child;

  /// Builds a primary sidebar drag source and reorder drop target.
  @override
  Widget build(BuildContext context) {
    final controller =
        MediaQuery.sizeOf(context).width >= navigationTabletBreakpoint
        ? SidebarDockScope.maybeOf(context)
        : null;
    if (controller == null) {
      return child;
    }
    return DragTarget<SidebarDockDragPayload>(
      onWillAcceptWithDetails: (details) {
        return details.data.entryId != entry.entryId &&
            controller.canMove(
              details.data.entryId,
              SidebarDockLocation.primary,
            );
      },
      onAcceptWithDetails: (details) {
        final entries = controller.primaryEntries;
        final index = entries.indexWhere(
          (candidate) => candidate.entryId == entry.entryId,
        );
        controller.move(
          details.data.entryId,
          location: SidebarDockLocation.primary,
          insertionIndex: index < 0 ? entries.length : index,
        );
      },
      builder: (context, candidateData, rejectedData) {
        final decoratedChild = DecoratedBox(
          decoration: candidateData.isEmpty
              ? const BoxDecoration()
              : BoxDecoration(
                  border: BorderDirectional(
                    start: BorderSide(
                      width: 3,
                      color: Theme.of(context).colorScheme.primary,
                    ),
                  ),
                ),
          child: child,
        );
        if (!controller.canMove(entry.entryId, SidebarDockLocation.secondary)) {
          return decoratedChild;
        }
        return Draggable<SidebarDockDragPayload>(
          data: SidebarDockDragPayload(entryId: entry.entryId),
          feedback: Material(
            elevation: 8,
            color: Colors.transparent,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 260),
              child: Opacity(opacity: 0.92, child: child),
            ),
          ),
          childWhenDragging: Opacity(opacity: 0.32, child: child),
          child: decoratedChild,
        );
      },
    );
  }
}

class SidebarStatusText extends StatelessWidget {
  const SidebarStatusText({
    super.key,
    required this.text,
    required this.appearance,
  });

  final String text;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsetsDirectional.fromSTEB(28, 6, 16, 10),
      child: Text(
        text,
        maxLines: 3,
        overflow: TextOverflow.ellipsis,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: appearance.itemColor.withValues(alpha: 0.72),
        ),
      ),
    );
  }
}

class BottomSidebarAction extends StatelessWidget {
  const BottomSidebarAction({
    super.key,
    required this.icon,
    required this.label,
    required this.appearance,
    required this.onClick,
    this.selected = false,
  });

  final IconData icon;
  final String label;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    return ThemeBottomSidebarAction(
      icon: icon,
      label: label,
      appearance: appearance,
      onClick: onClick,
      selected: selected,
    );
  }
}

class _ConversationMoreMenuButton extends StatelessWidget {
  const _ConversationMoreMenuButton({
    required this.history,
    required this.selected,
    required this.appearance,
    required this.onRename,
    required this.onTogglePinned,
    required this.onToggleLocked,
    required this.onDelete,
    this.onMenuOpenChanged,
    this.compact = false,
  });

  final core_proxy.ChatHistoryListItem history;
  final bool selected;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onRename;
  final VoidCallback onTogglePinned;
  final VoidCallback onToggleLocked;
  final VoidCallback onDelete;
  final ValueChanged<bool>? onMenuOpenChanged;
  final bool compact;

  PopupMenuItem<_ConversationQuickAction> _menuItem({
    required _ConversationQuickAction value,
    required IconData icon,
    required String label,
    required Color iconColor,
    required Color textColor,
  }) {
    return PopupMenuItem<_ConversationQuickAction>(
      value: value,
      height: 32,
      padding: const EdgeInsets.symmetric(horizontal: 10),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 14.5, color: iconColor),
          const SizedBox(width: 9),
          Text(
            label,
            style: TextStyle(
              fontSize: 12.5,
              fontWeight: FontWeight.w500,
              color: textColor,
              letterSpacing: -0.1,
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final color =
        (selected ? appearance.selectedContentColor : appearance.itemColor)
            .withValues(alpha: 0.78);
    final side = compact ? 20.0 : 24.0;
    final itemIconColor = colorScheme.onSurfaceVariant.withValues(alpha: 0.85);
    final itemTextColor = colorScheme.onSurface.withValues(alpha: 0.92);
    final dangerColor = colorScheme.error.withValues(alpha: 0.90);

    return SizedBox(
      width: side,
      height: side,
      child: PopupMenuButton<_ConversationQuickAction>(
        tooltip: '更多操作',
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
            width: 1,
          ),
        ),
        constraints: const BoxConstraints(minWidth: 118, maxWidth: 138),
        menuPadding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
        onOpened: () => onMenuOpenChanged?.call(true),
        onCanceled: () => onMenuOpenChanged?.call(false),
        child: ThemeHistoryMoreGlyph(
          color: color,
          side: side,
          compact: compact,
        ),
        onSelected: (action) {
          onMenuOpenChanged?.call(false);
          switch (action) {
            case _ConversationQuickAction.rename:
              onRename();
            case _ConversationQuickAction.togglePinned:
              onTogglePinned();
            case _ConversationQuickAction.toggleLocked:
              onToggleLocked();
            case _ConversationQuickAction.delete:
              onDelete();
          }
        },
        itemBuilder: (context) => <PopupMenuEntry<_ConversationQuickAction>>[
          _menuItem(
            value: _ConversationQuickAction.rename,
            icon: Icons.edit_outlined,
            label: '编辑标题',
            iconColor: itemIconColor,
            textColor: itemTextColor,
          ),
          _menuItem(
            value: _ConversationQuickAction.togglePinned,
            icon: history.pinned
                ? Icons.push_pin_outlined
                : Icons.push_pin_rounded,
            label: history.pinned ? '取消置顶' : '置顶',
            iconColor: itemIconColor,
            textColor: itemTextColor,
          ),
          _menuItem(
            value: _ConversationQuickAction.toggleLocked,
            icon: history.locked ? Icons.lock_open_rounded : Icons.lock_rounded,
            label: history.locked ? '解锁' : '锁定',
            iconColor: itemIconColor,
            textColor: itemTextColor,
          ),
          const PopupMenuDivider(height: 8),
          _menuItem(
            value: _ConversationQuickAction.delete,
            icon: Icons.delete_outline_rounded,
            label: '删除',
            iconColor: dangerColor,
            textColor: dangerColor,
          ),
        ],
      ),
    );
  }
}

class _ConversationStatusHandle extends StatelessWidget {
  /// Creates the existing generic activity handle without an unused widget key.
  const _ConversationStatusHandle({
    required this.isRunning,
    required this.selected,
    required this.hovered,
    required this.appearance,
    this.animation,
    this.compact = false,
  });

  final bool isRunning;
  final bool selected;
  final bool hovered;
  final NavigationDrawerAppearance appearance;
  final Animation<double>? animation;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    return ThemeConversationStatusHandle(
      isRunning: isRunning,
      selected: selected,
      hovered: hovered,
      appearance: appearance,
      animation: animation,
      compact: compact,
    );
  }
}

class _StaticDotsPainter extends CustomPainter {
  const _StaticDotsPainter({required this.color});

  final Color color;

  static const List<Offset> _positions = <Offset>[
    Offset(0.22, 0.16), // Top-Left
    Offset(0.22, 0.50), // Mid-Left
    Offset(0.22, 0.84), // Bottom-Left
    Offset(0.78, 0.84), // Bottom-Right
    Offset(0.78, 0.50), // Mid-Right
    Offset(0.78, 0.16), // Top-Right
  ];

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.fill;
    final dotRadius = size.width * 0.11;

    for (final pos in _positions) {
      canvas.drawCircle(
        Offset(size.width * pos.dx, size.height * pos.dy),
        dotRadius,
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _StaticDotsPainter oldDelegate) {
    return oldDelegate.color != color;
  }
}

class _DraggingConversationItem extends StatelessWidget {
  const _DraggingConversationItem({
    required this.history,
    required this.title,
    required this.appearance,
  });

  final core_proxy.ChatHistoryListItem history;
  final String title;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: appearance.selectedContainerColor,
        borderRadius: BorderRadius.circular(12),
        boxShadow: <BoxShadow>[
          BoxShadow(
            blurRadius: 18,
            color: Colors.black.withValues(alpha: 0.18),
          ),
        ],
      ),
      child: Padding(
        padding: const EdgeInsetsDirectional.fromSTEB(10, 7, 12, 7),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            CustomPaint(
              size: const Size(14, 18),
              painter: _StaticDotsPainter(
                color: appearance.selectedContentColor.withValues(alpha: 0.72),
              ),
            ),
            const SizedBox(width: 8),
            Flexible(
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: appearance.selectedContentColor,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            if (history.pinned) ...<Widget>[
              const SizedBox(width: 6),
              Icon(
                Icons.push_pin,
                size: 13,
                color: appearance.selectedContentColor.withValues(alpha: 0.65),
              ),
            ],
            if (history.locked) ...<Widget>[
              const SizedBox(width: 6),
              Icon(
                Icons.lock,
                size: 13,
                color: appearance.selectedContentColor.withValues(alpha: 0.65),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _SwipeActionBackground extends StatelessWidget {
  const _SwipeActionBackground({
    required this.alignment,
    required this.color,
    required this.icon,
    required this.label,
  });

  final AlignmentGeometry alignment;
  final Color color;
  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Align(
        alignment: alignment,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(icon, color: Colors.white, size: 18),
              const SizedBox(width: 6),
              Text(
                label,
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: Colors.white,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _RoundDrawerButton extends StatelessWidget {
  /// Creates a compact action that also supports the existing inactive drag preview.
  const _RoundDrawerButton({
    required this.selected,
    required this.appearance,
    required this.icon,
    required this.onClick,
  });

  final bool selected;
  final NavigationDrawerAppearance appearance;
  final IconData icon;
  final VoidCallback? onClick;

  /// Builds the compact plugin drag source and reorder drop target.
  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 40,
      height: 40,
      child: Material(
        color: selected
            ? appearance.selectedContainerColor
            : Colors.transparent,
        shape: const CircleBorder(),
        child: IconButton(
          onPressed: onClick,
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints.tightFor(width: 40, height: 40),
          iconSize: 20,
          icon: Icon(
            icon,
            color: selected
                ? appearance.selectedContentColor
                : appearance.itemColor,
          ),
        ),
      ),
    );
  }
}

class _DockedPluginRoundButton extends StatelessWidget {
  const _DockedPluginRoundButton({
    required this.entry,
    required this.insertionIndex,
    required this.selected,
    required this.appearance,
    required this.onClick,
  });

  final NavigationEntrySpec entry;
  final int insertionIndex;
  final bool selected;
  final NavigationDrawerAppearance appearance;
  final VoidCallback? onClick;

  @override
  Widget build(BuildContext context) {
    final controller = SidebarDockScope.maybeOf(context);
    final button = _RoundDrawerButton(
      selected: selected,
      appearance: appearance,
      icon: entry.icon,
      onClick: onClick,
    );
    if (controller == null ||
        !controller.canMove(entry.entryId, SidebarDockLocation.secondary)) {
      return button;
    }
    return DragTarget<SidebarDockDragPayload>(
      onWillAcceptWithDetails: (details) {
        return details.data.entryId != entry.entryId &&
            controller.canMove(
              details.data.entryId,
              SidebarDockLocation.primary,
            );
      },
      onAcceptWithDetails: (details) {
        controller.move(
          details.data.entryId,
          location: SidebarDockLocation.primary,
          insertionIndex: insertionIndex,
        );
      },
      builder: (context, candidateData, rejectedData) {
        final target = DecoratedBox(
          decoration: candidateData.isEmpty
              ? const BoxDecoration()
              : BoxDecoration(
                  border: Border.all(
                    color: Theme.of(context).colorScheme.primary,
                  ),
                  shape: BoxShape.circle,
                ),
          child: button,
        );
        return Draggable<SidebarDockDragPayload>(
          data: SidebarDockDragPayload(entryId: entry.entryId),
          feedback: Material(
            color: Colors.transparent,
            child: Opacity(opacity: 0.92, child: button),
          ),
          childWhenDragging: Opacity(opacity: 0.32, child: button),
          child: target,
        );
      },
    );
  }
}

class SidebarDockEndDropTarget extends StatelessWidget {
  const SidebarDockEndDropTarget({
    super.key,
    required this.controller,
    required this.location,
    this.height = 24,
    this.onAccepted,
  });

  final SidebarDockController? controller;
  final SidebarDockLocation location;
  final double height;
  final VoidCallback? onAccepted;

  /// Builds the trailing drop zone for one sidebar list.
  @override
  Widget build(BuildContext context) {
    final dockController = controller;
    if (dockController == null) {
      return SizedBox(height: height);
    }
    return DragTarget<SidebarDockDragPayload>(
      onWillAcceptWithDetails: (details) =>
          dockController.canMove(details.data.entryId, location),
      onAcceptWithDetails: (details) {
        final insertionIndex = location == SidebarDockLocation.primary
            ? dockController.primaryEntries.length
            : dockController.secondaryViews.length;
        dockController.move(
          details.data.entryId,
          location: location,
          insertionIndex: insertionIndex,
        );
        onAccepted?.call();
      },
      builder: (context, candidateData, rejectedData) {
        return SizedBox(
          height: height,
          child: candidateData.isEmpty
              ? null
              : DecoratedBox(
                  decoration: BoxDecoration(
                    border: Border(
                      bottom: BorderSide(
                        color: Theme.of(context).colorScheme.primary,
                        width: 2,
                      ),
                    ),
                  ),
                ),
        );
      },
    );
  }
}
