// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

/// Shares section styling across workspace settings.
class WorkspaceSettingsSection extends StatelessWidget {
  /// Creates a section with an optional introductory description.
  const WorkspaceSettingsSection({
    super.key,
    required this.title,
    this.description,
    required this.children,
  });

  final String title;
  final String? description;
  final List<Widget> children;

  /// Lays out the title, description and settings controls on one surface.
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colors.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colors.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              const SizedBox(height: 6),
              if (description != null)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  child: Text(
                    description!,
                    style: TextStyle(color: colors.onSurfaceVariant),
                  ),
                ),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}
