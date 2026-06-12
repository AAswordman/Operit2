// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'XmlCanvasBlockComponents.dart';

class FontTagRenderer extends StatelessWidget {
  const FontTagRenderer({
    super.key,
    required this.xmlContent,
    required this.textColor,
  });

  final String xmlContent;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final innerText = _extractContentFromXml(xmlContent, 'font');
    final base = Theme.of(context).textTheme.bodyMedium!;
    final parsedSize = _parseFontSize(_extractXmlAttribute(xmlContent, 'size'));
    final style =
        (parsedSize == null
                ? base
                : base.apply(fontSizeFactor: parsedSize / base.fontSize!))
            .copyWith(
              fontFamily:
                  _parseFontFamily(_extractXmlAttribute(xmlContent, 'face')) ??
                  base.fontFamily,
              fontWeight:
                  _parseFontWeight(_extractXmlAttribute(xmlContent, 'style')) ??
                  base.fontWeight,
              fontStyle:
                  _parseFontItalic(_extractXmlAttribute(xmlContent, 'style')) ??
                  base.fontStyle,
              decoration:
                  _parseTextDecoration(
                    _extractXmlAttribute(xmlContent, 'style'),
                  ) ??
                  base.decoration,
            );
    return CanvasFontTextBlock(
      text: innerText,
      style: style,
      textColor:
          _parseFontColor(_extractXmlAttribute(xmlContent, 'color')) ??
          textColor,
      backgroundColor:
          _parseFontColor(_extractXmlAttribute(xmlContent, 'bgcolor')) ??
          Colors.transparent,
    );
  }
}

String _extractContentFromXml(String content, String tagName) {
  final open = RegExp(
    '<${RegExp.escape(tagName)}\\b[^>]*>',
    caseSensitive: false,
    dotAll: true,
  ).firstMatch(content);
  if (open == null) {
    return content;
  }
  final endTag = '</$tagName>';
  final endIndex = content.toLowerCase().lastIndexOf(endTag.toLowerCase());
  final bodyEnd = endIndex > open.end ? endIndex : content.length;
  return content.substring(open.end, bodyEnd).trim();
}

String? _extractXmlAttribute(String content, String attributeName) {
  return RegExp(
    '$attributeName\\s*=\\s*(["\'])(.*?)\\1',
    caseSensitive: false,
  ).firstMatch(content)?.group(2);
}

Color? _parseFontColor(String? value) {
  if (value == null || value.trim().isEmpty) {
    return null;
  }
  final raw = value.trim();
  final hex = raw.startsWith('#') ? raw.substring(1) : raw;
  if (RegExp(r'^[0-9a-fA-F]{6}$').hasMatch(hex)) {
    return Color(int.parse('ff$hex', radix: 16));
  }
  if (RegExp(r'^[0-9a-fA-F]{8}$').hasMatch(hex)) {
    return Color(int.parse(hex, radix: 16));
  }
  return const <String, Color>{
    'black': Colors.black,
    'white': Colors.white,
    'red': Colors.red,
    'green': Colors.green,
    'blue': Colors.blue,
    'yellow': Colors.yellow,
    'gray': Colors.grey,
    'grey': Colors.grey,
  }[raw.toLowerCase()];
}

double? _parseFontSize(String? value) {
  if (value == null || value.trim().isEmpty) {
    return null;
  }
  final raw = value.trim().toLowerCase();
  final htmlSize = int.tryParse(raw);
  if (htmlSize != null && htmlSize >= 1 && htmlSize <= 7) {
    return const <int, double>{
      1: 10,
      2: 12,
      3: 14,
      4: 16,
      5: 18,
      6: 20,
      7: 24,
    }[htmlSize];
  }
  return double.tryParse(
    raw.replaceAll('sp', '').replaceAll('px', '').replaceAll('dp', ''),
  );
}

String? _parseFontFamily(String? value) {
  if (value == null || value.trim().isEmpty) {
    return null;
  }
  switch (value.trim().toLowerCase()) {
    case 'monospace':
    case 'mono':
      return 'monospace';
    case 'serif':
      return 'serif';
    case 'sans-serif':
    case 'sansserif':
    case 'sans':
      return 'sans-serif';
  }
  return null;
}

FontWeight? _parseFontWeight(String? value) {
  if (value == null) {
    return null;
  }
  return value.toLowerCase().contains('bold') ? FontWeight.bold : null;
}

FontStyle? _parseFontItalic(String? value) {
  if (value == null) {
    return null;
  }
  return value.toLowerCase().contains('italic') ? FontStyle.italic : null;
}

TextDecoration? _parseTextDecoration(String? value) {
  if (value == null) {
    return null;
  }
  final raw = value.toLowerCase();
  final decorations = <TextDecoration>[];
  if (raw.contains('underline')) {
    decorations.add(TextDecoration.underline);
  }
  if (raw.contains('line-through') || raw.contains('strikethrough')) {
    decorations.add(TextDecoration.lineThrough);
  }
  if (decorations.isEmpty) {
    return null;
  }
  return decorations.length == 1
      ? decorations.first
      : TextDecoration.combine(decorations);
}
