// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../navigation/AppNavigationModels.dart';
import '../screens/OperitScreens.dart';

class AppContent extends StatefulWidget {
  const AppContent({
    super.key,
    required this.routerState,
    required this.currentScreen,
    required this.currentRouteEntry,
    required this.useTabletLayout,
    required this.isTabletSidebarExpanded,
    required this.canGoBack,
    required this.onGoBack,
    required this.onNavigationButtonPressed,
  });

  final AppRouterState routerState;
  final OperitScreen currentScreen;
  final RouteEntry currentRouteEntry;
  final bool useTabletLayout;
  final bool isTabletSidebarExpanded;
  final bool canGoBack;
  final VoidCallback onGoBack;
  final VoidCallback onNavigationButtonPressed;

  @override
  State<AppContent> createState() => _AppContentState();
}

class _AppContentState extends State<AppContent> {
  static const Duration _pageTransitionDuration = Duration(milliseconds: 260);
  static const double _pageTransitionOffset = 42;
  static const double _topBarHeight = 64;
  static const double _navigationIconStartPadding = 4;
  static const double _navigationIconSize = 48;

  final Map<String, Widget> _screenCache = <String, Widget>{};
  final Map<String, bool> _screenKeepAliveCache = <String, bool>{};

  String? _lastObservedCurrentKey;
  OperitScreen? _lastObservedScreen;
  String? _transitionFromKey;
  String? _pendingRemovalKey;
  bool _isTransitioning = false;
  bool _transitionAllowsCrossfade = true;
  bool _isNavigatingBack = false;
  int _lastBackStackLength = 0;

  @override
  void initState() {
    super.initState();
    _lastObservedCurrentKey = _currentScreenKey;
    _lastObservedScreen = widget.currentScreen;
    _lastBackStackLength = widget.routerState.backStack.length;
    _ensureScreenCached(_currentScreenKey, widget.currentScreen);
  }

  @override
  void didUpdateWidget(covariant AppContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    final currentBackStackLength = widget.routerState.backStack.length;
    _isNavigatingBack = currentBackStackLength < _lastBackStackLength;
    _lastBackStackLength = currentBackStackLength;
    final currentScreenKey = _currentScreenKey;
    _ensureScreenCached(currentScreenKey, widget.currentScreen);
    _updateTransition(currentScreenKey, widget.currentScreen);
  }

  String get _currentScreenKey {
    return widget.currentScreen.stableScreenKey() ??
        widget.currentRouteEntry.instanceId;
  }

  void _ensureScreenCached(String screenKey, OperitScreen screen) {
    _screenKeepAliveCache[screenKey] = screen.keepAlive;
    _screenCache[screenKey] = Builder(builder: screen.build);
  }

  void _updateTransition(String currentScreenKey, OperitScreen currentScreen) {
    final fromKey = _lastObservedCurrentKey;
    final fromScreen = _lastObservedScreen;
    if (fromKey == null || fromScreen == null || currentScreenKey == fromKey) {
      return;
    }

    final canCrossfade =
        fromScreen.participatesInCrossfadeTransition &&
        currentScreen.participatesInCrossfadeTransition;

    _transitionAllowsCrossfade = canCrossfade;
    _transitionFromKey = canCrossfade ? fromKey : null;
    _pendingRemovalKey = _isNavigatingBack ? fromKey : null;
    _isTransitioning = canCrossfade;
    _lastObservedCurrentKey = currentScreenKey;
    _lastObservedScreen = currentScreen;

    if (!canCrossfade) {
      _removePendingScreen(currentScreenKey);
      return;
    }

    Future<void>.delayed(_pageTransitionDuration, () {
      if (!mounted) {
        return;
      }
      setState(() {
        _isTransitioning = false;
        _transitionFromKey = null;
        _transitionAllowsCrossfade = true;
        _removePendingScreen(_currentScreenKey);
      });
    });
  }

  void _removePendingScreen(String currentScreenKey) {
    final keyToRemove = _pendingRemovalKey;
    if (keyToRemove != null && keyToRemove != currentScreenKey) {
      _screenCache.remove(keyToRemove);
      _screenKeepAliveCache.remove(keyToRemove);
    }
    _pendingRemovalKey = null;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final appBarContentColor = theme.colorScheme.onPrimary;
    final topPadding = MediaQuery.paddingOf(context).top;
    final currentScreenKey = _currentScreenKey;
    final effectivePreviousKey = !_transitionAllowsCrossfade
        ? null
        : currentScreenKey != _lastObservedCurrentKey
        ? _lastObservedCurrentKey
        : _isTransitioning
        ? _transitionFromKey
        : null;

    final renderKeys = <String>{
      for (final entry in _screenKeepAliveCache.entries)
        if (entry.value && entry.key != currentScreenKey) entry.key,
      if (effectivePreviousKey != null &&
          effectivePreviousKey != currentScreenKey)
        effectivePreviousKey,
      currentScreenKey,
    }.toList(growable: false);

    return SizedBox.expand(
      child: Column(
        children: <Widget>[
          Material(
            color: theme.colorScheme.primary,
            child: SizedBox(
              height: topPadding + _topBarHeight,
              child: Padding(
                padding: EdgeInsets.only(top: topPadding),
                child: Row(
                  children: <Widget>[
                    const SizedBox(width: _navigationIconStartPadding),
                    SizedBox(
                      width: _navigationIconSize,
                      height: _navigationIconSize,
                      child: IconButton(
                        onPressed: widget.canGoBack
                            ? widget.onGoBack
                            : widget.onNavigationButtonPressed,
                        icon: Icon(
                          widget.canGoBack
                              ? Icons.arrow_back
                              : widget.useTabletLayout &&
                                    widget.isTabletSidebarExpanded
                              ? Icons.chevron_left
                              : Icons.menu,
                          color: appBarContentColor,
                        ),
                        tooltip: widget.canGoBack
                            ? 'Back'
                            : widget.useTabletLayout &&
                                  widget.isTabletSidebarExpanded
                            ? 'Collapse sidebar'
                            : 'Navigation',
                      ),
                    ),
                    Expanded(
                      child: Text(
                        widget.currentScreen.title ?? '',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.titleSmall?.copyWith(
                          color: appBarContentColor,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
          Expanded(
            child: ColoredBox(
              color: theme.colorScheme.surface,
              child: Stack(
                children: <Widget>[
                  for (final screenKey in renderKeys)
                    _AnimatedScreenSlot(
                      key: ValueKey<String>(screenKey),
                      screenKey: screenKey,
                      isCurrentScreen: screenKey == currentScreenKey,
                      isNavigatingBack: _isNavigatingBack,
                      allowCrossfade: _transitionAllowsCrossfade,
                      duration: _pageTransitionDuration,
                      offset: _pageTransitionOffset,
                      child: _screenCache[screenKey]!,
                    ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _AnimatedScreenSlot extends StatelessWidget {
  const _AnimatedScreenSlot({
    super.key,
    required this.screenKey,
    required this.isCurrentScreen,
    required this.isNavigatingBack,
    required this.allowCrossfade,
    required this.duration,
    required this.offset,
    required this.child,
  });

  final String screenKey;
  final bool isCurrentScreen;
  final bool isNavigatingBack;
  final bool allowCrossfade;
  final Duration duration;
  final double offset;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final targetOpacity = !allowCrossfade || isCurrentScreen ? 1.0 : 0.0;
    final targetScale = !allowCrossfade || isCurrentScreen ? 1.0 : 0.992;
    final targetOffset = !allowCrossfade || isCurrentScreen
        ? Offset.zero
        : Offset(isNavigatingBack ? offset * 0.45 : -offset * 0.45, 0);

    return Positioned.fill(
      child: IgnorePointer(
        ignoring: !isCurrentScreen,
        child: AnimatedOpacity(
          opacity: targetOpacity,
          duration: duration,
          curve: isCurrentScreen
              ? Curves.linearToEaseOut
              : Curves.easeInToLinear,
          child: AnimatedSlide(
            offset: Offset(targetOffset.dx / 400, 0),
            duration: duration,
            curve: Curves.fastOutSlowIn,
            child: AnimatedScale(
              scale: targetScale,
              duration: duration,
              curve: Curves.fastOutSlowIn,
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}
