// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
import '../components/EmptyState.dart';
import '../components/PackageGrid.dart';
import '../components/PackageListItem.dart';
import '../utils/PackageDisplayUtils.dart';

class PluginTabContent extends StatelessWidget {
  /// Creates the plugin tab content.
  const PluginTabContent({
    super.key,
    required this.plugins,
    required this.morePlugins,
    required this.enabledPluginNames,
    required this.isLoading,
    required this.isSearchActive,
    required this.onOpenPluginUi,
    required this.onPluginTap,
    required this.onLoadMorePlugin,
    required this.onPluginEnabledChanged,
  });

  final List<core_proxy.ToolPkgContainerRuntime> plugins;
  final List<core_proxy.BundledExternalPackageCandidate> morePlugins;
  final Set<String> enabledPluginNames;
  final bool isLoading;
  final bool isSearchActive;
  final ValueChanged<core_proxy.ToolPkgContainerRuntime> onOpenPluginUi;
  final ValueChanged<core_proxy.ToolPkgContainerRuntime> onPluginTap;
  final ValueChanged<core_proxy.BundledExternalPackageCandidate>
  onLoadMorePlugin;
  final void Function(core_proxy.ToolPkgContainerRuntime plugin, bool enabled)
  onPluginEnabledChanged;

  /// Builds the plugin tab with lazily rendered expandable sections.
  @override
  Widget build(BuildContext context) {
    if (plugins.isEmpty && morePlugins.isEmpty && isLoading) {
      return const M3LoadingPane();
    }
    return Stack(
      children: <Widget>[
        CustomScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          slivers: <Widget>[
            if (plugins.isEmpty && morePlugins.isEmpty)
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
                sliver: SliverToBoxAdapter(
                  child: EmptyState(
                    icon: Icons.extension_off_outlined,
                    title: '没有插件',
                    message: isSearchActive
                        ? '没有匹配的插件。'
                        : '当前没有可显示的 ToolPkg 插件。',
                    scrollable: false,
                  ),
                ),
              )
            else ...<Widget>[
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
                sliver: const SliverToBoxAdapter(
                  child: _PluginSectionHeader(title: '当前插件'),
                ),
              ),
              if (plugins.isEmpty)
                SliverPadding(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 0),
                  sliver: SliverToBoxAdapter(
                    child: _PluginSectionEmpty(
                      message: isSearchActive ? '没有匹配的当前插件。' : '当前没有可显示的插件。',
                    ),
                  ),
                )
              else
                SliverPadding(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 0),
                  sliver: PackageSliverList(
                    itemCount: plugins.length,
                    itemBuilder: (context, index) {
                      final plugin = plugins[index];
                      return PackageListItem(
                        key: ValueKey<String>('plugin:${plugin.packageName}'),
                        icon: Icons.extension_outlined,
                        title: toolPkgContainerDisplayName(plugin),
                        subtitle: localizedText(plugin.description),
                        metadata: <String>[
                          plugin.packageName,
                          'v${plugin.version}',
                          '${plugin.subpackages.length} 子包',
                        ],
                        enabled: enabledPluginNames.contains(
                          plugin.packageName,
                        ),
                        onDetails: () => onPluginTap(plugin),
                        onEnabledChanged: (enabled) =>
                            onPluginEnabledChanged(plugin, enabled),
                        trailingActions: toolPkgHasUi(plugin)
                            ? <Widget>[
                                IconButton(
                                  tooltip:
                                      enabledPluginNames.contains(
                                        plugin.packageName,
                                      )
                                      ? '打开'
                                      : '启用后打开',
                                  onPressed:
                                      enabledPluginNames.contains(
                                        plugin.packageName,
                                      )
                                      ? () => onOpenPluginUi(plugin)
                                      : null,
                                  icon: const Icon(Icons.open_in_new_outlined),
                                ),
                              ]
                            : const <Widget>[],
                      );
                    },
                  ),
                ),
              if (morePlugins.isNotEmpty) ...<Widget>[
                const SliverToBoxAdapter(child: SizedBox(height: 16)),
                SliverPadding(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 0),
                  sliver: const SliverToBoxAdapter(
                    child: _PluginSectionHeader(
                      title: '更多插件',
                      subtitle: 'App 自带的官方额外插件，加载后进入当前插件。',
                    ),
                  ),
                ),
                SliverPadding(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 120),
                  sliver: PackageSliverList(
                    itemCount: morePlugins.length,
                    itemBuilder: (context, index) {
                      final plugin = morePlugins[index];
                      final kindLabel = plugin.isToolPkg ? 'ToolPkg' : '脚本包';
                      return PackageListItem(
                        key: ValueKey<String>(
                          'bundled-plugin:${plugin.packageName}',
                        ),
                        icon: plugin.isToolPkg
                            ? Icons.extension_outlined
                            : Icons.inventory_2_outlined,
                        title: bundledExternalPackageDisplayName(plugin),
                        subtitle: localizedText(plugin.description),
                        metadata: <String>[
                          plugin.packageName,
                          kindLabel,
                          if (plugin.version.trim().isNotEmpty)
                            'v${plugin.version}',
                          '${plugin.toolCount} 工具',
                          if (plugin.subpackageCount > 0)
                            '${plugin.subpackageCount} 子包',
                          '官方额外',
                        ],
                        enabled: false,
                        onEnabledChanged: (_) {},
                        showEnabledSwitch: false,
                        trailingActions: <Widget>[
                          FilledButton.tonalIcon(
                            onPressed: () => onLoadMorePlugin(plugin),
                            icon: const Icon(Icons.add, size: 18),
                            label: const Text('加载'),
                            style: FilledButton.styleFrom(
                              visualDensity: VisualDensity.compact,
                              padding: const EdgeInsets.symmetric(
                                horizontal: 10,
                              ),
                            ),
                          ),
                        ],
                      );
                    },
                  ),
                ),
              ] else
                const SliverToBoxAdapter(child: SizedBox(height: 120)),
            ],
          ],
        ),
        if ((plugins.isNotEmpty || morePlugins.isNotEmpty) && isLoading)
          const Positioned.fill(child: M3LoadingOverlay()),
      ],
    );
  }
}

class _PluginSectionHeader extends StatelessWidget {
  /// Creates a package section heading.
  const _PluginSectionHeader({required this.title, this.subtitle});

  final String title;
  final String? subtitle;

  /// Builds a section heading for a package group.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final subtitle = this.subtitle;
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 4, 4, 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            title,
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
          ),
          if (subtitle != null) ...<Widget>[
            const SizedBox(height: 2),
            Text(
              subtitle,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _PluginSectionEmpty extends StatelessWidget {
  /// Creates a package section empty state.
  const _PluginSectionEmpty({required this.message});

  final String message;

  /// Builds the empty state shown for a package section.
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 0, 4, 4),
      child: Text(
        message,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}
