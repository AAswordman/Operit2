// ignore_for_file: file_names

import 'package:flutter/material.dart';

Color getTextColorForBackground(Color backgroundColor) {
  final luminance =
      0.299 * backgroundColor.r +
      0.587 * backgroundColor.g +
      0.114 * backgroundColor.b;
  return luminance > 0.5 ? Colors.black : Colors.white;
}

bool isHighContrast(Color backgroundColor) {
  final luminance =
      0.299 * backgroundColor.r +
      0.587 * backgroundColor.g +
      0.114 * backgroundColor.b;
  return luminance < 0.3 || luminance > 0.7;
}
