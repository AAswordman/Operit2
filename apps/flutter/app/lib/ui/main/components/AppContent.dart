// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../common/components/RetainedPage.dart';
import '../MainLayoutController.dart';
import '../../theme/OperitTheme.dart';
import '../TopBarController.dart';
import '../navigation/AppNavigationModels.dart';
import '../layout/DrawerMotionScope.dart';
import '../screens/OperitScreens.dart';
import 'TopBarTitleText.dart';

class AppContent extends StatefulWidget {
  /// Creates a page host whose cached screens retain their transition state.
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
    required this.isNavigatingBack,
    required this.topBarController,
    required this.appBarEntries,
    required this.onGoBack,
    required this.onNavigationButtonPressed,
    required this.onAppBarEntrySelected,
  });

  final AppRouterState routerState;
  final OperitScreen currentScreen;
  final RouteEntry currentRouteEntry;
  final String currentRouteTitle;
  final bool useTabletLayout;
  final bool isTabletSidebarExpanded;
  final bool canGoBack;
  final bool enableNavigationAnimation;
  final bool isNavigatingBack;
  final TopBarController topBarController;
  final List<NavigationEntrySpec> appBarEntries;
  final VoidCallback onGoBack;
  final VoidCallback onNavigationButtonPressed;
  final ValueChanged<NavigationEntrySpec> onAppBarEntrySelected;

  /// Creates the cached page and transition lifecycle owner.
  @override
  State<AppContent> createState() => _AppContentState();
}

class _AppContentState extends State<AppContent> {
  static const Duration _enabledPageTransitionDuration = Duration(
    milliseconds: 240,
  );
  static const Duration _disabledPageTransitionDuration = Duration(
    milliseconds: 400,
  );
  static const double _phonePageTransitionOffset = 12;
  static const double _tabletPageTransitionOffset = 16;

  final Map<String, Widget> _screenCache = <String, Widget>{};
  final Map<String, bool> _screenKeepAliveCache = <String, bool>{};

  String? _lastObservedCurrentKey;
  OperitScreen? _lastObservedScreen;
  String? _transitionFromKey;
  String? _pendingRemovalKey;
  bool _isTransitioning = false;
  bool _transitionAllowsCrossfade = true;
  Timer? _transitionTimer;
  ValueListenable<bool>? _drawerMotion;
  ValueListenable<int>? _drawerContentActivation;
  String? _drawerSnapshotScreenKey;

  /// Mounts the initial page without marking it as an outgoing transition.
  @override
  void initState() {
    super.initState();
    _lastObservedCurrentKey = _currentScreenKey;
    _lastObservedScreen = widget.currentScreen;
    _ensureScreenCached(_currentScreenKey, widget.currentScreen);
  }

  /// Connects drawer motion to the page that was current when motion began.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final scope = DrawerMotionScope.maybeOf(context);
    if (_drawerMotion == scope?.isAnimating &&
        _drawerContentActivation == scope?.contentActivation) {
      return;
    }
    _drawerMotion?.removeListener(_handleDrawerMotion);
    _drawerContentActivation?.removeListener(_handleDrawerContentActivation);
    _drawerMotion = scope?.isAnimating;
    _drawerContentActivation = scope?.contentActivation;
    _drawerMotion?.addListener(_handleDrawerMotion);
    _drawerContentActivation?.addListener(_handleDrawerContentActivation);
    _drawerSnapshotScreenKey = null;
  }

  /// Freezes only an established page, leaving all entering pages live.
  void _handleDrawerMotion() {
    setState(() {
      _drawerSnapshotScreenKey =
          _drawerMotion?.value == true && !_isTransitioning
          ? _currentScreenKey
          : null;
    });
  }

  /// Releases a current-page snapshot when drawer content is activated.
  void _handleDrawerContentActivation() {
    setState(() {
      _drawerSnapshotScreenKey = null;
    });
  }

  /// Caches the new page and retargets the active page transition.
  @override
  void didUpdateWidget(covariant AppContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    final currentScreenKey = _currentScreenKey;
    if (oldWidget.currentRouteEntry.instanceId !=
        widget.currentRouteEntry.instanceId) {
      _drawerSnapshotScreenKey = null;
    }
    _ensureScreenCached(currentScreenKey, widget.currentScreen);
    _updateTransition(currentScreenKey, widget.currentScreen);
  }

  /// Resolves the page identity used by both caching and snapshot ownership.
  String get _currentScreenKey {
    return widget.currentScreen.stableScreenKey() ??
        widget.currentRouteEntry.instanceId;
  }

  /// Retains each page widget independently of its visibility in the stack.
  void _ensureScreenCached(String screenKey, OperitScreen screen) {
    _screenKeepAliveCache[screenKey] = screen.keepAlive;
    _screenCache.putIfAbsent(screenKey, () => Builder(builder: screen.build));
  }

  /// Starts the current transition and cancels cleanup from its predecessor.
  void _updateTransition(String currentScreenKey, OperitScreen currentScreen) {
    final fromKey = _lastObservedCurrentKey;
    final fromScreen = _lastObservedScreen;
    if (fromKey == null || fromScreen == null || currentScreenKey == fromKey) {
      return;
    }

    _drawerSnapshotScreenKey = null;
    _transitionTimer?.cancel();
    _removePendingScreen(currentScreenKey);
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

    _transitionTimer = Timer(_activeTransitionDuration, () {
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

  /// Cancels transition cleanup when the main content host is removed.
  @override
  void dispose() {
    _drawerMotion?.removeListener(_handleDrawerMotion);
    _drawerContentActivation?.removeListener(_handleDrawerContentActivation);
    _transitionTimer?.cancel();
    super.dispose();
  }

  /// Returns the configured page motion duration.
  Duration get _pageTransitionDuration {
    return widget.enableNavigationAnimation
        ? _enabledPageTransitionDuration
        : _disabledPageTransitionDuration;
  }

  /// Returns the duration used to clean up the active transition.
  Duration get _activeTransitionDuration {
    return _pageTransitionDuration;
  }

  /// Drops the screen left behind by a back navigation, keeping keep-alive
  /// screens cached so their scroll position and state survive navigation.
  void _removePendingScreen(String currentScreenKey) {
    final keyToRemove = _pendingRemovalKey;
    if (keyToRemove != null &&
        keyToRemove != currentScreenKey &&
        _screenKeepAliveCache[keyToRemove] != true) {
      _screenCache.remove(keyToRemove);
      _screenKeepAliveCache.remove(keyToRemove);
    }
    _pendingRemovalKey = null;
  }

  /// Builds live transition containers around individually snapshotted pages.
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

    final renderKeys = <String>[
      for (final entry in _screenKeepAliveCache.entries)
        if (entry.value &&
            entry.key != currentScreenKey &&
            entry.key != effectivePreviousKey)
          entry.key,
      currentScreenKey,
      if (effectivePreviousKey != null &&
          effectivePreviousKey != currentScreenKey)
        effectivePreviousKey,
    ];

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
                final shouldFlipNavigationIcon =
                    !widget.canGoBack &&
                    !(widget.useTabletLayout && widget.isTabletSidebarExpanded);
                return ThemeTopBarChrome(
                  title:
                      titleContent?.content(context) ??
                      TopBarTitleText(
                        primaryText: widget.currentRouteTitle,
                        contentColor: appBarContentColor,
                      ),
                  navigationIcon: navigationIcon,
                  navigationTooltip: widget.canGoBack
                      ? 'Back'
                      : widget.useTabletLayout && widget.isTabletSidebarExpanded
                      ? 'Collapse sidebar'
                      : 'Navigation',
                  onNavigationPressed: widget.canGoBack
                      ? widget.onGoBack
                      : widget.onNavigationButtonPressed,
                  contentColor: contentColor,
                  contentForeground: appBarContentColor,
                  topPadding: topPadding,
                  flipNavigationIcon: shouldFlipNavigationIcon,
                  actions: <Widget>[
                    if (actions != null) ...actions(context),
                    for (final entry in widget.appBarEntries)
                      IconButton(
                        tooltip: entry.title,
                        onPressed: () => widget.onAppBarEntrySelected(entry),
                        icon: Icon(entry.icon, color: appBarContentColor),
                      ),
                  ],
                );
              },
            ),
            Expanded(
              child: ColoredBox(
                color: contentColor,
                child: Stack(
                  fit: StackFit.expand,
                  children: <Widget>[
                    for (final screenKey in renderKeys)
                      _AnimatedScreenSlot(
                        key: ValueKey<String>(screenKey),
                        screenKey: screenKey,
                        isActiveInStack:
                            screenKey == currentScreenKey ||
                            screenKey == effectivePreviousKey,
                        isCurrentScreen: screenKey == currentScreenKey,
                        snapshotDuringExit:
                            screenKey == effectivePreviousKey &&
                            screenKey != currentScreenKey,
                        snapshotDuringDrawerMotion:
                            screenKey == _drawerSnapshotScreenKey &&
                            screenKey == currentScreenKey,
                        isNavigatingBack: widget.isNavigatingBack,
                        enableNavigationAnimation:
                            widget.enableNavigationAnimation,
                        allowCrossfade: _transitionAllowsCrossfade,
                        duration: _activeTransitionDuration,
                        pageOffset: widget.useTabletLayout
                            ? _tabletPageTransitionOffset
                            : _phonePageTransitionOffset,
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
        // Scaffold avoids the keyboard, but not system navigation controls.
        // Pages that own the bottom inset can extend their surfaces beneath
        // system controls. Other pages keep the shared safe boundary. The top
        // bar already handles the status-bar padding above, and Scaffold owns
        // the keyboard inset.
        return SafeArea(
          top: false,
          bottom: !widget.currentScreen.handlesBottomSafeArea,
          child: SizedBox.expand(
            child: mainLayoutController.decorate(context, frame),
          ),
        );
      },
    );
  }
}

class _AnimatedScreenSlot extends StatefulWidget {
  /// Creates a stable page slot with independent snapshot and motion controls.
  const _AnimatedScreenSlot({
    super.key,
    required this.screenKey,
    required this.isActiveInStack,
    required this.isCurrentScreen,
    required this.snapshotDuringExit,
    required this.snapshotDuringDrawerMotion,
    required this.isNavigatingBack,
    required this.enableNavigationAnimation,
    required this.allowCrossfade,
    required this.duration,
    required this.pageOffset,
    required this.child,
  });

  final String screenKey;
  final bool isActiveInStack;
  final bool isCurrentScreen;
  final bool snapshotDuringExit;
  final bool snapshotDuringDrawerMotion;
  final bool isNavigatingBack;
  final bool enableNavigationAnimation;
  final bool allowCrossfade;
  final Duration duration;
  final double pageOffset;
  final Widget child;

  /// Creates the page-local snapshot owner and visibility state.
  @override
  State<_AnimatedScreenSlot> createState() => _AnimatedScreenSlotState();
}

class _AnimatedScreenSlotState extends State<_AnimatedScreenSlot> {
  static const Duration _exitPageFadeDuration = Duration(milliseconds: 110);

  late final SnapshotController _snapshotController;
  bool _visible = false;
  int _showRequestId = 0;

  /// Keeps entering pages live and schedules their entrance motion.
  @override
  void initState() {
    super.initState();
    _snapshotController = SnapshotController(
      allowSnapshotting: _shouldSnapshot,
    );
    if (widget.isCurrentScreen) {
      _scheduleShow();
    }
  }

  /// Retargets visible pages directly so interrupted transitions stay continuous.
  @override
  void didUpdateWidget(covariant _AnimatedScreenSlot oldWidget) {
    super.didUpdateWidget(oldWidget);
    _snapshotController.allowSnapshotting = _shouldSnapshot;
    if (oldWidget.isCurrentScreen == widget.isCurrentScreen) {
      return;
    }
    if (widget.isCurrentScreen) {
      if (oldWidget.isActiveInStack) {
        _visible = true;
        return;
      }
      _visible = false;
      _scheduleShow();
      return;
    }
    _showRequestId++;
    _visible = false;
  }

  /// Limits freezing to this page's drawer motion or outgoing page transition.
  bool get _shouldSnapshot =>
      widget.snapshotDuringExit || widget.snapshotDuringDrawerMotion;

  /// Starts entrance motion on the first frame after mounting a page.
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

  /// Releases this page's captured image and snapshot notifications.
  @override
  void dispose() {
    _snapshotController.dispose();
    super.dispose();
  }

  /// Keeps the page ancestry stable across visible, exiting and offstage roles.
  @override
  Widget build(BuildContext context) {
    final targetOpacity = widget.snapshotDuringExit ? _targetOpacity : 1.0;
    final targetTranslationX = _targetTranslationX;
    final opacityDuration = widget.snapshotDuringExit
        ? _exitPageFadeDuration
        : widget.duration;
    final screenChild = RepaintBoundary(
      child: SnapshotWidget(
        controller: _snapshotController,
        mode: SnapshotMode.forced,
        autoresize: true,
        child: RepaintBoundary(child: widget.child),
      ),
    );

    final animatedScreen = IgnorePointer(
      ignoring: !widget.isCurrentScreen,
      child: AnimatedOpacity(
        opacity: targetOpacity,
        duration: opacityDuration,
        curve: Curves.easeOutCubic,
        child: TweenAnimationBuilder<double>(
          tween: Tween<double>(end: targetTranslationX),
          duration: widget.duration,
          curve: Curves.easeOutCubic,
          builder: (context, translationX, child) {
            return Transform.translate(
              offset: Offset(translationX, 0),
              child: child,
            );
          },
          child: screenChild,
        ),
      ),
    );

    return Positioned.fill(
      child: RetainedPage(
        active: widget.isActiveInStack,
        child: animatedScreen,
      ),
    );
  }

  /// Fades only the outgoing page, never a drawer-motion snapshot.
  double get _targetOpacity {
    if (!widget.allowCrossfade) {
      return 1.0;
    }
    return _visible ? 1.0 : 0.0;
  }

  /// Resolves the page-local translation outside its captured content.
  double get _targetTranslationX {
    if (!widget.allowCrossfade) {
      return 0.0;
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
}
