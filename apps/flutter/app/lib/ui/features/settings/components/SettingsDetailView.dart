// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import '../appearance/AppearanceSettingsPanel.dart';
import '../access_links/AccessLinksSettingsPanel.dart';
import '../about/AboutOperitScreen.dart';
import '../data/DataSettingsPanel.dart';
import '../global_behavior/GlobalBehaviorSettingsPanel.dart';
import '../model/ModelSettingsPanel.dart';
import '../local_models/LocalModelSettingsPanel.dart';
import '../models/SettingsModels.dart';
import '../profile/UserProfileSettingsPanel.dart';
import '../tools/ToolSettingsPanel.dart';
import '../workspace/WorkspaceSettingsPanel.dart';

class SettingsDetailView extends StatelessWidget {
  const SettingsDetailView({
    super.key,
    required this.category,
    required this.onOpenProfile,
    this.showHeader = true,
    this.onProfileChanged,
  });

  final SettingsCategory category;
  final VoidCallback onOpenProfile;
  final bool showHeader;
  final VoidCallback? onProfileChanged;

  /// Shares matching panel blurs within this page, including during transitions.
  @override
  Widget build(BuildContext context) {
    return ThemeGlassSurfaceGroup(
      child: switch (category) {
        SettingsCategory.profile => UserProfileSettingsPanel(
          onProfileChanged: onProfileChanged,
        ),
        SettingsCategory.model => const ModelSettingsPanel(),
        SettingsCategory.localModels => const LocalModelSettingsPanel(),
        SettingsCategory.tts => const ModelSettingsPanel(),
        SettingsCategory.tools => const ToolSettingsPanel(),
        SettingsCategory.workspace => const WorkspaceSettingsPanel(),
        SettingsCategory.globalBehavior => const GlobalBehaviorSettingsPanel(),
        SettingsCategory.appearance => const AppearanceSettingsPanel(),
        SettingsCategory.data => const DataSettingsPanel(),
        SettingsCategory.accessLinks => AccessLinksSettingsPanel(
          onOpenProfile: onOpenProfile,
        ),
        SettingsCategory.about => const AboutOperitScreen(),
      },
    );
  }
}
