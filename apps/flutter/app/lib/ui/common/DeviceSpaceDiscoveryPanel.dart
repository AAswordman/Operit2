// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'SpaceJoinWidgets.dart';
import 'package:flutter/services.dart';

import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../core/runtime/PeerEndpointTransport.dart';
import '../../l10n/generated/app_localizations.dart';
import '../features/settings/runtime/PeerListenerSettings.dart';
import '../theme/OperitFormStyles.dart';
import 'components/OperitDialog.dart';

enum _DeviceSpaceAction { requests, settings }

/// One primary action; technical controls and history live in secondary dialogs.
class DeviceSpaceDiscoveryPanel extends StatefulWidget {
  const DeviceSpaceDiscoveryPanel({
    super.key,
    required this.clients,
    required this.onJoined,
    this.enabled = true,
    this.autoScan = true,
    this.onBusyChanged,
    this.onRequestsChanged,
  });
  final GeneratedCoreProxyClients clients;
  final Future<void> Function(generated.CoreSpace) onJoined;
  final bool enabled, autoScan;
  final ValueChanged<bool>? onBusyChanged;
  final ValueChanged<List<generated.SpaceJoinRequest>>? onRequestsChanged;
  @override
  State<DeviceSpaceDiscoveryPanel> createState() =>
      _DeviceSpaceDiscoveryPanelState();
}

class _DeviceSpaceDiscoveryPanelState extends State<DeviceSpaceDiscoveryPanel> {
  Timer? _timer;
  int _pending = 0;
  bool _loadingRequests = false;
  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(
      const Duration(seconds: 3),
      (_) => unawaited(_loadRequests()),
    );
    unawaited(_loadRequests());
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _loadRequests() async {
    if (_loadingRequests || !widget.enabled) return;
    _loadingRequests = true;
    try {
      final service = widget.clients.server.runtimeRemoteLinkService;
      final outgoing = await service.outgoingDeviceSpaceJoins();
      final incoming = await service.incomingDeviceSpaceJoins();
      if (!mounted) return;
      final count =
          outgoing.where((r) => spaceJoinIsActive(r.status)).length +
          incoming.length;
      if (_pending != count) setState(() => _pending = count);
      widget.onRequestsChanged?.call(outgoing);
    } catch (_) {
      /* No new error section on the landing page. */
    } finally {
      _loadingRequests = false;
    }
  }

  /// Opens the focused device discovery and pairing dialog.
  Future<void> _addDevice() async {
    await showDialog<void>(
      context: context,
      builder: (_) => _AddDeviceDialog(
        clients: widget.clients,
        autoScan: widget.autoScan,
        onJoined: widget.onJoined,
      ),
    );
    await _loadRequests();
  }

  Future<void> _secondary(_DeviceSpaceAction action) async {
    final l10n = AppLocalizations.of(context)!;
    if (action == _DeviceSpaceAction.requests) {
      await showDialog<void>(
        context: context,
        builder: (dialogContext) => AlertDialog(
          title: Text(l10n.spaceJoinRequests),
          content: SpaceJoinRequestsPanel(
            clients: widget.clients,
            onJoined: widget.onJoined,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialogContext),
              child: Text(l10n.ok),
            ),
          ],
        ),
      );
      await _loadRequests();
    } else {
      await showDialog<void>(
        context: context,
        builder: (_) => PeerListenerSettingsDialog(
          clients: widget.clients,
          onBusyChanged: widget.onBusyChanged,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        FilledButton.icon(
          onPressed: widget.enabled ? _addDevice : null,
          icon: const Icon(Icons.add_rounded, size: 20),
          label: Text(l10n.deviceSpaceAddDevice),
        ),
        const SizedBox(width: 4),
        PopupMenuButton<_DeviceSpaceAction>(
          enabled: widget.enabled,
          tooltip: l10n.deviceSpaceMore,
          onSelected: _secondary,
          icon: Badge(
            isLabelVisible: _pending > 0,
            label: Text('$_pending'),
            child: const Icon(Icons.more_horiz_rounded),
          ),
          itemBuilder: (_) => [
            PopupMenuItem(
              value: _DeviceSpaceAction.requests,
              child: Row(
                children: [
                  const Icon(Icons.pending_actions_outlined, size: 20),
                  const SizedBox(width: 12),
                  Expanded(child: Text(l10n.spaceJoinRequests)),
                  if (_pending > 0)
                    Text(
                      '$_pending',
                      style: Theme.of(context).textTheme.labelMedium,
                    ),
                ],
              ),
            ),
            PopupMenuItem(
              value: _DeviceSpaceAction.settings,
              child: Row(
                children: [
                  const Icon(Icons.tune_rounded, size: 20),
                  const SizedBox(width: 12),
                  Text(l10n.deviceSpaceConnectionSettings),
                ],
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class _AddDeviceDialog extends StatefulWidget {
  /// Creates the dialog with the shared discovery and pairing service.
  const _AddDeviceDialog({
    required this.clients,
    required this.onJoined,
    this.autoScan = true,
    this.title,
    this.hint,
    this.onlyNodeId,
  });
  final GeneratedCoreProxyClients clients;
  final Future<void> Function(generated.CoreSpace) onJoined;
  final bool autoScan;

  /// Overrides the dialog title for a focused pairing purpose.
  final String? title;

  /// Overrides the body hint describing what pairing will achieve.
  final String? hint;

  /// Restricts the scanned candidates to one known device, so a per-device
  /// entry point cannot pair a different neighbour by accident.
  final String? onlyNodeId;

  /// Creates state that owns discovery results and pairing interactions.
  @override
  State<_AddDeviceDialog> createState() => _AddDeviceDialogState();
}

class _AddDeviceDialogState extends State<_AddDeviceDialog> {
  bool _busy = false;
  bool _scanning = false;
  String? _error;
  List<generated.DiscoveredPeer> _peers = [];

  /// Starts discovery after the dialog has entered the widget tree.
  @override
  void initState() {
    super.initState();
    if (widget.autoScan) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) unawaited(_scan());
      });
    }
  }

  /// Updates action availability and the explicit discovery status.
  void _setBusy(bool busy, {bool scanning = false}) {
    if (!mounted) return;
    setState(() {
      _busy = busy;
      _scanning = scanning;
      if (busy) _error = null;
    });
  }

  /// Refreshes nearby devices without displaying a progress bar.
  Future<void> _scan() async {
    if (_busy) return;
    _setBusy(true, scanning: true);
    try {
      final peers = await widget.clients.server.runtimeRemoteLinkService
          .discoverPeers(timeoutMs: 2000);
      final onlyNodeId = widget.onlyNodeId;
      if (mounted) {
        setState(() {
          _peers = onlyNodeId == null
              ? peers
              : peers
                    .where((peer) => peer.nodeId == onlyNodeId)
                    .toList(growable: false);
          _error = null;
        });
      }
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      _setBusy(false);
    }
  }

  /// Pairs the selected device or opens explicit address entry.
  Future<void> _pair([generated.DiscoveredPeer? peer]) async {
    if (_busy) return;
    final l10n = AppLocalizations.of(context)!;
    _setBusy(true);
    try {
      final _RemotePairResult? result;
      if (peer == null) {
        result = await _RemotePairDialog.show(context, clients: widget.clients);
      } else {
        // LAN 候选不携带 token；免 token 准入由 runtime/Host 实际来源判断。
        final pending = await widget.clients.server.runtimeRemoteLinkService
            .startPairing(
              address: peer.address,
              nodeId: peer.nodeId,
              transport: peerEndpointTransport(peer.address),
              token: null,
            );
        if (!mounted) {
          await widget.clients.server.runtimeRemoteLinkService.cancelPairing(
            pairingId: pending.pairingId,
          );
          return;
        }
        result = await _RemotePairCodeDialog.show(
          context,
          pairing: pending,
          clients: widget.clients,
        );
        if (result == null) {
          await widget.clients.server.runtimeRemoteLinkService.cancelPairing(
            pairingId: pending.pairingId,
          );
        }
      }
      if (result != null) {
        if (!mounted) return;
        // An admitted member needs only the direct link: the join submission
        // would be rejected as a duplicate membership, so pairing ends here.
        if (await _isSpaceMember(result.peer.nodeId)) {
          if (!mounted) return;
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                l10n.deviceSpaceDirectLinkEstablished(result.peer.displayName),
              ),
            ),
          );
          Navigator.pop(context);
          return;
        }
        if (!mounted) return;
        final space = await showSpaceJoinRequest(
          context,
          clients: widget.clients,
          deviceId: result.peer.nodeId,
          deviceName: result.peer.displayName,
        );
        if (mounted) {
          if (space != null) await widget.onJoined(space);
          if (mounted) Navigator.pop(context);
        }
      }
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      _setBusy(false);
    }
  }

  /// Reports whether one device already belongs to this device's Space.
  ///
  /// Membership is judged by the same projection the join contract uses:
  /// the coordinator rejects a request from an already admitted device, so the
  /// dialog must not offer a join that cannot succeed.
  Future<bool> _isSpaceMember(String nodeId) async {
    try {
      final space = await widget.clients.server.runtimeRemoteLinkService
          .deviceSpace();
      return space.members.contains(nodeId);
    } catch (_) {
      return false;
    }
  }

  /// Builds a compact device card with a single pairing action.
  Widget _buildDeviceCard(BuildContext context, generated.DiscoveredPeer peer) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final radius = BorderRadius.circular(16);
    return Material(
      color: colors.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: radius,
        side: BorderSide(color: colors.outlineVariant.withValues(alpha: .6)),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: _busy ? null : () => _pair(peer),
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Row(
            children: [
              Container(
                width: 44,
                height: 44,
                decoration: BoxDecoration(
                  color: colors.primaryContainer,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(
                  Icons.devices_rounded,
                  color: colors.onPrimaryContainer,
                  size: 24,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      peer.displayName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      peer.address,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Icon(
                Icons.chevron_right_rounded,
                size: 20,
                color: colors.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Shows the scan status or empty result without a second section title.
  Widget _buildEmptyState(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 64,
              height: 64,
              decoration: BoxDecoration(
                color: colors.surfaceContainerLow,
                shape: BoxShape.circle,
              ),
              child: Icon(
                Icons.devices_rounded,
                size: 30,
                color: colors.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 16),
            Text(
              _scanning ? l10n.settingsRuntimeScanning : l10n.devicePickerEmpty,
              textAlign: TextAlign.center,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: colors.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Keeps discovery errors readable inside the scrollable device content.
  Widget _buildError(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colors.errorContainer,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(Icons.error_outline_rounded, color: colors.onErrorContainer),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  _error!,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colors.onErrorContainer,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Builds the shared dialog shell with one header and a unified action bar.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    return OperitDialogScaffold(
      title: widget.title ?? l10n.deviceSpaceAddDevice,
      maxWidth: 480,
      maxHeight: (MediaQuery.sizeOf(context).height * .8).clamp(0.0, 520.0),
      expandContent: false,
      contentPadding: const EdgeInsets.fromLTRB(20, 0, 20, 16),
      actionsPadding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
      titleActions: [
        IconButton(
          tooltip: l10n.refresh,
          onPressed: _busy ? null : _scan,
          icon: const Icon(Icons.refresh_rounded),
        ),
      ],
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l10n.cancel),
        ),
        OutlinedButton.icon(
          onPressed: _busy ? null : () => _pair(),
          icon: const Icon(Icons.link_rounded, size: 18),
          label: Text(l10n.devicePickerManual),
        ),
      ],
      child: CustomScrollView(
        shrinkWrap: true,
        slivers: [
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.only(bottom: 16),
              child: Text(
                widget.hint ?? l10n.devicePickerHint,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                  height: 1.5,
                ),
              ),
            ),
          ),
          if (_error != null) SliverToBoxAdapter(child: _buildError(context)),
          if (_peers.isEmpty && _error == null)
            SliverToBoxAdapter(child: _buildEmptyState(context)),
          if (_peers.isNotEmpty)
            SliverList.separated(
              itemCount: _peers.length,
              separatorBuilder: (_, _) => const SizedBox(height: 10),
              itemBuilder: (context, index) =>
                  _buildDeviceCard(context, _peers[index]),
            ),
        ],
      ),
    );
  }
}

/// Opens the shared discovery and pairing dialog for one focused purpose.
///
/// `title` and `hint` describe what pairing achieves - adding a device versus
/// giving an already relayed Space member a direct link; `onlyNodeId` keeps the
/// scan list on that one device so a per-device entry point cannot pair a
/// different neighbour by accident. Pairing a device that already belongs to
/// this Space completes without a join request.
Future<void> showDeviceSpacePairingDialog(
  BuildContext context, {
  required GeneratedCoreProxyClients clients,
  String? title,
  String? hint,
  String? onlyNodeId,
  bool autoScan = true,
}) async {
  await showDialog<void>(
    context: context,
    builder: (_) => _AddDeviceDialog(
      clients: clients,
      autoScan: autoScan,
      title: title,
      hint: hint,
      onlyNodeId: onlyNodeId,
      onJoined: (_) async {},
    ),
  );
}

Future<generated.CoreSpace?> confirmAndJoinPairedDeviceSpace({
  required BuildContext context,
  required GeneratedCoreProxyClients clients,
  required String deviceId,
  required String deviceName,
}) async {
  final l10n = AppLocalizations.of(context)!;
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(l10n.settingsRuntimeJoinSpaceTitle(deviceName)),
      content: Text(l10n.settingsRuntimeJoinSpaceDescription),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, true),
          child: Text(l10n.settingsRuntimeJoinSpace),
        ),
      ],
    ),
  );
  if (confirmed != true) return null;
  if (!context.mounted) return null;
  return showSpaceJoinRequest(
    context,
    clients: clients,
    deviceId: deviceId,
    deviceName: deviceName,
  );
}

class _RemotePairDialog extends StatefulWidget {
  const _RemotePairDialog({required this.clients});

  final GeneratedCoreProxyClients clients;

  /// Displays the manual remote pairing dialog.
  static Future<_RemotePairResult?> show(
    BuildContext context, {
    required GeneratedCoreProxyClients clients,
  }) {
    return showDialog<_RemotePairResult>(
      context: context,
      builder: (_) => _RemotePairDialog(clients: clients),
    );
  }

  /// Creates state that owns manual pairing fields.
  @override
  State<_RemotePairDialog> createState() => _RemotePairDialogState();
}

class _RemotePairDialogState extends State<_RemotePairDialog> {
  final TextEditingController _baseUrlController = TextEditingController();
  final TextEditingController _tokenController = TextEditingController();
  final TextEditingController _codeController = TextEditingController();
  generated.PeerTransport _transport = generated.PeerTransport.http;
  generated.PendingPairing? _pairing;
  bool _busy = false;
  String? _error;

  /// Releases all dialog-owned text controllers.
  @override
  void dispose() {
    _baseUrlController.dispose();
    _tokenController.dispose();
    _codeController.dispose();
    super.dispose();
  }

  /// Starts manual pairing from an explicit address and token.
  Future<void> _start() async {
    final l10n = AppLocalizations.of(context)!;
    final baseUrl = _baseUrlController.text.trim();
    final token = _tokenController.text.trim();
    if (baseUrl.isEmpty || token.isEmpty) {
      setState(() {
        _error =
            '${l10n.settingsRuntimeBaseUrl} / ${l10n.settingsRuntimePairToken}: ${l10n.required}';
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final pairing = await widget.clients.server.runtimeRemoteLinkService
          .startPairing(
            address: baseUrl,
            nodeId: '',
            token: token,
            transport: _transport,
          );
      if (mounted) {
        setState(() => _pairing = pairing);
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error.toString());
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Completes manual pairing with the one-time code.
  Future<void> _finish() async {
    final pairing = _pairing;
    if (pairing == null) {
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    final pairingCode = _codeController.text.trim();
    if (!RegExp(r'^\d{6}$').hasMatch(pairingCode)) {
      setState(() {
        _error = l10n.settingsPeerSixDigitCode;
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final session = await widget.clients.server.runtimeRemoteLinkService
          .finishPairing(
            pairingId: pairing.pairingId,
            confirmationCode: pairingCode,
          );
      if (mounted) {
        Navigator.of(context).pop(_RemotePairResult(peer: session));
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error.toString());
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Builds the two-stage manual pairing dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final pairing = _pairing;
    return AlertDialog(
      title: Text(l10n.settingsRuntimePairRemote),
      content: SizedBox(
        width: 460,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            TextField(
              controller: _baseUrlController,
              enabled: pairing == null,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimeBaseUrl,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _tokenController,
              enabled: pairing == null,
              obscureText: true,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimePairToken,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            if (pairing == null) ...<Widget>[
              const SizedBox(height: 10),
              _LinkTransportSelector(
                value: _transport,
                onChanged: (value) => setState(() => _transport = value),
              ),
            ],
            if (pairing != null) ...<Widget>[
              const SizedBox(height: 10),
              TextField(
                controller: _codeController,
                keyboardType: TextInputType.number,
                inputFormatters: [
                  FilteringTextInputFormatter.digitsOnly,
                  LengthLimitingTextInputFormatter(6),
                ],
                decoration: InputDecoration(
                  labelText: l10n.settingsRuntimePairCode,
                  border: const OutlineInputBorder(),
                  isDense: true,
                ),
              ),
            ],
            if (_error != null) ...<Widget>[
              const SizedBox(height: 10),
              Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            ],
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _busy ? null : (pairing == null ? _start : _finish),
          child: Text(
            pairing == null
                ? l10n.settingsRuntimeStartPairing
                : l10n.settingsRuntimeFinishPairing,
          ),
        ),
      ],
    );
  }
}

class _RemotePairCodeDialog extends StatefulWidget {
  const _RemotePairCodeDialog({required this.pairing, required this.clients});

  final GeneratedCoreProxyClients clients;

  final generated.PendingPairing pairing;

  /// Displays the one-time code dialog for a discovered device.
  static Future<_RemotePairResult?> show(
    BuildContext context, {
    required generated.PendingPairing pairing,
    required GeneratedCoreProxyClients clients,
  }) {
    return showDialog<_RemotePairResult>(
      context: context,
      builder: (_) => _RemotePairCodeDialog(pairing: pairing, clients: clients),
    );
  }

  /// Creates state that owns the one-time pairing code field.
  @override
  State<_RemotePairCodeDialog> createState() => _RemotePairCodeDialogState();
}

class _RemotePairCodeDialogState extends State<_RemotePairCodeDialog> {
  final TextEditingController _codeController = TextEditingController();
  bool _busy = false;
  String? _error;

  /// Releases the one-time pairing code controller.
  @override
  void dispose() {
    _codeController.dispose();
    super.dispose();
  }

  /// Completes pairing with the discovered device.
  Future<void> _finish() async {
    final l10n = AppLocalizations.of(context)!;
    final pairingCode = _codeController.text.trim();
    if (!RegExp(r'^\d{6}$').hasMatch(pairingCode)) {
      setState(() {
        _error = l10n.settingsPeerSixDigitCode;
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final session = await widget.clients.server.runtimeRemoteLinkService
          .finishPairing(
            pairingId: widget.pairing.pairingId,
            confirmationCode: pairingCode,
          );
      if (mounted) {
        Navigator.of(context).pop(_RemotePairResult(peer: session));
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error.toString());
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Builds the discovered-device pairing confirmation dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsRuntimePairRemote),
      content: SizedBox(
        width: 420,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            TextField(
              controller: _codeController,
              keyboardType: TextInputType.number,
              inputFormatters: [
                FilteringTextInputFormatter.digitsOnly,
                LengthLimitingTextInputFormatter(6),
              ],
              autofocus: true,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimePairCode,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            if (_error != null) ...<Widget>[
              const SizedBox(height: 10),
              Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            ],
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _busy ? null : _finish,
          child: Text(l10n.settingsRuntimeFinishPairing),
        ),
      ],
    );
  }
}

class _RemotePairResult {
  const _RemotePairResult({required this.peer});
  final generated.PairedPeer peer;
}

class _LinkTransportSelector extends StatelessWidget {
  const _LinkTransportSelector({required this.value, required this.onChanged});

  final generated.PeerTransport value;
  final ValueChanged<generated.PeerTransport> onChanged;

  /// Builds the explicit Link carrier selector shared by pairing dialogs.
  @override
  Widget build(BuildContext context) {
    return OperitFormStyles.dropdownButtonFormField<generated.PeerTransport>(
      context,
      initialValue: value,
      decoration: const InputDecoration(
        labelText: 'Link transport',
        border: OutlineInputBorder(),
        isDense: true,
      ),
      items: const <DropdownMenuItem<generated.PeerTransport>>[
        DropdownMenuItem(
          value: generated.PeerTransport.http,
          child: Text('HTTP'),
        ),
        DropdownMenuItem(
          value: generated.PeerTransport.webSocket,
          child: Text('WebSocket'),
        ),
      ],
      onChanged: (selected) {
        if (selected != null) {
          onChanged(selected);
        }
      },
    );
  }
}
