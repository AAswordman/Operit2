// ignore_for_file: file_names

import 'package:flutter/material.dart';

class AnimatedLazyIndexedStack extends StatefulWidget {
  const AnimatedLazyIndexedStack({
    super.key,
    required this.index,
    required this.itemCount,
    required this.itemBuilder,
    this.duration = const Duration(milliseconds: 260),
  });

  final int index;
  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;
  final Duration duration;

  @override
  State<AnimatedLazyIndexedStack> createState() =>
      _AnimatedLazyIndexedStackState();
}

class _AnimatedLazyIndexedStackState extends State<AnimatedLazyIndexedStack>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _curve;
  final Set<int> _builtIndexes = <int>{};
  final Map<int, SnapshotController> _snapshotControllers =
      <int, SnapshotController>{};
  int? _previousIndex;
  int _transitionDirection = 1;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: widget.duration)
      ..value = 1;
    _controller.addStatusListener(_onAnimationStatusChanged);
    _curve = CurvedAnimation(parent: _controller, curve: Curves.easeOutCubic);
    _rememberIndex(widget.index);
  }

  @override
  void didUpdateWidget(covariant AnimatedLazyIndexedStack oldWidget) {
    super.didUpdateWidget(oldWidget);
    _controller.duration = widget.duration;
    _builtIndexes.removeWhere((index) => index >= widget.itemCount);
    _snapshotControllers.keys
        .where((index) => index >= widget.itemCount)
        .toList(growable: false)
        .forEach(_disposeSnapshotController);
    _rememberIndex(widget.index);
    if (oldWidget.index != widget.index) {
      _rememberIndex(oldWidget.index);
      _previousIndex = oldWidget.index;
      final previousSnapshotController = _snapshotControllerFor(
        oldWidget.index,
      );
      previousSnapshotController.allowSnapshotting = true;
      previousSnapshotController.clear();
      _snapshotControllerFor(widget.index).allowSnapshotting = false;
      _transitionDirection = widget.index > oldWidget.index ? 1 : -1;
      _controller.forward(from: 0);
    }
  }

  void _onAnimationStatusChanged(AnimationStatus status) {
    if (status == AnimationStatus.completed && _previousIndex != null) {
      _snapshotControllers[_previousIndex]?.allowSnapshotting = false;
      setState(() {
        _previousIndex = null;
      });
    }
  }

  @override
  void dispose() {
    _controller.removeStatusListener(_onAnimationStatusChanged);
    _controller.dispose();
    for (final controller in _snapshotControllers.values) {
      controller.dispose();
    }
    _snapshotControllers.clear();
    super.dispose();
  }

  void _rememberIndex(int index) {
    if (index >= 0 && index < widget.itemCount) {
      _builtIndexes.add(index);
    }
  }

  SnapshotController _snapshotControllerFor(int index) {
    return _snapshotControllers.putIfAbsent(index, SnapshotController.new);
  }

  void _disposeSnapshotController(int index) {
    _snapshotControllers.remove(index)?.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final indexes = _builtIndexes.toList()..sort();
    final visibleIndexes = <int>[
      if (_previousIndex != null &&
          _previousIndex != widget.index &&
          indexes.contains(_previousIndex)) ...<int>[_previousIndex!],
      if (indexes.contains(widget.index)) ...<int>[widget.index],
    ];
    return ClipRect(
      child: Stack(
        fit: StackFit.expand,
        children: <Widget>[
          // Cached tabs stay mounted; only active and exiting tabs paint.
          for (final index in indexes)
            if (!visibleIndexes.contains(index))
              _buildIndexedChild(context, index),
          for (final index in visibleIndexes)
            _buildIndexedChild(context, index),
        ],
      ),
    );
  }

  Widget _buildIndexedChild(BuildContext context, int index) {
    final isCurrent = index == widget.index;
    final isPrevious = index == _previousIndex;
    final visible = isCurrent || isPrevious;
    final snapshotController = _snapshotControllerFor(index);
    final child = KeyedSubtree(
      key: ValueKey<int>(index),
      child: SnapshotWidget(
        controller: snapshotController,
        mode: SnapshotMode.forced,
        autoresize: true,
        child: widget.itemBuilder(context, index),
      ),
    );

    if (!visible) {
      return Offstage(
        offstage: true,
        child: TickerMode(enabled: false, child: child),
      );
    }

    return Positioned.fill(
      child: IgnorePointer(
        ignoring: !isCurrent,
        child: AnimatedBuilder(
          animation: _curve,
          child: child,
          builder: (context, child) {
            final value = _previousIndex == null ? 1.0 : _curve.value;
            final opacity = isPrevious ? 1.0 - value : 1.0;
            final distance = isCurrent ? 0.035 : 0.022;
            final offsetDirection = isCurrent
                ? _transitionDirection
                : -_transitionDirection;
            final offset = isCurrent
                ? (1.0 - value) * distance * offsetDirection
                : value * distance * offsetDirection;
            final animatedChild = FractionalTranslation(
              translation: Offset(offset, 0),
              child: child,
            );
            if (!isPrevious) {
              return animatedChild;
            }
            return Opacity(opacity: opacity, child: animatedChild);
          },
        ),
      ),
    );
  }
}
