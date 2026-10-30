// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

/// Joins the workspace to its host panel with only a leading divider.
class WorkspacePanelSurface extends StatelessWidget {
  const WorkspacePanelSurface({
    super.key,
    required this.transparentSurface,
    required this.child,
  });

  final bool transparentSurface;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return DecoratedBox(
      position: DecorationPosition.foreground,
      // A one-sided border must be painted without a borderRadius, including
      // BorderRadius.zero. Keep it outside the glass surface's decoration.
      decoration: BoxDecoration(
        border: BorderDirectional(
          start: BorderSide(
            color: colors.outlineVariant.withValues(alpha: 0.45),
          ),
        ),
      ),
      child: ThemeGlassSurface(
        color: colors.surface,
        transparentSurface: transparentSurface,
        layer: ThemeGlassSurfaceLayer.panel,
        child: child,
      ),
    );
  }
}
