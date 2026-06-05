// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../appearance/AppearanceSettingsPanel.dart';
import '../characters/CharacterSettingsPanel.dart';
import '../data/DataSettingsPanel.dart';
import '../model/ModelSettingsPanel.dart';
import '../models/SettingsModels.dart';
import '../tools/ToolSettingsPanel.dart';
import '../workspace/WorkspaceSettingsPanel.dart';

class SettingsDetailView extends StatelessWidget {
  const SettingsDetailView({
    super.key,
    required this.category,
    this.showHeader = true,
  });

  final SettingsCategory category;
  final bool showHeader;

  @override
  Widget build(BuildContext context) {
    return switch (category) {
      SettingsCategory.model => const ModelSettingsPanel(),
      SettingsCategory.characters => const CharacterSettingsPanel(),
      SettingsCategory.tools => const ToolSettingsPanel(),
      SettingsCategory.workspace => const WorkspaceSettingsPanel(),
      SettingsCategory.appearance => const AppearanceSettingsPanel(),
      SettingsCategory.data => const DataSettingsPanel(),
    };
  }
}
