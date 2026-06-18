// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';
import '../runtime/RuntimeSettingsPanel.dart';
import '../web_access/WebAccessSettingsPanel.dart';

class AccessLinksSettingsPanel extends StatelessWidget {
  const AccessLinksSettingsPanel({super.key});

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: <Widget>[
        const RuntimeSettingsPanel(embedded: true),
        const SizedBox(height: 2),
        _buildAdvancedSection(context),
      ],
    );
  }

  Widget _buildAdvancedSection(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: ClipRRect(
          borderRadius: BorderRadius.circular(12),
          child: ExpansionTile(
            shape: const Border(),
            collapsedShape: const Border(),
            initiallyExpanded: false,
            title: Text(
              l10n.settingsAdvanced,
              style: SettingsControlStyles.sectionTitleTextStyle(context),
            ),
            children: <Widget>[const WebAccessSettingsPanel(embedded: true)],
          ),
        ),
      ),
    );
  }
}
