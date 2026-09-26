// ignore_for_file: file_names

import 'package:flutter/material.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core;
import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../viewmodel/ChatViewModel.dart';

/// Displays Core route metadata without selecting or combining chat data sources.
class ChatRouteStatusHint extends StatefulWidget {
  /// Creates a route-status decoration around the existing composer.
  const ChatRouteStatusHint({
    super.key,
    required this.chatId,
    required this.viewModel,
    required this.builder,
  });

  final String? chatId;
  final ChatViewModel viewModel;
  final Widget Function(String suffix) builder;

  /// Creates the subscription owner for the current binding.
  @override
  State<ChatRouteStatusHint> createState() => _ChatRouteStatusHintState();
}

class _ChatRouteStatusHintState extends State<ChatRouteStatusHint> {
  Stream<core.BindingRouteStatus?>? _status;

  /// Opens only the status stream; chat data remains owned by its existing Core flows.
  void _bindStatus() {
    final chatId = widget.chatId;
    _status = chatId == null ? null : widget.viewModel.watchRouteStatus(chatId);
  }

  /// Subscribes once when the composer is created.
  @override
  void initState() {
    super.initState();
    _bindStatus();
  }

  /// Rebinds status when the selected chat or runtime changes.
  @override
  void didUpdateWidget(covariant ChatRouteStatusHint oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.chatId != widget.chatId ||
        oldWidget.viewModel != widget.viewModel) {
      _bindStatus();
    }
  }

  /// Renders the effective device and preserves the remote-owner offline explanation.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    if (widget.chatId == null) return widget.builder('');
    return StreamBuilder<core.BindingRouteStatus?>(
      key: ValueKey(widget.chatId),
      stream: _status,
      builder: (context, snapshot) {
        final status = snapshot.data;
        final suffix =
            snapshot.hasError ||
                (snapshot.connectionState == ConnectionState.active &&
                    status == null)
            ? l10n.chatRouteUnknown
            : status == null
            ? l10n.chatRouteLoading
            : chatRouteHint(l10n, status);
        return widget.builder(' ($suffix)');
      },
    );
  }
}

/// Formats reported metadata only; routing decisions stay in Core.
String chatRouteHint(AppLocalizations l10n, core.BindingRouteStatus status) {
  if (status.ownerIsLocal) return l10n.chatRouteLocal;
  final rawPlatform = status.ownerPlatform;
  final platform = rawPlatform.isEmpty
      ? rawPlatform
      : rawPlatform[0].toUpperCase() + rawPlatform.substring(1);
  return status.ownerReachable
      ? l10n.chatRouteRemote(platform)
      : l10n.chatRouteRemoteOffline(platform);
}
