// ignore_for_file: file_names

import 'package:flutter/material.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core;
import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../viewmodel/ChatViewModel.dart';

/// Route label and the send action to use while that label is visible.
class ChatRouteComposerHint {
  /// Creates the composer decoration for one route-status snapshot.
  const ChatRouteComposerHint({
    required this.suffix,
    required this.hostMissing,
    required this.onClaimHost,
  });

  final String suffix;
  final bool hostMissing;
  final VoidCallback onClaimHost;
}

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
  final Widget Function(ChatRouteComposerHint hint) builder;

  /// Creates the subscription owner for the current binding.
  @override
  State<ChatRouteStatusHint> createState() => _ChatRouteStatusHintState();
}

class _ChatRouteStatusHintState extends State<ChatRouteStatusHint> {
  Stream<core.BindingRouteStatus?>? _status;
  bool _claimInFlight = false;

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

  /// Asks to move a chat with missing route metadata onto this device.
  Future<void> _claimHost() async {
    if (_claimInFlight) return;
    final chatId = widget.chatId;
    if (chatId == null) return;
    final l10n = AppLocalizations.of(context)!;
    _claimInFlight = true;
    try {
      final accepted = await showDialog<bool>(
        context: context,
        builder: (dialogContext) {
          return AlertDialog(
            content: Text(l10n.chatRouteClaimHostPrompt),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(false),
                child: Text(l10n.chatRouteClaimHostNo),
              ),
              FilledButton(
                onPressed: () => Navigator.of(dialogContext).pop(true),
                child: Text(l10n.chatRouteClaimHostYes),
              ),
            ],
          );
        },
      );
      if (!mounted || accepted != true) return;
      await widget.viewModel.claimMissingBindingLocal(chatId);
      if (!mounted || widget.chatId != chatId) return;
      setState(_bindStatus);
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.maybeOf(
        context,
      )?.showSnackBar(SnackBar(content: Text('$error')));
    } finally {
      _claimInFlight = false;
    }
  }

  /// Renders the effective device and preserves the remote-owner offline explanation.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    if (widget.chatId == null) {
      return widget.builder(
        const ChatRouteComposerHint(
          suffix: '',
          hostMissing: false,
          onClaimHost: _noopClaim,
        ),
      );
    }
    return StreamBuilder<core.BindingRouteStatus?>(
      key: ValueKey(widget.chatId),
      stream: _status,
      builder: (context, snapshot) {
        final status = snapshot.data;
        final hostMissing = chatRouteHostMissing(snapshot);
        final suffix = hostMissing
            ? l10n.chatRouteUnknown
            : status == null
            ? l10n.chatRouteLoading
            : chatRouteHint(l10n, status);
        return widget.builder(
          ChatRouteComposerHint(
            suffix: !hostMissing && status?.ownerIsLocal == true
                ? ''
                : ' ($suffix)',
            hostMissing: hostMissing,
            onClaimHost: _claimHost,
          ),
        );
      },
    );
  }
}

/// True when Core reported that this chat's route metadata cannot be shown.
bool chatRouteHostMissing(AsyncSnapshot<core.BindingRouteStatus?> snapshot) {
  final status = snapshot.data;
  return snapshot.hasError ||
      (snapshot.connectionState == ConnectionState.active && status == null);
}

/// Keeps the no-chat composer free of a claim callback.
void _noopClaim() {}

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
