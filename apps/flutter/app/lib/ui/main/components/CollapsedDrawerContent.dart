// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../common/OperitLogoMark.dart';
import '../navigation/AppNavigationModels.dart';
import '../screens/ScreenRouteRegistry.dart';
import 'NavigationDrawerAppearance.dart';

class CollapsedDrawerContent extends StatelessWidget {
  const CollapsedDrawerContent({
    super.key,
    required this.navigationEntries,
    required this.selectedRouteId,
    required this.appearance,
    required this.onNavigationEntrySelected,
    required this.onConversationActivated,
    this.bridge = const ProxyCoreRuntimeBridge(),
  });

  final List<NavigationEntrySpec> navigationEntries;
  final String selectedRouteId;
  final NavigationDrawerAppearance appearance;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;
  final VoidCallback onConversationActivated;
  final OperitRuntimeBridge bridge;
  static const double _topBarHeight = 64;

  Future<void> _createConversation() async {
    await GeneratedCoreProxyClients(bridge).chatRuntimeHolderMain.createNewChat(
      characterCardName: null,
      group: null,
      inheritGroupFromCurrent: true,
      setAsCurrentChat: true,
      characterGroupId: null,
    );
    onConversationActivated();
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
    return ListView(
      padding: const EdgeInsets.only(bottom: 24),
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
        const SizedBox(height: 24),
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Center(
            child: _RoundDrawerButton(
              selected: selectedRouteId == navigationEntries.first.routeId,
              appearance: appearance,
              icon: Icons.chat_bubble_outline,
              onClick: onConversationActivated,
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Center(
            child: _RoundDrawerButton(
              selected: false,
              appearance: appearance,
              icon: Icons.add_comment_outlined,
              onClick: _createConversation,
            ),
          ),
        ),
        const SizedBox(height: 16),
        Center(
          child: _RoundDrawerButton(
            selected: selectedRouteId == _packageManagerRouteId,
            appearance: appearance,
            icon: Icons.inventory_2_outlined,
            onClick: _openPackageManager,
          ),
        ),
        const SizedBox(height: 8),
        Center(
          child: _RoundDrawerButton(
            selected: selectedRouteId == _settingsRouteId,
            appearance: appearance,
            icon: Icons.settings_outlined,
            onClick: _openSettings,
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
  });

  final String brandName;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            brandName,
            style: Theme.of(context).textTheme.titleLarge?.copyWith(
              letterSpacing: 0,
              color: appearance.titleColor,
              fontWeight: FontWeight.bold,
            ),
          ),
        ],
      ),
    );
  }
}

class NewConversationButton extends StatelessWidget {
  const NewConversationButton({
    super.key,
    required this.appearance,
    required this.onClick,
    required this.onCreateGroup,
  });

  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;
  final VoidCallback onCreateGroup;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(16);
    final actionColor = appearance.selectedContainerColor;
    final actionContentColor = appearance.selectedContentColor;
    return Row(
      children: <Widget>[
        Expanded(
          child: Material(
            color: actionColor,
            borderRadius: shape,
            child: InkWell(
              borderRadius: shape,
              onTap: onClick,
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 12,
                ),
                child: Row(
                  children: <Widget>[
                    Icon(Icons.add, size: 21, color: actionContentColor),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        '新建对话',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: actionContentColor,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
        const SizedBox(width: 8),
        SizedBox(
          width: 44,
          height: 44,
          child: Material(
            color: Colors.transparent,
            borderRadius: BorderRadius.circular(22),
            child: IconButton(
              onPressed: onCreateGroup,
              icon: Icon(
                Icons.add_circle_outline,
                size: 24,
                color: appearance.titleColor,
              ),
              tooltip: '新建分组',
              style: IconButton.styleFrom(
                shape: const CircleBorder(),
                backgroundColor: Colors.transparent,
                foregroundColor: appearance.itemColor,
                overlayColor: appearance.statusAvailableColor.withValues(
                  alpha: 0.20,
                ),
              ),
            ),
          ),
        ),
      ],
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
  const HistoryRail({
    super.key,
    required this.height,
    required this.appearance,
  });

  final double height;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 24,
      height: height,
      child: Center(
        child: Container(
          width: 2,
          height: height,
          decoration: BoxDecoration(
            color: appearance.dividerColor,
            borderRadius: BorderRadius.circular(1),
          ),
        ),
      ),
    );
  }
}

class ConversationDrawerItem extends StatelessWidget {
  const ConversationDrawerItem({
    super.key,
    required this.history,
    required this.title,
    required this.selected,
    required this.appearance,
    required this.onClick,
    required this.onRename,
    required this.onDelete,
    required this.onLongPress,
    required this.onMoveTo,
    this.nested = false,
  });

  final core_proxy.ChatHistory history;
  final String title;
  final bool selected;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;
  final VoidCallback onRename;
  final VoidCallback onDelete;
  final VoidCallback onLongPress;
  final ValueChanged<core_proxy.ChatHistory> onMoveTo;
  final bool nested;

  static const double _endPadding = 12;

  @override
  Widget build(BuildContext context) {
    final itemShape = BorderRadius.circular(12);
    return DragTarget<core_proxy.ChatHistory>(
      onWillAcceptWithDetails: (details) => details.data.id != history.id,
      onAcceptWithDetails: (details) => onMoveTo(details.data),
      builder: (context, candidateData, rejectedData) {
        final dragHovering = candidateData.isNotEmpty;
        return Padding(
          padding: EdgeInsetsDirectional.only(
            start: nested ? 22 : 12,
            end: _endPadding,
            bottom: 3,
          ),
          child: Row(
            children: <Widget>[
              if (nested) HistoryRail(height: 34, appearance: appearance),
              Expanded(
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    borderRadius: itemShape,
                    border: dragHovering
                        ? Border.all(
                            color: appearance.statusAvailableColor.withValues(
                              alpha: 0.55,
                            ),
                          )
                        : null,
                  ),
                  child: Dismissible(
                    key: ValueKey<String>('conversation-${history.id}'),
                    confirmDismiss: (direction) async {
                      if (direction == DismissDirection.startToEnd) {
                        onRename();
                      } else {
                        onDelete();
                      }
                      return false;
                    },
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
                    child: Material(
                      color: selected
                          ? appearance.selectedContainerColor
                          : Colors.transparent,
                      borderRadius: itemShape,
                      child: InkWell(
                        borderRadius: itemShape,
                        onTap: onClick,
                        onLongPress: onLongPress,
                        child: Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 5,
                          ),
                          child: Row(
                            children: <Widget>[
                              Draggable<core_proxy.ChatHistory>(
                                data: history,
                                dragAnchorStrategy: pointerDragAnchorStrategy,
                                feedback: Material(
                                  color: Colors.transparent,
                                  child: ConstrainedBox(
                                    constraints: const BoxConstraints(
                                      maxWidth: 280,
                                    ),
                                    child: _DraggingConversationItem(
                                      history: history,
                                      title: title,
                                      appearance: appearance,
                                    ),
                                  ),
                                ),
                                childWhenDragging: Opacity(
                                  opacity: 0.35,
                                  child: _HistoryDragHandle(
                                    selected: selected,
                                    appearance: appearance,
                                  ),
                                ),
                                child: _HistoryDragHandle(
                                  selected: selected,
                                  appearance: appearance,
                                ),
                              ),
                              const SizedBox(width: 6),
                              Expanded(
                                child: Text(
                                  title,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: Theme.of(context).textTheme.bodySmall
                                      ?.copyWith(
                                        color: selected
                                            ? appearance.selectedContentColor
                                            : appearance.itemColor,
                                        fontWeight: selected
                                            ? FontWeight.w600
                                            : FontWeight.w400,
                                      ),
                                ),
                              ),
                              if (history.pinned) ...<Widget>[
                                const SizedBox(width: 6),
                                Icon(
                                  Icons.push_pin,
                                  size: 13,
                                  color:
                                      (selected
                                              ? appearance.selectedContentColor
                                              : appearance.itemColor)
                                          .withValues(alpha: 0.65),
                                ),
                              ],
                              if (history.locked) ...<Widget>[
                                const SizedBox(width: 6),
                                Icon(
                                  Icons.lock,
                                  size: 13,
                                  color:
                                      (selected
                                              ? appearance.selectedContentColor
                                              : appearance.itemColor)
                                          .withValues(alpha: 0.65),
                                ),
                              ],
                            ],
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
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
    final shape = BorderRadius.circular(14);
    final backgroundColor = selected
        ? appearance.selectedContainerColor
        : Colors.transparent;
    final contentColor = selected
        ? appearance.selectedContentColor
        : appearance.itemColor;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: backgroundColor,
        borderRadius: shape,
        border: selected ? null : Border.all(color: appearance.dividerColor),
      ),
      child: Material(
        color: Colors.transparent,
        borderRadius: shape,
        child: InkWell(
          borderRadius: shape,
          onTap: onClick,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: <Widget>[
                Icon(icon, size: 18, color: contentColor),
                const SizedBox(width: 6),
                Flexible(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.labelLarge?.copyWith(
                      color: contentColor,
                      fontWeight: selected ? FontWeight.w700 : FontWeight.w600,
                    ),
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

class _HistoryDragHandle extends StatelessWidget {
  const _HistoryDragHandle({required this.selected, required this.appearance});

  final bool selected;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    final color =
        (selected ? appearance.selectedContentColor : appearance.itemColor)
            .withValues(alpha: 0.72);
    return SizedBox(
      width: 28,
      height: 28,
      child: Tooltip(
        message: '拖动对话',
        child: Material(
          color: Colors.transparent,
          shape: const CircleBorder(),
          child: InkResponse(
            onTap: () {},
            radius: 16,
            containedInkWell: true,
            customBorder: const CircleBorder(),
            child: Icon(Icons.drag_handle, size: 18, color: color),
          ),
        ),
      ),
    );
  }
}

class _DraggingConversationItem extends StatelessWidget {
  const _DraggingConversationItem({
    required this.history,
    required this.title,
    required this.appearance,
  });

  final core_proxy.ChatHistory history;
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
            Icon(
              Icons.drag_handle,
              size: 20,
              color: appearance.selectedContentColor.withValues(alpha: 0.72),
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
  const _RoundDrawerButton({
    required this.selected,
    required this.appearance,
    required this.icon,
    required this.onClick,
  });

  final bool selected;
  final NavigationDrawerAppearance appearance;
  final IconData icon;
  final VoidCallback onClick;

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
