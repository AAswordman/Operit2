import 'package:flutter/widgets.dart';
import 'package:flutter/foundation.dart';

const _kDefaultFontSize = 13.0;

const _kDefaultHeight = 1.2;

const _kDefaultFontFamily = 'monospace';

const _kDefaultFontFamilyFallback = [
  'Menlo',
  'Monaco',
  'Consolas',
  'Liberation Mono',
  'Courier New',
  'Noto Sans Mono CJK SC',
  'Noto Sans Mono CJK TC',
  'Noto Sans Mono CJK KR',
  'Noto Sans Mono CJK JP',
  'Noto Sans Mono CJK HK',
  'Noto Color Emoji',
  'Noto Sans Symbols',
  'monospace',
  'sans-serif',
];

class TerminalStyle {
  /// Creates the font configuration shared by terminal cells.
  const TerminalStyle({
    this.fontSize = _kDefaultFontSize,
    this.height = _kDefaultHeight,
    this.fontFamily = _kDefaultFontFamily,
    this.fontFamilyFallback = _kDefaultFontFamilyFallback,
    this.fontFeatures = const [],
    this.letterSpacing = 0,
  });

  /// Preserves the font settings used to measure and paint terminal cells.
  factory TerminalStyle.fromTextStyle(TextStyle textStyle) {
    return TerminalStyle(
      fontSize: textStyle.fontSize ?? _kDefaultFontSize,
      height: textStyle.height ?? _kDefaultHeight,
      fontFamily: textStyle.fontFamily ??
          textStyle.fontFamilyFallback?.first ??
          _kDefaultFontFamily,
      fontFamilyFallback:
          textStyle.fontFamilyFallback ?? _kDefaultFontFamilyFallback,
      fontFeatures: textStyle.fontFeatures ?? const [],
      letterSpacing: textStyle.letterSpacing ?? 0,
    );
  }

  final double fontSize;

  final double height;

  final String fontFamily;

  final List<String> fontFamilyFallback;

  final List<FontFeature> fontFeatures;

  final double letterSpacing;

  /// Compares font values so widget rebuilds preserve glyph and line caches.
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TerminalStyle &&
          other.fontSize == fontSize &&
          other.height == height &&
          other.fontFamily == fontFamily &&
          listEquals(other.fontFamilyFallback, fontFamilyFallback) &&
          listEquals(other.fontFeatures, fontFeatures) &&
          other.letterSpacing == letterSpacing;

  /// Hashes the same font values used by equality.
  @override
  int get hashCode => Object.hash(
      fontSize,
      height,
      fontFamily,
      Object.hashAll(fontFamilyFallback),
      Object.hashAll(fontFeatures),
      letterSpacing);

  /// Builds the text style for a terminal glyph or composing text.
  TextStyle toTextStyle({
    Color? color,
    Color? backgroundColor,
    bool bold = false,
    bool italic = false,
    bool underline = false,
  }) {
    return TextStyle(
      fontSize: fontSize,
      height: height,
      fontFamily: fontFamily,
      fontFamilyFallback: fontFamilyFallback,
      fontFeatures: fontFeatures,
      letterSpacing: letterSpacing,
      color: color,
      backgroundColor: backgroundColor,
      fontWeight: bold ? FontWeight.bold : FontWeight.normal,
      fontStyle: italic ? FontStyle.italic : FontStyle.normal,
      decoration: underline ? TextDecoration.underline : TextDecoration.none,
    );
  }

  /// Copies font settings while preserving unspecified values.
  TerminalStyle copyWith({
    double? fontSize,
    double? height,
    String? fontFamily,
    List<String>? fontFamilyFallback,
    List<FontFeature>? fontFeatures,
    double? letterSpacing,
  }) {
    return TerminalStyle(
      fontSize: fontSize ?? this.fontSize,
      height: height ?? this.height,
      fontFamily: fontFamily ?? this.fontFamily,
      fontFamilyFallback: fontFamilyFallback ?? this.fontFamilyFallback,
      fontFeatures: fontFeatures ?? this.fontFeatures,
      letterSpacing: letterSpacing ?? this.letterSpacing,
    );
  }
}
