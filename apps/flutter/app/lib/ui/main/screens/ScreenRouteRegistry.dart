// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../l10n/generated/app_localizations.dart';
import '../../features/packages/components/PackageTab.dart';
import '../../features/packages/screens/UnifiedMarketScreen.dart';
import '../../features/settings/models/SettingsModels.dart';
import '../navigation/AppNavigationModels.dart';
import 'OperitScreens.dart';

const String _internalNativeScreenKey = '__native_screen';

typedef _ScreenFactory = OperitScreen Function(Map<String, Object?> args);

class _HostScreenDefinition {
  const _HostScreenDefinition({required this.screen, this.factory});

  final OperitScreen screen;
  final _ScreenFactory? factory;
}

class ScreenRouteRegistry {
  const ScreenRouteRegistry._();

  static const OperitScreen aiChat = AiChatScreenRoute();
  static const OperitScreen packageManager = PackageManagerScreenRoute();
  static const OperitScreen market = MarketScreenRoute();
  static const OperitScreen settings = SettingsScreenRoute();
  static final List<_HostScreenDefinition> _hostEntryDefinitions =
      <_HostScreenDefinition>[
        const _HostScreenDefinition(screen: aiChat),
        _HostScreenDefinition(
          screen: packageManager,
          factory: _buildPackageManagerScreen,
        ),
        _HostScreenDefinition(screen: market, factory: _buildMarketScreen),
        _HostScreenDefinition(screen: settings, factory: _buildSettingsScreen),
      ];

  static final Map<String, _HostScreenDefinition> _definitionsByRouteId =
      <String, _HostScreenDefinition>{
        for (final definition in _hostEntryDefinitions)
          routeIdOf(definition.screen): definition,
      };

  static List<RouteSpec> hostRouteSpecs(AppLocalizations l10n) {
    return _hostEntryDefinitions
        .map((definition) => _hostSpec(definition.screen, l10n))
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
      NavigationEntrySpec(
        entryId: 'main.package_manager',
        routeId: routeIdOf(packageManager),
        surface: NavigationSurface.mainSidebarAi,
        title: l10n.packageManager,
        icon: Icons.extension_outlined,
        order: 20,
      ),
      NavigationEntrySpec(
        entryId: 'main.market',
        routeId: routeIdOf(market),
        surface: NavigationSurface.mainSidebarAi,
        title: l10n.market,
        icon: Icons.store_outlined,
        order: 30,
      ),
      NavigationEntrySpec(
        entryId: 'main.settings',
        routeId: routeIdOf(settings),
        surface: NavigationSurface.mainSidebarAi,
        title: l10n.settings,
        icon: Icons.settings_outlined,
        order: 40,
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
    return RouteEntry(
      routeId: routeIdOf(screen),
      args: <String, Object?>{
        _internalNativeScreenKey: screen,
        ...screen.routeArgs(),
      },
      source: source,
    );
  }

  static OperitScreen screenFromEntry(RouteEntry entry) {
    final directScreen = entry.args[_internalNativeScreenKey];
    if (directScreen is OperitScreen) {
      return directScreen;
    }
    final definition = _definitionsByRouteId[entry.routeId];
    if (definition == null) {
      throw StateError('Unknown native screen routeId: ${entry.routeId}');
    }
    if (entry.args.isEmpty) {
      return definition.screen;
    }
    final factory = definition.factory;
    if (factory == null) {
      return definition.screen;
    }
    return factory(entry.args);
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

  static OperitScreen _buildPackageManagerScreen(Map<String, Object?> args) {
    final initialTab = args['initialTab'];
    if (initialTab == null) {
      return const PackageManagerScreenRoute();
    }
    if (initialTab is! String) {
      throw StateError('Invalid PackageManager.initialTab: $initialTab');
    }
    return PackageManagerScreenRoute(
      initialTab: _enumByName(PackageTab.values, initialTab),
    );
  }

  static OperitScreen _buildMarketScreen(Map<String, Object?> args) {
    final initialTab = args['initialTab'];
    if (initialTab == null) {
      return const MarketScreenRoute();
    }
    if (initialTab is! String) {
      throw StateError('Invalid Market.initialTab: $initialTab');
    }
    return MarketScreenRoute(
      initialTab: _enumByName(MarketHomeTab.values, initialTab),
    );
  }

  static OperitScreen _buildSettingsScreen(Map<String, Object?> args) {
    final category = args['category'];
    if (category == null) {
      return const SettingsScreenRoute();
    }
    if (category is! String) {
      throw StateError('Invalid Settings.category: $category');
    }
    return SettingsScreenRoute(
      category: _enumByName(SettingsCategory.values, category),
    );
  }

  static T _enumByName<T extends Enum>(List<T> values, String name) {
    for (final value in values) {
      if (value.name == name) {
        return value;
      }
    }
    throw StateError('Unknown enum value: $name');
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
