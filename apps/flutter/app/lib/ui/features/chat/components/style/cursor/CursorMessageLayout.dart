// ignore_for_file: file_names

import 'package:flutter/material.dart';

/// Keeps command-style messages and metadata on the original content column.
class CursorMessageLayout extends StatelessWidget {
  /// Creates a message layout with the original transcript inset.
  const CursorMessageLayout({
    super.key,
    required this.child,
    this.verticalPadding = 4,
  });

  final Widget child;
  final double verticalPadding;

  /// Applies the original transcript inset to user and assistant messages.
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.symmetric(horizontal: 16, vertical: verticalPadding),
      child: child,
    );
  }
}
