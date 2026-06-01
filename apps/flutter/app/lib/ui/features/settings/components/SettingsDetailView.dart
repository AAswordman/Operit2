// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../models/SettingsModels.dart';

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
    final spec = SettingsCategorySpec.of(category);
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 24, 28, 36),
      children: <Widget>[
        if (showHeader) ...<Widget>[
          Row(
            children: <Widget>[
              Icon(
                spec.icon,
                size: 28,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  spec.title,
                  style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                    fontWeight: FontWeight.w700,
                    letterSpacing: 0,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            spec.description,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 24),
        ],
        for (final section in spec.sections) SettingsSection(section: section),
      ],
    );
  }
}

class SettingsSection extends StatelessWidget {
  const SettingsSection({super.key, required this.section});

  final SettingsSectionSpec section;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            section.title,
            style: theme.textTheme.titleSmall?.copyWith(
              color: colorScheme.primary,
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 8),
          DecoratedBox(
            decoration: BoxDecoration(
              border: Border.all(
                color: colorScheme.outlineVariant.withValues(alpha: 0.55),
              ),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              children: <Widget>[
                for (var index = 0; index < section.items.length; index++)
                  SettingsPlaceholderRow(
                    item: section.items[index],
                    showDivider: index < section.items.length - 1,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class SettingsPlaceholderRow extends StatelessWidget {
  const SettingsPlaceholderRow({
    super.key,
    required this.item,
    required this.showDivider,
  });

  final SettingsItemSpec item;
  final bool showDivider;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Column(
      children: <Widget>[
        Material(
          color: Colors.transparent,
          child: InkWell(
            onTap: () {},
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
              child: Row(
                children: <Widget>[
                  Icon(
                    item.icon,
                    size: 21,
                    color: colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 14),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          item.title,
                          style: theme.textTheme.bodyMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          item.description,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 10),
                  Icon(
                    Icons.chevron_right,
                    size: 18,
                    color: colorScheme.onSurfaceVariant.withValues(alpha: 0.72),
                  ),
                ],
              ),
            ),
          ),
        ),
        if (showDivider)
          Divider(
            height: 1,
            indent: 49,
            color: colorScheme.outlineVariant.withValues(alpha: 0.55),
          ),
      ],
    );
  }
}
