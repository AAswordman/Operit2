// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

class TopBarTitleText extends StatelessWidget {
  const TopBarTitleText({
    super.key,
    required this.primaryText,
    required this.contentColor,
    this.secondaryText = '',
  });

  final String primaryText;
  final String secondaryText;
  final Color contentColor;

  @override
  Widget build(BuildContext context) {
    return ThemeTopBarTitle(
      primaryText: primaryText,
      secondaryText: secondaryText,
      contentColor: contentColor,
    );
  }
}
