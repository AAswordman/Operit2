// ignore_for_file: file_names

import 'package:flutter/material.dart';

/// Shares the surrounding Markdown selection or owns a standalone text region.
class MarkdownSelectionArea extends StatelessWidget {
  /// Defines a selection boundary only when no surrounding registrar exists.
  const MarkdownSelectionArea({super.key, required this.child});

  final Widget child;

  /// Keeps keyboard focus and selected content in the same region.
  @override
  Widget build(BuildContext context) =>
      SelectionContainer.maybeOf(context) == null
      ? SelectionArea(child: child)
      : child;
}
