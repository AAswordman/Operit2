// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../../../../core/application/PluginHotReload.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../packages/screens/ToolPkgUiLauncherScreen.dart';
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
  int _contentGeneration = 0;

  /// Starts listening for plugin hot-reload notifications.
  @override
  void initState() {
    super.initState();
    PluginHotReload.revision.addListener(_reloadSlots);
  }

  /// Refreshes slot content when the chat identity or processing state changes.
  @override
  void didUpdateWidget(covariant ChatComposerSlotHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.chatId != widget.chatId) {
      _slotsFuture = _loadSlots();
    }
    if (oldWidget.isProcessing != widget.isProcessing) {
      _refreshSlotContent();
    }
  }

  /// Stops listening for plugin hot-reload notifications.
  @override
  void dispose() {
    PluginHotReload.revision.removeListener(_reloadSlots);
    super.dispose();
  }

  /// Reloads declarations after the host installs updated development packages.
  void _reloadSlots() {
    setState(() {
      _slotsFuture = _loadSlots();
    });
  }

  /// Recreates mounted composer content after chat processing changes.
  void _refreshSlotContent() {
    _contentGeneration++;
  }

  /// Reads host-composed slot declarations for the active chat surface.
  Future<List<core_proxy.ToolPkgChatComposerSlot>> _loadSlots() async {
    return await widget.viewModel.clients.application
        .chatComposerSlotBridge()
        .createSlotDefinitions(slot: 'above_input');
  }

  /// Builds every resolved DSL contribution for the current chat composer.
  @override
  Widget build(BuildContext context) {
    return FutureBuilder<List<core_proxy.ToolPkgChatComposerSlot>>(
      future: _slotsFuture,
      builder: (context, snapshot) {
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
                    '${contribution.contributionId}:$_contentGeneration',
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
                    'slot': contribution.slot,
                    'keepAlive': contribution.keepAlive,
                  },
                ),
              )
              .toList(growable: false),
        );
      },
    );
  }
}
