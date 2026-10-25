// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../components/DrawerConversationState.dart';
import '../navigation/AppNavigationModels.dart';
import 'NavigationLayoutMetrics.dart';
import 'PhoneLayout.dart';
import 'TabletLayout.dart';

/// Retains the page host when window resizing changes navigation layouts.
class AdaptiveMainLayout extends StatefulWidget {
  const AdaptiveMainLayout({
    super.key,
    this.bridge = const ProxyCoreRuntimeBridge(),
    required this.content,
    required this.navigationEntries,
    required this.pluginSidebarEntries,
    required this.selectedRouteId,
    required this.drawerConversationState,
    required this.isTabletSidebarExpanded,
    required this.drawerOpenState,
    required this.enableNavigationAnimation,
    required this.onOpenDrawer,
    required this.onCloseDrawer,
    required this.onNavigationEntrySelected,
    required this.onConversationActivated,
  });

  final OperitRuntimeBridge bridge;
  final Widget content;
  final List<NavigationEntrySpec> navigationEntries;
  final List<NavigationEntrySpec> pluginSidebarEntries;
  final String selectedRouteId;
  final ValueListenable<DrawerConversationState> drawerConversationState;
  final bool isTabletSidebarExpanded;
  final ValueListenable<bool> drawerOpenState;
  final bool enableNavigationAnimation;
  final VoidCallback onOpenDrawer;
  final VoidCallback onCloseDrawer;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;
  final VoidCallback onConversationActivated;

  @override
  State<AdaptiveMainLayout> createState() => _AdaptiveMainLayoutState();
}

class _AdaptiveMainLayoutState extends State<AdaptiveMainLayout> {
  // Reparent the entire page cache in the same frame instead of disposing it
  // together with the outgoing navigation layout.
  final GlobalKey _contentKey = GlobalKey();

  @override
  Widget build(BuildContext context) {
    final width = MediaQuery.sizeOf(context).width;
    final content = KeyedSubtree(key: _contentKey, child: widget.content);
    if (useTabletLayoutForWidth(width)) {
      return TabletLayout(
        bridge: widget.bridge,
        content: content,
        navigationEntries: widget.navigationEntries,
        pluginSidebarEntries: widget.pluginSidebarEntries,
        selectedRouteId: widget.selectedRouteId,
        drawerConversationState: widget.drawerConversationState,
        isTabletSidebarExpanded: widget.isTabletSidebarExpanded,
        tabletSidebarWidth: 280,
        collapsedTabletSidebarWidth: 56,
        onNavigationEntrySelected: widget.onNavigationEntrySelected,
        onConversationActivated: widget.onConversationActivated,
      );
    }
    return PhoneLayout(
      bridge: widget.bridge,
      content: content,
      navigationEntries: widget.navigationEntries,
      pluginSidebarEntries: widget.pluginSidebarEntries,
      selectedRouteId: widget.selectedRouteId,
      drawerConversationState: widget.drawerConversationState,
      drawerWidth: width * 0.75,
      drawerOpenState: widget.drawerOpenState,
      enableNavigationAnimation: widget.enableNavigationAnimation,
      onOpenDrawer: widget.onOpenDrawer,
      onCloseDrawer: widget.onCloseDrawer,
      onNavigationEntrySelected: widget.onNavigationEntrySelected,
      onConversationActivated: widget.onConversationActivated,
    );
  }
}
