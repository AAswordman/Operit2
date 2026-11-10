// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'RetainedPage.dart';

class AnimatedLazyIndexedStack extends StatefulWidget {
  const AnimatedLazyIndexedStack({
    super.key,
    required this.index,
    required this.itemCount,
    required this.itemBuilder,
    this.duration = const Duration(milliseconds: 200),
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
  static const double _entryOffset = 12;
  static const double _exitOffset = 6;
  static const Curve _fadeCurve = Interval(0, 0.55, curve: Curves.easeOutCubic);

  late final AnimationController _controller;
  late final CurvedAnimation _curve;
  final Set<int> _builtIndexes = <int>{};
  final Map<int, SnapshotController> _snapshotControllers =
      <int, SnapshotController>{};
  int? _previousIndex;
  Tween<double> _incomingOffset = Tween<double>(begin: 0, end: 0);
  Tween<double> _outgoingOffset = Tween<double>(begin: 0, end: 0);
  Tween<double> _incomingOpacity = Tween<double>(begin: 1, end: 1);
  Tween<double> _outgoingOpacity = Tween<double>(begin: 1, end: 0);

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: widget.duration)
      ..value = 1;
    _controller.addStatusListener(_onAnimationStatusChanged);
    _curve = CurvedAnimation(parent: _controller, curve: Curves.easeOutCubic);
    _rememberIndex(widget.index);
  }

  /// Updates tab membership and releases snapshots from interrupted exits.
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
      final direction = widget.index > oldWidget.index ? 1 : -1;
      final motion = _curve.value;
      final fade = _fadeCurve.transform(_controller.value);
      final outgoingOffset = _incomingOffset.transform(motion);
      final outgoingOpacity = _incomingOpacity.transform(fade);
      final reversing = widget.index == _previousIndex;
      final incomingOffset = reversing
          ? _outgoingOffset.transform(motion)
          : _entryOffset * direction;
      final incomingOpacity = reversing
          ? _outgoingOpacity.transform(fade)
          : 1.0;
      _incomingOffset = Tween<double>(begin: incomingOffset, end: 0);
      _outgoingOffset = Tween<double>(
        begin: outgoingOffset,
        end: -_exitOffset * direction,
      );
      _incomingOpacity = Tween<double>(begin: incomingOpacity, end: 1);
      _outgoingOpacity = Tween<double>(begin: outgoingOpacity, end: 0);
      _snapshotControllers[_previousIndex]?.allowSnapshotting = false;
      _rememberIndex(oldWidget.index);
      _previousIndex = oldWidget.index;
      final previousSnapshotController = _snapshotControllerFor(
        oldWidget.index,
      );
      previousSnapshotController.allowSnapshotting = true;
      previousSnapshotController.clear();
      _snapshotControllerFor(widget.index).allowSnapshotting = false;
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

  /// Releases animation listeners and tab snapshot controllers.
  @override
  void dispose() {
    _controller.removeStatusListener(_onAnimationStatusChanged);
    _curve.dispose();
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

  /// Preserves each tab's keyed ancestry while its visibility and role change.
  Widget _buildIndexedChild(BuildContext context, int index) {
    final isCurrent = index == widget.index;
    final isPrevious = index == _previousIndex;
    final visible = isCurrent || isPrevious;
    final snapshotController = _snapshotControllerFor(index);
    final child = RepaintBoundary(
      child: SnapshotWidget(
        controller: snapshotController,
        // Native WebViews must stay in the live scene during transitions.
        mode: SnapshotMode.permissive,
        autoresize: true,
        child: Builder(
          builder: (context) => widget.itemBuilder(context, index),
        ),
      ),
    );

    return Positioned.fill(
      key: ValueKey<int>(index),
      child: RetainedPage(
        active: visible,
        child: IgnorePointer(
          ignoring: !isCurrent,
          child: AnimatedBuilder(
            animation: visible ? _curve : kAlwaysCompleteAnimation,
            child: child,
            builder: (context, child) {
              final value = _previousIndex == null ? 1.0 : _curve.value;
              final fade = _fadeCurve.transform(_controller.value);
              final opacity = isCurrent
                  ? _incomingOpacity.transform(fade)
                  : _outgoingOpacity.transform(fade);
              final offset = isCurrent
                  ? _incomingOffset.transform(value)
                  : _outgoingOffset.transform(value);
              return Opacity(
                opacity: opacity,
                child: Transform.translate(
                  offset: Offset(offset, 0),
                  child: child,
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}
