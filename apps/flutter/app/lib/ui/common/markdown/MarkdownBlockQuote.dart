// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'StreamMarkdownRenderer.dart';

class MarkdownBlockQuote extends StatelessWidget {
  const MarkdownBlockQuote({
    super.key,
    required this.content,
    required this.textColor,
    required this.backgroundColor,
    required this.isStreaming,
  });

  final String content;
  final Color textColor;
  final Color backgroundColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final quoteText = stripBlockQuoteMarkers(content);
    if (quoteText.trim().isEmpty) {
      return const SizedBox.shrink();
    }
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(vertical: 1),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.2),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(
          width: 2,
          color: colorScheme.primary.withValues(alpha: 0.15),
        ),
      ),
      padding: const EdgeInsets.all(4),
      child: StreamMarkdownRenderer(
        content: quoteText,
        isStreaming: isStreaming,
        textColor: textColor,
        backgroundColor: backgroundColor,
        selectionRoot: false,
      ),
    );
  }
}

String stripBlockQuoteMarkers(String content) {
  return content
      .split('\n')
      .map((line) => line.replaceFirst(RegExp(r'^\s*>\s?'), ''))
      .join('\n')
      .trim();
}
