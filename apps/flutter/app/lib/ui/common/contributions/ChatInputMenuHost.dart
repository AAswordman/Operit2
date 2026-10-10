// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../core/application/PluginHotReload.dart';
import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import '../../main/navigation/ToolPkgCatalogChangeBus.dart';
import 'ChatUiContributionModels.dart';

/// Retains one validated registration and its existing generic Compose container.
class _EmbeddedMenu {
  /// Binds the registered surface to its exact owning route and runtime.
  const _EmbeddedMenu({
    required this.entry,
    required this.route,
    required this.plugin,
  });
  final core_proxy.ToolPkgNavigationEntry entry;
  final core_proxy.ToolPkgUiRoute route;
  final core_proxy.ToolPkgContainerRuntime plugin;
}

/// Embeds registered DSL menu content without decoding any plugin display fields.
class ChatInputMenuHost extends StatefulWidget {
  /// Supplies only the runtime, current chat and existing generic presentation handler.
  const ChatInputMenuHost({
    super.key,
    required this.clients,
    required this.chatId,
    required this.onPresent,
  });
  final GeneratedCoreProxyClients clients;
  final String? chatId;
  final Future<void> Function(ChatUiAction) onPresent;

  /// Creates a catalog lifetime owned by this exact menu surface.
  @override
  State<ChatInputMenuHost> createState() => _ChatInputMenuHostState();
}

class _ChatInputMenuHostState extends State<ChatInputMenuHost> {
  Future<List<_EmbeddedMenu>>? _catalog;
  StreamSubscription<void>? _catalogSubscription;
  String? _languageCode;
  int _generation = 0;

  /// Watches package changes without consulting plugin business APIs.
  @override
  void initState() {
    super.initState();
    _catalogSubscription = ToolPkgCatalogChangeBus.listen(_reload);
    PluginHotReload.revision.addListener(_reload);
  }

  /// Refreshes registered metadata when the host language changes.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final languageCode = Localizations.localeOf(context).languageCode;
    if (_languageCode != languageCode) {
      _languageCode = languageCode;
      _generation++;
      _catalog = _readCatalog(languageCode);
    }
  }

  /// Invalidates retained actions when the runtime or selected chat changes.
  @override
  void didUpdateWidget(covariant ChatInputMenuHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.clients != widget.clients ||
        oldWidget.chatId != widget.chatId) {
      _generation++;
      final languageCode = _languageCode;
      if (languageCode != null) _catalog = _readCatalog(languageCode);
    }
  }

  /// Resolves exact package-owned DSL routes from the shared navigation registry.
  Future<List<_EmbeddedMenu>> _readCatalog(String languageCode) async {
    final manager = widget.clients.application.packageManager();
    final entries = await manager.getToolPkgNavigationEntries(
      useEnglish: languageCode == 'en',
    );
    final routes = await manager.getToolPkgUiRoutes(
      runtime: 'compose_dsl',
      useEnglish: languageCode == 'en',
    );
    final menus = <_EmbeddedMenu>[];
    final ids = <(String, String)>{};
    for (final entry in entries) {
      if (entry.surface != 'chat_input_menu') continue;
      if (entry.containerPackageName.trim().isEmpty ||
          entry.entryId.trim().isEmpty ||
          !ids.add((entry.containerPackageName, entry.entryId))) {
        throw StateError('Invalid or duplicate chat input menu registration.');
      }
      final matches = routes.where(
        (route) =>
            route.containerPackageName == entry.containerPackageName &&
            route.routeId == entry.routeId,
      );
      if (matches.length != 1 ||
          matches.single.runtime != 'compose_dsl' ||
          matches.single.screen.trim().isEmpty ||
          entry.action != null) {
        throw StateError(
          'A chat input menu requires one registered Compose route.',
        );
      }
      final plugin = await manager.getToolPkgContainerRuntime(
        containerPackageName: entry.containerPackageName,
      );
      if (plugin == null) {
        throw StateError(
          'Chat input menu owner is disabled: ${entry.containerPackageName}',
        );
      }
      menus.add(
        _EmbeddedMenu(entry: entry, route: matches.single, plugin: plugin),
      );
    }
    menus.sort((left, right) => left.entry.order.compareTo(right.entry.order));
    return List<_EmbeddedMenu>.unmodifiable(menus);
  }

  /// Reloads registrations and discards actions from the previous catalog generation.
  void _reload() {
    final languageCode = _languageCode;
    if (!mounted || languageCode == null) return;
    setState(() {
      _generation++;
      _catalog = _readCatalog(languageCode);
    });
  }

  /// Opens only a currently registered route owned by the requesting DSL container.
  Future<void> _handleAction(
    Object? raw,
    _EmbeddedMenu menu,
    int generation,
    String? chatId,
  ) async {
    if (raw == null) return;
    try {
      if (!mounted || generation != _generation || chatId != widget.chatId) {
        return;
      }
      if (raw is! Map ||
          raw.length != 3 ||
          raw['type'] != 'toolpkg.ui.present' ||
          !raw.containsKey('input')) {
        throw const FormatException('Invalid generic UI presentation request.');
      }
      final routeId = raw['routeId'];
      if (routeId is! String || routeId.trim().isEmpty) {
        throw const FormatException('A presentation requires a route ID.');
      }
      final languageCode = _languageCode;
      if (languageCode == null) {
        throw StateError('Chat input menu language is unavailable.');
      }
      final routes = await widget.clients.application
          .packageManager()
          .getToolPkgUiRoutes(
            runtime: 'compose_dsl',
            useEnglish: languageCode == 'en',
          );
      if (!mounted || generation != _generation || chatId != widget.chatId) {
        return;
      }
      final matches = routes.where(
        (route) =>
            route.containerPackageName == menu.entry.containerPackageName &&
            route.routeId == routeId,
      );
      if (matches.length != 1) {
        throw StateError('Unregistered or ambiguous UI presentation route.');
      }
      await widget.onPresent(
        ChatUiAction(route: matches.single, input: raw['input']),
      );
    } catch (error) {
      if (mounted && generation == _generation) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('插件操作失败：$error')));
      }
    }
  }

  /// Releases catalog observers together with the containing input menu.
  @override
  void dispose() {
    unawaited(_catalogSubscription?.cancel());
    PluginHotReload.revision.removeListener(_reload);
    super.dispose();
  }

  /// Renders the plugin's DSL tree directly above the native statistics section.
  @override
  Widget build(BuildContext context) => FutureBuilder<List<_EmbeddedMenu>>(
    future: _catalog,
    builder: (context, snapshot) {
      if (snapshot.connectionState != ConnectionState.done) {
        return const SizedBox.shrink();
      }
      if (snapshot.hasError) {
        return Padding(
          padding: const EdgeInsets.all(12),
          child: Text('会话插件加载失败：${snapshot.error}'),
        );
      }
      final generation = _generation;
      final chatId = widget.chatId;
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final menu in snapshot.requireData)
            ToolPkgUiLauncherScreen(
              key: ValueKey((
                menu.entry.containerPackageName,
                menu.entry.entryId,
                generation,
              )),
              clients: widget.clients,
              plugin: menu.plugin,
              initialRouteId: menu.route.routeId,
              embeddedScreenPath: menu.route.screen,
              showLauncherChrome: false,
              showLoadingIndicator: false,
              initialModuleSpec: menu.route.moduleSpec,
              initialState: <String, Object?>{
                'chatId': chatId,
                'input': menu.entry.params,
              },
              onActionResult: (raw) =>
                  unawaited(_handleAction(raw, menu, generation, chatId)),
            ),
        ],
      );
    },
  );
}
