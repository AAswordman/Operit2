// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart' show ThemePreferenceSnapshot;

import '../../theme/OperitTheme.dart';

/// The bundled code face used when the user has not overridden the code font.
///
/// The bundled OperitCodeMono subset (OFL Noto Sans Mono CJK SC; ASCII +
/// GB2312 Han + CJK punctuation, ~3.2 MB) is the default primary so code
/// surfaces render identically on every machine, including systems without
/// JetBrains Mono, Cascadia, Consolas or any installed CJK font; rare
/// characters outside the subset fall through to the system fonts below and
/// the platform's per-character fallback.
///
/// It must stay the primary family: a generic 'monospace' primary resolves
/// through fontconfig, and on Linux boxes with Windows fonts installed
/// fontconfig ranks serif CJK faces (SimSun/NSimSun) as the best `monospace`
/// match, which would render code-block Han as Song type before the fallback
/// list is ever consulted.
const String markdownCodeFontFamily = 'OperitCodeMono';

const List<String> markdownCodeFontFamilyFallback = <String>[
  'JetBrains Mono',
  'Cascadia Mono',
  'Consolas',
  'SF Mono',
  'Menlo',
  'Roboto Mono',
  'monospace',
  // Without an explicit CJK family, the engine's per-character fallback
  // resolves through fontconfig coverage ranking and lands on serif Windows
  // fonts (SimSun/NSimSun) or Japanese gothic faces, which look like Song
  // type inside code blocks. Listing sans CJK families keeps Han glyphs on
  // the same sans face as body text on every platform.
  'Microsoft YaHei UI',
  'Microsoft YaHei',
  'PingFang SC',
  'Hiragino Sans GB',
  'Noto Sans CJK SC',
  'Source Han Sans SC',
];

/// Resolves the configured code font family for the given theme state.
///
/// User-selected fonts are runtime-registered families (never resolved by
/// name through fontconfig), so they cannot be hijacked into serif CJK faces.
/// Missing byte preparation falls back to the bundled face instead of throwing.
String operitCodeFontFamily(ThemePreferenceSnapshot? snapshot) {
  if (snapshot == null) {
    return markdownCodeFontFamily;
  }
  if (_usesFontAsset(snapshot.codeFontType)) {
    final family = loadedCustomFontFamilyOrNull(snapshot.codeFontPath);
    if (family != null) {
      return family;
    }
  }
  return markdownCodeFontFamily;
}

/// Resolves the code font fallback chain, keeping the bundled face between a
/// user-selected primary and the system fonts so uncovered glyphs (for
/// example CJK in a Latin-only code font) stay on a sans mono/CJK face.
List<String> operitCodeFontFamilyFallback(ThemePreferenceSnapshot? snapshot) {
  final family = operitCodeFontFamily(snapshot);
  if (family == markdownCodeFontFamily) {
    return markdownCodeFontFamilyFallback;
  }
  return <String>[markdownCodeFontFamily, ...markdownCodeFontFamilyFallback];
}

/// Resolves the configured code font family for the given build context.
String operitCodeFontFamilyOf(BuildContext context) =>
    operitCodeFontFamily(_snapshotOf(context));

/// Resolves the configured code font fallback chain for the given context.
List<String> operitCodeFontFamilyFallbackOf(BuildContext context) =>
    operitCodeFontFamilyFallback(_snapshotOf(context));

/// Builds the shared code text style, honoring the configured code font.
TextStyle? markdownCodeTextStyle(BuildContext context, {Color? color}) {
  return Theme.of(context).textTheme.bodySmall?.copyWith(
    color: color,
    fontFamily: operitCodeFontFamilyOf(context),
    fontFamilyFallback: operitCodeFontFamilyFallbackOf(context),
    height: 1.4,
  );
}

ThemePreferenceSnapshot? _snapshotOf(BuildContext context) =>
    OperitTheme.maybeOf(context)?.themePreferenceSnapshot;

bool _usesFontAsset(String fontType) =>
    fontType == 'file' || fontType == 'device';
