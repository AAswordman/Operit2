// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/EmptyState.dart';
import '../components/PackageGrid.dart';
import '../components/ExtensionScopeControls.dart';
import '../components/PackageListItem.dart';
import '../utils/PackageDisplayUtils.dart';

class PluginTabContent extends StatelessWidget {
  /// Creates the plugin tab content.
  const PluginTabContent({
    super.key,
    required this.plugins,
    required this.morePlugins,
    required this.loadIssues,
    required this.scopes,
    required this.bundledUpdates,
    required this.onUpdatePlugin,
    this.customizedPackageNames = const <String>{},
    required this.onMoveScope,
    required this.enabledPluginNames,
    required this.isSearchActive,
    required this.onOpenPluginUi,
    required this.onPluginTap,
    required this.onLoadMorePlugin,
    required this.onPluginEnabledChanged,
    this.onPluginReordered,
    required this.onLoadIssueTap,
  });

  final List<core_proxy.ToolPkgContainerRuntime> plugins;
  final List<core_proxy.BundledExternalPackageCandidate> morePlugins;
  final List<core_proxy.ToolPkgLoadIssue> loadIssues;
  final Map<String, String> scopes;
  final Map<String, core_proxy.BundledExternalPackageCandidate> bundledUpdates;
  final ValueChanged<core_proxy.BundledExternalPackageCandidate> onUpdatePlugin;
  final Set<String> customizedPackageNames;
  final ValueChanged<String> onMoveScope;

  final Set<String> enabledPluginNames;
  final bool isSearchActive;
  final ValueChanged<core_proxy.ToolPkgContainerRuntime> onOpenPluginUi;
  final ValueChanged<core_proxy.ToolPkgContainerRuntime> onPluginTap;
  final ValueChanged<core_proxy.BundledExternalPackageCandidate>
  onLoadMorePlugin;
  final void Function(core_proxy.ToolPkgContainerRuntime plugin, bool enabled)
  onPluginEnabledChanged;
  final void Function(String sourcePackageName, String targetPackageName)?
  onPluginReordered;
  final ValueChanged<core_proxy.ToolPkgLoadIssue> onLoadIssueTap;

  /// Builds the plugin tab with lazily rendered expandable sections.
  @override
  Widget build(BuildContext context) {
    return CustomScrollView(
      physics: const AlwaysScrollableScrollPhysics(),
      slivers: <Widget>[
        if (plugins.isEmpty && morePlugins.isEmpty && loadIssues.isEmpty)
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
            sliver: SliverToBoxAdapter(
              child: EmptyState(
                icon: Icons.extension_off_outlined,
                title: '没有插件',
                message: isSearchActive ? '没有匹配的插件。' : '当前没有可显示的 ToolPkg 插件。',
                scrollable: false,
              ),
            ),
          )
        else ...<Widget>[
          if (plugins.isEmpty && loadIssues.isEmpty)
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
              sliver: ScopedExtensionSliver<core_proxy.ToolPkgContainerRuntime>(
                items: plugins,
                scopes: scopes,
                identity: (plugin) => plugin.packageName,
                itemBuilder: (context, plugin) {
                  final issueMessage = plugin.dependencyIssues.isEmpty
                      ? null
                      : _pluginDependencyIssueMessage(plugin);
                  final item = PackageListItem(
                    key: ValueKey<String>('plugin:${plugin.packageName}'),
                    primaryAction:
                        bundledUpdates.containsKey(plugin.packageName)
                        ? FilledButton.tonalIcon(
                            key: ValueKey<String>(
                              'bundled-update:${plugin.packageName}',
                            ),
                            onPressed: () => onUpdatePlugin(
                              bundledUpdates[plugin.packageName]!,
                            ),
                            icon: const Icon(Icons.system_update_alt, size: 18),
                            label: const Text('更新'),
                            style: FilledButton.styleFrom(
                              visualDensity: VisualDensity.compact,
                              padding: const EdgeInsets.symmetric(
                                horizontal: 10,
                              ),
                            ),
                          )
                        : null,
                    icon: Icons.extension_outlined,
                    title: toolPkgContainerDisplayName(plugin),
                    statusLabel:
                        customizedPackageNames.contains(plugin.packageName)
                        ? '自定义'
                        : null,
                    subtitle: localizedText(plugin.description),
                    metadata: <String>[
                      plugin.packageName,
                      'v${plugin.version}',
                      '${plugin.subpackages.length} 子包',
                      if (plugin.dependencyIssues.isNotEmpty)
                        '${plugin.dependencyIssues.length} 个前置插件问题',
                    ],
                    hasError: plugin.dependencyIssues.isNotEmpty,
                    errorMessage: issueMessage,
                    enabled: enabledPluginNames.contains(plugin.packageName),
                    onDetails: () => onPluginTap(plugin),
                    onEnabledChanged: (enabled) =>
                        onPluginEnabledChanged(plugin, enabled),
                    trailingActions: <Widget>[
                      ExtensionScopeAction(
                        scope: scopes[plugin.packageName]!,
                        requiredScope:
                            plugin.manifestExtensions['installation_scope']
                                as String?,
                        onMove: () => onMoveScope(plugin.packageName),
                      ),
                      if (toolPkgHasUi(plugin))
                        IconButton(
                          tooltip:
                              enabledPluginNames.contains(plugin.packageName)
                              ? '打开'
                              : '启用后打开',
                          onPressed:
                              enabledPluginNames.contains(plugin.packageName)
                              ? () => onOpenPluginUi(plugin)
                              : null,
                          icon: const Icon(Icons.open_in_new_outlined),
                        ),
                    ],
                  );
                  final onReordered = onPluginReordered;
                  if (onReordered == null) {
                    return item;
                  }
                  return _PluginReorderTarget(
                    plugin: plugin,
                    scopes: scopes,
                    onReordered: onReordered,
                    child: item,
                  );
                },
              ),
            ),
          if (loadIssues.isNotEmpty) ...<Widget>[
            const SliverToBoxAdapter(child: SizedBox(height: 16)),
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 0),
              sliver: const SliverToBoxAdapter(
                child: _PluginSectionHeader(
                  title: '加载失败',
                  subtitle: '这些插件未能完成解析或注册，点击卡片查看完整错误。',
                ),
              ),
            ),
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 0),
              sliver: PackageSliverList(
                itemCount: loadIssues.length,
                itemBuilder: (context, index) {
                  final issue = loadIssues[index];
                  return PackageListItem(
                    key: ValueKey<String>(
                      'plugin-load-issue:${issue.sourcePath}:${issue.code}:$index',
                    ),
                    icon: Icons.error_outline,
                    title: issue.displayName,
                    subtitle: issue.message,
                    metadata: <String>[
                      issue.packageName ?? '',
                      issue.packageKind,
                      issue.code,
                      issue.sourcePath,
                    ],
                    enabled: false,
                    showEnabledSwitch: false,
                    hasError: true,
                    errorMessage: issue.message,
                    onEnabledChanged: (_) {},
                    onDetails: () => onLoadIssueTap(issue),
                  );
                },
              ),
            ),
          ],
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
                          padding: const EdgeInsets.symmetric(horizontal: 10),
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
    );
  }
}

/// Resolves the compact dependency error shown on a plugin card.
String _pluginDependencyIssueMessage(
  core_proxy.ToolPkgContainerRuntime plugin,
) {
  final issues = plugin.dependencyIssues;
  if (issues.length > 1) {
    return '${issues.length} 个前置插件不可用';
  }
  final issue = issues.single;
  return switch (issue.code) {
    'missing' => '缺少前置插件：${issue.id}',
    'disabled' => '前置插件未启用：${issue.id}',
    'version_incompatible' => '前置插件版本不满足：${issue.id}',
    'load_order' => '前置插件加载顺序错误：${issue.id}',
    _ => throw StateError(
      'Unsupported ToolPkg dependency issue code: ${issue.code}',
    ),
  };
}

class _PluginReorderTarget extends StatelessWidget {
  /// Creates a draggable plugin card and its reorder drop target.
  const _PluginReorderTarget({
    required this.plugin,
    required this.scopes,
    required this.onReordered,
    required this.child,
  });

  final core_proxy.ToolPkgContainerRuntime plugin;
  final Map<String, String> scopes;
  final void Function(String sourcePackageName, String targetPackageName)
  onReordered;
  final Widget child;

  /// Builds the long-press drag source for one plugin card.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return LayoutBuilder(
      builder: (context, constraints) {
        return DragTarget<String>(
          onWillAcceptWithDetails: (details) =>
              details.data != plugin.packageName &&
              scopes[details.data] == scopes[plugin.packageName],
          onAcceptWithDetails: (details) =>
              onReordered(details.data, plugin.packageName),
          builder: (context, candidateData, rejectedData) {
            final isDropTarget = candidateData.isNotEmpty;
            return LongPressDraggable<String>(
              data: plugin.packageName,
              delay: const Duration(milliseconds: 160),
              hapticFeedbackOnStart: true,
              maxSimultaneousDrags: 1,
              feedback: Material(
                color: Colors.transparent,
                elevation: 8,
                shadowColor: colorScheme.shadow,
                child: SizedBox(
                  width: constraints.maxWidth,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: colorScheme.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: child,
                  ),
                ),
              ),
              childWhenDragging: child,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(12),
                  border: isDropTarget
                      ? Border.all(color: colorScheme.primary, width: 2)
                      : null,
                ),
                child: child,
              ),
            );
          },
        );
      },
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
