// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/EmptyState.dart';
import '../components/PackageGrid.dart';
import '../components/PackageListItem.dart';
import '../utils/PackageDisplayUtils.dart';

class PluginTabContent extends StatelessWidget {
  const PluginTabContent({
    super.key,
    required this.plugins,
    required this.enabledPluginNames,
    required this.isLoading,
    required this.isSearchActive,
    required this.onPluginTap,
    required this.onPluginEnabledChanged,
  });

  final List<core_proxy.ToolPkgContainerRuntime> plugins;
  final Set<String> enabledPluginNames;
  final bool isLoading;
  final bool isSearchActive;
  final ValueChanged<core_proxy.ToolPkgContainerRuntime> onPluginTap;
  final void Function(core_proxy.ToolPkgContainerRuntime plugin, bool enabled)
  onPluginEnabledChanged;

  @override
  Widget build(BuildContext context) {
    if (plugins.isEmpty && isLoading) {
      return const Center(child: CircularProgressIndicator());
    }
    return Stack(
      children: <Widget>[
        if (plugins.isEmpty)
          EmptyState(
            icon: Icons.extension_off_outlined,
            title: '没有插件',
            message: isSearchActive ? '没有匹配的插件。' : '当前没有可显示的 ToolPkg 插件。',
          )
        else
          PackageGrid(
            itemCount: plugins.length,
            itemBuilder: (context, index) {
              final plugin = plugins[index];
              return PackageListItem(
                icon: Icons.extension_outlined,
                title: toolPkgContainerDisplayName(plugin),
                subtitle: localizedText(plugin.description),
                metadata: <String>[
                  plugin.packageName,
                  'v${plugin.version}',
                  '${plugin.subpackages.length} 子包',
                ],
                enabled: enabledPluginNames.contains(plugin.packageName),
                onTap: () => onPluginTap(plugin),
                onEnabledChanged: (enabled) =>
                    onPluginEnabledChanged(plugin, enabled),
              );
            },
          ),
        if (plugins.isNotEmpty && isLoading)
          const Center(child: CircularProgressIndicator()),
      ],
    );
  }
}
