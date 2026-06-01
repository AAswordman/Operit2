// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../models/SettingsModels.dart';

class SettingsCategoryList extends StatelessWidget {
  const SettingsCategoryList({
    super.key,
    required this.selectedCategory,
    required this.onCategorySelected,
  });

  final SettingsCategory? selectedCategory;
  final ValueChanged<SettingsCategory> onCategorySelected;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 18, 16, 24),
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.fromLTRB(4, 0, 4, 14),
          child: Text(
            '设置',
            style: Theme.of(context).textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.w700,
              letterSpacing: 0,
            ),
          ),
        ),
        for (final category in SettingsCategory.values)
          SettingsCategoryTile(
            spec: SettingsCategorySpec.of(category),
            selected: selectedCategory == category,
            onTap: () => onCategorySelected(category),
          ),
      ],
    );
  }
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

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final background = selected
        ? colorScheme.primaryContainer.withValues(alpha: 0.72)
        : Colors.transparent;
    final foreground = selected
        ? colorScheme.onPrimaryContainer
        : colorScheme.onSurface;
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Material(
        color: background,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          borderRadius: BorderRadius.circular(8),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
            child: Row(
              children: <Widget>[
                Icon(spec.icon, size: 21, color: foreground),
                const SizedBox(width: 12),
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
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        spec.subtitle,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: foreground.withValues(alpha: 0.72),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Icon(
                  Icons.chevron_right,
                  size: 18,
                  color: foreground.withValues(alpha: 0.62),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
