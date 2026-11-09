// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
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
import 'NavigationLayoutMetrics.dart';

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
    this.tabletSidebarWidth = navigationSidebarDefaultWidth,
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

  /// Initial width before the user resizes the expanded conversation list.
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
  double? _preferredSidebarWidth;
  bool _resizing = false;

  @override
  void initState() {
    super.initState();
    _isSidebarWidthExpanded = widget.isTabletSidebarExpanded;
    _isSidebarContentExpanded = widget.isTabletSidebarExpanded;
  }

  @override
  void didUpdateWidget(covariant TabletLayout oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selectedRouteId != widget.selectedRouteId) {
      _resizing = false;
    }
    if (oldWidget.isTabletSidebarExpanded == widget.isTabletSidebarExpanded) {
      return;
    }

    _contentSwitchTimer?.cancel();
    _widthSwitchTimer?.cancel();
    _resizing = false;

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
    final contentInsets = DesktopContentPanel.contentInsets;
    final content = RepaintBoundary(child: widget.content);

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
            child: LayoutBuilder(
              builder: (context, constraints) {
                final maximumWidth = math.min(
                  navigationSidebarMaxWidth,
                  math.max(
                    0.0,
                    constraints.maxWidth - navigationMinContentWidth,
                  ),
                );
                final hasRoom = maximumWidth >= navigationSidebarMinWidth;
                final sidebarWidth =
                    (_preferredSidebarWidth ?? widget.tabletSidebarWidth)
                        .clamp(
                          navigationSidebarMinWidth,
                          math.max(navigationSidebarMinWidth, maximumWidth),
                        )
                        .toDouble();
                final showConversationPanel =
                    isChatRoute && _isSidebarWidthExpanded && hasRoom;
                final targetWidth = showConversationPanel ? sidebarWidth : 0.0;
                return TweenAnimationBuilder<double>(
                  tween: Tween<double>(begin: targetWidth, end: targetWidth),
                  duration: _resizing
                      ? Duration.zero
                      : _sidebarWidthAnimationDuration,
                  curve: Curves.fastOutSlowIn,
                  child: _conversationSidebar(
                    appearance: appearance,
                    width: sidebarWidth,
                    isChatRoute: isChatRoute,
                  ),
                  builder: (context, animatedWidth, sidebar) {
                    // Window resizing can tighten constraints mid-transition.
                    final visibleWidth = animatedWidth
                        .clamp(0.0, maximumWidth)
                        .toDouble();
                    return Stack(
                      children: <Widget>[
                        Row(
                          children: <Widget>[
                            SizedBox(
                              key: const ValueKey('desktop-conversation-panel'),
                              width: visibleWidth,
                              height: double.infinity,
                              child: sidebar,
                            ),
                            Expanded(child: content),
                          ],
                        ),
                        if (showConversationPanel &&
                            widget.isTabletSidebarExpanded &&
                            _isSidebarContentExpanded &&
                            visibleWidth >= navigationSidebarMinWidth)
                          Positioned(
                            left:
                                visibleWidth -
                                navigationSidebarResizeHitWidth / 2,
                            top: 0,
                            bottom: 0,
                            width: navigationSidebarResizeHitWidth,
                            child: _ConversationSidebarResizeHandle(
                              key: const ValueKey(
                                'desktop-conversation-resize-handle',
                              ),
                              resizing: _resizing,
                              onDragStart: (_) {
                                setState(() {
                                  _preferredSidebarWidth = visibleWidth;
                                  _resizing = true;
                                });
                              },
                              onDragUpdate: (details) {
                                setState(() {
                                  _preferredSidebarWidth =
                                      (_preferredSidebarWidth! +
                                              details.delta.dx)
                                          .clamp(
                                            navigationSidebarMinWidth,
                                            maximumWidth,
                                          )
                                          .toDouble();
                                });
                              },
                              onDragEnd: () =>
                                  setState(() => _resizing = false),
                            ),
                          ),
                      ],
                    );
                  },
                );
              },
            ),
          ),
        ),
      ],
    );
  }

  /// Clips the reveal animation while laying out the list at its usable width.
  Widget _conversationSidebar({
    required NavigationDrawerAppearance appearance,
    required double width,
    required bool isChatRoute,
  }) => ClipRect(
    child: OverflowBox(
      alignment: Alignment.topLeft,
      minWidth: width,
      maxWidth: width,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: appearance.containerColor,
          border: Border(right: BorderSide(color: appearance.dividerColor)),
        ),
        child: isChatRoute
            ? AnimatedSwitcher(
                duration: _sidebarContentFadeDuration,
                child: _isSidebarContentExpanded
                    ? ValueListenableBuilder<DrawerConversationState>(
                        key: const ValueKey('conversationSidebarContent'),
                        valueListenable: widget.drawerConversationState,
                        builder: (context, drawerState, _) => DrawerContent(
                          bridge: widget.bridge,
                          navigationEntries: widget.navigationEntries,
                          pluginEntries: const [],
                          selectedRouteId: widget.selectedRouteId,
                          appearance: appearance,
                          histories: drawerState.histories,
                          activeStreamingChatIds:
                              drawerState.activeStreamingChatIds,
                          currentChatId: drawerState.currentChatId,
                          errorMessage: drawerState.errorMessage,
                          loading: drawerState.loading,
                          onNavigationEntrySelected:
                              widget.onNavigationEntrySelected,
                          onConversationActivated:
                              widget.onConversationActivated,
                          onCollapseSidebar: widget.onCollapseSidebar,
                          conversationsOnly: true,
                        ),
                      )
                    : const SizedBox.shrink(),
              )
            : const SizedBox.shrink(),
      ),
    ),
  );

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

/// Places a forgiving hit target over the divider without taking layout space.
class _ConversationSidebarResizeHandle extends StatefulWidget {
  const _ConversationSidebarResizeHandle({
    super.key,
    required this.resizing,
    required this.onDragStart,
    required this.onDragUpdate,
    required this.onDragEnd,
  });

  final bool resizing;
  final GestureDragStartCallback onDragStart;
  final GestureDragUpdateCallback onDragUpdate;
  final VoidCallback onDragEnd;

  @override
  State<_ConversationSidebarResizeHandle> createState() =>
      _ConversationSidebarResizeHandleState();
}

class _ConversationSidebarResizeHandleState
    extends State<_ConversationSidebarResizeHandle> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) => MouseRegion(
    cursor: SystemMouseCursors.resizeColumn,
    onEnter: (_) => setState(() => _hovered = true),
    onExit: (_) => setState(() => _hovered = false),
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      dragStartBehavior: DragStartBehavior.down,
      onHorizontalDragStart: widget.onDragStart,
      onHorizontalDragUpdate: widget.onDragUpdate,
      onHorizontalDragEnd: (_) => widget.onDragEnd(),
      onHorizontalDragCancel: widget.onDragEnd,
      child: Center(
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          width: 2,
          height: double.infinity,
          color: _hovered || widget.resizing
              ? Theme.of(context).colorScheme.primary.withValues(alpha: 0.75)
              : Colors.transparent,
        ),
      ),
    ),
  );
}
