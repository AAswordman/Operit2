// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
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
    required this.onOpenPluginUi,
    required this.onPluginTap,
    required this.onPluginEnabledChanged,
  });

  final List<core_proxy.ToolPkgContainerRuntime> plugins;
  final Set<String> enabledPluginNames;
  final bool isLoading;
  final bool isSearchActive;
  final ValueChanged<core_proxy.ToolPkgContainerRuntime> onOpenPluginUi;
  final ValueChanged<core_proxy.ToolPkgContainerRuntime> onPluginTap;
  final void Function(core_proxy.ToolPkgContainerRuntime plugin, bool enabled)
  onPluginEnabledChanged;

  @override
  Widget build(BuildContext context) {
    if (plugins.isEmpty && isLoading) {
      return const M3LoadingPane();
    }
    return Stack(
      children: <Widget>[
        ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
          children: <Widget>[
            if (plugins.isEmpty)
              EmptyState(
                icon: Icons.extension_off_outlined,
                title: '没有插件',
                message: isSearchActive ? '没有匹配的插件。' : '当前没有可显示的 ToolPkg 插件。',
                scrollable: false,
              )
            else
              PackageInlineGrid(
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
          ],
        ),
        if (plugins.isNotEmpty && isLoading)
          const Positioned.fill(child: M3LoadingOverlay()),
      ],
    );
  }
}
