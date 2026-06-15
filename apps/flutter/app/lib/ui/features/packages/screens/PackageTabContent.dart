// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';
import '../components/PackageGrid.dart';
import '../components/PackageListItem.dart';
import '../utils/PackageDisplayUtils.dart';

class PackageTabContent extends StatelessWidget {
  const PackageTabContent({
    super.key,
    required this.packages,
    required this.enabledPackageNames,
    required this.isLoading,
    required this.isSearchActive,
    required this.onQuickPluginCreatorClick,
    required this.onPackageTap,
    required this.onPackageEnabledChanged,
  });

  final List<core_proxy.ToolPackage> packages;
  final Set<String> enabledPackageNames;
  final bool isLoading;
  final bool isSearchActive;
  final VoidCallback onQuickPluginCreatorClick;
  final ValueChanged<core_proxy.ToolPackage> onPackageTap;
  final void Function(core_proxy.ToolPackage package, bool enabled)
  onPackageEnabledChanged;

  @override
  Widget build(BuildContext context) {
    if (packages.isEmpty && isLoading) {
      return const M3LoadingPane();
    }
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

    return Stack(
      children: <Widget>[
        ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
          children: <Widget>[
            if (!isSearchActive) ...<Widget>[
              _QuickPluginCreatorEntry(onTap: onQuickPluginCreatorClick),
              const SizedBox(height: 12),
            ],
            if (packages.isEmpty)
              EmptyState(
                icon: Icons.inventory_2_outlined,
                title: '没有包',
                message: isSearchActive ? '没有匹配的包。' : '当前没有可显示的工具包。',
                scrollable: false,
              ),
            if (packages.isNotEmpty)
              PackageInlineGrid(
                itemCount: orderedPackages.length,
                itemBuilder: (context, index) {
                  final package = orderedPackages[index];
                  return PackageListItem(
                    icon: packageCategoryIcon(package.category),
                    title: toolPackageDisplayName(package),
                    subtitle: localizedText(package.description),
                    metadata: <String>[
                      package.name,
                      package.category,
                      '${package.tools.length} 工具',
                      package.isBuiltIn ? '内置' : '外部',
                    ],
                    enabled: enabledPackageNames.contains(package.name),
                    onTap: () => onPackageTap(package),
                    onEnabledChanged: (enabled) =>
                        onPackageEnabledChanged(package, enabled),
                  );
                },
              ),
          ],
        ),
        if (packages.isNotEmpty && isLoading)
          const Positioned.fill(child: M3LoadingOverlay()),
      ],
    );
  }
}

class _QuickPluginCreatorEntry extends StatelessWidget {
  const _QuickPluginCreatorEntry({required this.onTap});

  final VoidCallback onTap;

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
