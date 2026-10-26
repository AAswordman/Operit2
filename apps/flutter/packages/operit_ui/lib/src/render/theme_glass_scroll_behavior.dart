import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/physics.dart';
import 'package:flutter/services.dart';

/// Keeps inherited scroll behavior while composing stretch with a live backdrop.
class ThemeGlassScrollConfiguration extends StatelessWidget {
  /// Creates the scroll configuration used by frosted surface collections.
  const ThemeGlassScrollConfiguration({super.key, required this.child});

  final Widget child;

  /// Adapts the inherited indicator without changing input or scroll physics.
  @override
  Widget build(BuildContext context) {
    return ScrollConfiguration(
      behavior: _ThemeGlassScrollBehavior(ScrollConfiguration.of(context)),
      child: child,
    );
  }
}

class _ThemeGlassScrollBehavior extends ScrollBehavior {
  /// Retains the application's configured scrolling policies.
  const _ThemeGlassScrollBehavior(this.delegate);

  final ScrollBehavior delegate;

  /// Preserves the accepted touch, mouse, pen, and trackpad devices.
  @override
  Set<PointerDeviceKind> get dragDevices => delegate.dragDevices;

  /// Preserves keyboard modifiers used to change pointer scroll axes.
  @override
  Set<LogicalKeyboardKey> get pointerAxisModifiers =>
      delegate.pointerAxisModifiers;

  /// Leaves platform selection with Flutter's existing compatibility layer.
  @override
  TargetPlatform getPlatform(BuildContext context) =>
      delegate.getPlatform(context);

  /// Preserves the inherited policy for combining simultaneous pointer drags.
  @override
  MultitouchDragStrategy getMultitouchDragStrategy(BuildContext context) =>
      delegate.getMultitouchDragStrategy(context);

  /// Preserves the inherited scrollbar presentation.
  @override
  Widget buildScrollbar(
    BuildContext context,
    Widget child,
    ScrollableDetails details,
  ) => delegate.buildScrollbar(context, child, details);

  /// Replaces image-based stretch with a transform that keeps backdrop sampling live.
  @override
  Widget buildOverscrollIndicator(
    BuildContext context,
    Widget child,
    ScrollableDetails details,
  ) {
    final indicator = delegate.buildOverscrollIndicator(
      context,
      child,
      details,
    );
    return switch (indicator) {
      StretchingOverscrollIndicator() => ThemeGlassStretchIndicator(
        axisDirection: indicator.axisDirection,
        notificationPredicate: indicator.notificationPredicate,
        clipBehavior: indicator.clipBehavior,
        child: child,
      ),
      _ => indicator,
    };
  }

  /// Preserves the inherited fling velocity estimator.
  @override
  GestureVelocityTrackerBuilder velocityTrackerBuilder(BuildContext context) =>
      delegate.velocityTrackerBuilder(context);

  /// Preserves native clamping, bouncing, and application-specific scroll physics.
  @override
  ScrollPhysics getScrollPhysics(BuildContext context) =>
      delegate.getScrollPhysics(context);

  /// Preserves the inherited keyboard dismissal policy.
  @override
  ScrollViewKeyboardDismissBehavior getKeyboardDismissBehavior(
    BuildContext context,
  ) => delegate.getKeyboardDismissBehavior(context);

  /// Reconfigures descendants when the inherited scrolling policies change.
  @override
  bool shouldNotify(covariant _ThemeGlassScrollBehavior oldDelegate) =>
      delegate != oldDelegate.delegate;
}

/// Stretches the live scroll layer so its frost can still sample external paint.
class ThemeGlassStretchIndicator extends StatefulWidget {
  /// Creates an indicator with Flutter's edge response and spring timing.
  const ThemeGlassStretchIndicator({
    super.key,
    required this.axisDirection,
    this.notificationPredicate = defaultScrollNotificationPredicate,
    this.clipBehavior = Clip.hardEdge,
    required this.child,
  });

  final AxisDirection axisDirection;
  final ScrollNotificationPredicate notificationPredicate;
  final Clip clipBehavior;
  final Widget child;

  /// Creates the owner of the live stretch and its release animation.
  @override
  State<ThemeGlassStretchIndicator> createState() =>
      _ThemeGlassStretchIndicatorState();
}

class _ThemeGlassStretchIndicatorState extends State<ThemeGlassStretchIndicator>
    with SingleTickerProviderStateMixin {
  // Match Flutter's stretch response, damping, and corrected spring frequency.
  static final _springDescription = SpringDescription.withDampingRatio(
    mass: 1,
    stiffness: 24.657 * 24.657 * 0.8 * 0.8,
    ratio: 0.98,
  );
  late final _stretch = AnimationController.unbounded(vsync: this);
  double _totalOverscroll = 0;
  double _interruptedStretch = 0;
  bool _previousWasOverscroll = false;
  bool _accepted = true;
  int _animationGeneration = 0;

  /// Resolves the physical axis from Flutter's scroll direction.
  Axis get _axis => axisDirectionToAxis(widget.axisDirection);

  /// Tracks direct pulls, edge impacts, and release without consuming notifications.
  bool _handleScrollNotification(ScrollNotification notification) {
    if (!widget.notificationPredicate(notification) ||
        notification.metrics.axis != _axis) {
      return false;
    }
    if (notification is OverscrollNotification) {
      if (!_previousWasOverscroll) {
        final indication = _ThemeGlassOverscrollNotification(
          leading: notification.overscroll < 0,
        )..dispatch(context);
        _accepted = indication.isAccepted;
      }
      if (_accepted) {
        _totalOverscroll += notification.overscroll;
        if (notification.velocity != 0) {
          _animate((notification.velocity / 3000).clamp(-1.25, 1.25));
        } else if (notification.dragDetails != null) {
          _pull(
            (_totalOverscroll / notification.metrics.viewportDimension).clamp(
              -1,
              1,
            ),
          );
        }
      }
    } else if (notification is ScrollEndNotification) {
      _totalOverscroll = 0;
      if (!_stretch.isAnimating && _stretch.value != 0) {
        final velocity = notification.dragDetails?.velocity.pixelsPerSecond;
        var mainVelocity = switch (_axis) {
          Axis.horizontal => velocity?.dx ?? 0,
          Axis.vertical => velocity?.dy ?? 0,
        };
        if (axisDirectionIsReversed(widget.axisDirection)) {
          mainVelocity = -mainVelocity;
        }
        _animate((-mainVelocity / 6000).clamp(-0.5, 0.5));
      }
    }
    _previousWasOverscroll = notification is OverscrollNotification;
    return false;
  }

  /// Applies Flutter's bounded linear and exponential stretch response.
  void _pull(double distance) {
    if (_stretch.isAnimating) {
      _interruptedStretch = _stretch.value;
      _animationGeneration++;
      _stretch.stop();
    }
    final magnitude = distance.abs();
    final intensity =
        0.016 * (magnitude + 1 - math.exp(-magnitude * math.e / 0.33));
    _stretch.value = (distance.sign * intensity + _interruptedStretch).clamp(
      -1,
      1,
    );
  }

  /// Returns the live layer to its resting geometry with Flutter's stretch spring.
  void _animate(double velocity) {
    final generation = ++_animationGeneration;
    unawaited(
      _stretch
          .animateWith(
            SpringSimulation(
              _springDescription,
              _stretch.value,
              0,
              velocity * 0.8,
            ),
          )
          .then((_) {
            if (mounted && generation == _animationGeneration) {
              _interruptedStretch = 0;
              _stretch.value = 0;
            }
          }),
    );
  }

  /// Releases the stretch ticker when the scrollable leaves the tree.
  @override
  void dispose() {
    _animationGeneration++;
    _stretch.dispose();
    super.dispose();
  }

  /// Transforms retained live layers without creating an image-filter save layer.
  @override
  Widget build(BuildContext context) {
    return NotificationListener<ScrollNotification>(
      onNotification: _handleScrollNotification,
      child: AnimatedBuilder(
        animation: _stretch,
        child: widget.child,
        builder: (context, child) {
          var strength = -_stretch.value;
          if (axisDirectionIsReversed(widget.axisDirection)) {
            strength = -strength;
          }
          final alignment = switch (_axis) {
            Axis.horizontal =>
              strength >= 0 ? Alignment.centerLeft : Alignment.centerRight,
            Axis.vertical =>
              strength >= 0 ? Alignment.topCenter : Alignment.bottomCenter,
          };
          return ClipRect(
            clipBehavior: widget.clipBehavior,
            child: Transform(
              alignment: alignment,
              transform: Matrix4.diagonal3Values(
                _axis == Axis.horizontal ? 1 + strength.abs() : 1,
                _axis == Axis.vertical ? 1 + strength.abs() : 1,
                1,
              ),
              child: child,
            ),
          );
        },
      ),
    );
  }
}

class _ThemeGlassOverscrollNotification
    extends OverscrollIndicatorNotification {
  /// Preserves Flutter's notification contract for suppressing edge indicators.
  _ThemeGlassOverscrollNotification({required super.leading});

  /// Exposes the inherited acceptance state to this indicator implementation.
  bool get isAccepted => accepted;
}
