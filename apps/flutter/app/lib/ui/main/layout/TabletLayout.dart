// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../window/DesktopWindowChrome.dart';
import '../components/CollapsedDrawerContent.dart';
import '../components/DrawerConversationState.dart';
import '../components/DrawerContent.dart';
import '../components/NavigationDrawerAppearance.dart';
import '../navigation/AppNavigationModels.dart';
import '../screens/ScreenRouteRegistry.dart';

class TabletLayout extends StatefulWidget {
  const TabletLayout({
    super.key,
    this.bridge = const ProxyCoreRuntimeBridge(),
    required this.content,
    required this.navigationEntries,
    required this.pluginSidebarEntries,
    required this.selectedRouteId,
    required this.drawerConversationState,
    required this.isTabletSidebarExpanded,
    required this.tabletSidebarWidth,
    required this.collapsedTabletSidebarWidth,
    required this.onNavigationEntrySelected,
    required this.onConversationActivated,
    this.onExpandSidebar,
    this.onCollapseSidebar,
  });

  final OperitRuntimeBridge bridge;
  final Widget content;
  final List<NavigationEntrySpec> navigationEntries;
  final List<NavigationEntrySpec> pluginSidebarEntries;
  final String selectedRouteId;
  final ValueListenable<DrawerConversationState> drawerConversationState;
  final bool isTabletSidebarExpanded;
  final double tabletSidebarWidth;
  final double collapsedTabletSidebarWidth;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;
  final VoidCallback onConversationActivated;
  final VoidCallback? onExpandSidebar;
  final VoidCallback? onCollapseSidebar;

  @override
  State<TabletLayout> createState() => _TabletLayoutState();
}

class _TabletLayoutState extends State<TabletLayout> {
  static const Duration _sidebarWidthAnimationDuration = Duration(
    milliseconds: 280,
  );
  static const Duration _sidebarContentFadeDuration = Duration(
    milliseconds: 160,
  );

  late bool _isSidebarWidthExpanded;
  late bool _isSidebarContentExpanded;
  Timer? _contentSwitchTimer;
  Timer? _widthSwitchTimer;

  @override
  void initState() {
    super.initState();
    _isSidebarWidthExpanded = widget.isTabletSidebarExpanded;
    _isSidebarContentExpanded = widget.isTabletSidebarExpanded;
  }

  @override
  void didUpdateWidget(covariant TabletLayout oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.isTabletSidebarExpanded == widget.isTabletSidebarExpanded) {
      return;
    }

    _contentSwitchTimer?.cancel();
    _widthSwitchTimer?.cancel();

    if (widget.isTabletSidebarExpanded) {
      setState(() {
        _isSidebarWidthExpanded = true;
      });
      _contentSwitchTimer = Timer(_sidebarWidthAnimationDuration, () {
        if (!mounted) {
          return;
        }
        setState(() {
          _isSidebarContentExpanded = true;
        });
      });
    } else {
      setState(() {
        _isSidebarContentExpanded = false;
      });
      _widthSwitchTimer = Timer(_sidebarContentFadeDuration, () {
        if (!mounted) {
          return;
        }
        setState(() {
          _isSidebarWidthExpanded = false;
        });
      });
    }
  }

  @override
  void dispose() {
    _contentSwitchTimer?.cancel();
    _widthSwitchTimer?.cancel();
    super.dispose();
  }

  /// Toggles the conversation list independently from chat activation and creation.
  void _handleConversationNavigationPressed() {
    final isChatRoute =
        widget.selectedRouteId ==
        ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.aiChat);
    if (isChatRoute && widget.isTabletSidebarExpanded) {
      widget.onCollapseSidebar?.call();
      return;
    }
    widget.onExpandSidebar?.call();
    widget.onConversationActivated();
  }

  @override
  Widget build(BuildContext context) {
    final appearance = navigationDrawerAppearanceOf(context);
    final isChatRoute =
        widget.selectedRouteId ==
        ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.aiChat);
    final showConversationPanel = isChatRoute && _isSidebarWidthExpanded;
    final contentInsets = DesktopContentPanel.contentInsets;

    return Row(
      children: <Widget>[
        SizedBox(
          key: const ValueKey('desktop-primary-navigation'),
          width: widget.collapsedTabletSidebarWidth,
          height: double.infinity,
          child: RepaintBoundary(
            child: _sidebarSurface(
              appearance: appearance,
              child: Padding(
                padding: EdgeInsets.only(
                  top: contentInsets.top,
                  bottom: contentInsets.bottom,
                ),
                child: ValueListenableBuilder<DrawerConversationState>(
                  valueListenable: widget.drawerConversationState,
                  builder: (context, drawerState, _) => CollapsedDrawerContent(
                    bridge: widget.bridge,
                    navigationEntries: widget.navigationEntries,
                    pluginEntries: widget.pluginSidebarEntries,
                    selectedRouteId: widget.selectedRouteId,
                    appearance: appearance,
                    currentChatId: drawerState.currentChatId,
                    histories: drawerState.histories,
                    onNavigationEntrySelected: widget.onNavigationEntrySelected,
                    onConversationNavigationPressed:
                        _handleConversationNavigationPressed,
                    onConversationActivated: widget.onConversationActivated,
                  ),
                ),
              ),
            ),
          ),
        ),
        Expanded(
          child: DesktopContentPanel(
            child: Row(
              children: <Widget>[
                AnimatedContainer(
                  key: const ValueKey('desktop-conversation-panel'),
                  duration: _sidebarWidthAnimationDuration,
                  curve: Curves.fastOutSlowIn,
                  width: showConversationPanel ? widget.tabletSidebarWidth : 0,
                  height: double.infinity,
                  child: ClipRect(
                    child: OverflowBox(
                      alignment: Alignment.topLeft,
                      minWidth: widget.tabletSidebarWidth,
                      maxWidth: widget.tabletSidebarWidth,
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          color: appearance.containerColor,
                          border: Border(
                            right: BorderSide(color: appearance.dividerColor),
                          ),
                        ),
                        child: isChatRoute
                            ? AnimatedSwitcher(
                                duration: _sidebarContentFadeDuration,
                                child: _isSidebarContentExpanded
                                    ? ValueListenableBuilder<
                                        DrawerConversationState
                                      >(
                                        key: const ValueKey(
                                          'conversationSidebarContent',
                                        ),
                                        valueListenable:
                                            widget.drawerConversationState,
                                        builder: (context, drawerState, _) =>
                                            DrawerContent(
                                              bridge: widget.bridge,
                                              navigationEntries:
                                                  widget.navigationEntries,
                                              pluginEntries: const [],
                                              selectedRouteId:
                                                  widget.selectedRouteId,
                                              appearance: appearance,
                                              histories: drawerState.histories,
                                              activeStreamingChatIds:
                                                  drawerState
                                                      .activeStreamingChatIds,
                                              currentChatId:
                                                  drawerState.currentChatId,
                                              errorMessage:
                                                  drawerState.errorMessage,
                                              loading: drawerState.loading,
                                              onNavigationEntrySelected: widget
                                                  .onNavigationEntrySelected,
                                              onConversationActivated: widget
                                                  .onConversationActivated,
                                              onCollapseSidebar:
                                                  widget.onCollapseSidebar,
                                              conversationsOnly: true,
                                            ),
                                      )
                                    : const SizedBox.shrink(),
                              )
                            : const SizedBox.shrink(),
                      ),
                    ),
                  ),
                ),
                Expanded(child: RepaintBoundary(child: widget.content)),
              ],
            ),
          ),
        ),
      ],
    );
  }

  /// Desktop chrome already paints the same sidebar surface around the panel.
  Widget _sidebarSurface({
    required NavigationDrawerAppearance appearance,
    required Widget child,
  }) => usesDesktopWindowChrome
      ? child
      : ThemeDesktopSidebarSurface(
          color: appearance.containerColor,
          transparentSurface: appearance.transparentSurfaceEnabled,
          child: child,
        );
}
