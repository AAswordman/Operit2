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
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(vertical: 2),
      padding: const EdgeInsets.symmetric(horizontal: 8),
      alignment: Alignment.center,
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Math.tex(
          latexContent,
          mathStyle: MathStyle.display,
          textStyle: TextStyle(color: textColor, fontSize: 14),
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
    final style = DefaultTextStyle.of(context).style;
    return Math.tex(
      extractLatexContent(content.trim()),
      mathStyle: MathStyle.text,
      textStyle: style.copyWith(color: textColor, fontSize: style.fontSize),
    );
  }
}
