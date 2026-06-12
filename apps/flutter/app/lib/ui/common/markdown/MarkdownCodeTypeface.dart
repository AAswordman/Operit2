// ignore_for_file: file_names

import 'package:flutter/material.dart';

const String markdownCodeFontFamily = 'monospace';

TextStyle? markdownCodeTextStyle(BuildContext context, {Color? color}) {
  return Theme.of(context).textTheme.bodySmall?.copyWith(
    color: color,
    fontFamily: markdownCodeFontFamily,
    height: 1.25,
  );
}
