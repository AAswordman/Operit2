// ignore_for_file: file_names

import 'package:flutter/material.dart';

class ClassicChatSettingsBar extends StatelessWidget {
  const ClassicChatSettingsBar({
    super.key,
    required this.settingsKey,
    required this.onSettings,
  });

  final GlobalKey settingsKey;
  final VoidCallback? onSettings;

  /// Builds the floating classic settings strip above the input surface.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.92),
      elevation: 2,
      shadowColor: Colors.black.withValues(alpha: 0.12),
      borderRadius: BorderRadius.circular(18),
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
        child: _ClassicSettingsButton(
          targetKey: settingsKey,
          onTap: onSettings,
        ),
      ),
    );
  }
}

class _ClassicSettingsButton extends StatelessWidget {
  const _ClassicSettingsButton({required this.targetKey, required this.onTap});

  final GlobalKey targetKey;
  final VoidCallback? onTap;

  /// Builds the menu button used by the classic settings strip.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return InkResponse(
      key: targetKey,
      onTap: onTap,
      radius: 18,
      child: SizedBox(
        width: 30,
        height: 30,
        child: Icon(
          Icons.tune_outlined,
          size: 18,
          color: colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}
