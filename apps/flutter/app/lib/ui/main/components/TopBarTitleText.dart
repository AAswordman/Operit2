// ignore_for_file: file_names

import 'package:flutter/material.dart';

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
    final theme = Theme.of(context);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: <Widget>[
        Flexible(
          fit: FlexFit.loose,
          child: Text(
            primaryText,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.titleSmall?.copyWith(
              color: contentColor,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        if (secondaryText.isNotEmpty)
          Flexible(
            child: Text(
              '- $secondaryText',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: contentColor.withValues(alpha: 0.8),
              ),
            ),
          ),
      ],
    );
  }
}
