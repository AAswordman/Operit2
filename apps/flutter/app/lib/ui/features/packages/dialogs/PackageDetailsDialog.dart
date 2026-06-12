// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../utils/PackageDisplayUtils.dart';

class PluginDetailsDialog extends StatefulWidget {
  const PluginDetailsDialog({
    super.key,
    required this.plugin,
    required this.enabled,
    required this.packageManager,
    required this.onEnabledChanged,
    required this.onOpenUi,
    required this.onDeletePackage,
  });

  final core_proxy.ToolPkgContainerRuntime plugin;
  final bool enabled;
  final GeneratedPermissionsPackToolPackageManagerCoreProxy packageManager;
  final ValueChanged<bool> onEnabledChanged;
  final ValueChanged<String?> onOpenUi;
  final VoidCallback? onDeletePackage;

  @override
  State<PluginDetailsDialog> createState() => _PluginDetailsDialogState();
}

class _PluginDetailsDialogState extends State<PluginDetailsDialog> {
  core_proxy.ToolPkgContainerDetails? _details;
  bool _loadingDetails = true;
  String? _toggleError;
  final Set<String> _togglingSubpackages = <String>{};

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_loadingDetails && _details == null) {
      unawaited(_loadDetails());
    }
  }

  Future<void> _loadDetails() async {
    final useEnglish = _useEnglishForToolPkgText(context);
    try {
      final details = await widget.packageManager.getToolPkgContainerDetails(
        packageName: widget.plugin.packageName,
        useEnglish: useEnglish,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _details = details;
        _loadingDetails = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load ToolPkg details: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _loadingDetails = false;
      });
    }
  }

  Future<void> _setSubpackageEnabled(
    core_proxy.ToolPkgSubpackageInfo subpackage,
    bool enabled,
  ) async {
    setState(() {
      _toggleError = null;
      _togglingSubpackages.add(subpackage.packageName);
    });
    try {
      final success = await widget.packageManager.setToolPkgSubpackageEnabled(
        subpackagePackageName: subpackage.packageName,
        enabled: enabled,
      );
      if (!mounted) {
        return;
      }
      if (!success) {
        setState(() {
          _toggleError = '子包状态切换失败：${subpackage.packageName}';
        });
      }
      await _loadDetails();
    } catch (error, stackTrace) {
      debugPrint('Failed to toggle ToolPkg subpackage: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _toggleError = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _togglingSubpackages.remove(subpackage.packageName);
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final details = _details;
    final displayName = details?.displayName.trim().isNotEmpty == true
        ? details!.displayName
        : toolPkgContainerDisplayName(widget.plugin);
    final description = details?.description.trim().isNotEmpty == true
        ? details!.description
        : localizedText(widget.plugin.description);
    return AlertDialog(
      icon: const Icon(Icons.extension_outlined),
      title: Text(displayName),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 620, maxHeight: 620),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              _DetailLine(label: 'ID', value: widget.plugin.packageName),
              _DetailLine(
                label: l10n.version,
                value: details?.version ?? widget.plugin.version,
              ),
              _DetailLine(
                label: l10n.author,
                value: (details?.author ?? widget.plugin.author).join(', '),
              ),
              _DetailLine(label: l10n.entry, value: widget.plugin.mainEntry),
              _DetailLine(label: l10n.source, value: widget.plugin.sourcePath),
              const SizedBox(height: 12),
              _DescriptionText(description),
              const SizedBox(height: 16),
              _SectionTitle(text: l10n.toolPkgResources),
              const SizedBox(height: 8),
              _SummaryCard(
                rows: <String>[
                  l10n.resourcesCount(
                    details?.resourceCount ?? widget.plugin.resources.length,
                  ),
                  l10n.uiModulesCount(
                    details?.uiModuleCount ?? widget.plugin.uiModules.length,
                  ),
                  l10n.navigationEntriesCount(
                    widget.plugin.navigationEntries.length,
                  ),
                  l10n.desktopWidgetsCount(widget.plugin.desktopWidgets.length),
                  l10n.workspaceTemplatesCount(
                    details?.workspaceTemplateCount ??
                        widget.plugin.workspaceTemplates.length,
                  ),
                  'AI Provider ${widget.plugin.aiProviders.length}',
                ],
              ),
              if (_loadingDetails) ...<Widget>[
                const SizedBox(height: 14),
                const Center(child: CircularProgressIndicator()),
              ],
              if (details != null &&
                  details.toolboxUiModules.isNotEmpty) ...<Widget>[
                const SizedBox(height: 14),
                _SectionTitle(text: l10n.pluginConfiguration),
                const SizedBox(height: 8),
                for (final module in details.toolboxUiModules)
                  _ModuleTile(
                    title: module.title,
                    subtitle: '${module.uiModuleId} · ${module.runtime}',
                    icon: Icons.tune_outlined,
                    trailing: FilledButton.tonalIcon(
                      onPressed: widget.enabled
                          ? () => widget.onOpenUi(module.routeId)
                          : null,
                      icon: const Icon(Icons.open_in_new, size: 18),
                      label: Text(widget.enabled ? '打开' : '启用后打开'),
                      style: FilledButton.styleFrom(
                        visualDensity: VisualDensity.compact,
                        padding: const EdgeInsets.symmetric(horizontal: 10),
                      ),
                    ),
                  ),
              ],
              const SizedBox(height: 14),
              _SectionTitle(text: l10n.subpackages),
              const SizedBox(height: 8),
              if (_toggleError != null) ...<Widget>[
                Text(
                  _toggleError!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
                const SizedBox(height: 8),
              ],
              if (details == null && !_loadingDetails)
                _EmptyCard(message: l10n.toolPkgNoSubpackages)
              else if (details != null && details.subpackages.isEmpty)
                _EmptyCard(message: l10n.toolPkgNoSubpackages)
              else if (details != null)
                for (final subpackage in details.subpackages)
                  _ModuleTile(
                    title: subpackage.displayName,
                    subtitle: subpackage.description.trim().isNotEmpty
                        ? subpackage.description
                        : l10n.subpackageToolCount(
                            subpackage.packageName,
                            subpackage.toolCount,
                          ),
                    icon: Icons.inventory_2_outlined,
                    trailing: Switch(
                      value: subpackage.enabled,
                      onChanged:
                          widget.enabled &&
                              !_togglingSubpackages.contains(
                                subpackage.packageName,
                              )
                          ? (enabled) =>
                                _setSubpackageEnabled(subpackage, enabled)
                          : null,
                    ),
                  ),
              if (details != null &&
                  details.workspaceTemplates.isNotEmpty) ...<Widget>[
                const SizedBox(height: 14),
                _SectionTitle(text: l10n.workspaceTemplates),
                const SizedBox(height: 8),
                for (final template in details.workspaceTemplates)
                  _ModuleTile(
                    title: template.displayName,
                    subtitle: template.description,
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
        if (widget.onDeletePackage != null)
          OutlinedButton.icon(
            onPressed: widget.onDeletePackage,
            icon: const Icon(Icons.delete_outline),
            label: const Text('删除'),
            style: OutlinedButton.styleFrom(
              foregroundColor: Theme.of(context).colorScheme.error,
            ),
          ),
        if (toolPkgHasUi(widget.plugin))
          OutlinedButton.icon(
            onPressed: widget.enabled ? () => widget.onOpenUi(null) : null,
            icon: const Icon(Icons.open_in_new_outlined),
            label: Text(widget.enabled ? '打开' : '启用后打开'),
          ),
        FilledButton.icon(
          onPressed: () => widget.onEnabledChanged(!widget.enabled),
          icon: Icon(
            widget.enabled ? Icons.toggle_off_outlined : Icons.toggle_on,
          ),
          label: Text(widget.enabled ? l10n.disable : l10n.enable),
        ),
      ],
    );
  }
}

bool _useEnglishForToolPkgText(BuildContext context) {
  return Localizations.localeOf(context).languageCode.toLowerCase() != 'zh';
}

class PackageDetailsDialog extends StatelessWidget {
  const PackageDetailsDialog({
    super.key,
    required this.package,
    required this.enabled,
    required this.onEnabledChanged,
    required this.onDeletePackage,
    required this.onRunTool,
  });

  final core_proxy.ToolPackage package;
  final bool enabled;
  final ValueChanged<bool> onEnabledChanged;
  final VoidCallback onDeletePackage;
  final ValueChanged<core_proxy.PackageTool> onRunTool;

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
                for (final tool in package.tools)
                  _ToolTile(tool: tool, onRun: () => onRunTool(tool)),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.close),
        ),
        if (!package.isBuiltIn)
          OutlinedButton.icon(
            onPressed: onDeletePackage,
            icon: const Icon(Icons.delete_outline),
            label: const Text('删除'),
            style: OutlinedButton.styleFrom(
              foregroundColor: Theme.of(context).colorScheme.error,
            ),
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
  const _ToolTile({required this.tool, required this.onRun});

  final core_proxy.PackageTool tool;
  final VoidCallback onRun;

  @override
  Widget build(BuildContext context) {
    return _ModuleTile(
      title: tool.name,
      subtitle: localizedText(tool.description),
      icon: Icons.build_outlined,
      trailing: FilledButton.tonalIcon(
        onPressed: onRun,
        icon: const Icon(Icons.play_arrow, size: 18),
        label: const Text('运行'),
        style: FilledButton.styleFrom(
          visualDensity: VisualDensity.compact,
          padding: const EdgeInsets.symmetric(horizontal: 10),
        ),
      ),
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
