// ignore_for_file: file_names

import 'package:flutter/material.dart';

/// Shared header metrics for thinking and tool activity in chat messages.
abstract final class ChatActivityRowStyle {
  static const double iconSize = 16;
  static const double iconGap = 8;
  static const double contentInset = iconSize + iconGap;
  static const EdgeInsets headerPadding = EdgeInsets.symmetric(vertical: 5);

  /// Uses the chat theme's activity typography for labels and inline summaries.
  static TextStyle textStyle(BuildContext context) => Theme.of(
    context,
  ).textTheme.bodySmall!.copyWith(fontWeight: FontWeight.w400);
}
