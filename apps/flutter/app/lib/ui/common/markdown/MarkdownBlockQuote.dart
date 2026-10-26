// ignore_for_file: file_names

import 'package:flutter/material.dart';

/// Quote decoration around the already parsed semantic body.
class MarkdownBlockQuote extends StatelessWidget {
  const MarkdownBlockQuote({super.key, required this.child});
  final Widget child;
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(vertical: 1),
      decoration: BoxDecoration(
        color: colors.surfaceContainerHighest.withValues(alpha: .2),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(
          width: 2,
          color: colors.primary.withValues(alpha: .15),
        ),
      ),
      padding: const EdgeInsets.all(4),
      child: child,
    );
  }
}
