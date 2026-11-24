// ignore_for_file: file_names

import 'package:flutter/material.dart';

const String markdownCodeFontFamily = 'monospace';

const List<String> markdownCodeFontFamilyFallback = <String>[
  // The bundled OperitCodeMono subset (OFL Noto Sans Mono CJK SC; ASCII +
  // GB2312 Han + CJK punctuation, ~3.2 MB) is the primary code face so code
  // blocks render identically on every machine, including systems without
  // JetBrains Mono, Cascadia, Consolas or any installed CJK font; rare
  // characters outside the subset fall through to the system fonts below and
  // the platform's per-character fallback.
  'OperitCodeMono',
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

TextStyle? markdownCodeTextStyle(BuildContext context, {Color? color}) {
  return Theme.of(context).textTheme.bodySmall?.copyWith(
    color: color,
    fontFamily: markdownCodeFontFamily,
    fontFamilyFallback: markdownCodeFontFamilyFallback,
    height: 1.4,
  );
}
