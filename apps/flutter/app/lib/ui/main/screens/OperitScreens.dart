// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../features/chat/screens/AIChatScreen.dart';
import '../../features/packages/components/PackageTab.dart';
import '../../features/packages/market/MarketStatsSupport.dart';
import '../../features/packages/screens/ExtensionDetailScreen.dart';
import '../../features/packages/screens/MarketCreatorScreen.dart';
import '../../features/packages/screens/MarketEntryDetailScreen.dart';
import '../../features/packages/screens/PackageManagerScreen.dart';
import '../../features/packages/screens/StoreBrowseScreen.dart';
import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import '../../features/settings/models/SettingsModels.dart';
import '../../features/settings/screens/SettingsScreen.dart';
import '../navigation/AppNavigationModels.dart';

abstract class OperitScreen {
  /// Creates a route with explicit ownership of its bottom system inset.
  const OperitScreen({
    required this.routeTypeName,
    this.title,
    this.participatesInCrossfadeTransition = true,
    this.keepAlive = false,
    this.handlesBottomSafeArea = false,
  });

  final String routeTypeName;
  final String? title;
  final bool participatesInCrossfadeTransition;
  final bool keepAlive;

  /// Lets a page extend its surfaces while keeping its own controls inset.
  final bool handlesBottomSafeArea;

  Map<String, Object?> routeArgs() {
    return const <String, Object?>{};
  }

  String? stableScreenKey() {
    return null;
  }

  /// Identifies the mounted page that shows [entry]. Routes with a stable key
  /// share one cached page; any other entry gets a page of its own.
  String pageKey(RouteEntry entry) {
    return stableScreenKey() ?? entry.instanceId;
  }

  Widget build(BuildContext context);
}

class AiChatScreenRoute extends OperitScreen {
  /// Lets the chat composer own the navigation-bar inset inside its surface.
  const AiChatScreenRoute()
    : super(
        routeTypeName: 'AiChat',
        title: 'AI Chat',
        handlesBottomSafeArea: true,
      );

  @override
  String? stableScreenKey() {
    return 'AiChat';
  }

  @override
  Widget build(BuildContext context) {
    return AIChatScreen();
  }
}

class PackageManagerScreenRoute extends OperitScreen {
  const PackageManagerScreenRoute({this.initialTab = PackageTab.extensions})
    : super(routeTypeName: 'PackageManager', title: '包管理', keepAlive: true);

  final PackageTab initialTab;

  @override
  Map<String, Object?> routeArgs() {
    return <String, Object?>{'initialTab': initialTab.name};
  }

  @override
  String? stableScreenKey() {
    return 'PackageManager:${initialTab.name}';
  }

  @override
  Widget build(BuildContext context) {
    return PackageManagerScreen(initialTab: initialTab);
  }
}

/// The sidebar market entry; it shows the extension center's market section
/// and shares that page with the package manager route.
class MarketScreenRoute extends OperitScreen {
  const MarketScreenRoute()
    : super(routeTypeName: 'Market', title: '市场', keepAlive: true);

  @override
  String? stableScreenKey() {
    return 'PackageManager:${PackageTab.market.name}';
  }

  @override
  Widget build(BuildContext context) {
    return const PackageManagerScreen(initialTab: PackageTab.market);
  }
}

class ExtensionDetailScreenRoute extends OperitScreen {
  const ExtensionDetailScreenRoute({required this.kind, required this.id})
    : super(routeTypeName: 'ExtensionDetail', title: '扩展详情');

  final String kind;
  final String id;

  @override
  Map<String, Object?> routeArgs() {
    return <String, Object?>{'kind': kind, 'id': id};
  }

  @override
  String? stableScreenKey() {
    return 'ExtensionDetail:$kind:$id';
  }

  @override
  Widget build(BuildContext context) {
    return ExtensionDetailScreen(kind: kind, id: id);
  }
}

class MarketEntryDetailScreenRoute extends OperitScreen {
  const MarketEntryDetailScreenRoute({required this.entryId})
    : super(routeTypeName: 'MarketEntryDetail', title: '市场详情');

  final String entryId;

  @override
  Map<String, Object?> routeArgs() {
    return <String, Object?>{'entryId': entryId};
  }

  @override
  String? stableScreenKey() {
    return 'MarketEntryDetail:$entryId';
  }

  @override
  Widget build(BuildContext context) {
    return MarketEntryDetailScreen(entryId: entryId);
  }
}

/// Full marketplace listing for one sort, type or category.
class MarketBrowseScreenRoute extends OperitScreen {
  const MarketBrowseScreenRoute({
    required this.heading,
    this.type,
    this.categoryId,
    this.sort = MarketSortOption.updated,
  }) : super(routeTypeName: 'MarketBrowse', title: '市场');

  final String heading;
  final String? type;
  final String? categoryId;
  final MarketSortOption sort;

  @override
  Map<String, Object?> routeArgs() {
    return <String, Object?>{
      'heading': heading,
      'type': ?type,
      'categoryId': ?categoryId,
      'sort': sort.name,
    };
  }

  @override
  String? stableScreenKey() {
    return 'MarketBrowse:${type ?? ''}:${categoryId ?? ''}:${sort.name}';
  }

  @override
  Widget build(BuildContext context) {
    return StoreBrowseScreen(
      heading: heading,
      type: type,
      categoryId: categoryId,
      initialSort: sort,
    );
  }
}

class MarketCreatorScreenRoute extends OperitScreen {
  const MarketCreatorScreenRoute()
    : super(routeTypeName: 'MarketCreator', title: '创作者中心');

  @override
  String? stableScreenKey() {
    return 'MarketCreator';
  }

  @override
  Widget build(BuildContext context) {
    return const MarketCreatorScreen();
  }
}

class SettingsScreenRoute extends OperitScreen {
  const SettingsScreenRoute({this.category})
    : super(routeTypeName: 'Settings', title: '设置', keepAlive: true);

  final SettingsCategory? category;

  @override
  Map<String, Object?> routeArgs() {
    final selectedCategory = category;
    return <String, Object?>{
      if (selectedCategory != null) 'category': selectedCategory.name,
    };
  }

  @override
  String? stableScreenKey() {
    return 'Settings:${category?.name ?? 'root'}';
  }

  @override
  Widget build(BuildContext context) {
    return SettingsScreen(initialCategory: category);
  }
}

class ToolPkgComposeDslScreenRoute extends OperitScreen {
  const ToolPkgComposeDslScreenRoute({
    required this.containerPackageName,
    required this.uiModuleId,
    super.title,
    super.keepAlive = false,
  }) : super(routeTypeName: 'ToolPkgComposeDsl');

  final String containerPackageName;
  final String uiModuleId;

  @override
  String? stableScreenKey() {
    return 'ToolPkgComposeDsl:$containerPackageName:$uiModuleId';
  }

  @override
  Widget build(BuildContext context) {
    return _ToolPkgComposeDslRouteHost(
      containerPackageName: containerPackageName,
      uiModuleId: uiModuleId,
    );
  }
}

class _ToolPkgComposeDslRouteHost extends StatefulWidget {
  const _ToolPkgComposeDslRouteHost({
    required this.containerPackageName,
    required this.uiModuleId,
  });

  final String containerPackageName;
  final String uiModuleId;

  @override
  State<_ToolPkgComposeDslRouteHost> createState() =>
      _ToolPkgComposeDslRouteHostState();
}

class _ToolPkgComposeDslRouteHostState
    extends State<_ToolPkgComposeDslRouteHost> {
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );

  late Future<core_proxy.ToolPkgContainerRuntime> _pluginFuture;

  @override
  void initState() {
    super.initState();
    _pluginFuture = _loadPlugin();
  }

  @override
  void didUpdateWidget(covariant _ToolPkgComposeDslRouteHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.containerPackageName != widget.containerPackageName ||
        oldWidget.uiModuleId != widget.uiModuleId) {
      _pluginFuture = _loadPlugin();
    }
  }

  Future<core_proxy.ToolPkgContainerRuntime> _loadPlugin() async {
    final plugin = await _clients.application
        .packageManager()
        .getToolPkgContainerRuntime(
          containerPackageName: widget.containerPackageName,
        );
    if (plugin == null) {
      throw StateError(
        'ToolPkg container not found: ${widget.containerPackageName}',
      );
    }
    return plugin;
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<core_proxy.ToolPkgContainerRuntime>(
      future: _pluginFuture,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return Center(child: Text(snapshot.error.toString()));
        }
        final plugin = snapshot.data;
        if (plugin == null) {
          return const Center(child: CircularProgressIndicator());
        }
        return ToolPkgUiLauncherScreen(
          clients: _clients,
          plugin: plugin,
          initialRouteId: widget.uiModuleId,
          showLauncherChrome: false,
        );
      },
    );
  }
}
