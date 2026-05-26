// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../components/AppContent.dart';
import '../layout/PhoneLayout.dart';
import '../layout/TabletLayout.dart';
import '../navigation/AppNavigationModels.dart';
import '../navigation/AppRouteCatalog.dart';

class OperitMainScreen extends StatefulWidget {
  const OperitMainScreen({super.key});

  @override
  State<OperitMainScreen> createState() => _OperitMainScreenState();
}

class _OperitMainScreenState extends State<OperitMainScreen> {
  late final AppNavigationModel _navigationModel;
  late final AppRouterState _routerState;
  bool _drawerOpen = false;
  bool _isTabletSidebarExpanded = false;

  @override
  void initState() {
    super.initState();
    _navigationModel = AppRouteCatalog.build();
    _routerState = AppRouterState(AppRouteCatalog.initialEntry());
    AppRouterGateway.install(handler: _navigateToRoute, reset: _resetToRoute);
    AppRouteDiscoveryGateway.install(() => _navigationModel.routes);
  }

  @override
  void dispose() {
    AppRouterGateway.clear();
    AppRouteDiscoveryGateway.clear();
    _routerState.dispose();
    super.dispose();
  }

  void _navigateToRoute(
    String routeId,
    Map<String, Object?> args,
    RouteEntrySource source,
  ) {
    final routeSpec = _navigationModel.routesById[routeId];
    if (routeSpec == null) {
      throw StateError('Unknown routeId: $routeId');
    }
    _routerState.navigate(
      routeId: routeId,
      args: args,
      source: source,
      routeSpec: routeSpec,
    );
  }

  void _resetToRoute(
    String routeId,
    Map<String, Object?> args,
    RouteEntrySource source,
  ) {
    if (!_navigationModel.routesById.containsKey(routeId)) {
      throw StateError('Unknown routeId: $routeId');
    }
    _routerState.resetTo(
      RouteEntry(routeId: routeId, args: args, source: source),
    );
  }

  void _navigateToNavigationEntry(NavigationEntrySpec entry) {
    setState(() {
      _drawerOpen = false;
    });
    _resetToRoute(entry.routeId, entry.routeArgs, RouteEntrySource.drawer);
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _routerState,
      builder: (context, _) {
        final currentRouteEntry = _routerState.currentEntry;
        final currentScreen = AppRouteCatalog.resolveScreen(
          _navigationModel,
          currentRouteEntry,
        );
        final mediaQuery = MediaQuery.of(context);
        final useTabletLayout = mediaQuery.size.width >= 600;
        final content = AppContent(
          routerState: _routerState,
          currentScreen: currentScreen,
          currentRouteEntry: currentRouteEntry,
          useTabletLayout: useTabletLayout,
          isTabletSidebarExpanded: _isTabletSidebarExpanded,
          canGoBack: _routerState.canPop,
          onGoBack: () {
            _routerState.pop();
          },
          onNavigationButtonPressed: () {
            if (useTabletLayout) {
              setState(() {
                _isTabletSidebarExpanded = !_isTabletSidebarExpanded;
              });
            } else {
              setState(() {
                _drawerOpen = true;
              });
            }
          },
        );

        return PopScope(
          canPop: !_drawerOpen && !_routerState.canPop,
          onPopInvokedWithResult: (didPop, result) {
            if (didPop) {
              return;
            }
            if (_drawerOpen) {
              setState(() {
                _drawerOpen = false;
              });
            } else {
              _routerState.pop();
            }
          },
          child: Scaffold(
            body: useTabletLayout
                ? TabletLayout(
                    content: content,
                    navigationEntries: _navigationModel.navigationEntries,
                    selectedRouteId: currentRouteEntry.routeId,
                    isNetworkAvailable: true,
                    networkType: 'Network',
                    isTabletSidebarExpanded: _isTabletSidebarExpanded,
                    tabletSidebarWidth: 280,
                    collapsedTabletSidebarWidth: 64,
                    onNavigationEntrySelected: _navigateToNavigationEntry,
                  )
                : PhoneLayout(
                    content: content,
                    navigationEntries: _navigationModel.navigationEntries,
                    selectedRouteId: currentRouteEntry.routeId,
                    isNetworkAvailable: true,
                    networkType: 'Network',
                    drawerWidth: mediaQuery.size.width * 0.75,
                    drawerOpen: _drawerOpen,
                    enableNavigationAnimation: true,
                    onCloseDrawer: () {
                      setState(() {
                        _drawerOpen = false;
                      });
                    },
                    onNavigationEntrySelected: _navigateToNavigationEntry,
                  ),
          ),
        );
      },
    );
  }
}
