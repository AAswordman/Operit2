// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';

enum SettingsCategory {
  model,
  localModels,
  tts,
  characters,
  tools,
  workspace,
  appearance,
  data,
  accessLinks,
  about,
}

class SettingsCategorySpec {
  const SettingsCategorySpec({
    required this.title,
    required this.subtitle,
    required this.description,
    required this.icon,
  });

  final String title;
  final String subtitle;
  final String description;
  final IconData icon;

  static SettingsCategorySpec of(
    SettingsCategory category,
    AppLocalizations l10n,
  ) {
    return switch (category) {
      SettingsCategory.model => SettingsCategorySpec(
        title: l10n.settingsCategoryModelTitle,
        subtitle: l10n.settingsCategoryModelSubtitle,
        description: l10n.settingsCategoryModelDescription,
        icon: Icons.hub_outlined,
      ),
      SettingsCategory.localModels => const SettingsCategorySpec(
        title: '本地模型',
        subtitle: '下载、引擎、STT / TTS',
        description: '管理按需安装的本地模型和推理引擎。',
        icon: Icons.memory_outlined,
      ),
      SettingsCategory.tts => const SettingsCategorySpec(
        title: '语音与识别',
        subtitle: 'TTS 与 STT 供应商',
        description: '管理语音合成与语音识别供应商，并分别选择当前配置。',
        icon: Icons.record_voice_over_outlined,
      ),
      SettingsCategory.characters => SettingsCategorySpec(
        title: l10n.settingsCategoryCharactersTitle,
        subtitle: l10n.settingsCategoryCharactersSubtitle,
        description: l10n.settingsCategoryCharactersDescription,
        icon: Icons.badge_outlined,
      ),
      SettingsCategory.tools => SettingsCategorySpec(
        title: l10n.settingsCategoryToolsTitle,
        subtitle: l10n.settingsCategoryToolsSubtitle,
        description: l10n.settingsCategoryToolsDescription,
        icon: Icons.admin_panel_settings_outlined,
      ),
      SettingsCategory.workspace => SettingsCategorySpec(
        title: l10n.settingsCategoryWorkspaceTitle,
        subtitle: l10n.settingsCategoryWorkspaceSubtitle,
        description: l10n.settingsCategoryWorkspaceDescription,
        icon: Icons.folder_outlined,
      ),
      SettingsCategory.appearance => SettingsCategorySpec(
        title: l10n.settingsCategoryAppearanceTitle,
        subtitle: l10n.settingsCategoryAppearanceSubtitle,
        description: l10n.settingsCategoryAppearanceDescription,
        icon: Icons.palette_outlined,
      ),
      SettingsCategory.data => SettingsCategorySpec(
        title: l10n.settingsCategoryDataTitle,
        subtitle: l10n.settingsCategoryDataSubtitle,
        description: l10n.settingsCategoryDataDescription,
        icon: Icons.backup_outlined,
      ),
      SettingsCategory.accessLinks => SettingsCategorySpec(
        title: l10n.settingsCategoryAccessLinksTitle,
        subtitle: l10n.settingsCategoryAccessLinksSubtitle,
        description: l10n.settingsCategoryAccessLinksDescription,
        icon: Icons.devices_outlined,
      ),
      SettingsCategory.about => const SettingsCategorySpec(
        title: '关于 Operit2',
        subtitle: '版本、项目与许可',
        description: '查看 Operit2 的版本信息、项目链接和开源许可证。',
        icon: Icons.info_outline,
      ),
    };
  }
}
