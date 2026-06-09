// ignore_for_file: file_names

import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/AnimatedLazyIndexedStack.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../main/navigation/AppNavigationModels.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../../main/screens/ScreenRouteRegistry.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';
import '../components/PackageTab.dart';
import '../dialogs/MCPImportDialog.dart';
import '../dialogs/PackageDetailsDialog.dart';
import '../dialogs/PackageToolRunDialog.dart';
import '../dialogs/SkillImportDialog.dart';
import '../model/PackageManagerModels.dart';
import '../utils/PackageDisplayUtils.dart';
import 'MCPConfigScreen.dart';
import 'PackageTabContent.dart';
import 'PluginTabContent.dart';
import 'SkillConfigScreen.dart';
import 'ToolPkgUiLauncherScreen.dart';
import 'UnifiedMarketScreen.dart';

class PackageManagerScreen extends StatefulWidget {
  const PackageManagerScreen({
    super.key,
    this.initialTab = PackageTab.plugins,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final PackageTab initialTab;
  final GeneratedCoreProxyClients clients;

  @override
  State<PackageManagerScreen> createState() => _PackageManagerScreenState();
}

class _PackageManagerScreenState extends State<PackageManagerScreen> {
  late PackageTab _selectedTab = widget.initialTab;
  bool _loading = true;
  bool _searchFiltering = false;
  String? _errorMessage;
  String _searchInput = '';
  String _searchQuery = '';
  int _skillReloadRevision = 0;
  int _mcpReloadRevision = 0;
  PackageManagerSnapshot _snapshot = PackageManagerSnapshot.empty();
  Timer? _searchDebounce;

  GeneratedPermissionsPackToolPackageManagerCoreProxy get _packageManager =>
      widget.clients.permissionsPackToolPackageManager;

  @override
  void initState() {
    super.initState();
    _loadSnapshot();
  }

  @override
  void didUpdateWidget(covariant PackageManagerScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.initialTab != widget.initialTab) {
      setState(() {
        _selectedTab = widget.initialTab;
      });
    }
  }

  @override
  void dispose() {
    _searchDebounce?.cancel();
    super.dispose();
  }

  Future<void> _loadSnapshot() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      await _packageManager.loadAvailablePackages();
      final results = await Future.wait<Object>(<Future<Object>>[
        _packageManager.getAvailablePackages(),
        _packageManager.getEnabledPackageNames(),
        _packageManager.getToolPkgContainerRuntimes(),
        _packageManager.getEnabledToolPkgContainerRuntimes(),
      ]);
      final availablePackages =
          results[0] as Map<String, core_proxy.ToolPackage>;
      final enabledPackages = results[1] as List<String>;
      final pluginContainers =
          results[2] as List<core_proxy.ToolPkgContainerRuntime>;
      final enabledPluginContainers =
          results[3] as List<core_proxy.ToolPkgContainerRuntime>;
      if (!mounted) {
        return;
      }
      setState(() {
        _snapshot = PackageManagerSnapshot(
          availablePackages: availablePackages,
          enabledPackageNames: enabledPackages.toSet(),
          pluginContainers: pluginContainers,
          enabledPluginContainerNames: enabledPluginContainers
              .map((item) => item.packageName)
              .toSet(),
        );
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint(
        'Failed to load package manager snapshot: $error\n$stackTrace',
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  Future<void> _setPluginEnabled(
    core_proxy.ToolPkgContainerRuntime plugin,
    bool enabled,
  ) async {
    final previous = _snapshot.enabledPluginContainerNames.contains(
      plugin.packageName,
    );
    _setOptimisticPluginEnabled(plugin.packageName, enabled);
    try {
      if (enabled) {
        await _packageManager.enableToolPkgContainer(
          containerPackageName: plugin.packageName,
        );
      } else {
        await _packageManager.disableToolPkgContainer(
          containerPackageName: plugin.packageName,
        );
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to update plugin state: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      _setOptimisticPluginEnabled(plugin.packageName, previous);
      _showSnackBar(error.toString());
    }
  }

  Future<void> _setPackageEnabled(
    core_proxy.ToolPackage package,
    bool enabled,
  ) async {
    final previous = _snapshot.enabledPackageNames.contains(package.name);
    _setOptimisticPackageEnabled(package.name, enabled);
    try {
      if (enabled) {
        await _packageManager.enablePackage(packageName: package.name);
      } else {
        await _packageManager.disablePackage(packageName: package.name);
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to update package state: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      _setOptimisticPackageEnabled(package.name, previous);
      _showSnackBar(error.toString());
    }
  }

  Future<void> _deletePackage(core_proxy.ToolPackage package) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) {
        return AlertDialog(
          title: const Text('删除包'),
          content: Text('确定删除 ${toolPackageDisplayName(package)}？此操作不可撤销。'),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(context).pop(true),
              style: FilledButton.styleFrom(
                backgroundColor: Theme.of(context).colorScheme.error,
                foregroundColor: Theme.of(context).colorScheme.onError,
              ),
              child: const Text('删除'),
            ),
          ],
        );
      },
    );
    if (confirmed != true) {
      return;
    }
    try {
      final deleted = await _packageManager.deletePackage(
        packageName: package.name,
      );
      await _loadSnapshot();
      if (!mounted) {
        return;
      }
      _showSnackBar(deleted ? '已删除 ${package.name}' : '删除失败：${package.name}');
    } catch (error, stackTrace) {
      debugPrint('Failed to delete package: $error\n$stackTrace');
      await _loadSnapshot();
      if (!mounted) {
        return;
      }
      _showSnackBar(error.toString());
    }
  }

  void _setOptimisticPluginEnabled(String packageName, bool enabled) {
    setState(() {
      final next = Set<String>.from(_snapshot.enabledPluginContainerNames);
      if (enabled) {
        next.add(packageName);
      } else {
        next.remove(packageName);
      }
      _snapshot = _snapshot.copyWith(enabledPluginContainerNames: next);
    });
  }

  void _setOptimisticPackageEnabled(String packageName, bool enabled) {
    setState(() {
      final next = Set<String>.from(_snapshot.enabledPackageNames);
      if (enabled) {
        next.add(packageName);
      } else {
        next.remove(packageName);
      }
      _snapshot = _snapshot.copyWith(enabledPackageNames: next);
    });
  }

  void _showSnackBar(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), behavior: SnackBarBehavior.floating),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.transparent,
      floatingActionButton: _buildFloatingActions(context),
      body: SafeArea(
        top: false,
        child: Column(
          children: <Widget>[
            _PackageTabBar(
              selectedTab: _selectedTab,
              onTabSelected: (tab) {
                if (tab == _selectedTab) {
                  return;
                }
                setState(() {
                  _selectedTab = tab;
                  _searchInput = '';
                  _searchQuery = '';
                  _searchFiltering = false;
                  _searchDebounce?.cancel();
                });
              },
            ),
            _PackageSearchBar(
              query: _searchInput,
              hintText: _searchHintText,
              onChanged: _onSearchInputChanged,
            ),
            const SizedBox(height: 2),
            Expanded(child: _buildContent(context)),
          ],
        ),
      ),
    );
  }

  Widget _buildContent(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _snapshot.isEmpty) {
      return const M3LoadingPane();
    }
    if (error != null && _snapshot.isEmpty) {
      return EmptyState(
        icon: Icons.error_outline,
        title: '加载失败',
        message: error,
        action: TextButton.icon(
          onPressed: _loadSnapshot,
          icon: const Icon(Icons.refresh),
          label: const Text('刷新'),
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: _loadSnapshot,
      child: AnimatedLazyIndexedStack(
        index: _selectedTab.index,
        itemCount: PackageTab.values.length,
        itemBuilder: (context, index) {
          return switch (PackageTab.values[index]) {
            PackageTab.plugins => PluginTabContent(
              plugins: _filteredPlugins,
              enabledPluginNames: _snapshot.enabledPluginContainerNames,
              isLoading: _loading || _searchFiltering,
              isSearchActive: _searchQuery.trim().isNotEmpty,
              onOpenPluginUi: _openPluginUi,
              onPluginTap: _showPluginDetails,
              onPluginEnabledChanged: _setPluginEnabled,
            ),
            PackageTab.packages => PackageTabContent(
              packages: _filteredPackages,
              enabledPackageNames: _snapshot.enabledPackageNames,
              isLoading: _loading || _searchFiltering,
              isSearchActive: _searchQuery.trim().isNotEmpty,
              onQuickPluginCreatorClick: () {
                _showSnackBar('Quick Plugin Creator');
              },
              onPackageTap: _showPackageDetails,
              onPackageEnabledChanged: _setPackageEnabled,
            ),
            PackageTab.skills => SkillConfigScreen(
              clients: widget.clients,
              searchQuery: _searchQuery,
              reloadRevision: _skillReloadRevision,
            ),
            PackageTab.mcp => MCPConfigScreen(
              clients: widget.clients,
              searchQuery: _searchQuery,
              reloadRevision: _mcpReloadRevision,
            ),
          };
        },
      ),
    );
  }

  Future<void> _openMarket(MarketHomeTab initialTab) async {
    final entry = ScreenRouteRegistry.toEntry(
      screen: MarketScreenRoute(initialTab: initialTab),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  Widget _buildFloatingActions(BuildContext context) {
    final marketTab = switch (_selectedTab) {
      PackageTab.plugins => MarketHomeTab.artifact,
      PackageTab.packages => MarketHomeTab.artifact,
      PackageTab.skills => MarketHomeTab.skill,
      PackageTab.mcp => MarketHomeTab.mcp,
    };
    final marketTooltip = switch (_selectedTab) {
      PackageTab.plugins => '打开 Artifact 市场',
      PackageTab.packages => '打开 Artifact 市场',
      PackageTab.skills => '打开技能市场',
      PackageTab.mcp => '打开 MCP 市场',
    };
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        FloatingActionButton.small(
          heroTag: null,
          onPressed: _loadSnapshot,
          tooltip: '刷新',
          child: const Icon(Icons.refresh),
        ),
        const SizedBox(height: 12),
        FloatingActionButton(
          heroTag: null,
          onPressed: () => _openMarket(marketTab),
          tooltip: marketTooltip,
          child: const Icon(Icons.store_outlined),
        ),
        const SizedBox(height: 12),
        FloatingActionButton(
          heroTag: null,
          onPressed: _handleAddAction,
          tooltip: _addActionTooltip,
          child: const Icon(Icons.add),
        ),
      ],
    );
  }

  String get _searchHintText {
    return switch (_selectedTab) {
      PackageTab.plugins => '搜索插件',
      PackageTab.packages => '搜索包',
      PackageTab.skills => '搜索技能',
      PackageTab.mcp => '搜索 MCP',
    };
  }

  String get _addActionTooltip {
    return switch (_selectedTab) {
      PackageTab.plugins => '导入插件',
      PackageTab.packages => '导入包',
      PackageTab.skills => '添加技能',
      PackageTab.mcp => '添加 MCP',
    };
  }

  void _onSearchInputChanged(String value) {
    _searchDebounce?.cancel();
    setState(() {
      _searchInput = value;
      _searchFiltering = value.trim() != _searchQuery.trim();
    });
    _searchDebounce = Timer(const Duration(milliseconds: 320), () {
      if (!mounted) {
        return;
      }
      setState(() {
        _searchQuery = _searchInput.trim();
        _searchFiltering = false;
      });
    });
  }

  List<core_proxy.ToolPkgContainerRuntime> get _filteredPlugins {
    final query = _searchQuery.trim().toLowerCase();
    final items = _snapshot.pluginContainers.toList()
      ..sort(
        (left, right) => toolPkgContainerDisplayName(
          left,
        ).compareTo(toolPkgContainerDisplayName(right)),
      );
    if (query.isEmpty) {
      return items;
    }
    return items
        .where((item) {
          return toolPkgContainerDisplayName(
                item,
              ).toLowerCase().contains(query) ||
              item.packageName.toLowerCase().contains(query) ||
              localizedText(item.description).toLowerCase().contains(query);
        })
        .toList(growable: false);
  }

  List<core_proxy.ToolPackage> get _filteredPackages {
    final query = _searchQuery.trim().toLowerCase();
    final pluginNames = <String>{
      for (final plugin in _snapshot.pluginContainers) plugin.packageName,
      for (final plugin in _snapshot.pluginContainers)
        for (final subpackage in plugin.subpackages) subpackage.packageName,
    };
    final items =
        _snapshot.availablePackages.values
            .where((package) => !pluginNames.contains(package.name))
            .toList()
          ..sort(
            (left, right) => toolPackageDisplayName(
              left,
            ).compareTo(toolPackageDisplayName(right)),
          );
    if (query.isEmpty) {
      return items;
    }
    return items
        .where((item) {
          return toolPackageDisplayName(item).toLowerCase().contains(query) ||
              item.name.toLowerCase().contains(query) ||
              localizedText(item.description).toLowerCase().contains(query);
        })
        .toList(growable: false);
  }

  void _showPluginDetails(core_proxy.ToolPkgContainerRuntime plugin) {
    showDialog<void>(
      context: context,
      builder: (context) {
        return PluginDetailsDialog(
          plugin: plugin,
          enabled: _snapshot.enabledPluginContainerNames.contains(
            plugin.packageName,
          ),
          onEnabledChanged: (enabled) {
            Navigator.of(context).pop();
            _setPluginEnabled(plugin, enabled);
          },
          onOpenUi: (initialRouteId) {
            Navigator.of(context).pop();
            _openPluginUi(plugin, initialRouteId: initialRouteId);
          },
        );
      },
    );
  }

  void _openPluginUi(
    core_proxy.ToolPkgContainerRuntime plugin, {
    String? initialRouteId,
  }) {
    if (!_snapshot.enabledPluginContainerNames.contains(plugin.packageName)) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('启用插件后可打开专属界面')));
      return;
    }
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => ToolPkgUiLauncherScreen(
          clients: widget.clients,
          plugin: plugin,
          initialRouteId: initialRouteId,
        ),
      ),
    );
  }

  void _showPackageDetails(core_proxy.ToolPackage package) {
    showDialog<void>(
      context: context,
      builder: (context) {
        return PackageDetailsDialog(
          package: package,
          enabled: _snapshot.enabledPackageNames.contains(package.name),
          onEnabledChanged: (enabled) {
            Navigator.of(context).pop();
            _setPackageEnabled(package, enabled);
          },
          onDeletePackage: () {
            Navigator.of(context).pop();
            _deletePackage(package);
          },
          onRunTool: (tool) {
            showDialog<void>(
              context: context,
              builder: (context) {
                return PackageToolRunDialog(
                  packageName: package.name,
                  tool: tool,
                  clients: widget.clients,
                );
              },
            );
          },
        );
      },
    );
  }

  Future<void> _handleAddAction() async {
    switch (_selectedTab) {
      case PackageTab.plugins:
        await _importPlugin();
      case PackageTab.packages:
        await _importPackage();
      case PackageTab.skills:
        await _showSkillImportDialog();
      case PackageTab.mcp:
        await _showMcpImportDialog();
    }
  }

  Future<void> _importPlugin() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(label: 'ToolPkg', extensions: <String>['toolpkg']),
      ],
    );
    if (file == null) {
      return;
    }
    await _runAddAction(
      () => _packageManager.addPackageFileFromExternalStorage(
        filePath: file.path,
      ),
    );
  }

  Future<void> _importPackage() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(
          label: 'Operit package',
          extensions: <String>['toolpkg', 'hjson', 'js', 'ts'],
        ),
      ],
    );
    if (file == null) {
      return;
    }
    await _runAddAction(
      () => _packageManager.addPackageFileFromExternalStorage(
        filePath: file.path,
      ),
    );
  }

  Future<void> _showMcpImportDialog() async {
    final result = await showDialog<MCPImportResult>(
      context: context,
      builder: (context) {
        return MCPImportDialog(clients: widget.clients);
      },
    );
    if (result == null || !mounted) {
      return;
    }
    _showSnackBar(result.message);
    setState(() {
      _mcpReloadRevision += 1;
    });
  }

  Future<void> _showSkillImportDialog() async {
    final result = await showDialog<SkillImportResult>(
      context: context,
      builder: (context) {
        return SkillImportDialog(clients: widget.clients);
      },
    );
    if (result == null || !mounted) {
      return;
    }
    _showSnackBar(result.message);
    setState(() {
      _skillReloadRevision += 1;
    });
  }

  Future<void> _runAddAction(Future<String> Function() action) async {
    try {
      final message = await action();
      if (!mounted) {
        return;
      }
      _showSnackBar(message);
      await _loadSnapshot();
    } catch (error, stackTrace) {
      debugPrint('Failed to run package add action: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      _showSnackBar(error.toString());
    }
  }
}

class _PackageTabBar extends StatefulWidget {
  const _PackageTabBar({
    required this.selectedTab,
    required this.onTabSelected,
  });

  final PackageTab selectedTab;
  final ValueChanged<PackageTab> onTabSelected;

  @override
  State<_PackageTabBar> createState() => _PackageTabBarState();
}

class _PackageTabBarState extends State<_PackageTabBar>
    with SingleTickerProviderStateMixin {
  late final TabController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TabController(
      length: PackageTab.values.length,
      initialIndex: widget.selectedTab.index,
      vsync: this,
    );
  }

  @override
  void didUpdateWidget(covariant _PackageTabBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selectedTab != widget.selectedTab &&
        _controller.index != widget.selectedTab.index) {
      _controller.animateTo(widget.selectedTab.index);
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surface.withValues(alpha: 0.72),
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.035,
      clip: false,
      material: true,
      child: TabBar(
        controller: _controller,
        labelPadding: const EdgeInsets.symmetric(horizontal: 4),
        onTap: (index) => widget.onTabSelected(PackageTab.values[index]),
        dividerHeight: 1,
        indicatorSize: TabBarIndicatorSize.label,
        tabs: <Widget>[
          _PackageTabItem(
            selected: widget.selectedTab == PackageTab.plugins,
            icon: Icons.apps,
            label: '插件',
          ),
          _PackageTabItem(
            selected: widget.selectedTab == PackageTab.packages,
            icon: Icons.extension,
            label: '包',
          ),
          _PackageTabItem(
            selected: widget.selectedTab == PackageTab.skills,
            icon: Icons.build,
            label: '技能',
          ),
          _PackageTabItem(
            selected: widget.selectedTab == PackageTab.mcp,
            icon: Icons.cloud,
            label: 'MCP',
          ),
        ],
      ),
    );
  }
}

class _PackageTabItem extends StatelessWidget {
  const _PackageTabItem({
    required this.selected,
    required this.icon,
    required this.label,
  });

  final bool selected;
  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final color = selected ? colorScheme.primary : colorScheme.onSurfaceVariant;
    return SizedBox(
      width: 86,
      height: 48,
      child: Center(
        child: Row(
          mainAxisSize: MainAxisSize.min,
          mainAxisAlignment: MainAxisAlignment.center,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            Icon(icon, size: 16, color: color),
            const SizedBox(width: 6),
            Text(
              label,
              softWrap: false,
              overflow: TextOverflow.fade,
              style: Theme.of(
                context,
              ).textTheme.bodySmall?.copyWith(color: color),
            ),
          ],
        ),
      ),
    );
  }
}

class _PackageSearchBar extends StatelessWidget {
  const _PackageSearchBar({
    required this.query,
    required this.hintText,
    required this.onChanged,
  });

  final String query;
  final String hintText;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 10, 16, 8),
      child: Align(
        alignment: Alignment.center,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 760),
          child: OperitGlassSurface(
            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.56),
            layer: OperitGlassSurfaceLayer.control,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(alpha: 0.26),
            ),
            child: SearchBar(
              constraints: const BoxConstraints(minHeight: 44, maxHeight: 44),
              leading: Icon(
                Icons.search,
                size: 20,
                color: colorScheme.onSurfaceVariant,
              ),
              hintText: hintText,
              elevation: const WidgetStatePropertyAll<double>(0),
              backgroundColor: const WidgetStatePropertyAll<Color>(
                Colors.transparent,
              ),
              shape: const WidgetStatePropertyAll<OutlinedBorder>(
                RoundedRectangleBorder(
                  borderRadius: BorderRadius.all(Radius.circular(16)),
                ),
              ),
              textStyle: WidgetStatePropertyAll<TextStyle?>(
                Theme.of(context).textTheme.bodyMedium,
              ),
              hintStyle: WidgetStatePropertyAll<TextStyle?>(
                Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              controller: TextEditingController(text: query)
                ..selection = TextSelection.collapsed(offset: query.length),
              onChanged: onChanged,
              trailing: <Widget>[
                if (query.isNotEmpty)
                  IconButton(
                    tooltip: '清空',
                    onPressed: () => onChanged(''),
                    icon: const Icon(Icons.close, size: 18),
                    visualDensity: VisualDensity.compact,
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
