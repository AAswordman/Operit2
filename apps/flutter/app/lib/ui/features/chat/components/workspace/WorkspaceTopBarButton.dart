// ignore_for_file: file_names

import 'package:flutter/material.dart';

class WorkspaceTopBarButton extends StatelessWidget {
  const WorkspaceTopBarButton({
    super.key,
    required this.open,
    required this.onPressed,
  });

  final bool open;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsetsDirectional.only(end: 4),
      child: SizedBox(
        width: 48,
        height: 48,
        child: IconButton(
          onPressed: onPressed,
          icon: Icon(
            open ? Icons.code : Icons.code_off,
            color: open ? colorScheme.primary : colorScheme.onSurface,
          ),
          tooltip: open ? 'Close workspace' : 'Workspace',
        ),
      ),
    );
  }
}
