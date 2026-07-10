// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../features/chat/screens/AIChatScreen.dart';
import '../../features/packages/components/PackageTab.dart';
import '../../features/packages/screens/PackageManagerScreen.dart';
import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import '../../features/packages/screens/UnifiedMarketScreen.dart';
import '../../features/settings/models/SettingsModels.dart';
import '../../features/settings/screens/SettingsScreen.dart';

abstract class OperitScreen {
  const OperitScreen({
    required this.routeTypeName,
    this.title,
    this.participatesInCrossfadeTransition = true,
    this.keepAlive = false,
  });

  final String routeTypeName;
  final String? title;
  final bool participatesInCrossfadeTransition;
  final bool keepAlive;

  Map<String, Object?> routeArgs() {
    return const <String, Object?>{};
  }

  String? stableScreenKey() {
    return null;
  }

  bool preserveTopBarTitleWhenReplacingWith(OperitScreen nextScreen) {
    return false;
  }

  Widget build(BuildContext context);
}

class AiChatScreenRoute extends OperitScreen {
  const AiChatScreenRoute() : super(routeTypeName: 'AiChat', title: 'AI Chat');

  @override
  String? stableScreenKey() {
    return 'AiChat';
  }

  @override
  bool preserveTopBarTitleWhenReplacingWith(OperitScreen nextScreen) {
    return nextScreen is AiChatScreenRoute;
  }

  @override
  Widget build(BuildContext context) {
    return AIChatScreen();
  }
}

class PackageManagerScreenRoute extends OperitScreen {
  const PackageManagerScreenRoute({this.initialTab = PackageTab.plugins})
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

class MarketScreenRoute extends OperitScreen {
  const MarketScreenRoute({
    this.initialTab = MarketHomeTab.all,
    this.categoryId,
    this.categoryName,
  }) : super(routeTypeName: 'Market', title: '市场', keepAlive: true);

  final MarketHomeTab initialTab;
  final String? categoryId;
  final String? categoryName;

  @override
  Map<String, Object?> routeArgs() {
    return <String, Object?>{
      'initialTab': initialTab.name,
      if (categoryId != null) 'categoryId': categoryId,
      if (categoryName != null) 'categoryName': categoryName,
    };
  }

  @override
  String? stableScreenKey() {
    return 'Market:${initialTab.name}:${categoryId ?? 'root'}';
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedMarketScreen(
      initialTab: initialTab,
      categoryId: categoryId,
      categoryName: categoryName,
    );
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
