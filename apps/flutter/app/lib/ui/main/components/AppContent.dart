// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../MainLayoutController.dart';
import '../../theme/OperitTheme.dart';
import '../TopBarController.dart';
import '../navigation/AppNavigationModels.dart';
import '../screens/OperitScreens.dart';
import 'TopBarTitleText.dart';

class AppContent extends StatefulWidget {
  const AppContent({
    super.key,
    required this.routerState,
    required this.currentScreen,
    required this.currentRouteEntry,
    required this.currentRouteTitle,
    required this.useTabletLayout,
    required this.isTabletSidebarExpanded,
    required this.canGoBack,
    required this.enableNavigationAnimation,
    required this.navigationTransitionSource,
    required this.isNavigatingBack,
    required this.topBarController,
    required this.onGoBack,
    required this.onNavigationButtonPressed,
  });

  final AppRouterState routerState;
  final OperitScreen currentScreen;
  final RouteEntry currentRouteEntry;
  final String currentRouteTitle;
  final bool useTabletLayout;
  final bool isTabletSidebarExpanded;
  final bool canGoBack;
  final bool enableNavigationAnimation;
  final NavigationTransitionSource navigationTransitionSource;
  final bool isNavigatingBack;
  final TopBarController topBarController;
  final VoidCallback onGoBack;
  final VoidCallback onNavigationButtonPressed;

  @override
  State<AppContent> createState() => _AppContentState();
}

class _AppContentState extends State<AppContent> {
  static const Duration _enabledPageTransitionDuration = Duration(
    milliseconds: 280,
  );
  static const Duration _disabledPageTransitionDuration = Duration(
    milliseconds: 400,
  );
  static const Duration _drawerRelayTransitionDuration = Duration(
    milliseconds: 320,
  );
  static const double _phonePageTransitionOffset = 20;
  static const double _tabletPageTransitionOffset = 28;
  static const double _phoneDrawerNavigationOffset = 30;
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

  @override
  void initState() {
    super.initState();
    _lastObservedCurrentKey = _currentScreenKey;
    _lastObservedScreen = widget.currentScreen;
    _ensureScreenCached(_currentScreenKey, widget.currentScreen);
  }

  @override
  void didUpdateWidget(covariant AppContent oldWidget) {
    super.didUpdateWidget(oldWidget);
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
    _pendingRemovalKey = widget.isNavigatingBack ? fromKey : null;
    _isTransitioning = canCrossfade;
    _lastObservedCurrentKey = currentScreenKey;
    _lastObservedScreen = currentScreen;

    if (!canCrossfade) {
      _removePendingScreen(currentScreenKey);
      return;
    }

    Future<void>.delayed(_activeTransitionDuration, () {
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

  bool get _isDrawerRelayTransition {
    return !widget.useTabletLayout &&
        widget.navigationTransitionSource ==
            NavigationTransitionSource.drawer &&
        !widget.isNavigatingBack &&
        _transitionAllowsCrossfade;
  }

  Duration get _pageTransitionDuration {
    return widget.enableNavigationAnimation
        ? _enabledPageTransitionDuration
        : _disabledPageTransitionDuration;
  }

  Duration get _activeTransitionDuration {
    return _isDrawerRelayTransition
        ? _drawerRelayTransitionDuration
        : _pageTransitionDuration;
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
    final themeSnapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final backgroundVisible =
        themeSnapshot.useBackgroundImage &&
        themeSnapshot.backgroundImageUri != null &&
        themeSnapshot.backgroundImageUri!.isNotEmpty;
    final transparentSurface = themeSnapshot.transparentSurfaceEnabled;
    final contentColor = backgroundVisible || transparentSurface
        ? Colors.transparent
        : theme.colorScheme.surface;
    final appBarContentColor = theme.colorScheme.onSurface;
    final topPadding = MediaQuery.paddingOf(context).top;
    final mainLayoutController = MainLayoutScope.of(context);
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

    return AnimatedBuilder(
      animation: mainLayoutController,
      builder: (context, _) {
        final frame = Column(
          children: <Widget>[
            AnimatedBuilder(
              animation: widget.topBarController,
              builder: (context, _) {
                final titleContent = widget.topBarController.titleContent;
                final actions = widget.topBarController.actions;
                final navigationIcon = widget.canGoBack
                    ? Icons.arrow_back
                    : widget.useTabletLayout && widget.isTabletSidebarExpanded
                    ? Icons.chevron_left
                    : Icons.segment;
                final navigationIconWidget = Icon(
                  navigationIcon,
                  color: appBarContentColor,
                );
                final shouldFlipNavigationIcon =
                    !widget.canGoBack &&
                    !(widget.useTabletLayout && widget.isTabletSidebarExpanded);
                return ColoredBox(
                  color: contentColor,
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
                              icon: shouldFlipNavigationIcon
                                  ? Transform(
                                      alignment: Alignment.center,
                                      transform: Matrix4.identity()
                                        ..scaleByDouble(-1.0, 1.0, 1.0, 1.0),
                                      child: navigationIconWidget,
                                    )
                                  : navigationIconWidget,
                              tooltip: widget.canGoBack
                                  ? 'Back'
                                  : widget.useTabletLayout &&
                                        widget.isTabletSidebarExpanded
                                  ? 'Collapse sidebar'
                                  : 'Navigation',
                            ),
                          ),
                          Expanded(
                            child:
                                titleContent?.content(context) ??
                                TopBarTitleText(
                                  primaryText: widget.currentRouteTitle,
                                  contentColor: appBarContentColor,
                                ),
                          ),
                          if (actions != null) ...actions(context),
                        ],
                      ),
                    ),
                  ),
                );
              },
            ),
            Expanded(
              child: ColoredBox(
                color: contentColor,
                child: Stack(
                  children: <Widget>[
                    for (final screenKey in renderKeys)
                      _AnimatedScreenSlot(
                        key: ValueKey<String>(screenKey),
                        screenKey: screenKey,
                        isCurrentScreen: screenKey == currentScreenKey,
                        isNavigatingBack: widget.isNavigatingBack,
                        enableNavigationAnimation:
                            widget.enableNavigationAnimation,
                        isDrawerRelayTransition: _isDrawerRelayTransition,
                        allowCrossfade: _transitionAllowsCrossfade,
                        duration: _activeTransitionDuration,
                        pageOffset: widget.useTabletLayout
                            ? _tabletPageTransitionOffset
                            : _phonePageTransitionOffset,
                        drawerNavigationOffset: _phoneDrawerNavigationOffset,
                        child: MainScreenActivityScope(
                          isCurrentScreen: screenKey == currentScreenKey,
                          child: _screenCache[screenKey]!,
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ],
        );
        return SizedBox.expand(
          child: mainLayoutController.decorate(context, frame),
        );
      },
    );
  }
}

class _AnimatedScreenSlot extends StatefulWidget {
  const _AnimatedScreenSlot({
    super.key,
    required this.screenKey,
    required this.isCurrentScreen,
    required this.isNavigatingBack,
    required this.enableNavigationAnimation,
    required this.isDrawerRelayTransition,
    required this.allowCrossfade,
    required this.duration,
    required this.pageOffset,
    required this.drawerNavigationOffset,
    required this.child,
  });

  final String screenKey;
  final bool isCurrentScreen;
  final bool isNavigatingBack;
  final bool enableNavigationAnimation;
  final bool isDrawerRelayTransition;
  final bool allowCrossfade;
  final Duration duration;
  final double pageOffset;
  final double drawerNavigationOffset;
  final Widget child;

  @override
  State<_AnimatedScreenSlot> createState() => _AnimatedScreenSlotState();
}

class _AnimatedScreenSlotState extends State<_AnimatedScreenSlot> {
  bool _visible = false;
  int _showRequestId = 0;

  @override
  void initState() {
    super.initState();
    if (widget.isCurrentScreen) {
      _scheduleShow();
    }
  }

  @override
  void didUpdateWidget(covariant _AnimatedScreenSlot oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.isCurrentScreen == widget.isCurrentScreen) {
      return;
    }
    if (widget.isCurrentScreen) {
      _visible = false;
      _scheduleShow();
      return;
    }
    _showRequestId++;
    _visible = false;
  }

  void _scheduleShow() {
    final requestId = ++_showRequestId;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || requestId != _showRequestId || !widget.isCurrentScreen) {
        return;
      }
      setState(() {
        _visible = true;
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    final targetOpacity = _targetOpacity;
    final targetScale = _targetScale;
    final targetTranslationX = _targetTranslationX;

    return Positioned.fill(
      child: IgnorePointer(
        ignoring: !widget.isCurrentScreen,
        child: AnimatedOpacity(
          opacity: targetOpacity,
          duration: widget.duration,
          curve: widget.isDrawerRelayTransition
              ? _alphaCurve
              : widget.enableNavigationAnimation
              ? _alphaCurve
              : Curves.fastOutSlowIn,
          child: TweenAnimationBuilder<double>(
            tween: Tween<double>(end: targetTranslationX),
            duration: widget.duration,
            curve: Curves.fastOutSlowIn,
            builder: (context, translationX, child) {
              return Transform.translate(
                offset: Offset(translationX, 0),
                child: child,
              );
            },
            child: TweenAnimationBuilder<double>(
              tween: Tween<double>(end: targetScale),
              duration: widget.duration,
              curve: Curves.fastOutSlowIn,
              builder: (context, scale, child) {
                return Transform.scale(scale: scale, child: child);
              },
              child: widget.child,
            ),
          ),
        ),
      ),
    );
  }

  Curve get _alphaCurve {
    return _visible ? Curves.linearToEaseOut : Curves.easeInToLinear;
  }

  double get _targetOpacity {
    if (!widget.allowCrossfade) {
      return 1.0;
    }
    return _visible ? 1.0 : 0.0;
  }

  double get _targetTranslationX {
    if (!widget.allowCrossfade) {
      return 0.0;
    }
    if (widget.isDrawerRelayTransition) {
      if (_visible) {
        return 0.0;
      }
      if (widget.isCurrentScreen) {
        return -widget.drawerNavigationOffset;
      }
      return widget.drawerNavigationOffset * 0.18;
    }
    if (!widget.enableNavigationAnimation) {
      return 0.0;
    }
    if (_visible) {
      return 0.0;
    }
    if (widget.isCurrentScreen) {
      return widget.isNavigatingBack ? -widget.pageOffset : widget.pageOffset;
    }
    return widget.isNavigatingBack
        ? widget.pageOffset * 0.45
        : -widget.pageOffset * 0.45;
  }

  double get _targetScale {
    if (!widget.allowCrossfade) {
      return 1.0;
    }
    if (widget.isDrawerRelayTransition) {
      if (_visible) {
        return 1.0;
      }
      if (widget.isCurrentScreen) {
        return 0.975;
      }
      return 0.995;
    }
    if (!widget.enableNavigationAnimation) {
      return 1.0;
    }
    if (_visible) {
      return 1.0;
    }
    return widget.isCurrentScreen ? 0.985 : 0.992;
  }
}
