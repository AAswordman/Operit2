// ignore_for_file: file_names

import 'package:flutter/material.dart';

/// Shared thinking and tool disclosure motion, anchored at the header edge.
class ChatActivityDisclosure extends StatelessWidget {
  const ChatActivityDisclosure({super.key, this.child});

  static const duration = Duration(milliseconds: 220);
  static const curve = Curves.easeOutCubic;

  final Widget? child;

  /// Animates both insertion and removal without abruptly dropping the body.
  @override
  Widget build(BuildContext context) => AnimatedSwitcher(
    duration: duration,
    reverseDuration: duration,
    switchInCurve: curve,
    switchOutCurve: curve,
    transitionBuilder: _buildTransition,
    child: child ?? const SizedBox.shrink(key: ValueKey('empty')),
  );

  /// Uses the same animation for vertical reveal and content opacity.
  static Widget _buildTransition(Widget child, Animation<double> animation) =>
      SizeTransition(
        sizeFactor: animation,
        axisAlignment: -1,
        child: FadeTransition(opacity: animation, child: child),
      );
}
