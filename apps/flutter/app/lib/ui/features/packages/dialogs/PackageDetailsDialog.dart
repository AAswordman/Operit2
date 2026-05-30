// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../utils/PackageDisplayUtils.dart';

class PluginDetailsDialog extends StatelessWidget {
  const PluginDetailsDialog({
    super.key,
    required this.plugin,
    required this.enabled,
    required this.onEnabledChanged,
  });

  final core_proxy.ToolPkgContainerRuntime plugin;
  final bool enabled;
  final ValueChanged<bool> onEnabledChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      icon: const Icon(Icons.extension_outlined),
      title: Text(toolPkgContainerDisplayName(plugin)),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 620, maxHeight: 620),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              _DetailLine(label: 'ID', value: plugin.packageName),
              _DetailLine(label: l10n.version, value: plugin.version),
              _DetailLine(label: l10n.author, value: plugin.author.join(', ')),
              _DetailLine(label: l10n.entry, value: plugin.mainEntry),
              _DetailLine(label: l10n.source, value: plugin.sourcePath),
              const SizedBox(height: 12),
              _DescriptionText(localizedText(plugin.description)),
              const SizedBox(height: 16),
              _SectionTitle(text: l10n.toolPkgResources),
              const SizedBox(height: 8),
              _SummaryCard(
                rows: <String>[
                  l10n.resourcesCount(plugin.resources.length),
                  l10n.uiModulesCount(plugin.uiModules.length),
                  l10n.navigationEntriesCount(plugin.navigationEntries.length),
                  l10n.desktopWidgetsCount(plugin.desktopWidgets.length),
                  l10n.workflowTemplatesCount(plugin.workflowTemplates.length),
                  l10n.workspaceTemplatesCount(
                    plugin.workspaceTemplates.length,
                  ),
                  'AI Provider ${plugin.aiProviders.length}',
                ],
              ),
              if (plugin.uiModules.isNotEmpty) ...<Widget>[
                const SizedBox(height: 14),
                _SectionTitle(text: l10n.pluginConfiguration),
                const SizedBox(height: 8),
                for (final module in plugin.uiModules)
                  _ModuleTile(
                    title: localizedText(module.title),
                    subtitle: '${module.id} · ${module.runtime}',
                    icon: Icons.tune_outlined,
                  ),
              ],
              const SizedBox(height: 14),
              _SectionTitle(text: l10n.subpackages),
              const SizedBox(height: 8),
              if (plugin.subpackages.isEmpty)
                _EmptyCard(message: l10n.toolPkgNoSubpackages)
              else
                for (final subpackage in plugin.subpackages)
                  _ModuleTile(
                    title: toolPkgSubpackageDisplayName(subpackage),
                    subtitle: l10n.subpackageToolCount(
                      subpackage.packageName,
                      subpackage.toolCount,
                    ),
                    icon: Icons.inventory_2_outlined,
                    trailing: subpackage.enabledByDefault
                        ? _SmallBadge(text: l10n.enabledByDefault)
                        : null,
                  ),
              if (plugin.workflowTemplates.isNotEmpty) ...<Widget>[
                const SizedBox(height: 14),
                _SectionTitle(text: l10n.workflowTemplates),
                const SizedBox(height: 8),
                for (final template in plugin.workflowTemplates)
                  _ModuleTile(
                    title: localizedText(template.displayName),
                    subtitle: localizedText(template.description),
                    icon: Icons.account_tree_outlined,
                    trailing: _SmallBadge(text: template.id),
                  ),
              ],
              if (plugin.workspaceTemplates.isNotEmpty) ...<Widget>[
                const SizedBox(height: 14),
                _SectionTitle(text: l10n.workspaceTemplates),
                const SizedBox(height: 8),
                for (final template in plugin.workspaceTemplates)
                  _ModuleTile(
                    title: localizedText(template.displayName),
                    subtitle: localizedText(template.description),
                    icon: Icons.folder_copy_outlined,
                    trailing: _SmallBadge(text: template.projectType),
                  ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.close),
        ),
        FilledButton.icon(
          onPressed: () => onEnabledChanged(!enabled),
          icon: Icon(enabled ? Icons.toggle_off_outlined : Icons.toggle_on),
          label: Text(enabled ? l10n.disable : l10n.enable),
        ),
      ],
    );
  }
}

class PackageDetailsDialog extends StatelessWidget {
  const PackageDetailsDialog({
    super.key,
    required this.package,
    required this.enabled,
    required this.onEnabledChanged,
  });

  final core_proxy.ToolPackage package;
  final bool enabled;
  final ValueChanged<bool> onEnabledChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      icon: Icon(packageCategoryIcon(package.category)),
      title: Text(toolPackageDisplayName(package)),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 620, maxHeight: 620),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              _DetailLine(label: 'ID', value: package.name),
              _DetailLine(label: l10n.category, value: package.category),
              _DetailLine(label: l10n.author, value: package.author.join(', ')),
              _DetailLine(
                label: l10n.source,
                value: package.isBuiltIn ? l10n.builtIn : l10n.external,
              ),
              _DetailLine(
                label: l10n.defaultStatus,
                value: package.enabledByDefault
                    ? l10n.enabledByDefault
                    : l10n.disabledByDefault,
              ),
              const SizedBox(height: 12),
              _DescriptionText(localizedText(package.description)),
              if (package.env.isNotEmpty) ...<Widget>[
                const SizedBox(height: 16),
                _SectionTitle(text: l10n.environmentVariables),
                const SizedBox(height: 8),
                for (final env in package.env)
                  _ModuleTile(
                    title: env.name,
                    subtitle: localizedText(env.description),
                    icon: Icons.key_outlined,
                    trailing: env.requiredValue
                        ? _SmallBadge(text: l10n.required)
                        : null,
                  ),
              ],
              if (package.states.isNotEmpty) ...<Widget>[
                const SizedBox(height: 16),
                _SectionTitle(text: l10n.states),
                const SizedBox(height: 8),
                for (final state in package.states)
                  _ModuleTile(
                    title: state.id,
                    subtitle: l10n.stateToolSummary(
                      state.condition,
                      state.tools.length,
                      state.excludeTools.length,
                    ),
                    icon: Icons.rule_outlined,
                    trailing: state.inheritTools
                        ? _SmallBadge(text: l10n.inherit)
                        : null,
                  ),
              ],
              const SizedBox(height: 16),
              _SectionTitle(text: l10n.tools),
              const SizedBox(height: 8),
              if (package.tools.isEmpty)
                _EmptyCard(message: l10n.packageNoTools)
              else
                for (final tool in package.tools) _ToolTile(tool: tool),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.close),
        ),
        FilledButton.icon(
          onPressed: () => onEnabledChanged(!enabled),
          icon: Icon(enabled ? Icons.toggle_off_outlined : Icons.toggle_on),
          label: Text(enabled ? l10n.disable : l10n.enable),
        ),
      ],
    );
  }
}

class _ToolTile extends StatelessWidget {
  const _ToolTile({required this.tool});

  final core_proxy.PackageTool tool;

  @override
  Widget build(BuildContext context) {
    return _ModuleTile(
      title: tool.name,
      subtitle: localizedText(tool.description),
      icon: Icons.build_outlined,
      footer: tool.parameters.isEmpty
          ? null
          : Wrap(
              spacing: 6,
              runSpacing: 6,
              children: tool.parameters
                  .map(
                    (param) => _SmallBadge(
                      text:
                          '${param.name}:${param.parameterType}${param.requiredValue ? "*" : ""}',
                    ),
                  )
                  .toList(growable: false),
            ),
    );
  }
}

class _SummaryCard extends StatelessWidget {
  const _SummaryCard({required this.rows});

  final List<String> rows;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Wrap(
          spacing: 8,
          runSpacing: 8,
          children: rows.map((row) => _SmallBadge(text: row)).toList(),
        ),
      ),
    );
  }
}

class _ModuleTile extends StatelessWidget {
  const _ModuleTile({
    required this.title,
    required this.subtitle,
    required this.icon,
    this.trailing,
    this.footer,
  });

  final String title;
  final String subtitle;
  final IconData icon;
  final Widget? trailing;
  final Widget? footer;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.32),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(icon, size: 18, color: colorScheme.primary),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      if (subtitle.trim().isNotEmpty)
                        Text(
                          subtitle,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(color: colorScheme.onSurfaceVariant),
                        ),
                    ],
                  ),
                ),
                if (trailing != null) ...<Widget>[
                  const SizedBox(width: 8),
                  trailing!,
                ],
              ],
            ),
            if (footer != null) ...<Widget>[const SizedBox(height: 8), footer!],
          ],
        ),
      ),
    );
  }
}

class _EmptyCard extends StatelessWidget {
  const _EmptyCard({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: 0,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Center(child: Text(message)),
      ),
    );
  }
}

class _SmallBadge extends StatelessWidget {
  const _SmallBadge({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.secondaryContainer,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Text(
          text,
          style: Theme.of(context).textTheme.labelSmall?.copyWith(
            color: colorScheme.onSecondaryContainer,
          ),
        ),
      ),
    );
  }
}

class _DescriptionText extends StatelessWidget {
  const _DescriptionText(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    if (text.trim().isEmpty) {
      return const SizedBox.shrink();
    }
    return Text(
      text,
      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
        color: Theme.of(context).colorScheme.onSurfaceVariant,
      ),
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: Theme.of(
        context,
      ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
    );
  }
}

class _DetailLine extends StatelessWidget {
  const _DetailLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    if (value.trim().isEmpty) {
      return const SizedBox.shrink();
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Text(
        '$label: $value',
        style: Theme.of(context).textTheme.bodySmall,
      ),
    );
  }
}
