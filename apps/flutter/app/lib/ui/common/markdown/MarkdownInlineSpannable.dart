// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';

import 'MarkdownCodeTypeface.dart';
import 'MarkdownFootnotes.dart';
import 'MarkdownLatexBlock.dart';
import 'MarkdownLink.dart';
import 'MarkdownNodeGrouper.dart';

const double markdownLineHeight = 1.4;
const double markdownLetterSpacing = 0.35;

/// Paints Core's semantic inline tree. This layer never reads Markdown syntax.
TextSpan buildMarkdownInlineSpannableFromMarkdownNodes({
  required BuildContext context,
  required List<MarkdownNodeStable> children,
  required Color textColor,
  required MarkdownLinkGestureOwner gestures,
  TextStyle? baseStyle,
  void Function(String url)? onLinkClick,
}) {
  final style = (Theme.of(context).textTheme.bodyMedium ?? const TextStyle())
      .copyWith(
        color: textColor,
        height: markdownLineHeight,
        letterSpacing: markdownLetterSpacing,
      )
      .merge(baseStyle);
  return TextSpan(
    style: style,
    children: [
      for (final node in children)
        _inlineSpan(context, node, style, gestures, onLinkClick),
    ],
  );
}

InlineSpan _inlineSpan(
  BuildContext context,
  MarkdownNodeStable node,
  TextStyle inherited,
  MarkdownLinkGestureOwner gestures,
  void Function(String url)? onLinkClick, [
  String? linkUrl,
]) {
  if (node.type == MarkdownNodeType.footnoteReference) {
    return WidgetSpan(
      alignment: PlaceholderAlignment.baseline,
      baseline: TextBaseline.alphabetic,
      child: MarkdownFootnoteReference(label: node.content, style: inherited),
    );
  }
  if (node.type == MarkdownNodeType.inlineLatex) {
    return WidgetSpan(
      alignment: PlaceholderAlignment.baseline,
      baseline: TextBaseline.alphabetic,
      child: MarkdownInlineLatex(
        content: node.content,
        textColor: inherited.color ?? Colors.black,
      ),
    );
  }
  var style = inherited;
  switch (node.type) {
    case MarkdownNodeType.bold:
      style = style.copyWith(fontWeight: FontWeight.w700);
      break;
    case MarkdownNodeType.italic:
      style = style.copyWith(fontStyle: FontStyle.italic);
      break;
    case MarkdownNodeType.strikethrough:
      style = _decorate(style, TextDecoration.lineThrough);
      break;
    case MarkdownNodeType.underline:
      style = _decorate(style, TextDecoration.underline);
      break;
    case MarkdownNodeType.link:
      linkUrl = node.linkUrl;
      final color = Theme.of(context).colorScheme.primary;
      style = _decorate(
        style,
        TextDecoration.underline,
      ).copyWith(color: color, decorationColor: color);
      break;
    case MarkdownNodeType.inlineCode:
      final color = style.color ?? Colors.black;
      style = style
          .apply(fontSizeFactor: .9)
          .copyWith(
            fontFamily: markdownCodeFontFamily,
            fontFamilyFallback: markdownCodeFontFamilyFallback,
            backgroundColor: color.withValues(
              alpha: color.computeLuminance() > .5 ? .18 : .12,
            ),
          );
      break;
    default:
      break;
  }
  if (node.children.isNotEmpty) {
    return TextSpan(
      style: style,
      children: [
        for (final child in node.children)
          _inlineSpan(context, child, style, gestures, onLinkClick, linkUrl),
      ],
    );
  }
  return TextSpan(
    text: node.type == MarkdownNodeType.htmlBreak ? '\n' : node.content,
    style: style,
    recognizer: linkUrl == null
        ? null
        : gestures.recognizerFor(linkUrl, onLinkClick),
  );
}

TextStyle _decorate(TextStyle style, TextDecoration decoration) =>
    style.copyWith(
      decoration: TextDecoration.combine([
        if (style.decoration != null) style.decoration!,
        decoration,
      ]),
    );

/// Owns the gesture recognizers for one Markdown text build.
///
/// Recognizers are reused across frames in encounter order and released when
/// that text is no longer built.
class MarkdownLinkGestureOwner {
  final List<TapGestureRecognizer> _recognizers = <TapGestureRecognizer>[];
  var _count = 0;

  /// Starts a new build pass.
  void beginFrame() {
    _count = 0;
  }

  /// Returns the recognizer for the next link in this build pass.
  TapGestureRecognizer? recognizerFor(
    String url,
    void Function(String url)? onLinkClick,
  ) {
    final destination = url.trim();
    if (destination.isEmpty) {
      return null;
    }
    void openLink() {
      activateMarkdownLink(destination, onLinkClick);
    }

    if (_count < _recognizers.length) {
      final recognizer = _recognizers[_count];
      recognizer.onTap = openLink;
      _count++;
      return recognizer;
    }
    final recognizer = TapGestureRecognizer()..onTap = openLink;
    _recognizers.add(recognizer);
    _count++;
    return recognizer;
  }

  /// Disposes recognizers that this build pass did not use.
  void endFrame() {
    while (_recognizers.length > _count) {
      _recognizers.removeLast().dispose();
    }
  }

  /// Disposes every recognizer owned by this text.
  void dispose() {
    for (final recognizer in _recognizers) {
      recognizer.dispose();
    }
    _recognizers.clear();
    _count = 0;
  }
}
