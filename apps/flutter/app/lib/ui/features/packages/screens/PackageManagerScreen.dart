// ignore_for_file: file_names

import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
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
import 'UnifiedMarketScreen.dart';

class PackageManagerScreen extends StatefulWidget {
  const PackageManagerScreen({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<PackageManagerScreen> createState() => _PackageManagerScreenState();
}

class _PackageManagerScreenState extends State<PackageManagerScreen> {
  PackageTab _selectedTab = PackageTab.plugins;
  bool _loading = true;
  bool _tabPreparing = false;
  bool _searchFiltering = false;
  String? _errorMessage;
  String _searchInput = '';
  String _searchQuery = '';
  int _contentReloadNonce = 0;
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
      backgroundColor: Theme.of(context).colorScheme.surface,
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
                  _tabPreparing = _shouldPrepareTab(tab);
                  _searchInput = '';
                  _searchQuery = '';
                  _searchFiltering = false;
                  _searchDebounce?.cancel();
                });
                _completeTabPreparation(tab);
              },
            ),
            _PackageSearchBar(
              query: _searchInput,
              hintText: _searchHintText,
              onChanged: _onSearchInputChanged,
            ),
            AnimatedSwitcher(
              duration: const Duration(milliseconds: 180),
              child: _loading && !_snapshot.isEmpty
                  ? const LinearProgressIndicator(minHeight: 2)
                  : const SizedBox(height: 2),
            ),
            Expanded(child: _buildContent(context)),
          ],
        ),
      ),
    );
  }

  Widget _buildContent(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _snapshot.isEmpty) {
      return const Center(child: CircularProgressIndicator());
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
    if (_tabPreparing && _shouldPrepareTab(_selectedTab)) {
      return const Center(child: CircularProgressIndicator());
    }

    return RefreshIndicator(
      onRefresh: _loadSnapshot,
      child: switch (_selectedTab) {
        PackageTab.plugins => PluginTabContent(
          plugins: _filteredPlugins,
          enabledPluginNames: _snapshot.enabledPluginContainerNames,
          isLoading: _loading || _searchFiltering,
          isSearchActive: _searchQuery.trim().isNotEmpty,
          onOpenMarket: () => _openMarket(MarketHomeTab.artifact),
          onPluginTap: _showPluginDetails,
          onPluginEnabledChanged: _setPluginEnabled,
        ),
        PackageTab.packages => PackageTabContent(
          packages: _filteredPackages,
          enabledPackageNames: _snapshot.enabledPackageNames,
          isLoading: _loading || _searchFiltering,
          isSearchActive: _searchQuery.trim().isNotEmpty,
          onOpenMarket: () => _openMarket(MarketHomeTab.artifact),
          onQuickPluginCreatorClick: () {
            _showSnackBar('Quick Plugin Creator');
          },
          onPackageTap: _showPackageDetails,
          onPackageEnabledChanged: _setPackageEnabled,
        ),
        PackageTab.skills => SkillConfigScreen(
          key: ValueKey<String>('skills-$_contentReloadNonce'),
          clients: widget.clients,
          searchQuery: _searchQuery,
          onOpenMarket: () => _openMarket(MarketHomeTab.skill),
        ),
        PackageTab.mcp => MCPConfigScreen(
          key: ValueKey<String>('mcp-$_contentReloadNonce'),
          clients: widget.clients,
          searchQuery: _searchQuery,
          onOpenMarket: () => _openMarket(MarketHomeTab.mcp),
        ),
      },
    );
  }

  Future<void> _openMarket(MarketHomeTab initialTab) async {
    await Navigator.of(context).push<void>(
      MaterialPageRoute<void>(
        builder: (context) => UnifiedMarketScreen(
          initialTab: initialTab,
          showBackButton: true,
          clients: widget.clients,
        ),
      ),
    );
  }

  Widget _buildFloatingActions(BuildContext context) {
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

  bool _shouldPrepareTab(PackageTab tab) {
    return tab == PackageTab.plugins || tab == PackageTab.packages;
  }

  void _completeTabPreparation(PackageTab tab) {
    if (!_shouldPrepareTab(tab)) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _selectedTab != tab) {
        return;
      }
      setState(() {
        _tabPreparing = false;
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
        );
      },
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
      _contentReloadNonce += 1;
    });
    await _loadSnapshot();
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
      _contentReloadNonce += 1;
    });
    await _loadSnapshot();
  }

  Future<void> _runAddAction(
    Future<String> Function() action, {
    bool reloadSnapshot = true,
  }) async {
    try {
      final message = await action();
      if (!mounted) {
        return;
      }
      _showSnackBar(message);
      setState(() {
        _contentReloadNonce += 1;
      });
      if (reloadSnapshot) {
        await _loadSnapshot();
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to run package add action: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      _showSnackBar(error.toString());
    }
  }
}

class _PackageTabBar extends StatelessWidget {
  const _PackageTabBar({
    required this.selectedTab,
    required this.onTabSelected,
  });

  final PackageTab selectedTab;
  final ValueChanged<PackageTab> onTabSelected;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Theme.of(context).colorScheme.surface,
      child: DefaultTabController(
        key: ValueKey<PackageTab>(selectedTab),
        length: PackageTab.values.length,
        initialIndex: selectedTab.index,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final scrollable = constraints.maxWidth < 420;
            return TabBar(
              isScrollable: scrollable,
              tabAlignment: scrollable ? TabAlignment.start : TabAlignment.fill,
              onTap: (index) => onTabSelected(PackageTab.values[index]),
              dividerHeight: 1,
              indicatorSize: TabBarIndicatorSize.tab,
              tabs: <Widget>[
                _PackageTabItem(
                  selected: selectedTab == PackageTab.plugins,
                  icon: Icons.apps,
                  label: '插件',
                  minWidth: scrollable ? 112 : null,
                ),
                _PackageTabItem(
                  selected: selectedTab == PackageTab.packages,
                  icon: Icons.extension,
                  label: '包',
                  minWidth: scrollable ? 112 : null,
                ),
                _PackageTabItem(
                  selected: selectedTab == PackageTab.skills,
                  icon: Icons.build,
                  label: '技能',
                  minWidth: scrollable ? 112 : null,
                ),
                _PackageTabItem(
                  selected: selectedTab == PackageTab.mcp,
                  icon: Icons.cloud,
                  label: 'MCP',
                  minWidth: scrollable ? 112 : null,
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _PackageTabItem extends StatelessWidget {
  const _PackageTabItem({
    required this.selected,
    required this.icon,
    required this.label,
    this.minWidth,
  });

  final bool selected;
  final IconData icon;
  final String label;
  final double? minWidth;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final color = selected ? colorScheme.primary : colorScheme.onSurfaceVariant;
    return ConstrainedBox(
      constraints: BoxConstraints(minWidth: minWidth ?? 0),
      child: Center(
        child: SizedBox(
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
          child: SearchBar(
            constraints: const BoxConstraints(minHeight: 44, maxHeight: 44),
            leading: Icon(
              Icons.search,
              size: 20,
              color: colorScheme.onSurfaceVariant,
            ),
            hintText: hintText,
            elevation: const WidgetStatePropertyAll<double>(0),
            backgroundColor: WidgetStatePropertyAll<Color>(
              colorScheme.surfaceContainerHighest.withValues(alpha: 0.72),
            ),
            side: WidgetStatePropertyAll<BorderSide>(
              BorderSide(
                color: colorScheme.outlineVariant.withValues(alpha: 0.34),
              ),
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
    );
  }
}
