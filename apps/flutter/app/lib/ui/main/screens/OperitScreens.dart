// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

import '../../features/chat/screens/AIChatScreen.dart';
import '../../features/packages/screens/PackageManagerScreen.dart';
import '../../features/packages/screens/UnifiedMarketScreen.dart';
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
  bool preserveTopBarTitleWhenReplacingWith(OperitScreen nextScreen) {
    return nextScreen is AiChatScreenRoute;
  }

  @override
  Widget build(BuildContext context) {
    return AIChatScreen();
  }
}

class PackageManagerScreenRoute extends OperitScreen {
  const PackageManagerScreenRoute()
    : super(routeTypeName: 'PackageManager', title: '包管理', keepAlive: true);

  @override
  Widget build(BuildContext context) {
    return PackageManagerScreen();
  }
}

class MarketScreenRoute extends OperitScreen {
  const MarketScreenRoute()
    : super(routeTypeName: 'Market', title: '市场', keepAlive: true);

  @override
  Widget build(BuildContext context) {
    return UnifiedMarketScreen();
  }
}

class SettingsScreenRoute extends OperitScreen {
  const SettingsScreenRoute()
    : super(routeTypeName: 'Settings', title: '设置', keepAlive: true);

  @override
  Widget build(BuildContext context) {
    return const SettingsScreen();
  }
}
