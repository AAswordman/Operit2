// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../l10n/generated/app_localizations.dart';
import '../../features/packages/components/PackageTab.dart';
import '../../features/packages/market/MarketStatsSupport.dart';
import '../../features/settings/models/SettingsModels.dart';
import '../navigation/AppNavigationModels.dart';
import 'OperitScreens.dart';

const String _internalNativeScreenKey = '__native_screen';

typedef _ScreenFactory = OperitScreen Function(Map<String, Object?> args);

class _HostScreenDefinition {
  const _HostScreenDefinition({required this.screen, this.factory})
    : requiresArgs = false;

  /// A route that only exists for concrete arguments; [prototype] supplies
  /// its route id and title but is never shown.
  const _HostScreenDefinition.parameterized({
    required OperitScreen prototype,
    required _ScreenFactory this.factory,
  }) : screen = prototype,
       requiresArgs = true;

  final OperitScreen screen;
  final _ScreenFactory? factory;
  final bool requiresArgs;
}

class ScreenRouteRegistry {
  const ScreenRouteRegistry._();

  static const OperitScreen aiChat = AiChatScreenRoute();
  static const OperitScreen packageManager = PackageManagerScreenRoute();
  static const OperitScreen market = MarketScreenRoute();
  static const OperitScreen settings = SettingsScreenRoute();
  static const OperitScreen marketCreator = MarketCreatorScreenRoute();
  static final List<_HostScreenDefinition> _hostEntryDefinitions =
      <_HostScreenDefinition>[
        const _HostScreenDefinition(screen: aiChat),
        _HostScreenDefinition(
          screen: packageManager,
          factory: _buildPackageManagerScreen,
        ),
        _HostScreenDefinition(screen: market, factory: _buildMarketScreen),
        _HostScreenDefinition(screen: settings, factory: _buildSettingsScreen),
        const _HostScreenDefinition(screen: marketCreator),
        const _HostScreenDefinition(screen: OfficialExtensionsScreenRoute()),
        _HostScreenDefinition.parameterized(
          prototype: const ExtensionDetailScreenRoute(kind: '', id: ''),
          factory: _buildExtensionDetailScreen,
        ),
        _HostScreenDefinition.parameterized(
          prototype: const MarketEntryDetailScreenRoute(entryId: ''),
          factory: _buildMarketEntryDetailScreen,
        ),
        _HostScreenDefinition.parameterized(
          prototype: const MarketBrowseScreenRoute(heading: ''),
          factory: _buildMarketBrowseScreen,
        ),
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
      if (definition.requiresArgs) {
        throw StateError('Native route ${entry.routeId} needs arguments');
      }
      return definition.screen;
    }
    final factory = definition.factory;
    if (factory == null) {
      return definition.screen;
    }
    return factory(entry.args);
  }

  /// Whether [routeId] can only be opened with arguments, like a detail page.
  static bool requiresArguments(String routeId) {
    return _definitionsByRouteId[routeId]?.requiresArgs ?? false;
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
    // Older links selected a kind or the removed device tab.
    final compatibleTab = switch (initialTab) {
      'plugins' || 'packages' || 'skills' || 'mcp' || 'devices' => 'extensions',
      _ => initialTab,
    };
    return PackageManagerScreenRoute(
      initialTab: _enumByName(PackageTab.values, compatibleTab),
    );
  }

  /// Maps the released standalone market's arguments onto the extension center.
  static OperitScreen _buildMarketScreen(Map<String, Object?> args) {
    final categoryId = _optionalString(args, 'categoryId');
    if (categoryId != null) {
      final categoryName = _optionalString(args, 'categoryName');
      return MarketBrowseScreenRoute(
        heading: categoryName ?? categoryId,
        categoryId: categoryId,
      );
    }
    final initialTab = _optionalString(args, 'initialTab');
    return switch (initialTab) {
      null || 'all' || 'categories' => const MarketScreenRoute(),
      'mine' => const MarketCreatorScreenRoute(),
      _ => throw StateError('Unknown Market.initialTab: $initialTab'),
    };
  }

  static OperitScreen _buildExtensionDetailScreen(Map<String, Object?> args) {
    return ExtensionDetailScreenRoute(
      kind: _requiredString(args, 'kind'),
      id: _requiredString(args, 'id'),
    );
  }

  static OperitScreen _buildMarketEntryDetailScreen(
    Map<String, Object?> args,
  ) {
    return MarketEntryDetailScreenRoute(
      entryId: _requiredString(args, 'entryId'),
    );
  }

  static OperitScreen _buildMarketBrowseScreen(Map<String, Object?> args) {
    final sort = _optionalString(args, 'sort');
    return MarketBrowseScreenRoute(
      heading: _requiredString(args, 'heading'),
      type: _optionalString(args, 'type'),
      categoryId: _optionalString(args, 'categoryId'),
      sort: sort == null
          ? MarketSortOption.updated
          : _enumByName(MarketSortOption.values, sort),
    );
  }

  static String _requiredString(Map<String, Object?> args, String key) {
    final value = args[key];
    if (value is! String || value.trim().isEmpty) {
      throw StateError('Route argument $key must be a non-empty string');
    }
    return value;
  }

  static String? _optionalString(Map<String, Object?> args, String key) {
    final value = args[key];
    if (value == null) {
      return null;
    }
    if (value is! String || value.trim().isEmpty) {
      throw StateError('Route argument $key must be a non-empty string');
    }
    return value;
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
