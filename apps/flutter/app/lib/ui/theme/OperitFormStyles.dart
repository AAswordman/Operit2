// ignore_for_file: file_names

import 'package:flutter/material.dart';

class OperitFormStyles {
  const OperitFormStyles._();

  static TextStyle? dropdownTextStyle(BuildContext context) {
    final theme = Theme.of(context);
    return theme.textTheme.bodyMedium?.copyWith(
      color: theme.colorScheme.onSurface,
    );
  }
}
