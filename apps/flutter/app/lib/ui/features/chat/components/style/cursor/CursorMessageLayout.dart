// ignore_for_file: file_names

import 'package:flutter/material.dart';
import '../../../../../theme/OperitTheme.dart';

/// Keeps command-style messages and metadata on the original content column.
class CursorMessageLayout extends StatelessWidget {
  /// Creates a message layout with the configured transcript inset.
  const CursorMessageLayout({
    super.key,
    required this.child,
    this.verticalPadding = 4,
  });

  final Widget child;
  final double verticalPadding;

  /// Applies the configured transcript inset to user and assistant messages.
  @override
  Widget build(BuildContext context) {
    // 两侧留白来自主题设置（默认 16dp）。调到 0 即正文贴边显示，
    // 气泡与输入框自身的内留白不受影响。
    final horizontalPadding = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.chatContentHorizontalPadding;
    return Padding(
      padding: EdgeInsets.symmetric(
        horizontal: horizontalPadding,
        vertical: verticalPadding,
      ),
      child: child,
    );
  }
}
