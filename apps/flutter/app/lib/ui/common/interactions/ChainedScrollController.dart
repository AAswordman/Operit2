// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart' show precisionErrorTolerance;
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Lets capped message content hand unused user scrolling to its ancestors.
class ChainedScrollController extends ScrollController {
  ChainedScrollController({super.debugLabel});

  @override
  ScrollPosition createScrollPosition(
    ScrollPhysics physics,
    ScrollContext context,
    ScrollPosition? oldPosition,
  ) {
    return _ChainedScrollPosition(
      // An inner bounce would consume movement that belongs to the parent.
      physics: ClampingScrollPhysics(parent: physics),
      context: context,
      initialPixels: initialScrollOffset,
      keepScrollOffset: keepScrollOffset,
      oldPosition: oldPosition,
      debugLabel: debugLabel,
    );
  }
}

class _ChainedScrollPosition extends ScrollPositionWithSingleContext {
  _ChainedScrollPosition({
    required super.physics,
    required super.context,
    super.initialPixels,
    super.keepScrollOffset,
    super.oldPosition,
    super.debugLabel,
  });

  Iterable<ScrollPosition> get _ancestors =>
      _scrollAncestors(context, axisDirection);

  double _directionFactor(ScrollPosition position) =>
      _axisFactor(axisDirection, position.axisDirection);

  ScrollPosition? _forward(double delta) =>
      _forwardScroll(context, this, delta);

  void _pointerScrollLocally(double delta) => super.pointerScroll(delta);

  @override
  void pointerScroll(double delta) {
    // New wheel input also takes ownership of any inherited fling.
    for (final parent in _ancestors) {
      if (parent is ScrollPositionWithSingleContext &&
          parent.isScrollingNotifier.value) {
        parent.goIdle();
      }
    }
    final before = pixels;
    super.pointerScroll(delta);
    _forward(delta - (pixels - before));
  }

  @override
  void applyUserOffset(double delta) {
    final movement = -physics.applyPhysicsToUserOffset(this, delta);
    final before = pixels;
    super.applyUserOffset(delta);
    _forward(movement - (pixels - before));
  }

  @override
  double setPixels(double newPixels) {
    final ballistic = activity is BallisticScrollActivity;
    final velocity = activity?.velocity ?? 0;
    final overscroll = super.setPixels(newPixels);
    if (ballistic && overscroll != 0) {
      final parent = _forward(overscroll);
      if (parent is ScrollPositionWithSingleContext) {
        parent.goBallistic(velocity * _directionFactor(parent));
      }
    }
    return overscroll;
  }

  @override
  void goBallistic(double velocity) {
    // A fling can start after a drag has already reached the inner boundary.
    if (velocity != 0 &&
        ((velocity < 0 && pixels <= minScrollExtent) ||
            (velocity > 0 && pixels >= maxScrollExtent))) {
      for (final parent in _ancestors) {
        final parentVelocity = velocity * _directionFactor(parent);
        final hasRoom = parentVelocity < 0
            ? parent.pixels > parent.minScrollExtent
            : parent.pixels < parent.maxScrollExtent;
        if (hasRoom && parent is ScrollPositionWithSingleContext) {
          parent.goBallistic(parentVelocity);
          break;
        }
      }
      super.goBallistic(0);
      return;
    }
    super.goBallistic(velocity);
  }
}

/// A vertical message viewport with its own scroll position and edge handoff.
class ChainedSingleChildScrollView extends StatefulWidget {
  const ChainedSingleChildScrollView({super.key, required this.child});

  final Widget child;

  @override
  State<ChainedSingleChildScrollView> createState() =>
      _ChainedSingleChildScrollViewState();
}

class _ChainedSingleChildScrollViewState
    extends State<ChainedSingleChildScrollView> {
  final _controller = ChainedScrollController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      controller: _controller,
      primary: false,
      child: widget.child,
    );
  }
}

Iterable<ScrollPosition> _scrollAncestors(
  ScrollContext context,
  AxisDirection direction,
) sync* {
  var ancestor = Scrollable.maybeOf(context.storageContext);
  while (ancestor != null) {
    final position = ancestor.position;
    if (axisDirectionToAxis(position.axisDirection) ==
            axisDirectionToAxis(direction) &&
        position.hasContentDimensions &&
        position.physics.shouldAcceptUserOffset(position)) {
      yield position;
    }
    ancestor = Scrollable.maybeOf(ancestor.context);
  }
}

double _axisFactor(AxisDirection source, AxisDirection target) =>
    axisDirectionIsReversed(source) == axisDirectionIsReversed(target) ? 1 : -1;

/// Each ancestor consumes only its share, including across ordinary viewports.
ScrollPosition? _forwardScroll(
  ScrollContext context,
  ScrollPosition source,
  double delta,
) {
  ScrollPosition? lastMoved;
  for (final parent in _scrollAncestors(context, source.axisDirection)) {
    if (delta.abs() < precisionErrorTolerance) break;
    final factor = _axisFactor(source.axisDirection, parent.axisDirection);
    final before = parent.pixels;
    if (parent is _ChainedScrollPosition) {
      parent._pointerScrollLocally(delta * factor);
    } else {
      parent.pointerScroll(delta * factor);
    }
    final consumed = (parent.pixels - before) * factor;
    if (consumed != 0) lastMoved = parent;
    delta -= consumed;
  }
  return lastMoved;
}

/// Covers message renderers that create their own ordinary scroll controllers.
class ChainedScrollScope extends StatefulWidget {
  const ChainedScrollScope({super.key, required this.child});

  final Widget child;

  @override
  State<ChainedScrollScope> createState() => _ChainedScrollScopeState();
}

class _ChainedScrollScopeState extends State<ChainedScrollScope> {
  final _coordinator = _ScrollHandoffCoordinator();

  @override
  Widget build(BuildContext context) => ScrollConfiguration(
    behavior: _HandoffScrollBehavior(
      ScrollConfiguration.of(context),
      _coordinator,
    ),
    child: widget.child,
  );
}

class _ScrollHandoffCoordinator {
  final _claimedSignals = Expando<bool>();
  final _userScrolls = Expando<bool>();

  void pointerSignal(
    ScrollableState scrollable,
    PointerSignalEvent event,
    Set<LogicalKeyboardKey> axisModifiers,
  ) {
    if (event is! PointerScrollEvent) return;
    final original = event.original ?? event;
    if (_claimedSignals[original] == true) return;
    final source = scrollable.position;
    if (!source.hasContentDimensions ||
        !source.physics.shouldAcceptUserOffset(source)) {
      return;
    }
    var axis = axisDirectionToAxis(source.axisDirection);
    if (event.kind == PointerDeviceKind.mouse &&
        HardwareKeyboard.instance.logicalKeysPressed.any(
          axisModifiers.contains,
        )) {
      axis = flipAxis(axis);
    }
    var delta = axis == Axis.vertical
        ? event.scrollDelta.dy
        : event.scrollDelta.dx;
    if (axisDirectionIsReversed(source.axisDirection)) delta = -delta;
    if (delta == 0 ||
        (source.pixels + delta).clamp(
              source.minScrollExtent,
              source.maxScrollExtent,
            ) ==
            source.pixels) {
      return;
    }
    // Hit testing reaches the deepest eligible viewport first. All wrappers
    // share this claim, so a three-level event is reconciled exactly once.
    _claimedSignals[original] = true;
    if (source is _ChainedScrollPosition) return;
    final positions = [
      source,
      ..._scrollAncestors(scrollable, source.axisDirection),
    ];
    for (final parent in positions.skip(1)) {
      if (parent is ScrollPositionWithSingleContext &&
          parent.isScrollingNotifier.value) {
        parent.goIdle();
      }
    }
    final offsets = [for (final position in positions) position.pixels];
    scheduleMicrotask(() {
      if (!scrollable.mounted || !identical(scrollable.position, source)) {
        return;
      }
      var remainder = delta;
      for (var index = 0; index < positions.length; index++) {
        final position = positions[index];
        remainder -=
            (position.pixels - offsets[index]) *
            _axisFactor(source.axisDirection, position.axisDirection);
      }
      // Flutter clamps an ordinary pointerScroll before emitting notifications.
      // Reconcile after its signal resolver, before the next painted frame.
      if (remainder.abs() >= precisionErrorTolerance) {
        _forwardScroll(scrollable, source, remainder);
      }
    });
  }

  bool notification(
    ScrollableState scrollable,
    ScrollNotification notification,
  ) {
    final source = scrollable.position;
    final origin = notification.context;
    if (source is _ChainedScrollPosition ||
        origin == null ||
        Scrollable.maybeOf(origin)?.position != source) {
      return false;
    }
    if (notification is ScrollStartNotification) {
      _userScrolls[source] = notification.dragDetails != null;
    } else if (notification is OverscrollNotification &&
        _userScrolls[source] == true) {
      final parent = _forwardScroll(
        scrollable,
        source,
        notification.overscroll,
      );
      if (parent is ScrollPositionWithSingleContext &&
          notification.velocity != 0) {
        parent.goBallistic(
          notification.velocity *
              _axisFactor(source.axisDirection, parent.axisDirection),
        );
        // The next ordinary viewport must keep forwarding this same fling.
        _userScrolls[parent] = true;
      }
      return parent != null;
    } else if (notification is ScrollEndNotification) {
      final velocity = notification.dragDetails?.primaryVelocity ?? 0;
      if (_userScrolls[source] == true && velocity != 0) {
        final direction = axisDirectionIsReversed(source.axisDirection)
            ? 1
            : -1;
        final scrollVelocity = velocity * direction;
        if ((scrollVelocity < 0 && source.pixels <= source.minScrollExtent) ||
            (scrollVelocity > 0 && source.pixels >= source.maxScrollExtent)) {
          for (final parent in _scrollAncestors(
            scrollable,
            source.axisDirection,
          )) {
            final targetVelocity =
                scrollVelocity *
                _axisFactor(source.axisDirection, parent.axisDirection);
            final hasRoom = targetVelocity < 0
                ? parent.pixels > parent.minScrollExtent
                : parent.pixels < parent.maxScrollExtent;
            if (hasRoom && parent is ScrollPositionWithSingleContext) {
              parent.goBallistic(targetVelocity);
              _userScrolls[parent] = true;
              break;
            }
          }
        }
      }
      _userScrolls[source] = null;
    }
    return false;
  }
}

class _HandoffScrollBehavior extends ScrollBehavior {
  const _HandoffScrollBehavior(this.delegate, this.coordinator);

  final ScrollBehavior delegate;
  final _ScrollHandoffCoordinator coordinator;

  @override
  Set<PointerDeviceKind> get dragDevices => delegate.dragDevices;
  @override
  Set<LogicalKeyboardKey> get pointerAxisModifiers =>
      delegate.pointerAxisModifiers;
  @override
  TargetPlatform getPlatform(BuildContext context) =>
      delegate.getPlatform(context);
  @override
  MultitouchDragStrategy getMultitouchDragStrategy(BuildContext context) =>
      delegate.getMultitouchDragStrategy(context);
  @override
  GestureVelocityTrackerBuilder velocityTrackerBuilder(BuildContext context) =>
      delegate.velocityTrackerBuilder(context);
  @override
  ScrollViewKeyboardDismissBehavior getKeyboardDismissBehavior(
    BuildContext context,
  ) => delegate.getKeyboardDismissBehavior(context);
  @override
  ScrollPhysics getScrollPhysics(BuildContext context) {
    final physics = delegate.getScrollPhysics(context);
    final widget = context.widget;
    return widget is Scrollable &&
            axisDirectionToAxis(widget.axisDirection) == Axis.vertical
        ? ClampingScrollPhysics(parent: physics)
        : physics;
  }

  @override
  Widget buildScrollbar(
    BuildContext context,
    Widget child,
    ScrollableDetails details,
  ) => delegate.buildScrollbar(context, child, details);
  @override
  Widget buildOverscrollIndicator(
    BuildContext context,
    Widget child,
    ScrollableDetails details,
  ) {
    final decorated = delegate.buildOverscrollIndicator(
      context,
      child,
      details,
    );
    if (context is! StatefulElement || context.state is! ScrollableState) {
      return decorated;
    }
    final scrollable = context.state as ScrollableState;
    return Listener(
      onPointerSignal: (event) =>
          coordinator.pointerSignal(scrollable, event, pointerAxisModifiers),
      child: NotificationListener<ScrollNotification>(
        onNotification: (notification) =>
            coordinator.notification(scrollable, notification),
        child: decorated,
      ),
    );
  }

  @override
  bool shouldNotify(covariant _HandoffScrollBehavior oldDelegate) =>
      delegate != oldDelegate.delegate ||
      coordinator != oldDelegate.coordinator;
}
