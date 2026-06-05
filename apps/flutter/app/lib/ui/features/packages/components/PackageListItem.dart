// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../theme/OperitGlassSurface.dart';

class PackageListItem extends StatelessWidget {
  const PackageListItem({
    super.key,
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.metadata,
    required this.enabled,
    required this.onTap,
    required this.onEnabledChanged,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final List<String> metadata;
  final bool enabled;
  final VoidCallback onTap;
  final ValueChanged<bool> onEnabledChanged;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final nonEmptyMetadata = metadata
        .where((item) => item.trim().isNotEmpty)
        .toList(growable: false);
    final borderRadius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.all(4),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.42),
        layer: OperitGlassSurfaceLayer.card,
        borderRadius: borderRadius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: SizedBox.expand(
          child: InkWell(
            borderRadius: borderRadius,
            onTap: onTap,
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.center,
                children: <Widget>[
                  CircleAvatar(
                    radius: 19,
                    backgroundColor: colorScheme.primary.withValues(
                      alpha: 0.12,
                    ),
                    child: Icon(icon, color: colorScheme.primary, size: 20),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          title,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.titleSmall
                              ?.copyWith(fontWeight: FontWeight.w700),
                        ),
                        if (subtitle.isNotEmpty)
                          Flexible(
                            child: Padding(
                              padding: const EdgeInsets.only(top: 3),
                              child: Text(
                                subtitle,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: Theme.of(context).textTheme.bodySmall
                                    ?.copyWith(
                                      color: colorScheme.onSurfaceVariant,
                                    ),
                              ),
                            ),
                          ),
                        if (nonEmptyMetadata.isNotEmpty) ...<Widget>[
                          const SizedBox(height: 3),
                          SizedBox(
                            height: 24,
                            child: ListView.separated(
                              scrollDirection: Axis.horizontal,
                              itemCount: nonEmptyMetadata.length,
                              separatorBuilder: (context, index) =>
                                  const SizedBox(width: 6),
                              itemBuilder: (context, index) =>
                                  _MetadataChip(text: nonEmptyMetadata[index]),
                            ),
                          ),
                        ],
                      ],
                    ),
                  ),
                  const SizedBox(width: 8),
                  Switch(value: enabled, onChanged: onEnabledChanged),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _MetadataChip extends StatelessWidget {
  const _MetadataChip({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
        child: Text(
          text,
          style: Theme.of(
            context,
          ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ),
    );
  }
}
