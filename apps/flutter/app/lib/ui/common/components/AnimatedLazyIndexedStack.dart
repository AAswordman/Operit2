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
    _rememberIndex(widget.index);
    if (oldWidget.index != widget.index) {
      _rememberIndex(oldWidget.index);
      _previousIndex = oldWidget.index;
      _transitionDirection = widget.index > oldWidget.index ? 1 : -1;
      _controller.forward(from: 0);
    }
  }

  void _onAnimationStatusChanged(AnimationStatus status) {
    if (status == AnimationStatus.completed && _previousIndex != null) {
      setState(() {
        _previousIndex = null;
      });
    }
  }

  @override
  void dispose() {
    _controller.removeStatusListener(_onAnimationStatusChanged);
    _controller.dispose();
    super.dispose();
  }

  void _rememberIndex(int index) {
    if (index >= 0 && index < widget.itemCount) {
      _builtIndexes.add(index);
    }
  }

  @override
  Widget build(BuildContext context) {
    final indexes = _builtIndexes.toList()..sort();
    return ClipRect(
      child: Stack(
        fit: StackFit.expand,
        children: <Widget>[
          for (final index in indexes) _buildIndexedChild(context, index),
        ],
      ),
    );
  }

  Widget _buildIndexedChild(BuildContext context, int index) {
    final isCurrent = index == widget.index;
    final isPrevious = index == _previousIndex;
    final visible = isCurrent || isPrevious;
    final child = KeyedSubtree(
      key: ValueKey<int>(index),
      child: widget.itemBuilder(context, index),
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
            final opacity = isCurrent ? value : 1.0 - value;
            final distance = isCurrent ? 0.035 : 0.022;
            final offsetDirection = isCurrent
                ? _transitionDirection
                : -_transitionDirection;
            final offset = isCurrent
                ? (1.0 - value) * distance * offsetDirection
                : value * distance * offsetDirection;
            return Opacity(
              opacity: opacity,
              child: FractionalTranslation(
                translation: Offset(offset, 0),
                child: child,
              ),
            );
          },
        ),
      ),
    );
  }
}
