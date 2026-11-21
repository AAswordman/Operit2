// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../../../../core/application/PluginHotReload.dart';
import '../../../../../../../core/logging/ClientLogger.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../packages/screens/ToolPkgUiLauncherScreen.dart';
import '../../../../../../main/navigation/ToolPkgCatalogChangeBus.dart';
import '../../../../../../theme/OperitTheme.dart';
import '../../../../viewmodel/ChatViewModel.dart';

/// Renders enabled Compose DSL contributions for the host-owned composer slot.
class ChatComposerSlotHost extends StatefulWidget {
  /// Creates the host for one chat composer slot.
  const ChatComposerSlotHost({
    super.key,
    required this.viewModel,
    required this.chatId,
    required this.isProcessing,
    required this.pendingQueueCount,
  });

  final ChatViewModel viewModel;
  final String chatId;
  final bool isProcessing;
  final int pendingQueueCount;

  /// Creates the state that owns slot loading and refresh generations.
  @override
  State<ChatComposerSlotHost> createState() => _ChatComposerSlotHostState();
}

/// Owns the asynchronous slot query and plugin hot-reload lifecycle.
class _ChatComposerSlotHostState extends State<ChatComposerSlotHost> {
  late Future<List<core_proxy.ToolPkgChatComposerSlot>> _slotsFuture =
      _loadSlots();
  StreamSubscription<void>? _catalogSubscription;

  /// Starts listening for catalog changes and debug hot-reload notifications.
  @override
  void initState() {
    super.initState();
    PluginHotReload.revision.addListener(_reloadSlots);
    // PluginHotReload only fires in debug builds; the catalog bus also covers
    // release installs, enables, and disables while this chat stays open.
    _catalogSubscription = ToolPkgCatalogChangeBus.listen(_reloadSlots);
  }

  /// Reloads declarations only when the chat identity changes; live inputs update the session.
  @override
  void didUpdateWidget(covariant ChatComposerSlotHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.chatId != widget.chatId) {
      _slotsFuture = _loadSlots();
    }
  }

  /// Stops listening for catalog changes and hot-reload notifications.
  @override
  void dispose() {
    PluginHotReload.revision.removeListener(_reloadSlots);
    _catalogSubscription?.cancel();
    _catalogSubscription = null;
    super.dispose();
  }

  /// Reloads declarations after catalog mutations or development hot reloads.
  void _reloadSlots() {
    if (!mounted) return;
    setState(() {
      _slotsFuture = _loadSlots();
    });
  }

  /// Reads host-composed slot declarations for the active chat surface.
  Future<List<core_proxy.ToolPkgChatComposerSlot>> _loadSlots() async {
    try {
      return await widget.viewModel.clients.application
          .chatComposerSlotBridge()
          .createSlotDefinitions(slot: 'above_input');
    } catch (error, stackTrace) {
      ClientLogger.w(
        'Composer slot query failed',
        tag: 'ChatComposerSlotHost',
        error: error,
        stackTrace: stackTrace,
      );
      rethrow;
    }
  }

  /// Aligns contributions with the input surface edges and builds the current slot.
  @override
  Widget build(BuildContext context) {
    final themeSnapshot = OperitTheme.of(context).themePreferenceSnapshot;
    return Padding(
      padding: EdgeInsets.symmetric(
        horizontal: themeSnapshot.chatInputFloating ? 8 : 0,
      ),
      child: SizedBox(
        width: double.infinity,
        child: FutureBuilder<List<core_proxy.ToolPkgChatComposerSlot>>(
          future: _slotsFuture,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              final errorColor = Theme.of(context).colorScheme.error;
              return Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 4,
                ),
                child: Row(
                  children: [
                    Icon(Icons.error_outline, size: 14, color: errorColor),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        AppLocalizations.of(context)?.pageLoadFailed ??
                            'Page load failed',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(fontSize: 12, color: errorColor),
                      ),
                    ),
                  ],
                ),
              );
            }
            if (!snapshot.hasData) {
              return const SizedBox.shrink();
            }
            return Column(
              mainAxisSize: MainAxisSize.min,
              children: snapshot.data!
                  .map(
                    (contribution) => ToolPkgUiLauncherScreen(
                      key: ValueKey<String>(
                        '${widget.chatId}:${contribution.containerPackageName}:'
                        '${contribution.contributionId}',
                      ),
                      clients: widget.viewModel.clients,
                      plugin: contribution.containerRuntime,
                      initialRouteId: contribution.contributionId,
                      embeddedScreenPath: contribution.screen,
                      showLauncherChrome: false,
                      showLoadingIndicator: false,
                      initialState: <String, Object?>{
                        'chatId': widget.chatId,
                        'isProcessing': widget.isProcessing,
                        'pendingQueueCount': widget.pendingQueueCount,
                      },
                      initialModuleSpec: <String, Object?>{
                        'id': contribution.contributionId,
                        'screen': contribution.screen,
                        if (contribution.screenExport != null)
                          'screenExport': contribution.screenExport,
                        'slot': contribution.slot,
                        'keepAlive': contribution.keepAlive,
                      },
                    ),
                  )
                  .toList(growable: false),
            );
          },
        ),
      ),
    );
  }
}
