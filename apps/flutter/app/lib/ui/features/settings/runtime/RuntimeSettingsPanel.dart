// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/runtime/RemotePairingBridge.dart';
import '../../../../core/runtime/RuntimeDataSyncBridge.dart';
import '../../../../core/runtime/RuntimeConnectionManager.dart';
import '../../../../core/link_host/LinkHostServer.dart';
import '../../../../core/link_host/LinkHostConfig.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class RuntimeSettingsPanel extends StatefulWidget {
  const RuntimeSettingsPanel({super.key, this.embedded = false});

  final bool embedded;

  @override
  State<RuntimeSettingsPanel> createState() => _RuntimeSettingsPanelState();
}

class _RuntimeSettingsPanelState extends State<RuntimeSettingsPanel> {
  bool _busy = false;
  bool _discoverable = false;
  String? _connectionMessage;
  bool _connectionFailed = false;
  List<_DiscoveredRuntimeDevice> _discoveredDevices =
      <_DiscoveredRuntimeDevice>[];
  bool _scanning = false;
  String? _scanError;
  Map<String, InboundLinkSessionRecord> _acceptedSessions =
      <String, InboundLinkSessionRecord>{};
  String? _pendingPairingId;
  Map<String, _PairedRemoteProbeState> _pairedRemoteStates =
      <String, _PairedRemoteProbeState>{};
  int _pairedRemoteProbeGeneration = 0;
  String? _syncingRemoteName;

  RuntimeConnectionManager get _manager => RuntimeConnectionManager.instance;

  @override
  void initState() {
    super.initState();
    _manager.addListener(_onManagerChanged);
    LinkHostServer.instance.addListener(_onWebAccessChanged);
    _loadAcceptedSessions();
    _loadDiscoverable();
    unawaited(_refreshPairedRemoteStates());
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        unawaited(_scanForDevices());
      }
    });
  }

  @override
  void dispose() {
    _manager.removeListener(_onManagerChanged);
    LinkHostServer.instance.removeListener(_onWebAccessChanged);
    super.dispose();
  }

  void _onManagerChanged() {
    if (mounted) {
      setState(() {});
      unawaited(_refreshPairedRemoteStates());
    }
  }

  void _onWebAccessChanged() {
    final code = LinkHostServer.instance.lastPairingCode;
    if (code != null && _pendingPairingId == null) {
      // A new pairing code appeared - someone started pairing
      _pendingPairingId = code.pairingId;
      // Defer a check to see if pairing completes or gets rejected
      Future.delayed(const Duration(seconds: 12), () {
        if (_pendingPairingId != null && mounted) {
          _loadAcceptedSessions();
        }
      });
    }
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _loadAcceptedSessions() async {
    final sessions = await InboundLinkSessionStore.read();
    if (mounted) {
      if (_pendingPairingId != null) {
        final currentCount = sessions.length;
        final previousCount = _acceptedSessions.length;
        if (currentCount <= previousCount) {
          // Pairing code was consumed but no accepted session was created
          // -> the connection was rejected
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                AppLocalizations.of(context)!.settingsRuntimePairingRejected,
              ),
            ),
          );
        }
        _pendingPairingId = null;
      }
      setState(() {
        _acceptedSessions = sessions;
      });
    }
  }

  Future<void> _loadDiscoverable() async {
    final config = await LinkHostConfigStore.read();
    if (mounted) {
      setState(() => _discoverable = config.discoveryEnabled);
    }
  }

  Future<void> _refreshPairedRemoteStates() async {
    final sessions = Map<String, PairedRemoteSessionRecord>.of(
      _manager.config.remoteSessions,
    );
    final generation = ++_pairedRemoteProbeGeneration;
    if (mounted) {
      setState(() {
        _pairedRemoteStates = <String, _PairedRemoteProbeState>{
          for (final name in sessions.keys)
            name: _PairedRemoteProbeState.checking,
        };
      });
    }
    final results = await Future.wait(
      sessions.entries.map((entry) async {
        final state = await _probePairedRemote(entry.value);
        return MapEntry(entry.key, state);
      }),
    );
    if (!mounted || generation != _pairedRemoteProbeGeneration) {
      return;
    }
    setState(() {
      _pairedRemoteStates = Map<String, _PairedRemoteProbeState>.fromEntries(
        results,
      );
    });
  }

  Future<_PairedRemoteProbeState> _probePairedRemote(
    PairedRemoteSessionRecord session,
  ) async {
    final client = RemoteRuntimeLinkClient(session: session);
    try {
      final info = await client.sessionInfo().timeout(
        const Duration(seconds: 2),
      );
      return info.coreDeviceId == session.coreDeviceId
          ? _PairedRemoteProbeState.online
          : _PairedRemoteProbeState.offline;
    } catch (_) {
      return _PairedRemoteProbeState.offline;
    } finally {
      client.dispose();
    }
  }

  Future<void> _useLocal() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      await _manager.setLocal();
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeSwitchedLocal;
          _connectionFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeTestFailed(error.toString());
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _usePairedRemote(String name) async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final connected = await _manager.usePairedRemote(name);
      if (connected && mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeSwitchedRemote;
          _connectionFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeTestFailed(error.toString());
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _deletePairedRemote(String name) async {
    setState(() => _busy = true);
    try {
      await _manager.removePairedRemote(name);
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _syncPairedRemote(String name) async {
    final l10n = AppLocalizations.of(context)!;
    final session = _manager.config.remoteSessions[name]!;
    setState(() {
      _busy = true;
      _syncingRemoteName = name;
      _connectionMessage = l10n.settingsRuntimeSyncing;
      _connectionFailed = false;
    });
    try {
      final result = await const RuntimeDataSyncBridge().syncPairedRemote(
        session: session,
      );
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeSyncCompleted(
            result.localApplied,
            result.remoteApplied,
          );
          _connectionFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeSyncFailed(error.toString());
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _syncingRemoteName = null;
        });
        unawaited(_refreshPairedRemoteStates());
      }
    }
  }

  Future<void> _testCurrent() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() {
      _busy = true;
      _connectionMessage = l10n.settingsRuntimeTesting;
      _connectionFailed = false;
    });
    try {
      final version = await const ProxyCoreRuntimeBridge().callApplication(
        'coreVersion',
      );
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeTestResult(
            version.toString(),
          );
          _connectionFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeTestFailed(error.toString());
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _deleteAcceptedSession(String sessionId) async {
    setState(() => _busy = true);
    try {
      final sessions = Map<String, InboundLinkSessionRecord>.of(
        _acceptedSessions,
      )..remove(sessionId);
      await InboundLinkSessionStore.write(sessions);
      if (mounted) {
        setState(() => _acceptedSessions = sessions);
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _setDiscoverable(bool value) async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final config = await LinkHostConfigStore.read();
      final server = LinkHostServer.instance;
      if (value) {
        final next = _linkHostConfigForWrite(
          config.copyWith(
            discoveryEnabled: true,
            updatedAt: DateTime.now().millisecondsSinceEpoch,
          ),
        );
        await server.start(next);
        await LinkHostConfigStore.write(next);
      } else {
        final next = _linkHostConfigForWrite(
          config.copyWith(
            discoveryEnabled: false,
            updatedAt: DateTime.now().millisecondsSinceEpoch,
          ),
        );
        if (config.webAccessEnabled) {
          await server.start(next);
          await LinkHostConfigStore.write(next);
        } else {
          await server.stop(updateConfig: false);
          await LinkHostConfigStore.write(next);
        }
      }
      if (mounted) {
        setState(() => _discoverable = value);
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              value
                  ? l10n.settingsRuntimeEnableDiscoveryFailed(error.toString())
                  : l10n.settingsRuntimeDisableDiscoveryFailed(
                      error.toString(),
                    ),
            ),
          ),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _scanForDevices() async {
    setState(() {
      _scanning = true;
      _scanError = null;
      _discoveredDevices = <_DiscoveredRuntimeDevice>[];
    });
    try {
      final json = await LinkHostServer.instance.discoverDevices(2000);
      final list = (jsonDecode(json) as List<dynamic>)
          .cast<Map<String, Object?>>();
      final devices = list
          .map(_DiscoveredRuntimeDevice.fromJson)
          .toList(growable: false);
      final visibleDevices = await _visibleDiscoveredDevices(devices);
      if (mounted) {
        setState(() {
          _discoveredDevices = visibleDevices;
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

  Future<List<_DiscoveredRuntimeDevice>> _visibleDiscoveredDevices(
    List<_DiscoveredRuntimeDevice> devices,
  ) async {
    final localDeviceId = await LinkHostDeviceIdStore.read();
    final visibleDevices = <_DiscoveredRuntimeDevice>[];
    final checkedStates = <String, _PairedRemoteProbeState>{};
    for (final device in devices) {
      if (device.deviceId == localDeviceId) {
        continue;
      }
      final pairedEntries = _manager.config.remoteSessions.entries
          .where((entry) => device.deviceId == entry.value.coreDeviceId)
          .toList(growable: false);
      if (pairedEntries.isEmpty) {
        visibleDevices.add(device);
        continue;
      }
      var anyOnline = false;
      for (final entry in pairedEntries) {
        final state = await _probePairedRemote(entry.value);
        checkedStates[entry.key] = state;
        if (state == _PairedRemoteProbeState.online) {
          anyOnline = true;
        }
      }
      if (!anyOnline) {
        visibleDevices.add(device);
      }
    }
    if (mounted && checkedStates.isNotEmpty) {
      setState(() {
        _pairedRemoteStates = <String, _PairedRemoteProbeState>{
          ..._pairedRemoteStates,
          ...checkedStates,
        };
      });
    }
    return visibleDevices;
  }

  Future<void> _pairRemote() async {
    final result = await _RemotePairDialog.show(context);
    if (result == null || !mounted) {
      return;
    }
    await _saveRemotePairResult(result);
  }

  Future<void> _pairDiscoveredRemote(_DiscoveredRuntimeDevice device) async {
    setState(() => _busy = true);
    try {
      final pairing = await const RemotePairingBridge().startWithTokenHash(
        baseUrl: device.baseUrl,
        tokenHash: device.tokenHash,
      );
      if (!mounted) {
        return;
      }
      setState(() => _busy = false);
      final result = await _RemotePairCodeDialog.show(
        context,
        pairing: pairing,
      );
      if (result == null || !mounted) {
        return;
      }
      await _saveRemotePairResult(result);
    } catch (error) {
      if (mounted) {
        setState(() {
          _busy = false;
          _connectionMessage = AppLocalizations.of(
            context,
          )!.settingsRuntimeTestFailed(error.toString());
          _connectionFailed = true;
        });
      }
    }
  }

  Future<void> _saveRemotePairResult(_RemotePairResult result) async {
    if (!mounted) {
      return;
    }
    setState(() => _busy = true);
    try {
      final connected = await _manager.setRemote(
        name: result.name,
        session: result.session,
      );
      if (connected && mounted) {
        setState(() {
          _connectionMessage = AppLocalizations.of(
            context,
          )!.settingsRuntimeSwitchedRemote;
          _connectionFailed = false;
        });
        unawaited(_refreshPairedRemoteStates());
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = AppLocalizations.of(
            context,
          )!.settingsRuntimeTestFailed(error.toString());
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final config = _manager.config;
    final children = <Widget>[
      _SectionCard(
        title: l10n.settingsRuntimeConnection,
        children: <Widget>[
          _CurrentDeviceLine(config: config),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              FilledButton.tonalIcon(
                style: SettingsControlStyles.sectionFilledButton(),
                onPressed: config.mode == RuntimeConnectionMode.local || _busy
                    ? null
                    : _useLocal,
                icon: const Icon(Icons.devices_outlined, size: 18),
                label: Text(l10n.settingsRuntimeUseLocal),
              ),
              TextButton.icon(
                style: SettingsControlStyles.sectionTextButton(),
                onPressed: _busy ? null : _testCurrent,
                icon: const Icon(Icons.network_check_outlined, size: 18),
                label: Text(l10n.settingsRuntimeTestCurrent),
              ),
            ],
          ),
          if (_connectionMessage != null) ...<Widget>[
            const SizedBox(height: 8),
            _InlineStatus(
              message: _connectionMessage!,
              failed: _connectionFailed,
            ),
          ],
        ],
      ),
      _SectionCard(
        title: l10n.settingsRuntimeRemoteTitle,
        children: <Widget>[
          Text(
            l10n.settingsRuntimeRemoteDescription,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 8),
          _RemoteSessionList(
            config: config,
            busy: _busy,
            syncingName: _syncingRemoteName,
            states: _pairedRemoteStates,
            onUse: _usePairedRemote,
            onSync: _syncPairedRemote,
            onDelete: _deletePairedRemote,
          ),
          if (_acceptedSessions.isNotEmpty)
            ..._acceptedSessions.entries.map(
              (entry) => _AcceptedSessionTile(
                sessionId: entry.key,
                record: entry.value,
                onDelete: _busy
                    ? null
                    : () => _deleteAcceptedSession(entry.key),
              ),
            ),
        ],
      ),
      _SectionCard(
        title: l10n.settingsRuntimeDiscoverDevices,
        children: <Widget>[
          Text(
            l10n.settingsRuntimeDiscoverDevicesDescription,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              FilledButton.tonalIcon(
                style: SettingsControlStyles.sectionFilledButton(),
                onPressed: _busy || _scanning ? null : _scanForDevices,
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
                style: SettingsControlStyles.sectionTextButton(),
                onPressed: _busy ? null : _pairRemote,
                icon: const Icon(Icons.add_outlined, size: 18),
                label: Text(l10n.settingsRuntimeEnterManually),
              ),
            ],
          ),
          if (_scanError != null) ...<Widget>[
            const SizedBox(height: 8),
            _InlineStatus(message: _scanError!, failed: true),
          ],
          if (_discoveredDevices.isNotEmpty) ...<Widget>[
            const SizedBox(height: 12),
            Divider(
              height: 1,
              color: Theme.of(
                context,
              ).colorScheme.outlineVariant.withValues(alpha: 0.3),
            ),
            const SizedBox(height: 4),
            ..._discoveredDevices.map((device) {
              return ListTile(
                dense: true,
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.devices_other_outlined),
                title: Text(
                  device.displayName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                subtitle: Text(
                  device.baseUrl,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                trailing: IconButton(
                  icon: const Icon(Icons.link_outlined),
                  tooltip: l10n.settingsRuntimeConnect,
                  onPressed: _busy ? null : () => _pairDiscoveredRemote(device),
                ),
              );
            }),
          ],
          const SizedBox(height: 12),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            dense: true,
            visualDensity: VisualDensity.compact,
            title: Text(l10n.settingsRuntimeEnableDiscovery),
            subtitle: Text(l10n.settingsRuntimeEnableDiscoveryDescription),
            value: _discoverable,
            onChanged: _busy ? null : _setDiscoverable,
          ),
        ],
      ),
    ];
    if (widget.embedded) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: children,
      );
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: children,
    );
  }
}

LinkHostConfig _linkHostConfigForWrite(LinkHostConfig config) {
  if (config.portMode == LinkHostPortMode.automatic) {
    return config.copyWith(bindAddress: LinkHostConfig.automaticBindAddress);
  }
  return config;
}

enum _PairedRemoteProbeState { checking, online, offline }

class _DiscoveredRuntimeDevice {
  const _DiscoveredRuntimeDevice({
    required this.deviceId,
    required this.displayName,
    required this.platform,
    required this.model,
    required this.baseUrl,
    required this.hostname,
    required this.port,
    required this.tokenHash,
    required this.version,
  });

  factory _DiscoveredRuntimeDevice.fromJson(Map<String, Object?> json) {
    return _DiscoveredRuntimeDevice(
      deviceId: json['device_id'] as String,
      displayName: json['display_name'] as String,
      platform: json['platform'] as String,
      model: json['model'] as String,
      baseUrl: json['base_url'] as String,
      hostname: json['hostname'] as String,
      port: json['port'] as int,
      tokenHash: json['token_hash'] as String,
      version: json['version'] as String,
    );
  }

  final String deviceId;
  final String displayName;
  final String platform;
  final String model;
  final String baseUrl;
  final String hostname;
  final int port;
  final String tokenHash;
  final String version;
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              const SizedBox(height: 8),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _CurrentDeviceLine extends StatelessWidget {
  const _CurrentDeviceLine({required this.config});

  final RuntimeConnectionConfig config;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final selectedLocal = config.mode == RuntimeConnectionMode.local;
    final title = selectedLocal
        ? l10n.settingsRuntimeUsingLocal
        : l10n.settingsRuntimeUsingRemote(
            config.activeRemoteSession!.remoteDeviceInfo.displayName,
          );
    final subtitle = selectedLocal
        ? l10n.settingsRuntimeLocalDescription
        : l10n.settingsRuntimeRemoteInUseDescription;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Icon(
          selectedLocal ? Icons.devices_outlined : Icons.cloud_done_outlined,
          color: colorScheme.primary,
        ),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: Theme.of(
                  context,
                ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
              ),
              const SizedBox(height: 4),
              Text(
                subtitle,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _RemoteSessionList extends StatelessWidget {
  const _RemoteSessionList({
    required this.config,
    required this.busy,
    required this.syncingName,
    required this.states,
    required this.onUse,
    required this.onSync,
    required this.onDelete,
  });

  final RuntimeConnectionConfig config;
  final bool busy;
  final String? syncingName;
  final Map<String, _PairedRemoteProbeState> states;
  final ValueChanged<String> onUse;
  final ValueChanged<String> onSync;
  final ValueChanged<String> onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final entries = config.remoteSessions.entries.toList(growable: false);
    if (entries.isEmpty) {
      return Text(
        l10n.settingsRuntimeNoPairedRemote,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      );
    }
    return Column(
      children: <Widget>[
        for (var index = 0; index < entries.length; index++) ...<Widget>[
          _RemoteSessionTile(
            name: entries[index].key,
            session: entries[index].value,
            active:
                config.mode == RuntimeConnectionMode.remote &&
                config.activeRemoteName == entries[index].key,
            busy: busy,
            syncing: syncingName == entries[index].key,
            state: states[entries[index].key],
            onUse: () => onUse(entries[index].key),
            onSync: () => onSync(entries[index].key),
            onDelete: () => onDelete(entries[index].key),
          ),
          if (index < entries.length - 1) const Divider(height: 12),
        ],
      ],
    );
  }
}

class _RemoteSessionTile extends StatelessWidget {
  const _RemoteSessionTile({
    required this.name,
    required this.session,
    required this.active,
    required this.busy,
    required this.syncing,
    required this.state,
    required this.onUse,
    required this.onSync,
    required this.onDelete,
  });

  final String name;
  final PairedRemoteSessionRecord session;
  final bool active;
  final bool busy;
  final bool syncing;
  final _PairedRemoteProbeState? state;
  final VoidCallback onUse;
  final VoidCallback onSync;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final probeState = state ?? _PairedRemoteProbeState.checking;
    return ListTile(
      dense: true,
      contentPadding: EdgeInsets.zero,
      leading: _RemoteProbeIcon(state: probeState),
      title: Text(
        session.remoteDeviceInfo.displayName,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(session.baseUrl, maxLines: 1, overflow: TextOverflow.ellipsis),
          const SizedBox(height: 2),
          _RemoteProbeText(state: probeState),
        ],
      ),
      trailing: Wrap(
        spacing: 2,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: <Widget>[
          if (active)
            SettingsActivePill(label: l10n.settingsActive)
          else
            SettingsSetActiveButton(
              label: l10n.settingsActivate,
              onPressed: busy ? null : onUse,
            ),
          IconButton(
            tooltip: syncing
                ? l10n.settingsRuntimeSyncing
                : l10n.settingsRuntimeSync,
            icon: syncing
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: M3LoadingIndicator(size: 18),
                  )
                : const Icon(Icons.sync_outlined),
            onPressed:
                busy || probeState != _PairedRemoteProbeState.online || syncing
                ? null
                : onSync,
          ),
          IconButton(
            tooltip: l10n.delete,
            icon: const Icon(Icons.delete_outline),
            onPressed: busy ? null : onDelete,
          ),
        ],
      ),
    );
  }
}

class _InlineStatus extends StatelessWidget {
  const _InlineStatus({required this.message, required this.failed});

  final String message;
  final bool failed;

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

class _RemoteProbeIcon extends StatelessWidget {
  const _RemoteProbeIcon({required this.state});

  final _PairedRemoteProbeState state;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return switch (state) {
      _PairedRemoteProbeState.checking => const SizedBox(
        width: 24,
        height: 24,
        child: Center(child: M3LoadingIndicator(size: 18)),
      ),
      _PairedRemoteProbeState.online => Icon(
        Icons.cloud_done_outlined,
        color: colorScheme.primary,
      ),
      _PairedRemoteProbeState.offline => Icon(
        Icons.cloud_off_outlined,
        color: colorScheme.error,
      ),
    };
  }
}

class _RemoteProbeText extends StatelessWidget {
  const _RemoteProbeText({required this.state});

  final _PairedRemoteProbeState state;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final label = switch (state) {
      _PairedRemoteProbeState.checking => l10n.settingsRuntimePairedChecking,
      _PairedRemoteProbeState.online => l10n.settingsRuntimePairedOnline,
      _PairedRemoteProbeState.offline => l10n.settingsRuntimePairedOffline,
    };
    final color = switch (state) {
      _PairedRemoteProbeState.checking => colorScheme.onSurfaceVariant,
      _PairedRemoteProbeState.online => colorScheme.primary,
      _PairedRemoteProbeState.offline => colorScheme.error,
    };
    return Text(
      label,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: Theme.of(context).textTheme.bodySmall?.copyWith(
        color: color,
        fontWeight: FontWeight.w700,
      ),
    );
  }
}

class _AcceptedSessionTile extends StatelessWidget {
  const _AcceptedSessionTile({
    required this.sessionId,
    required this.record,
    required this.onDelete,
  });

  final String sessionId;
  final InboundLinkSessionRecord record;
  final VoidCallback? onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return ListTile(
      dense: true,
      contentPadding: EdgeInsets.zero,
      leading: const Icon(Icons.devices_other_outlined),
      title: Text(
        record.deviceInfo.displayName,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(sessionId, maxLines: 1, overflow: TextOverflow.ellipsis),
      trailing: IconButton(
        tooltip: l10n.delete,
        icon: const Icon(Icons.delete_outline),
        onPressed: onDelete,
      ),
    );
  }
}

class _RemotePairDialog extends StatefulWidget {
  const _RemotePairDialog();

  static Future<_RemotePairResult?> show(BuildContext context) {
    return showDialog<_RemotePairResult>(
      context: context,
      builder: (_) => const _RemotePairDialog(),
    );
  }

  @override
  State<_RemotePairDialog> createState() => _RemotePairDialogState();
}

class _RemotePairDialogState extends State<_RemotePairDialog> {
  final TextEditingController _baseUrlController = TextEditingController();
  final TextEditingController _tokenController = TextEditingController();
  final TextEditingController _codeController = TextEditingController();
  RemotePairStartResult? _pairing;
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _baseUrlController.dispose();
    _tokenController.dispose();
    _codeController.dispose();
    super.dispose();
  }

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
      );
      if (mounted) {
        Navigator.of(context).pop(
          _RemotePairResult(
            name: pairing.coreDeviceInfo.displayName,
            session: session,
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

  static Future<_RemotePairResult?> show(
    BuildContext context, {
    required RemotePairStartResult pairing,
  }) {
    return showDialog<_RemotePairResult>(
      context: context,
      builder: (_) => _RemotePairCodeDialog(pairing: pairing),
    );
  }

  @override
  State<_RemotePairCodeDialog> createState() => _RemotePairCodeDialogState();
}

class _RemotePairCodeDialogState extends State<_RemotePairCodeDialog> {
  final TextEditingController _codeController = TextEditingController();
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _codeController.dispose();
    super.dispose();
  }

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
      );
      if (mounted) {
        Navigator.of(context).pop(
          _RemotePairResult(
            name: widget.pairing.coreDeviceInfo.displayName,
            session: session,
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
  const _RemotePairResult({required this.name, required this.session});

  final String name;
  final PairedRemoteSessionRecord session;
}
