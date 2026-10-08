// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/physics.dart';
import 'package:flutter/rendering.dart';

import '../components/DrawerConversationState.dart';
import '../components/DrawerContent.dart';
import '../components/NavigationDrawerAppearance.dart';
import '../navigation/AppNavigationModels.dart';
import 'DrawerMotionScope.dart';
import '../../theme/OperitGlassSurface.dart';
import '../../common/interactions/DrawerGestureExclusion.dart';

class PhoneLayout extends StatefulWidget {
  /// Creates the retained phone content and its animated navigation drawer.
  const PhoneLayout({
    super.key,
    required this.content,
    required this.navigationEntries,
    required this.pluginSidebarEntries,
    required this.selectedRouteId,
    required this.drawerConversationState,
    required this.drawerWidth,
    required this.drawerOpenState,
    required this.enableNavigationAnimation,
    required this.onOpenDrawer,
    required this.onCloseDrawer,
    required this.onNavigationEntrySelected,
    required this.onConversationActivated,
  });

  final Widget content;
  final List<NavigationEntrySpec> navigationEntries;
  final List<NavigationEntrySpec> pluginSidebarEntries;
  final String selectedRouteId;
  final ValueListenable<DrawerConversationState> drawerConversationState;
  final double drawerWidth;
  final ValueListenable<bool> drawerOpenState;
  final bool enableNavigationAnimation;
  final VoidCallback onOpenDrawer;
  final VoidCallback onCloseDrawer;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;
  final VoidCallback onConversationActivated;

  /// Creates the state that drives drawer motion independently of page content.
  @override
  State<PhoneLayout> createState() => _PhoneLayoutState();
}

class _PhoneLayoutState extends State<PhoneLayout>
    with SingleTickerProviderStateMixin {
  static const double _lowBouncyDampingRatio = 0.75;
  static const double _noBouncyDampingRatio = 1.0;
  static const double _springStiffness = 1000;
  static const double _dragThreshold = 40;

  late final AnimationController _drawerProgressController;
  final ValueNotifier<bool> _drawerIsAnimating = ValueNotifier<bool>(false);
  final ValueNotifier<int> _contentActivation = ValueNotifier<int>(0);
  double _currentDrag = 0;
  double _verticalDrag = 0;

  /// Connects the drawer state to its spring animation and motion lifecycle.
  @override
  void initState() {
    super.initState();
    _drawerProgressController = AnimationController.unbounded(
      vsync: this,
      value: widget.drawerOpenState.value ? 1.0 : 0.0,
    );
    _drawerProgressController.addStatusListener(_handleDrawerAnimationStatus);
    widget.drawerOpenState.addListener(_animateDrawerProgress);
  }

  /// Rebinds drawer state changes while retaining the animation controller.
  @override
  void didUpdateWidget(covariant PhoneLayout oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.drawerOpenState != widget.drawerOpenState) {
      oldWidget.drawerOpenState.removeListener(_animateDrawerProgress);
      widget.drawerOpenState.addListener(_animateDrawerProgress);
      _animateDrawerProgress();
    }
  }

  /// Releases the animation and its page-local motion notification source.
  @override
  void dispose() {
    widget.drawerOpenState.removeListener(_animateDrawerProgress);
    _drawerProgressController.removeStatusListener(
      _handleDrawerAnimationStatus,
    );
    _drawerProgressController.dispose();
    _drawerIsAnimating.dispose();
    _contentActivation.dispose();
    super.dispose();
  }

  /// Notifies page owners when a drawer motion starts or finishes.
  void _handleDrawerAnimationStatus(AnimationStatus status) {
    _drawerIsAnimating.value = _drawerProgressController.isAnimating;
  }

  /// Retargets the drawer spring from its current position and velocity.
  void _animateDrawerProgress() {
    final target = widget.drawerOpenState.value ? 1.0 : 0.0;
    final dampingRatio = widget.drawerOpenState.value
        ? _lowBouncyDampingRatio
        : _noBouncyDampingRatio;
    final simulation = SpringSimulation(
      SpringDescription(
        mass: 1.0,
        stiffness: _springStiffness,
        damping: dampingRatio * 2 * math.sqrt(_springStiffness),
      ),
      _drawerProgressController.value,
      target,
      _drawerProgressController.velocity,
    );
    _drawerProgressController.animateWith(simulation);
  }

  /// Releases current-page freezing after a drawer navigation action begins.
  void _handleNavigationEntrySelected(NavigationEntrySpec entry) {
    widget.onNavigationEntrySelected(entry);
    _contentActivation.value++;
  }

  /// Keeps in-page conversation loading live when the drawer closes.
  void _handleConversationActivated() {
    widget.onConversationActivated();
    _contentActivation.value++;
  }

  /// Resets accumulated movement for a new drawer gesture.
  void _handleHorizontalDragStart(DragStartDetails details) {
    _currentDrag = 0;
    _verticalDrag = 0;
  }

  /// Opens or closes the drawer after a deliberate horizontal gesture.
  void _handleHorizontalDragUpdate(DragUpdateDetails details) {
    _currentDrag += details.primaryDelta ?? 0;
    _verticalDrag += details.delta.dy;
    if (!widget.drawerOpenState.value &&
        _currentDrag > _dragThreshold &&
        _currentDrag.abs() > _verticalDrag.abs()) {
      _currentDrag = 0;
      _verticalDrag = 0;
      widget.onOpenDrawer();
    }
    if (widget.drawerOpenState.value && _currentDrag < -_dragThreshold) {
      _currentDrag = 0;
      _verticalDrag = 0;
      widget.onCloseDrawer();
    }
  }

  /// Clears gesture movement after the pointer is released.
  void _handleHorizontalDragEnd(DragEndDetails details) {
    _currentDrag = 0;
    _verticalDrag = 0;
  }

  /// Builds retained content layers and animates their drawer presentation.
  @override
  Widget build(BuildContext context) {
    final appearance = navigationDrawerAppearanceOf(context);
    final animatedChild = _PhoneLayoutAnimatedChild(
      content: RepaintBoundary(
        child: DrawerMotionScope(
          isAnimating: _drawerIsAnimating,
          contentActivation: _contentActivation,
          child: widget.content,
        ),
      ),
      drawerContent: RepaintBoundary(
        // Keep drawer controls within the system insets while its background
        // extends beneath both system bars.
        child: SafeArea(
          left: false,
          right: false,
          child: ValueListenableBuilder<DrawerConversationState>(
            valueListenable: widget.drawerConversationState,
            builder: (context, drawerState, _) {
              return DrawerContent(
                key: const ValueKey<String>('phoneDrawerContent'),
                navigationEntries: widget.navigationEntries,
                pluginEntries: widget.pluginSidebarEntries,
                selectedRouteId: widget.selectedRouteId,
                appearance: appearance,
                histories: drawerState.histories,
                activeStreamingChatIds: drawerState.activeStreamingChatIds,
                currentChatId: drawerState.currentChatId,
                errorMessage: drawerState.errorMessage,
                loading: drawerState.loading,
                onNavigationEntrySelected: _handleNavigationEntrySelected,
                onConversationActivated: _handleConversationActivated,
              );
            },
          ),
        ),
      ),
    );

    return RawGestureDetector(
      behavior: HitTestBehavior.translucent,
      gestures: <Type, GestureRecognizerFactory>{
        _DrawerHorizontalDragGestureRecognizer:
            GestureRecognizerFactoryWithHandlers<
              _DrawerHorizontalDragGestureRecognizer
            >(_DrawerHorizontalDragGestureRecognizer.new, (recognizer) {
              recognizer
                ..gestureSettings = MediaQuery.maybeGestureSettingsOf(context)
                ..onStart = _handleHorizontalDragStart
                ..onUpdate = _handleHorizontalDragUpdate
                ..onEnd = _handleHorizontalDragEnd
                ..onCancel = () {
                  _currentDrag = 0;
                  _verticalDrag = 0;
                };
            }),
      },
      child: AnimatedBuilder(
        animation: _drawerProgressController,
        child: animatedChild,
        builder: (context, child) {
          final animatedChild = child! as _PhoneLayoutAnimatedChild;
          final drawerProgress = _drawerProgressController.value;
          final contentTranslationX = widget.enableNavigationAnimation
              ? widget.drawerWidth * (0.82 * drawerProgress)
              : widget.drawerWidth * drawerProgress;
          final contentTranslationY = widget.enableNavigationAnimation
              ? 12.0 * drawerProgress
              : 0.0;
          final contentScale = widget.enableNavigationAnimation
              ? 1.0 - (0.08 * drawerProgress)
              : 1.0;
          final contentRotationY = widget.enableNavigationAnimation
              ? -7.0 * drawerProgress
              : 0.0;
          final contentCornerRadius = widget.enableNavigationAnimation
              ? 24.0 * drawerProgress
              : 0.0;
          final drawerOffset = -widget.drawerWidth * (1.0 - drawerProgress);
          // Keep blur kernels stable while the transform layer moves.
          final contentShadowVisible = drawerProgress > 0.001;
          final contentShadowBlur = widget.enableNavigationAnimation
              ? 18.0
              : 0.0;
          final sidebarShadowBlur = widget.enableNavigationAnimation
              ? 16.0
              : 3.0;
          final drawerScale = widget.enableNavigationAnimation
              ? 0.92 + (0.08 * drawerProgress)
              : 1.0;
          final drawerContentAlpha = widget.enableNavigationAnimation
              ? 0.72 + (0.28 * drawerProgress)
              : 0.8 + (0.2 * drawerProgress);
          final clampedContentCornerRadius = math.max(
            0.0,
            math.min(contentCornerRadius, 30.0),
          );
          final clampedDrawerContentAlpha = math.max(
            0.0,
            math.min(drawerContentAlpha, 1.0),
          );
          final isDrawerOpen =
              widget.drawerOpenState.value || drawerProgress > 0.001;

          return Stack(
            children: <Widget>[
              Positioned.fill(
                child: Transform.translate(
                  offset: Offset(contentTranslationX, contentTranslationY),
                  child: Transform(
                    alignment: Alignment.centerLeft,
                    transform: Matrix4.identity()
                      ..rotateY(contentRotationY * math.pi / 180),
                    child: Transform.scale(
                      alignment: Alignment.centerLeft,
                      scale: contentScale,
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          borderRadius: BorderRadius.circular(
                            clampedContentCornerRadius,
                          ),
                          boxShadow: <BoxShadow>[
                            if (contentShadowVisible && contentShadowBlur > 0)
                              BoxShadow(
                                blurRadius: contentShadowBlur,
                                color: Colors.black.withValues(alpha: 0.16),
                              ),
                          ],
                        ),
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(
                            clampedContentCornerRadius,
                          ),
                          child: animatedChild.content,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              if (isDrawerOpen)
                Positioned.fill(
                  key: const ValueKey<String>('phoneDrawerDismissBarrier'),
                  left: widget.drawerWidth,
                  child: GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onTap: widget.onCloseDrawer,
                    child: const ColoredBox(color: Colors.transparent),
                  ),
                ),
              Positioned(
                // Preserve the drawer subtree when the dismiss barrier changes.
                key: const ValueKey<String>('phoneDrawerLayer'),
                left: 0,
                top: 0,
                bottom: 0,
                width: widget.drawerWidth,
                child: Transform.translate(
                  // A fixed Stack slot keeps drag frames in the paint phase.
                  offset: Offset(drawerOffset, 0),
                  child: Opacity(
                    opacity: clampedDrawerContentAlpha,
                    child: Transform.scale(
                      alignment: Alignment.centerLeft,
                      scale: drawerScale,
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          boxShadow: <BoxShadow>[
                            if (drawerProgress > 0.001)
                              BoxShadow(
                                blurRadius: sidebarShadowBlur,
                                color: Colors.black.withValues(alpha: 0.12),
                              ),
                          ],
                        ),
                        child: OperitGlassSurface(
                          color: appearance.containerColor,
                          layer: OperitGlassSurfaceLayer.panel,
                          transparentAlpha: 0.035,
                          enableBackdropFilter: false,
                          borderRadius: const BorderRadiusDirectional.only(
                            topEnd: Radius.circular(16),
                            bottomEnd: Radius.circular(16),
                          ).resolve(Directionality.of(context)),
                          child: animatedChild.drawerContent,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _DrawerHorizontalDragGestureRecognizer
    extends HorizontalDragGestureRecognizer {
  /// Keeps gestures beginning in editable or excluded content out of the arena.
  @override
  bool isPointerAllowed(PointerEvent event) {
    if (!super.isPointerAllowed(event)) {
      return false;
    }
    return !_startsInExcludedRegion(event);
  }

  /// Applies the same exclusion policy to pointer pan and zoom gestures.
  @override
  bool isPointerPanZoomAllowed(PointerPanZoomStartEvent event) {
    return super.isPointerPanZoomAllowed(event) &&
        !_startsInExcludedRegion(event);
  }

  /// Resolves gesture exclusions using the transformed hit-test path.
  bool _startsInExcludedRegion(PointerEvent event) {
    // Reject before entering the arena, not in onUpdate: winning the arena
    // would already cancel the text field's cursor/selection recognizers.
    // Use the actual hit-test path so zoom and animated transforms are honored.
    final result = HitTestResult();
    RendererBinding.instance.hitTestInView(
      result,
      event.position,
      event.viewId,
    );
    return result.path.any(
      (entry) =>
          entry.target is RenderEditable ||
          DrawerGestureExclusion.containsTarget(entry.target),
    );
  }
}

class _PhoneLayoutAnimatedChild extends StatelessWidget {
  /// Retains the drawer and content widgets between presentation frames.
  const _PhoneLayoutAnimatedChild({
    required this.content,
    required this.drawerContent,
  });

  final Widget content;
  final Widget drawerContent;

  /// Provides an inert animation child while its retained fields are reused.
  @override
  Widget build(BuildContext context) {
    return const SizedBox.shrink();
  }
}
