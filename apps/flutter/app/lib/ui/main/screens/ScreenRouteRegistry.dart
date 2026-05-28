// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../l10n/generated/app_localizations.dart';
import '../navigation/AppNavigationModels.dart';
import 'OperitScreens.dart';

class ScreenRouteRegistry {
  const ScreenRouteRegistry._();

  static const OperitScreen aiChat = AiChatScreenRoute();
  static const List<OperitScreen> _hostScreens = <OperitScreen>[aiChat];

  static final Map<String, OperitScreen> _screensByRouteId =
      <String, OperitScreen>{
        for (final screen in _hostScreens) routeIdOf(screen): screen,
      };

  static List<RouteSpec> hostRouteSpecs(AppLocalizations l10n) {
    return _hostScreens
        .map((screen) => _hostSpec(screen, l10n))
        .toList(growable: false);
  }

  static List<NavigationEntrySpec> mainSidebarEntries(AppLocalizations l10n) {
    return <NavigationEntrySpec>[
      NavigationEntrySpec(
        entryId: 'main.ai_chat',
        routeId: routeIdOf(aiChat),
        surface: NavigationSurface.mainSidebarAi,
        title: l10n.aiChat,
        icon: Icons.chat_bubble_outline,
        order: 10,
      ),
    ];
  }

  static RouteEntry initialEntry() {
    return toEntry(screen: aiChat);
  }

  static String routeIdOf(OperitScreen screen) {
    return _nativeRouteIdForTypeName(screen.routeTypeName);
  }

  static RouteEntry toEntry({
    required OperitScreen screen,
    RouteEntrySource source = RouteEntrySource.defaultSource,
  }) {
    return RouteEntry(routeId: routeIdOf(screen), source: source);
  }

  static OperitScreen screenFromEntry(RouteEntry entry) {
    final screen = _screensByRouteId[entry.routeId];
    if (screen == null) {
      throw StateError('Unknown native screen routeId: ${entry.routeId}');
    }
    return screen;
  }

  static RouteSpec _hostSpec(OperitScreen screen, AppLocalizations l10n) {
    return RouteSpec(
      routeId: routeIdOf(screen),
      runtime: RouteRuntime.native,
      title: screen is AiChatScreenRoute ? 'Operit' : screen.title,
      keepAlive: screen.keepAlive,
    );
  }

  static String _nativeRouteIdForTypeName(String typeName) {
    return 'native.${_camelToSnakeCase(typeName)}';
  }

  static String _camelToSnakeCase(String name) {
    return name
        .replaceAllMapped(
          RegExp('([A-Z]+)([A-Z][a-z])'),
          (match) => '${match.group(1)}_${match.group(2)}',
        )
        .replaceAllMapped(
          RegExp(r'([a-z\d])([A-Z])'),
          (match) => '${match.group(1)}_${match.group(2)}',
        )
        .toLowerCase();
  }
}
