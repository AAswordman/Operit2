// ignore_for_file: file_names

import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../common/components/PageActivityMixin.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/AnimatedLazyIndexedStack.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../main/navigation/AppNavigationModels.dart';
import '../../../main/navigation/ToolPkgCatalogChangeBus.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../../main/screens/ScreenRouteRegistry.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../chat/PendingChatDraftHandler.dart';
import '../components/PackageTab.dart';
import '../components/ExtensionScopeControls.dart';
import '../dialogs/MCPImportDialog.dart';
import '../dialogs/PackageDetailsDialog.dart';
import '../dialogs/PackageEnvironmentVariablesDialog.dart';
import '../dialogs/PackageToolRunDialog.dart';
import '../dialogs/QuickPluginCreatorDialog.dart';
import '../dialogs/SkillImportDialog.dart';
import '../market/PluginCreationIntent.dart';
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

class _PackageManagerScreenState extends State<PackageManagerScreen>
    with PageActivityMixin<PackageManagerScreen> {
  late PackageTab _selectedTab = widget.initialTab;
  bool _loading = true;
  bool _searchFiltering = false;
  String _searchInput = '';
  String _searchQuery = '';
  int _skillReloadRevision = 0;
  int _mcpReloadRevision = 0;
  PackageManagerSnapshot _snapshot = PackageManagerSnapshot.empty();
  Timer? _searchDebounce;
  StreamSubscription<void>? _catalogSubscription;
  int _snapshotGeneration = 0;
  Map<String, String> _scopes = {};
  StreamSubscription<int>? _scopeSubscription;

  GeneratedApplicationPackageManagerCoreProxy get _packageManager =>
      widget.clients.application.packageManager();

  /// Watches catalog changes only while the retained manager is active.
  @override
  void onPageActivityChanged(bool active) {
    if (!active) {
      _searchDebounce?.cancel();
      _searchQuery = _searchInput.trim();
      _searchFiltering = false;
      _catalogSubscription?.cancel();
      _catalogSubscription = null;
      _scopeSubscription?.cancel();
      _scopeSubscription = null;
      _snapshotGeneration += 1;
      return;
    }
    _skillReloadRevision += 1;
    _mcpReloadRevision += 1;
    _catalogSubscription = ToolPkgCatalogChangeBus.listen(() {
      unawaited(_loadSnapshot(rescan: false));
    });
    _scopeSubscription = widget.clients.application
        .extensionCatalogRevisionFlow()
        .listen((_) {
          if (!mounted || !isPageActive) return;
          setState(() {
            _skillReloadRevision += 1;
            _mcpReloadRevision += 1;
          });
          unawaited(_loadSnapshot(rescan: false));
        });
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

  /// Releases catalog and search listeners when the page is removed.
  @override
  void dispose() {
    _searchDebounce?.cancel();
    _catalogSubscription?.cancel();
    _scopeSubscription?.cancel();
    super.dispose();
  }

  /// Loads the current package manager state into the screen.
  Future<void> _loadSnapshot({bool rescan = true}) async {
    if (!mounted || !isPageActive) {
      return;
    }
    final generation = ++_snapshotGeneration;
    setState(() {
      _loading = true;
    });
    try {
      if (rescan) {
        await _packageManager.loadAvailablePackages();
      }
      final results = await Future.wait<Object>(<Future<Object>>[
        _packageManager.getExecutableAvailablePackages(),
        _packageManager.getEnabledPackageNames(),
        _packageManager.getToolPkgContainerRuntimes(),
        _packageManager.getToolPkgContainerOrder(),
        _packageManager.getBundledExternalPackageCandidates(),
        _packageManager.getBundledExternalToolPkgContainerRuntimes(),
      ]);
      if (!mounted || !isPageActive || generation != _snapshotGeneration) {
        return;
      }
      final availablePackages =
          results[0] as Map<String, core_proxy.ToolPackage>;
      final enabledPackages = results[1] as List<String>;
      final pluginContainers =
          results[2] as List<core_proxy.ToolPkgContainerRuntime>;
      final pluginContainerOrder = results[3] as List<String>;
      final bundledExternalCandidates =
          results[4] as List<core_proxy.BundledExternalPackageCandidate>;
      final bundledExternalToolPkgContainers =
          results[5] as List<core_proxy.ToolPkgContainerRuntime>;
      final pluginLoadIssues = await _packageManager.getToolPkgLoadIssues();
      if (!mounted || !isPageActive || generation != _snapshotGeneration) {
        return;
      }
      final scopes = await widget.clients.application.getExtensionScopes(
        kind: 'package',
      );
      final bundledExternalPluginCandidates = _mergeBundledExternalCandidates(
        bundledExternalCandidates,
        bundledExternalToolPkgContainers,
      );
      final enabledPackageNameSet = enabledPackages.toSet();
      if (!mounted || generation != _snapshotGeneration) {
        return;
      }
      setState(() {
        _scopes = scopes;
        _snapshot = PackageManagerSnapshot(
          availablePackages: availablePackages,
          enabledPackageNames: enabledPackageNameSet,
          pluginContainers: pluginContainers,
          pluginContainerOrder: pluginContainerOrder,
          enabledPluginContainerNames: pluginContainers
              .where(
                (plugin) => enabledPackageNameSet.contains(plugin.packageName),
              )
              .map((plugin) => plugin.packageName)
              .toSet(),
          bundledExternalCandidates: bundledExternalPluginCandidates,
          pluginLoadIssues: pluginLoadIssues,
        );
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint(
        'Failed to load package manager snapshot: $error\n$stackTrace',
      );
      if (!mounted || generation != _snapshotGeneration) {
        return;
      }
      final loadIssue = core_proxy.ToolPkgLoadIssue(
        sourcePath: 'runtime://package-manager',
        packageName: null,
        displayName: '插件管理器',
        code: 'package_manager',
        message: error.toString(),
        packageKind: 'package_manager',
      );
      setState(() {
        _snapshot = _snapshot.copyWith(
          pluginLoadIssues: <core_proxy.ToolPkgLoadIssue>[
            ..._snapshot.pluginLoadIssues,
            loadIssue,
          ],
        );
        _loading = false;
      });
    }
  }

  /// Applies an enabled-state change to a plugin container.
  Future<void> _setPluginEnabled(
    core_proxy.ToolPkgContainerRuntime plugin,
    bool enabled,
  ) async {
    if (!mounted) {
      return;
    }
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
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await _loadSnapshot();
    } catch (error, stackTrace) {
      debugPrint('Failed to update plugin state: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      _setOptimisticPluginEnabled(plugin.packageName, previous);
      _showSnackBar(error.toString());
    }
  }

  /// Applies an enabled-state change to a package.
  Future<void> _setPackageEnabled(
    core_proxy.ToolPackage package,
    bool enabled,
  ) async {
    if (!mounted) {
      return;
    }
    final previous = _snapshot.enabledPackageNames.contains(package.name);
    _setOptimisticPackageEnabled(package.name, enabled);
    try {
      if (enabled) {
        await _packageManager.enablePackage(packageName: package.name);
      } else {
        await _packageManager.disablePackage(packageName: package.name);
      }
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
    } catch (error, stackTrace) {
      debugPrint('Failed to update package state: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      _setOptimisticPackageEnabled(package.name, previous);
      _showSnackBar(error.toString());
    }
  }

  /// Confirms and deletes a package from package storage.
  Future<void> _deletePackage(core_proxy.ToolPackage package) async {
    if (!mounted) {
      return;
    }
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
    if (confirmed != true || !mounted) {
      return;
    }
    try {
      final deleted = await _packageManager.deletePackage(
        packageName: package.name,
      );
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      if (!mounted) {
        return;
      }
      await _loadSnapshot();
      if (!mounted) {
        return;
      }
      if (!deleted) {
        _showSnackBar('删除失败：${package.name}');
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to delete package: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      await _loadSnapshot();
      if (!mounted) {
        return;
      }
      _showSnackBar(error.toString());
    }
  }

  /// Confirms and deletes a plugin container from package storage.
  Future<void> _deletePlugin(core_proxy.ToolPkgContainerRuntime plugin) async {
    if (!mounted) {
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) {
        return AlertDialog(
          title: const Text('删除插件'),
          content: Text('确定删除 ${toolPkgContainerDisplayName(plugin)}？此操作不可撤销。'),
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
    if (confirmed != true || !mounted) {
      return;
    }
    try {
      final deleted = await _packageManager.deletePackage(
        packageName: plugin.packageName,
      );
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      if (!mounted) {
        return;
      }
      await _loadSnapshot();
      if (!mounted) {
        return;
      }
      if (!deleted) {
        _showSnackBar('删除失败：${plugin.packageName}');
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to delete plugin: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      await _loadSnapshot();
      if (!mounted) {
        return;
      }
      _showSnackBar(error.toString());
    }
  }

  /// Updates the visible enabled state for a plugin container.
  void _setOptimisticPluginEnabled(String packageName, bool enabled) {
    if (!mounted) {
      return;
    }
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

  /// Updates the visible enabled state for a package.
  void _setOptimisticPackageEnabled(String packageName, bool enabled) {
    if (!mounted) {
      return;
    }
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

  /// Shows a package manager message while this screen is active.
  void _showSnackBar(String message) {
    if (!mounted) {
      return;
    }
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
    if (_loading && _snapshot.isEmpty) {
      return const M3LoadingPane();
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
              scopes: _scopes,
              onMoveScope: _movePackageScope,
              morePlugins: _filteredMorePlugins,
              loadIssues: _filteredPluginLoadIssues,
              enabledPluginNames: _snapshot.enabledPluginContainerNames,
              isLoading: _loading || _searchFiltering,
              isSearchActive: _searchQuery.trim().isNotEmpty,
              onOpenPluginUi: _openPluginUi,
              onPluginTap: _showPluginDetails,
              onLoadMorePlugin: _loadBundledExternalPlugin,
              onPluginEnabledChanged: _setPluginEnabled,
              onPluginReordered: _searchQuery.trim().isEmpty
                  ? _reorderPlugin
                  : null,
              onLoadIssueTap: _showPackageLoadIssue,
            ),
            PackageTab.packages => PackageTabContent(
              packages: _filteredPackages,
              scopes: _scopes,
              onMoveScope: _movePackageScope,
              enabledPackageNames: _snapshot.enabledPackageNames,
              loadIssues: _filteredPackageLoadIssues,
              isLoading: _loading || _searchFiltering,
              isSearchActive: _searchQuery.trim().isNotEmpty,
              onQuickPluginCreatorClick: _openQuickPluginCreator,
              onPackageTap: _showPackageDetails,
              morePackages: _filteredMorePackages,
              onLoadMorePackage: _loadBundledExternalPackage,
              onPackageEnabledChanged: _setPackageEnabled,
              onLoadIssueTap: _showPackageLoadIssue,
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

  Future<void> _openQuickPluginCreator() async {
    final requirement = await QuickPluginCreatorDialog.show(
      context: context,
      clients: widget.clients,
    );
    if (requirement == null || !mounted) {
      return;
    }
    PendingChatDraftHandler.setPendingDraft(
      FreshPluginCreationIntent(requirement: requirement).toPrompt(),
    );
    final chatEntry = ScreenRouteRegistry.toEntry(
      screen: const AiChatScreenRoute(),
    );
    AppRouterGateway.resetTo(
      routeId: chatEntry.routeId,
      args: chatEntry.args,
      source: chatEntry.source,
    );
  }

  /// Builds package management actions, including the shared environment editor.
  Widget _buildFloatingActions(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final marketTab = switch (_selectedTab) {
      PackageTab.plugins => MarketHomeTab.all,
      PackageTab.packages => MarketHomeTab.all,
      PackageTab.skills => MarketHomeTab.all,
      PackageTab.mcp => MarketHomeTab.all,
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
          tooltip: l10n.packageConfigureEnvironmentVariables,
          backgroundColor: Theme.of(context).colorScheme.secondaryContainer,
          foregroundColor: Theme.of(context).colorScheme.onSecondaryContainer,
          onPressed: () => PackageEnvironmentVariablesDialog.show(
            context: context,
            clients: widget.clients,
          ),
          child: const Icon(Icons.settings_outlined),
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
      ..sort(_comparePluginOrder);
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

  /// Filters structured plugin load issues using the package search query.
  List<core_proxy.ToolPkgLoadIssue> get _filteredPluginLoadIssues {
    final query = _searchQuery.trim().toLowerCase();
    final items = _snapshot.pluginLoadIssues
        .where(_isPluginLoadIssue)
        .toList(growable: false);
    return _filterLoadIssues(items, query);
  }

  /// Filters package-file issues using the package search query.
  List<core_proxy.ToolPkgLoadIssue> get _filteredPackageLoadIssues {
    final query = _searchQuery.trim().toLowerCase();
    final items = _snapshot.pluginLoadIssues
        .where((issue) => !_isPluginLoadIssue(issue))
        .toList(growable: false);
    return _filterLoadIssues(items, query);
  }

  /// Identifies ToolPkg errors that belong to the plugin tab.
  bool _isPluginLoadIssue(core_proxy.ToolPkgLoadIssue issue) {
    return switch (issue.packageKind) {
      'toolpkg' || 'market_toolpkg' || 'bundled_toolpkg' => true,
      _ => false,
    };
  }

  /// Applies the common search predicate to structured load issues.
  List<core_proxy.ToolPkgLoadIssue> _filterLoadIssues(
    List<core_proxy.ToolPkgLoadIssue> items,
    String query,
  ) {
    if (query.isEmpty) {
      return items;
    }
    return items
        .where(
          (issue) =>
              issue.displayName.toLowerCase().contains(query) ||
              issue.sourcePath.toLowerCase().contains(query) ||
              issue.message.toLowerCase().contains(query) ||
              issue.packageName?.toLowerCase().contains(query) == true,
        )
        .toList(growable: false);
  }

  /// Opens the complete structured explanation for one failed package load.
  void _showPackageLoadIssue(core_proxy.ToolPkgLoadIssue issue) {
    if (!mounted) {
      return;
    }
    showDialog<void>(
      context: context,
      builder: (context) {
        final colorScheme = Theme.of(context).colorScheme;
        return AlertDialog(
          title: Row(
            children: <Widget>[
              Icon(Icons.error_outline, color: colorScheme.error),
              const SizedBox(width: 8),
              Expanded(child: Text(issue.displayName)),
            ],
          ),
          content: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 680, maxHeight: 520),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  _IssueDetailLine(label: '错误类型', value: issue.code),
                  _IssueDetailLine(label: '插件类型', value: issue.packageKind),
                  _IssueDetailLine(
                    label: '包/插件 ID',
                    value: issue.packageName ?? '',
                  ),
                  _IssueDetailLine(label: '来源路径', value: issue.sourcePath),
                  const SizedBox(height: 12),
                  Text(
                    issue.message,
                    style: TextStyle(color: colorScheme.error),
                  ),
                ],
              ),
            ),
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('关闭'),
            ),
          ],
        );
      },
    );
  }

  /// Compares ToolPkg containers using the persisted package-manager order.
  int _comparePluginOrder(
    core_proxy.ToolPkgContainerRuntime left,
    core_proxy.ToolPkgContainerRuntime right,
  ) {
    final order = <String, int>{
      for (
        var index = 0;
        index < _snapshot.pluginContainerOrder.length;
        index += 1
      )
        _snapshot.pluginContainerOrder[index]: index,
    };
    return order[left.packageName]!.compareTo(order[right.packageName]!);
  }

  /// Moves one plugin before the selected plugin and persists the complete order.
  Future<void> _reorderPlugin(
    String sourcePackageName,
    String targetPackageName,
  ) async {
    if (sourcePackageName == targetPackageName) {
      return;
    }
    final ordered = _filteredPlugins
        .map((plugin) => plugin.packageName)
        .toList();
    final sourceIndex = ordered.indexOf(sourcePackageName);
    final targetIndex = ordered.indexOf(targetPackageName);
    if (sourceIndex < 0 || targetIndex < 0) {
      throw StateError(
        'Plugin reorder target is not present in the current plugin list',
      );
    }
    final moved = ordered.removeAt(sourceIndex);
    ordered.insert(targetIndex, moved);
    try {
      await _packageManager.setToolPkgContainerOrder(packageNames: ordered);
      if (!mounted) {
        return;
      }
      await _loadSnapshot();
    } catch (error, stackTrace) {
      debugPrint(
        'Failed to persist ToolPkg container order: $error\n$stackTrace',
      );
      if (!mounted) {
        return;
      }
      await _loadSnapshot();
      if (!mounted) {
        return;
      }
      _showSnackBar(error.toString());
    }
  }

  List<core_proxy.BundledExternalPackageCandidate> get _filteredMorePlugins {
    final query = _searchQuery.trim().toLowerCase();
    final items =
        _snapshot.bundledExternalCandidates
            .where((candidate) => candidate.isToolPkg)
            .toList()
          ..sort(
            (left, right) => bundledExternalPackageDisplayName(
              left,
            ).compareTo(bundledExternalPackageDisplayName(right)),
          );
    if (query.isEmpty) {
      return items;
    }
    return items
        .where((item) {
          return bundledExternalPackageDisplayName(
                item,
              ).toLowerCase().contains(query) ||
              item.packageName.toLowerCase().contains(query) ||
              localizedText(item.description).toLowerCase().contains(query);
        })
        .toList(growable: false);
  }

  List<core_proxy.BundledExternalPackageCandidate> get _filteredMorePackages {
    final query = _searchQuery.trim().toLowerCase();
    final items =
        _snapshot.bundledExternalCandidates
            .where((candidate) => !candidate.isToolPkg)
            .toList()
          ..sort(
            (left, right) => bundledExternalPackageDisplayName(
              left,
            ).compareTo(bundledExternalPackageDisplayName(right)),
          );
    if (query.isEmpty) {
      return items;
    }
    return items
        .where((item) {
          return bundledExternalPackageDisplayName(
                item,
              ).toLowerCase().contains(query) ||
              item.packageName.toLowerCase().contains(query) ||
              localizedText(item.description).toLowerCase().contains(query);
        })
        .toList(growable: false);
  }

  /// Merges bundled external package and ToolPkg container candidates.
  List<core_proxy.BundledExternalPackageCandidate>
  _mergeBundledExternalCandidates(
    List<core_proxy.BundledExternalPackageCandidate> packageCandidates,
    List<core_proxy.ToolPkgContainerRuntime> toolPkgContainers,
  ) {
    final merged = <core_proxy.BundledExternalPackageCandidate>[];
    final packageNames = <String>{};
    for (final candidate in packageCandidates) {
      if (packageNames.add(candidate.packageName)) {
        merged.add(candidate);
      }
    }
    for (final container in toolPkgContainers) {
      if (packageNames.add(container.packageName)) {
        merged.add(_bundledExternalCandidateFromToolPkgContainer(container));
      }
    }
    return merged;
  }

  /// Builds a bundled external candidate from a ToolPkg runtime.
  core_proxy.BundledExternalPackageCandidate
  _bundledExternalCandidateFromToolPkgContainer(
    core_proxy.ToolPkgContainerRuntime container,
  ) {
    return core_proxy.BundledExternalPackageCandidate(
      packageName: container.packageName,
      displayName: container.displayName,
      description: container.description,
      author: container.author,
      packageKind: 'toolpkg',
      sourcePath: container.sourcePath,
      sourceFileName: container.sourcePath,
      isToolPkg: true,
      version: container.version,
      category: 'ToolPkg',
      toolCount: 0,
      subpackageCount: container.subpackages.length,
    );
  }

  List<core_proxy.ToolPackage> get _filteredPackages {
    final query = _searchQuery.trim().toLowerCase();
    final items = _snapshot.availablePackages.values.toList()
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

  /// Shows details and actions for a plugin container.
  void _showPluginDetails(core_proxy.ToolPkgContainerRuntime plugin) {
    if (!mounted) {
      return;
    }
    showDialog<void>(
      context: context,
      builder: (context) {
        return PluginDetailsDialog(
          plugin: plugin,
          enabled: _snapshot.enabledPluginContainerNames.contains(
            plugin.packageName,
          ),
          packageManager: _packageManager,
          onOpenUi: (initialRouteId) {
            Navigator.of(context).pop();
            if (mounted) {
              _openPluginUi(plugin, initialRouteId: initialRouteId);
            }
          },
          onDeletePackage: _canDeletePlugin(plugin)
              ? () {
                  Navigator.of(context).pop();
                  if (mounted) {
                    unawaited(_deletePlugin(plugin));
                  }
                }
              : null,
        );
      },
    );
  }

  /// Returns whether the plugin source can be removed from package storage.
  bool _canDeletePlugin(core_proxy.ToolPkgContainerRuntime plugin) {
    return switch (plugin.sourceType) {
      core_proxy.ToolPkgSourceType.asset => false,
      core_proxy.ToolPkgSourceType.market ||
      core_proxy.ToolPkgSourceType.externalValue => true,
    };
  }

  /// Moves an installed package with its configuration and refreshes all rows.
  Future<void> _movePackageScope(String id) async {
    try {
      final moved = await changeExtensionScope(
        context: context,
        clients: widget.clients,
        kind: 'package',
        id: id,
        currentScope: _scopes[id]!,
      );
      if (moved) await _loadSnapshot();
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
      await _loadSnapshot();
    }
  }

  /// Imports one bundled external ToolPkg container from runtime assets.
  Future<void> _loadBundledExternalPlugin(
    core_proxy.BundledExternalPackageCandidate plugin,
  ) async {
    final scope = await chooseExtensionScope(context);
    if (scope == null) return;
    await _runAddAction(() async {
      final result = await _packageManager
          .importBundledExternalToolPkgContainer(
            containerPackageName: plugin.packageName,
          );
      await widget.clients.application.setExtensionScope(
        kind: 'package',
        id: plugin.packageName,
        scope: scope,
      );
      return result;
    });
  }

  /// Imports one bundled external standalone package from runtime assets.
  Future<void> _loadBundledExternalPackage(
    core_proxy.BundledExternalPackageCandidate package,
  ) async {
    final scope = await chooseExtensionScope(context);
    if (scope == null) return;
    await _runAddAction(() async {
      final result = await _packageManager.importBundledExternalPackage(
        packageName: package.packageName,
      );
      await widget.clients.application.setExtensionScope(
        kind: 'package',
        id: package.packageName,
        scope: scope,
      );
      return result;
    });
  }

  /// Opens the plugin-provided interface at an optional initial route.
  void _openPluginUi(
    core_proxy.ToolPkgContainerRuntime plugin, {
    String? initialRouteId,
  }) {
    if (!mounted) {
      return;
    }
    if (!_snapshot.enabledPluginContainerNames.contains(plugin.packageName)) {
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

  /// Shows details and actions for a package.
  void _showPackageDetails(core_proxy.ToolPackage package) {
    if (!mounted) {
      return;
    }
    showDialog<void>(
      context: context,
      builder: (context) {
        return PackageDetailsDialog(
          package: package,
          onDeletePackage: () {
            Navigator.of(context).pop();
            if (mounted) {
              unawaited(_deletePackage(package));
            }
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
    if (!mounted || file == null) {
      return;
    }
    final scope = await chooseExtensionScope(context);
    if (scope == null) return;
    await _runAddAction(() async {
      final result = await _packageManager
          .addPackageFileFromExternalStorageResult(filePath: file.path);
      await widget.clients.application.setExtensionScope(
        kind: 'package',
        id: result.packageName,
        scope: scope,
      );
      return result;
    });
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
    if (!mounted || file == null) {
      return;
    }
    final scope = await chooseExtensionScope(context);
    if (scope == null) return;
    await _runAddAction(() async {
      final result = await _packageManager
          .addPackageFileFromExternalStorageResult(filePath: file.path);
      await widget.clients.application.setExtensionScope(
        kind: 'package',
        id: result.packageName,
        scope: scope,
      );
      return result;
    });
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
    setState(() {
      _skillReloadRevision += 1;
    });
  }

  Future<void> _runAddAction(Future<Object?> Function() action) async {
    try {
      await action();
      if (!mounted) {
        return;
      }
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await _loadSnapshot();
    } catch (error, stackTrace) {
      debugPrint('Failed to run package add action: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
      await _loadSnapshot();
    }
  }
}

class _IssueDetailLine extends StatelessWidget {
  /// Creates one labeled line in a package-load error dialog.
  const _IssueDetailLine({required this.label, required this.value});

  final String label;
  final String value;

  /// Builds the line only when its value is present.
  @override
  Widget build(BuildContext context) {
    if (value.trim().isEmpty) {
      return const SizedBox.shrink();
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Text('$label: $value'),
    );
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
