// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

class WorkspaceTopBarButton extends StatelessWidget {
  const WorkspaceTopBarButton({
    super.key,
    required this.open,
    required this.hasBoundWorkspace,
    required this.onPressed,
  });

  final bool open;
  final bool hasBoundWorkspace;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return ThemeWorkspaceTopBarButton(
      open: open,
      hasBoundWorkspace: hasBoundWorkspace,
      onPressed: onPressed,
    );
  }
}
