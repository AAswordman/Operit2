// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../models/SettingsModels.dart';
import '../profile/UserProfileSummaryTile.dart';

class SettingsCategoryList extends StatelessWidget {
  const SettingsCategoryList({
    super.key,
    required this.selectedCategory,
    required this.onCategorySelected,
    this.profileRevision = 0,
  });

  final SettingsCategory? selectedCategory;
  final ValueChanged<SettingsCategory> onCategorySelected;
  final int profileRevision;

  /// Builds the categorized settings navigation list.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final groups = <_SettingsCategoryGroup>[
      _SettingsCategoryGroup(
        title: l10n.settingsCategoryGroupAssistant,
        categories: const <SettingsCategory>[
          SettingsCategory.model,
          SettingsCategory.localModels,
          SettingsCategory.tts,
          SettingsCategory.characters,
        ],
      ),
      _SettingsCategoryGroup(
        title: l10n.settingsCategoryGroupWorkspace,
        categories: const <SettingsCategory>[
          SettingsCategory.tools,
          SettingsCategory.workspace,
        ],
      ),
      _SettingsCategoryGroup(
        title: l10n.settingsCategoryGroupExperience,
        categories: const <SettingsCategory>[
          SettingsCategory.globalBehavior,
          SettingsCategory.appearance,
        ],
      ),
      _SettingsCategoryGroup(
        title: l10n.settingsCategoryGroupSystem,
        categories: const <SettingsCategory>[
          SettingsCategory.data,
          SettingsCategory.accessLinks,
          SettingsCategory.about,
        ],
      ),
    ];
    final theme = Theme.of(context);
    return ListView(
      padding: const EdgeInsets.fromLTRB(10, 10, 10, 16),
      children: <Widget>[
        UserProfileSummaryTile(
          selected: selectedCategory == SettingsCategory.profile,
          revision: profileRevision,
          onTap: () => onCategorySelected(SettingsCategory.profile),
        ),
        for (final groupEntry in groups.asMap().entries) ...<Widget>[
          Padding(
            padding: EdgeInsets.fromLTRB(8, groupEntry.key == 0 ? 2 : 12, 8, 6),
            child: Text(
              groupEntry.value.title,
              style: theme.textTheme.labelSmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
                fontWeight: FontWeight.w800,
                letterSpacing: 0.6,
              ),
            ),
          ),
          for (final category in groupEntry.value.categories)
            SettingsCategoryTile(
              spec: SettingsCategorySpec.of(category, l10n),
              selected: selectedCategory == category,
              onTap: () => onCategorySelected(category),
            ),
        ],
      ],
    );
  }
}

class _SettingsCategoryGroup {
  const _SettingsCategoryGroup({required this.title, required this.categories});

  final String title;
  final List<SettingsCategory> categories;
}

class SettingsCategoryTile extends StatelessWidget {
  const SettingsCategoryTile({
    super.key,
    required this.spec,
    required this.selected,
    required this.onTap,
  });

  final SettingsCategorySpec spec;
  final bool selected;
  final VoidCallback onTap;

  /// Builds a selectable tile for one settings category.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final background = selected
        ? colorScheme.primaryContainer
        : colorScheme.surfaceContainerHighest.withValues(alpha: 0.34);
    final foreground = selected
        ? colorScheme.onPrimaryContainer
        : colorScheme.onSurface;
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: OperitGlassSurface(
        color: background,
        layer: OperitGlassSurfaceLayer.control,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: selected
              ? colorScheme.primary.withValues(alpha: 0.24)
              : colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: InkWell(
          borderRadius: BorderRadius.circular(12),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 9),
            child: Row(
              children: <Widget>[
                CircleAvatar(
                  radius: 16,
                  backgroundColor: selected
                      ? colorScheme.primary.withValues(alpha: 0.16)
                      : colorScheme.surface,
                  child: Icon(spec.icon, size: 18, color: foreground),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        spec.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: foreground,
                          fontWeight: FontWeight.w800,
                        ),
                      ),
                      const SizedBox(height: 3),
                      Text(
                        spec.subtitle,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: foreground.withValues(alpha: 0.70),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
