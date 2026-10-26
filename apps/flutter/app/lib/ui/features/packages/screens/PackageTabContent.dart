// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';
import '../components/PackageGrid.dart';
import '../components/ExtensionScopeControls.dart';
import '../components/PackageListItem.dart';
import '../utils/PackageDisplayUtils.dart';

class PackageTabContent extends StatelessWidget {
  /// Creates the package tab content.
  const PackageTabContent({
    super.key,
    required this.packages,
    required this.morePackages,
    required this.loadIssues,
    required this.scopes,
    required this.bundledUpdates,
    required this.onUpdatePackage,
    this.customizedPackageNames = const <String>{},
    required this.onMoveScope,
    required this.enabledPackageNames,
    required this.isSearchActive,
    required this.onQuickPluginCreatorClick,
    required this.onPackageTap,
    required this.onLoadMorePackage,
    required this.onPackageEnabledChanged,
    required this.onLoadIssueTap,
  });

  final List<core_proxy.ToolPackage> packages;
  final List<core_proxy.BundledExternalPackageCandidate> morePackages;
  final List<core_proxy.ToolPkgLoadIssue> loadIssues;
  final Map<String, String> scopes;
  final Map<String, core_proxy.BundledExternalPackageCandidate> bundledUpdates;
  final ValueChanged<core_proxy.BundledExternalPackageCandidate>
  onUpdatePackage;
  final Set<String> customizedPackageNames;
  final ValueChanged<String> onMoveScope;

  final Set<String> enabledPackageNames;
  final bool isSearchActive;
  final VoidCallback onQuickPluginCreatorClick;
  final ValueChanged<core_proxy.ToolPackage> onPackageTap;
  final ValueChanged<core_proxy.BundledExternalPackageCandidate>
  onLoadMorePackage;
  final void Function(core_proxy.ToolPackage package, bool enabled)
  onPackageEnabledChanged;
  final ValueChanged<core_proxy.ToolPkgLoadIssue> onLoadIssueTap;

  /// Builds the package tab with a lazily rendered expandable list.
  @override
  Widget build(BuildContext context) {
    final grouped = <String, List<core_proxy.ToolPackage>>{};
    for (final package in packages) {
      grouped.putIfAbsent(package.category, () => <core_proxy.ToolPackage>[]);
      grouped[package.category]!.add(package);
    }
    final categories = grouped.keys.toList()
      ..sort(
        (left, right) =>
            packageCategoryOrder(left).compareTo(packageCategoryOrder(right)),
      );
    final orderedPackages = categories
        .expand((category) => grouped[category]!)
        .toList(growable: false);

    return CustomScrollView(
      physics: const AlwaysScrollableScrollPhysics(),
      slivers: <Widget>[
        if (!isSearchActive)
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            sliver: SliverToBoxAdapter(
              child: _QuickPluginCreatorEntry(onTap: onQuickPluginCreatorClick),
            ),
          ),
        if (!isSearchActive)
          const SliverToBoxAdapter(child: SizedBox(height: 12)),
        if (packages.isEmpty && morePackages.isEmpty && loadIssues.isEmpty)
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 120),
            sliver: SliverToBoxAdapter(
              child: EmptyState(
                icon: Icons.inventory_2_outlined,
                title: '没有包',
                message: isSearchActive ? '没有匹配的包。' : '当前没有可显示的工具包。',
                scrollable: false,
              ),
            ),
          ),
        if (packages.isEmpty && loadIssues.isEmpty)
          const SliverPadding(
            padding: EdgeInsets.fromLTRB(16, 0, 16, 0),
            sliver: SliverToBoxAdapter(
              child: _PackageSectionEmpty(message: '当前没有可显示的包。'),
            ),
          )
        else
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 0),
            sliver: ScopedExtensionSliver<core_proxy.ToolPackage>(
              items: orderedPackages,
              scopes: scopes,
              identity: (package) => package.name,
              itemBuilder: (context, package) {
                return PackageListItem(
                  key: ValueKey<String>('package:${package.name}'),
                  primaryAction: bundledUpdates.containsKey(package.name)
                      ? FilledButton.tonalIcon(
                          key: ValueKey<String>(
                            'bundled-update:${package.name}',
                          ),
                          onPressed: () =>
                              onUpdatePackage(bundledUpdates[package.name]!),
                          icon: const Icon(Icons.system_update_alt, size: 18),
                          label: const Text('更新'),
                          style: FilledButton.styleFrom(
                            visualDensity: VisualDensity.compact,
                            padding: const EdgeInsets.symmetric(horizontal: 10),
                          ),
                        )
                      : null,
                  icon: packageCategoryIcon(package.category),
                  title: toolPackageDisplayName(package),
                  statusLabel: customizedPackageNames.contains(package.name)
                      ? '自定义'
                      : null,
                  subtitle: localizedText(package.description),
                  metadata: <String>[
                    package.name,
                    package.category,
                    '${package.tools.length} 工具',
                    if (!customizedPackageNames.contains(package.name))
                      package.isBuiltIn ? '内置' : '外部',
                  ],
                  enabled: enabledPackageNames.contains(package.name),
                  onDetails: () => onPackageTap(package),
                  trailingActions: <Widget>[
                    ExtensionScopeAction(
                      scope: scopes[package.name]!,
                      onMove: () => onMoveScope(package.name),
                    ),
                  ],
                  onEnabledChanged: (enabled) =>
                      onPackageEnabledChanged(package, enabled),
                );
              },
            ),
          ),
        if (loadIssues.isNotEmpty) ...<Widget>[
          const SliverToBoxAdapter(child: SizedBox(height: 16)),
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 0),
            sliver: const SliverToBoxAdapter(
              child: _PackageSectionHeader(
                title: '加载失败',
                subtitle: '这些包未能完成解析或导入，点击卡片查看完整错误。',
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
                    'package-load-issue:${issue.sourcePath}:${issue.code}:$index',
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
        if (morePackages.isNotEmpty) ...<Widget>[
          const SliverToBoxAdapter(child: SizedBox(height: 16)),
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 0),
            sliver: const SliverToBoxAdapter(
              child: _PackageSectionHeader(
                title: '更多包',
                subtitle: 'App 自带的官方额外包，加载后进入当前包。',
              ),
            ),
          ),
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 120),
            sliver: PackageSliverList(
              itemCount: morePackages.length,
              itemBuilder: (context, index) {
                final package = morePackages[index];
                return PackageListItem(
                  key: ValueKey<String>(
                    'bundled-package:${package.packageName}',
                  ),
                  icon: Icons.inventory_2_outlined,
                  title: bundledExternalPackageDisplayName(package),
                  subtitle: localizedText(package.description),
                  metadata: <String>[
                    package.packageName,
                    package.category,
                    if (package.version.trim().isNotEmpty)
                      'v${package.version}',
                    '${package.toolCount} 工具',
                    if (package.subpackageCount > 0)
                      '${package.subpackageCount} 子包',
                    '官方额外',
                  ],
                  enabled: false,
                  onEnabledChanged: (_) {},
                  showEnabledSwitch: false,
                  trailingActions: <Widget>[
                    FilledButton.tonalIcon(
                      onPressed: () => onLoadMorePackage(package),
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
    );
  }
}

class _PackageSectionHeader extends StatelessWidget {
  /// Creates a package section heading.
  const _PackageSectionHeader({required this.title, this.subtitle});

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

class _PackageSectionEmpty extends StatelessWidget {
  /// Creates a package section empty state.
  const _PackageSectionEmpty({required this.message});

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

class _QuickPluginCreatorEntry extends StatelessWidget {
  /// Creates the quick plugin creator entry.
  const _QuickPluginCreatorEntry({required this.onTap});

  final VoidCallback onTap;

  /// Builds the quick plugin creator entry card.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final borderRadius = BorderRadius.circular(14);
    return OperitGlassSurface(
      color: colorScheme.primaryContainer.withValues(alpha: 0.54),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: borderRadius,
      border: Border.all(color: colorScheme.primary.withValues(alpha: 0.14)),
      material: true,
      child: InkWell(
        borderRadius: borderRadius,
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: <Widget>[
              Icon(Icons.auto_mode, color: colorScheme.onPrimaryContainer),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      '快速创作你的插件',
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                        color: colorScheme.onPrimaryContainer,
                      ),
                    ),
                    Text(
                      '内置和市场都找不到想要的插件？创作你自己想要的！',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onPrimaryContainer.withValues(
                          alpha: 0.74,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              Icon(Icons.chevron_right, color: colorScheme.onPrimaryContainer),
            ],
          ),
        ),
      ),
    );
  }
}
