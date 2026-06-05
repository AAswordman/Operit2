// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/physics.dart';

import '../components/DrawerContent.dart';
import '../components/NavigationDrawerAppearance.dart';
import '../navigation/AppNavigationModels.dart';
import '../../theme/OperitGlassSurface.dart';

class PhoneLayout extends StatefulWidget {
  const PhoneLayout({
    super.key,
    required this.content,
    required this.navigationEntries,
    required this.selectedRouteId,
    required this.drawerWidth,
    required this.drawerOpen,
    required this.enableNavigationAnimation,
    required this.onOpenDrawer,
    required this.onCloseDrawer,
    required this.onNavigationEntrySelected,
    required this.onConversationActivated,
  });

  final Widget content;
  final List<NavigationEntrySpec> navigationEntries;
  final String selectedRouteId;
  final double drawerWidth;
  final bool drawerOpen;
  final bool enableNavigationAnimation;
  final VoidCallback onOpenDrawer;
  final VoidCallback onCloseDrawer;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;
  final VoidCallback onConversationActivated;

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
  double _currentDrag = 0;
  double _verticalDrag = 0;

  @override
  void initState() {
    super.initState();
    _drawerProgressController = AnimationController.unbounded(
      vsync: this,
      value: widget.drawerOpen ? 1.0 : 0.0,
    );
  }

  @override
  void didUpdateWidget(covariant PhoneLayout oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.drawerOpen != widget.drawerOpen) {
      _animateDrawerProgress();
    }
  }

  @override
  void dispose() {
    _drawerProgressController.dispose();
    super.dispose();
  }

  void _animateDrawerProgress() {
    final target = widget.drawerOpen ? 1.0 : 0.0;
    final dampingRatio = widget.drawerOpen
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

  void _handleHorizontalDragStart(DragStartDetails details) {
    _currentDrag = 0;
    _verticalDrag = 0;
  }

  void _handleHorizontalDragUpdate(DragUpdateDetails details) {
    _currentDrag += details.primaryDelta ?? 0;
    _verticalDrag += details.delta.dy;
    if (!widget.drawerOpen &&
        _currentDrag > _dragThreshold &&
        _currentDrag.abs() > _verticalDrag.abs()) {
      _currentDrag = 0;
      _verticalDrag = 0;
      widget.onOpenDrawer();
    }
    if (widget.drawerOpen && _currentDrag < -_dragThreshold) {
      _currentDrag = 0;
      _verticalDrag = 0;
      widget.onCloseDrawer();
    }
  }

  void _handleHorizontalDragEnd(DragEndDetails details) {
    _currentDrag = 0;
    _verticalDrag = 0;
  }

  @override
  Widget build(BuildContext context) {
    final appearance = navigationDrawerAppearanceOf(context);
    final animatedChild = _PhoneLayoutAnimatedChild(
      content: RepaintBoundary(child: widget.content),
      drawerContent: RepaintBoundary(
        child: DrawerContent(
          navigationEntries: widget.navigationEntries,
          selectedRouteId: widget.selectedRouteId,
          appearance: appearance,
          onNavigationEntrySelected: widget.onNavigationEntrySelected,
          onConversationActivated: widget.onConversationActivated,
        ),
      ),
    );

    return GestureDetector(
      behavior: HitTestBehavior.translucent,
      onHorizontalDragStart: _handleHorizontalDragStart,
      onHorizontalDragUpdate: _handleHorizontalDragUpdate,
      onHorizontalDragEnd: _handleHorizontalDragEnd,
      onHorizontalDragCancel: () {
        _currentDrag = 0;
        _verticalDrag = 0;
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
          final contentShadowElevation = widget.enableNavigationAnimation
              ? 18.0 * drawerProgress
              : 0.0;
          final drawerOffset = -widget.drawerWidth * (1.0 - drawerProgress);
          final sidebarElevation = widget.enableNavigationAnimation
              ? 16.0 * drawerProgress
              : 3.0 * drawerProgress;
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
          final isDrawerOpen = widget.drawerOpen || drawerProgress > 0.001;

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
                            if (contentShadowElevation > 0)
                              BoxShadow(
                                blurRadius: contentShadowElevation,
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
                  left: widget.drawerWidth,
                  child: GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onTap: widget.onCloseDrawer,
                    child: const ColoredBox(color: Colors.transparent),
                  ),
                ),
              Positioned(
                left: drawerOffset,
                top: MediaQuery.paddingOf(context).top,
                bottom: 0,
                width: widget.drawerWidth,
                child: Opacity(
                  opacity: clampedDrawerContentAlpha,
                  child: Transform.scale(
                    alignment: Alignment.centerLeft,
                    scale: drawerScale,
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        boxShadow: <BoxShadow>[
                          if (sidebarElevation > 0)
                            BoxShadow(
                              blurRadius: sidebarElevation,
                              color: Colors.black.withValues(alpha: 0.12),
                            ),
                        ],
                      ),
                      child: OperitGlassSurface(
                        color: appearance.containerColor,
                        layer: OperitGlassSurfaceLayer.panel,
                        transparentAlpha: 0.035,
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
            ],
          );
        },
      ),
    );
  }
}

class _PhoneLayoutAnimatedChild extends StatelessWidget {
  const _PhoneLayoutAnimatedChild({
    required this.content,
    required this.drawerContent,
  });

  final Widget content;
  final Widget drawerContent;

  @override
  Widget build(BuildContext context) {
    return const SizedBox.shrink();
  }
}
