// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../core/link_access/LinkAccessHost.dart';
import '../../core/link_access/LinkAccessHostConfig.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../core/runtime/RemotePairingBridge.dart';
import '../../l10n/generated/app_localizations.dart';
import 'components/M3LoadingIndicator.dart';

class DeviceSpaceDiscoveryPanel extends StatefulWidget {
  const DeviceSpaceDiscoveryPanel({
    super.key,
    required this.clients,
    required this.onJoined,
    this.enabled = true,
    this.autoScan = true,
    this.onBusyChanged,
  });

  final GeneratedCoreProxyClients clients;
  final Future<void> Function(generated.CoreSpace deviceSpace) onJoined;
  final bool enabled;
  final bool autoScan;
  final ValueChanged<bool>? onBusyChanged;

  /// Creates the shared discovery and device-space joining state.
  @override
  State<DeviceSpaceDiscoveryPanel> createState() =>
      _DeviceSpaceDiscoveryPanelState();
}

class _DeviceSpaceDiscoveryPanelState extends State<DeviceSpaceDiscoveryPanel> {
  bool _busy = false;
  bool _discoverable = false;
  bool _scanning = false;
  String? _scanError;
  String? _connectionMessage;
  bool _connectionFailed = false;
  List<generated.RuntimeRemoteDiscoveredSpace> _discoveredDeviceSpaces =
      <generated.RuntimeRemoteDiscoveredSpace>[];

  /// Reports whether this panel can start another discovery operation.
  bool get _controlsEnabled => widget.enabled && !_busy;

  /// Loads discovery configuration and starts the initial nearby-space scan.
  @override
  void initState() {
    super.initState();
    unawaited(_loadDiscoverable());
    if (widget.autoScan) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          unawaited(_scanForDeviceSpaces());
        }
      });
    }
  }

  /// Builds the reusable scan, pairing, and discovery controls.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Text(
          l10n.settingsRuntimeDiscoverSpacesDescription,
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
        const SizedBox(height: 10),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: <Widget>[
            FilledButton.tonalIcon(
              onPressed: !_controlsEnabled || _scanning
                  ? null
                  : _scanForDeviceSpaces,
              icon: _scanning
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: M3LoadingIndicator(size: 18),
                    )
                  : const Icon(Icons.search_outlined, size: 18),
              label: Text(
                _scanning
                    ? l10n.settingsRuntimeScanning
                    : l10n.settingsRuntimeScan,
              ),
            ),
            TextButton.icon(
              onPressed: _controlsEnabled ? _pairRemoteManually : null,
              icon: const Icon(Icons.add_outlined, size: 18),
              label: Text(l10n.settingsRuntimeEnterManually),
            ),
          ],
        ),
        if (_scanError != null) ...<Widget>[
          const SizedBox(height: 8),
          _DiscoveryStatus(message: _scanError!, failed: true),
        ],
        if (_connectionMessage != null) ...<Widget>[
          const SizedBox(height: 8),
          _DiscoveryStatus(
            message: _connectionMessage!,
            failed: _connectionFailed,
          ),
        ],
        if (_discoveredDeviceSpaces.isNotEmpty) ...<Widget>[
          const SizedBox(height: 12),
          Divider(
            height: 1,
            color: colorScheme.outlineVariant.withValues(alpha: 0.3),
          ),
          const SizedBox(height: 4),
          for (final deviceSpace in _discoveredDeviceSpaces)
            ExpansionTile(
              dense: true,
              tilePadding: EdgeInsets.zero,
              childrenPadding: const EdgeInsets.only(left: 20),
              leading: const Icon(Icons.hub_outlined),
              title: Text(
                deviceSpace.spaceName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              subtitle: Text(
                l10n.settingsRuntimeDiscoveredSpaceSummary(
                  deviceSpace.memberCount,
                  deviceSpace.devices.length,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              children: <Widget>[
                for (final device in deviceSpace.devices)
                  ListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    leading: const Icon(Icons.devices_other_outlined),
                    title: Text(
                      _configuredUserName(context, device.userName),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    subtitle: Text(
                      '${device.displayName}\n${device.baseUrl}',
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                    trailing: IconButton(
                      icon: const Icon(Icons.group_add_outlined),
                      tooltip: l10n.settingsRuntimeJoinSpace,
                      onPressed: _controlsEnabled
                          ? () => _pairDiscoveredDevice(device)
                          : null,
                    ),
                  ),
              ],
            ),
        ],
        const SizedBox(height: 12),
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          dense: true,
          visualDensity: VisualDensity.compact,
          title: Text(l10n.settingsRuntimeEnableDiscovery),
          subtitle: Text(l10n.settingsRuntimeEnableDiscoveryDescription),
          value: _discoverable,
          onChanged: _controlsEnabled ? _setDiscoverable : null,
        ),
      ],
    );
  }

  /// Reads whether this device currently advertises itself on the LAN.
  Future<void> _loadDiscoverable() async {
    try {
      final config = await LinkAccessHostConfigStore.read();
      if (mounted) {
        setState(() => _discoverable = config.discoveryEnabled);
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = error.toString();
          _connectionFailed = true;
        });
      }
    }
  }

  /// Enables or disables LAN discovery through the shared link-access host.
  Future<void> _setDiscoverable(bool value) async {
    final l10n = AppLocalizations.of(context)!;
    _setBusy(true);
    try {
      final config = await LinkAccessHostConfigStore.read();
      final server = LinkAccessHost.instance;
      final next = _linkHostConfigForDiscovery(config, value);
      if (value || config.webAccessEnabled) {
        await server.start(next);
      } else {
        await server.stop(updateConfig: false);
      }
      await LinkAccessHostConfigStore.write(next);
      if (mounted) {
        setState(() => _discoverable = value);
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = value
              ? l10n.settingsRuntimeEnableDiscoveryFailed(error.toString())
              : l10n.settingsRuntimeDisableDiscoveryFailed(error.toString());
          _connectionFailed = true;
        });
      }
    } finally {
      _setBusy(false);
    }
  }

  /// Scans the LAN and groups directly connectable devices by device space.
  Future<void> _scanForDeviceSpaces() async {
    setState(() {
      _scanning = true;
      _scanError = null;
      _discoveredDeviceSpaces = <generated.RuntimeRemoteDiscoveredSpace>[];
    });
    try {
      final pairedDevices = await widget.clients.server.runtimeRemoteLinkService
          .pairedDevicesFlow()
          .first;
      final spaces = await widget.clients.server.runtimeRemoteLinkService
          .discoverSpaces(timeoutMs: 2000);
      final visibleDeviceSpaces = await _visibleDiscoveredDeviceSpaces(
        spaces,
        pairedDevices,
      );
      if (mounted) {
        setState(() {
          _discoveredDeviceSpaces = visibleDeviceSpaces;
          _scanning = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _scanError = error.toString();
          _scanning = false;
        });
      }
    }
  }

  /// Removes this device and still-valid paired devices from scan results.
  Future<List<generated.RuntimeRemoteDiscoveredSpace>>
  _visibleDiscoveredDeviceSpaces(
    List<generated.RuntimeRemoteDiscoveredSpace> spaces,
    Map<String, generated.RuntimePairedDevice> pairedDevices,
  ) async {
    final localDeviceId = LinkAccessHost.instance.deviceId;
    final visibleDeviceSpaces = <generated.RuntimeRemoteDiscoveredSpace>[];
    for (final deviceSpace in spaces) {
      final visibleDevices = <generated.RuntimeRemoteDiscoveredDevice>[];
      for (final device in deviceSpace.devices) {
        if (device.deviceId == localDeviceId) {
          continue;
        }
        if (!pairedDevices.containsKey(device.deviceId)) {
          visibleDevices.add(device);
          continue;
        }
        final status = await widget.clients.server.runtimeRemoteLinkService
            .pairedDeviceStatus(deviceId: device.deviceId);
        final showPairedDevice = switch (status) {
          generated.RuntimePairedDeviceStatus.online => false,
          generated.RuntimePairedDeviceStatus.offline => false,
          generated.RuntimePairedDeviceStatus.invalid => true,
          generated.RuntimePairedDeviceStatus.removedFromSpace => true,
        };
        if (showPairedDevice) {
          visibleDevices.add(device);
        }
      }
      if (visibleDevices.isNotEmpty) {
        visibleDeviceSpaces.add(
          generated.RuntimeRemoteDiscoveredSpace(
            spaceId: deviceSpace.spaceId,
            spaceName: deviceSpace.spaceName,
            spaceRevision: deviceSpace.spaceRevision,
            memberCount: deviceSpace.memberCount,
            devices: visibleDevices,
          ),
        );
      }
    }
    return visibleDeviceSpaces;
  }

  /// Opens the explicit address and connection-token pairing dialog.
  Future<void> _pairRemoteManually() async {
    _setBusy(true);
    try {
      final result = await _RemotePairDialog.show(context);
      if (result != null && mounted) {
        await _offerJoiningPairedDeviceSpace(result);
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = AppLocalizations.of(
            context,
          )!.settingsRuntimeConnectionFailed(error.toString());
          _connectionFailed = true;
        });
      }
    } finally {
      _setBusy(false);
    }
  }

  /// Starts pairing with one device selected from a discovered device space.
  Future<void> _pairDiscoveredDevice(
    generated.RuntimeRemoteDiscoveredDevice device,
  ) async {
    _setBusy(true);
    try {
      final pairing = await const RemotePairingBridge().startWithTokenHash(
        baseUrl: device.baseUrl,
        tokenHash: device.tokenHash,
      );
      if (!mounted) {
        return;
      }
      final result = await _RemotePairCodeDialog.show(
        context,
        pairing: pairing,
      );
      if (result != null && mounted) {
        await _offerJoiningPairedDeviceSpace(result);
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = AppLocalizations.of(
            context,
          )!.settingsRuntimeConnectionFailed(error.toString());
          _connectionFailed = true;
        });
      }
    } finally {
      _setBusy(false);
    }
  }

  /// Confirms the merge and synchronizes the newly paired device space.
  Future<void> _offerJoiningPairedDeviceSpace(_RemotePairResult result) async {
    final deviceInfo = result.session.remoteDeviceInfo;
    final joined = await confirmAndJoinPairedDeviceSpace(
      context: context,
      clients: widget.clients,
      sessionName: result.name,
      deviceName: _deviceJoiningName(
        context,
        result.userName,
        '${deviceInfo.platform}-${deviceInfo.model}',
      ),
    );
    if (!mounted) {
      return;
    }
    if (joined == null) {
      setState(() {
        _connectionMessage = AppLocalizations.of(
          context,
        )!.settingsRuntimePairingComplete;
        _connectionFailed = false;
      });
      return;
    }
    setState(() {
      _connectionMessage = null;
      _connectionFailed = false;
    });
    await widget.onJoined(joined);
  }

  /// Publishes internal pairing activity to the embedding workflow.
  void _setBusy(bool value) {
    if (mounted && _busy != value) {
      setState(() => _busy = value);
      widget.onBusyChanged?.call(value);
    }
  }
}

/// Confirms and joins a directly paired device space through the shared Core API.
Future<generated.CoreSpace?> confirmAndJoinPairedDeviceSpace({
  required BuildContext context,
  required GeneratedCoreProxyClients clients,
  required String sessionName,
  required String deviceName,
}) async {
  final l10n = AppLocalizations.of(context)!;
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      title: Text(l10n.settingsRuntimeJoinSpaceTitle(deviceName)),
      content: Text(l10n.settingsRuntimeJoinSpaceDescription),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(false),
          child: Text(l10n.cancel),
        ),
        FilledButton.icon(
          onPressed: () => Navigator.of(dialogContext).pop(true),
          icon: const Icon(Icons.group_add_outlined),
          label: Text(l10n.settingsRuntimeJoinSpace),
        ),
      ],
    ),
  );
  if (confirmed != true) {
    return null;
  }
  return clients.server.runtimeRemoteLinkService.joinPairedDeviceSpace(
    name: sessionName,
  );
}

/// Returns a normalized host config with the requested discovery capability.
LinkAccessHostConfig _linkHostConfigForDiscovery(
  LinkAccessHostConfig config,
  bool discoveryEnabled,
) {
  final next = config.copyWith(
    discoveryEnabled: discoveryEnabled,
    updatedAt: DateTime.now().millisecondsSinceEpoch,
  );
  if (next.portMode == LinkAccessHostPortMode.automatic) {
    return next.copyWith(
      bindAddress: LinkAccessHostConfig.automaticBindAddress,
    );
  }
  return next;
}

class _DiscoveryStatus extends StatelessWidget {
  const _DiscoveryStatus({required this.message, required this.failed});

  final String message;
  final bool failed;

  /// Builds one compact discovery or pairing result message.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Text(
      message,
      style: Theme.of(context).textTheme.bodySmall?.copyWith(
        color: failed ? colorScheme.error : colorScheme.primary,
        fontWeight: FontWeight.w700,
      ),
    );
  }
}

class _RemotePairDialog extends StatefulWidget {
  const _RemotePairDialog();

  /// Displays the manual remote pairing dialog.
  static Future<_RemotePairResult?> show(BuildContext context) {
    return showDialog<_RemotePairResult>(
      context: context,
      builder: (_) => const _RemotePairDialog(),
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
  generated.LinkTransportPreference _transport =
      generated.LinkTransportPreference.http;
  RemotePairStartResult? _pairing;
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
      final pairing = await const RemotePairingBridge().startWithToken(
        baseUrl: baseUrl,
        token: token,
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
    if (pairingCode.isEmpty) {
      setState(() {
        _error = '${l10n.settingsRuntimePairCode}: ${l10n.required}';
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final session = await const RemotePairingBridge().finish(
        pairingId: pairing.pairingId,
        pairingCode: pairingCode,
        name: remotePairingSessionName(pairing),
        transport: _transport,
      );
      if (mounted) {
        Navigator.of(context).pop(
          _RemotePairResult(
            name: remotePairingSessionName(pairing),
            session: session,
            userName: pairing.coreUserName,
          ),
        );
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
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimePairToken,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            if (pairing != null) ...<Widget>[
              const SizedBox(height: 10),
              TextField(
                controller: _codeController,
                decoration: InputDecoration(
                  labelText: l10n.settingsRuntimePairCode,
                  border: const OutlineInputBorder(),
                  isDense: true,
                ),
              ),
              const SizedBox(height: 10),
              _LinkTransportSelector(
                value: _transport,
                onChanged: (value) => setState(() => _transport = value),
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
  const _RemotePairCodeDialog({required this.pairing});

  final RemotePairStartResult pairing;

  /// Displays the one-time code dialog for a discovered device.
  static Future<_RemotePairResult?> show(
    BuildContext context, {
    required RemotePairStartResult pairing,
  }) {
    return showDialog<_RemotePairResult>(
      context: context,
      builder: (_) => _RemotePairCodeDialog(pairing: pairing),
    );
  }

  /// Creates state that owns the one-time pairing code field.
  @override
  State<_RemotePairCodeDialog> createState() => _RemotePairCodeDialogState();
}

class _RemotePairCodeDialogState extends State<_RemotePairCodeDialog> {
  final TextEditingController _codeController = TextEditingController();
  generated.LinkTransportPreference _transport =
      generated.LinkTransportPreference.http;
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
    if (pairingCode.isEmpty) {
      setState(() {
        _error = '${l10n.settingsRuntimePairCode}: ${l10n.required}';
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final session = await const RemotePairingBridge().finish(
        pairingId: widget.pairing.pairingId,
        pairingCode: pairingCode,
        name: remotePairingSessionName(widget.pairing),
        transport: _transport,
      );
      if (mounted) {
        Navigator.of(context).pop(
          _RemotePairResult(
            name: remotePairingSessionName(widget.pairing),
            session: session,
            userName: widget.pairing.coreUserName,
          ),
        );
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
              autofocus: true,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimePairCode,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            _LinkTransportSelector(
              value: _transport,
              onChanged: (value) => setState(() => _transport = value),
            ),
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
  const _RemotePairResult({
    required this.name,
    required this.session,
    required this.userName,
  });

  final String name;
  final generated.PairedRemoteSessionRecord session;
  final String userName;
}

class _LinkTransportSelector extends StatelessWidget {
  const _LinkTransportSelector({required this.value, required this.onChanged});

  final generated.LinkTransportPreference value;
  final ValueChanged<generated.LinkTransportPreference> onChanged;

  /// Builds the explicit Link carrier selector shared by pairing dialogs.
  @override
  Widget build(BuildContext context) {
    return DropdownButtonFormField<generated.LinkTransportPreference>(
      initialValue: value,
      decoration: const InputDecoration(
        labelText: 'Link transport',
        border: OutlineInputBorder(),
        isDense: true,
      ),
      items: const <DropdownMenuItem<generated.LinkTransportPreference>>[
        DropdownMenuItem(
          value: generated.LinkTransportPreference.http,
          child: Text('HTTP'),
        ),
        DropdownMenuItem(
          value: generated.LinkTransportPreference.webSocket,
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

/// Returns the configured user name or the explicit unconfigured label.
String _configuredUserName(BuildContext context, String userName) {
  final normalized = userName.trim();
  return normalized.isEmpty
      ? AppLocalizations.of(context)!.settingsUserProfileUnnamed
      : normalized;
}

/// Combines the user identity and hardware name used by join confirmation.
String _deviceJoiningName(
  BuildContext context,
  String userName,
  String deviceName,
) {
  return '${_configuredUserName(context, userName)} · $deviceName';
}
