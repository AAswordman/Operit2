// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter_math_fork/flutter_math.dart';

class MarkdownLatexBlock extends StatelessWidget {
  const MarkdownLatexBlock({
    super.key,
    required this.content,
    required this.textColor,
  });

  final String content;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final latexContent = extractLatexContent(content.trim());
    if (looksLikeOpenUiProgram(latexContent)) {
      return OpenUiProgramText(text: latexContent, textColor: textColor);
    }
    return ExcludeSemantics(
      child: Container(
        width: double.infinity,
        margin: const EdgeInsets.symmetric(vertical: 4),
        alignment: Alignment.centerLeft,
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Math.tex(
            latexContent,
            mathStyle: MathStyle.display,
            textStyle: Theme.of(
              context,
            ).textTheme.bodyMedium!.copyWith(color: textColor),
          ),
        ),
      ),
    );
  }
}

String extractLatexContent(String content) {
  if (content.startsWith(r'$$') && content.endsWith(r'$$')) {
    return content.substring(2, content.length - 2).trim();
  }
  if (content.startsWith(r'\[') && content.endsWith(r'\]')) {
    return content.substring(2, content.length - 2).trim();
  }
  if (content.startsWith(r'$') && content.endsWith(r'$')) {
    return content.substring(1, content.length - 1).trim();
  }
  if (content.startsWith(r'\(') && content.endsWith(r'\)')) {
    return content.substring(2, content.length - 2).trim();
  }
  return content;
}

/// Heuristic guard for OpenUI program text accidentally captured as math.
bool looksLikeOpenUiProgram(String content) {
  final trimmed = content.trim();
  if (trimmed.isEmpty) {
    return false;
  }
  var programSignals = 0;
  if (trimmed.contains('root=')) programSignals++;
  if (trimmed.contains('Card(')) programSignals++;
  if (trimmed.contains('@Set(')) programSignals++;
  if (RegExp(r'^\$\w+\s*=', multiLine: true).hasMatch(trimmed)) {
    programSignals++;
  }
  return programSignals >= 2;
}

class OpenUiProgramText extends StatelessWidget {
  const OpenUiProgramText({
    super.key,
    required this.text,
    required this.textColor,
  });

  final String text;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(vertical: 4),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(
          alpha: 0.35,
        ),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: Theme.of(context).colorScheme.outline.withValues(alpha: 0.2),
        ),
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Text(
          text,
          softWrap: false,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: textColor,
            fontFamily: 'Consolas',
            fontFamilyFallback: const <String>['Courier New', 'monospace'],
            height: 1.4,
          ),
        ),
      ),
    );
  }
}

class MarkdownInlineLatex extends StatelessWidget {
  const MarkdownInlineLatex({
    super.key,
    required this.content,
    required this.textColor,
  });

  final String content;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final latexContent = extractLatexContent(content.trim());
    if (looksLikeOpenUiProgram(latexContent)) {
      return Text(
        latexContent,
        style: DefaultTextStyle.of(context).style.copyWith(color: textColor),
      );
    }
    return ExcludeSemantics(
      child: Math.tex(
        latexContent,
        mathStyle: MathStyle.text,
        textStyle: DefaultTextStyle.of(
          context,
        ).style.copyWith(color: textColor),
      ),
    );
  }
}
