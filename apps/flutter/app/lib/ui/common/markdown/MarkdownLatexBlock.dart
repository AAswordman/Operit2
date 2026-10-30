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

  /// Renders display math without interpreting application-specific languages.
  @override
  Widget build(BuildContext context) {
    final latexContent = extractLatexContent(content.trim());
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

/// Removes the recognized outer math delimiter from one LaTeX expression.
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

class MarkdownInlineLatex extends StatelessWidget {
  const MarkdownInlineLatex({
    super.key,
    required this.content,
    required this.textColor,
  });

  final String content;
  final Color textColor;

  /// Renders inline math using the surrounding text style.
  @override
  Widget build(BuildContext context) {
    final latexContent = extractLatexContent(content.trim());
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
