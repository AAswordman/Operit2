// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

import '../../features/chat/screens/AIChatScreen.dart';
import '../../features/packages/components/PackageTab.dart';
import '../../features/packages/screens/PackageManagerScreen.dart';
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
  const MarketScreenRoute({this.initialTab = MarketHomeTab.artifact})
    : super(routeTypeName: 'Market', title: '市场', keepAlive: true);

  final MarketHomeTab initialTab;

  @override
  Map<String, Object?> routeArgs() {
    return <String, Object?>{'initialTab': initialTab.name};
  }

  @override
  String? stableScreenKey() {
    return 'Market:${initialTab.name}';
  }

  @override
  Widget build(BuildContext context) {
    return UnifiedMarketScreen(initialTab: initialTab);
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
