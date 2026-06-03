// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

class LazyIndexedStack extends StatefulWidget {
  const LazyIndexedStack({
    super.key,
    required this.index,
    required this.itemCount,
    required this.itemBuilder,
  });

  final int index;
  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;

  @override
  State<LazyIndexedStack> createState() => _LazyIndexedStackState();
}

class _LazyIndexedStackState extends State<LazyIndexedStack> {
  final Set<int> _builtIndexes = <int>{};

  @override
  void initState() {
    super.initState();
    _rememberCurrentIndex();
  }

  @override
  void didUpdateWidget(covariant LazyIndexedStack oldWidget) {
    super.didUpdateWidget(oldWidget);
    _builtIndexes.removeWhere((index) => index >= widget.itemCount);
    _rememberCurrentIndex();
  }

  void _rememberCurrentIndex() {
    if (widget.index >= 0 && widget.index < widget.itemCount) {
      _builtIndexes.add(widget.index);
    }
  }

  @override
  Widget build(BuildContext context) {
    return IndexedStack(
      index: widget.index,
      children: List<Widget>.generate(widget.itemCount, (index) {
        if (!_builtIndexes.contains(index)) {
          return const SizedBox.shrink();
        }
        return KeyedSubtree(
          key: ValueKey<int>(index),
          child: widget.itemBuilder(context, index),
        );
      }),
    );
  }
}
