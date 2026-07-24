// ignore_for_file: file_names

import 'dart:async';
import 'package:flutter/material.dart';

import '../../../../core/bridge/PlatformCoreProxy.dart';
import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../core/runtime/RemotePairingBridge.dart';
import '../../../../core/runtime/RuntimeAutoSyncManager.dart';
import '../../../../core/runtime/RuntimeConnectionManager.dart';
import '../../../../core/link_access/LinkAccessHost.dart';
import '../../../../core/link_access/LinkAccessHostConfig.dart';
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
  List<generated.RuntimeRemoteDiscoveredDevice> _discoveredDevices =
      <generated.RuntimeRemoteDiscoveredDevice>[];
  bool _scanning = false;
  String? _scanError;
  Map<String, _PairedRemoteProbeState> _pairedRemoteStates =
      <String, _PairedRemoteProbeState>{};
  Map<String, generated.PairedRemoteSessionRecord> _pairedRemoteSessions =
      <String, generated.PairedRemoteSessionRecord>{};
  generated.LinkAccessRoutingConfig? _currentRoute;
  int _pairedRemoteProbeGeneration = 0;
  final Set<String> _autoSyncTogglingRemoteNames = <String>{};

  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
  );

  RuntimeConnectionManager get _manager => RuntimeConnectionManager.instance;
  RuntimeAutoSyncManager get _autoSync => RuntimeAutoSyncManager.instance;

  @override
  void initState() {
    super.initState();
    _manager.addListener(_onManagerChanged);
    _autoSync.addListener(_onAutoSyncChanged);
    unawaited(_autoSync.initialize());
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
    _autoSync.removeListener(_onAutoSyncChanged);
    super.dispose();
  }

  void _onManagerChanged() {
    if (mounted) {
      setState(() {});
      unawaited(_refreshPairedRemoteStates());
    }
  }

  void _onAutoSyncChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _loadDiscoverable() async {
    final config = await LinkAccessHostConfigStore.read();
    if (mounted) {
      setState(() => _discoverable = config.discoveryEnabled);
    }
  }

  Future<void> _refreshPairedRemoteStates() async {
    final generation = ++_pairedRemoteProbeGeneration;
    final sessions = await _loadPairedRemoteSessions();
    final route = await _clients.runtimeRemoteLinkService.currentRoute();
    if (!mounted || generation != _pairedRemoteProbeGeneration) {
      return;
    }
    if (mounted) {
      setState(() {
        _pairedRemoteSessions = sessions;
        _currentRoute = route;
        _pairedRemoteStates = <String, _PairedRemoteProbeState>{
          for (final name in sessions.keys)
            name: _PairedRemoteProbeState.checking,
        };
      });
    }
    final results = await Future.wait(
      sessions.entries.map((entry) async {
        final state = await _probePairedRemote(entry.key);
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

  /// Reads paired remote sessions from the runtime-owned Link Access store.
  Future<Map<String, generated.PairedRemoteSessionRecord>>
  _loadPairedRemoteSessions() {
    return _clients.runtimeRemoteLinkService.pairedRemoteSessions();
  }

  /// Returns whether the local runtime is the persisted request destination.
  bool get _isLocalRoute => _currentRoute?.route.tag != 'remote';

  Future<_PairedRemoteProbeState> _probePairedRemote(String name) async {
    try {
      await _clients.runtimeRemoteLinkService
          .probePairedRemote(name: name)
          .timeout(const Duration(seconds: 2));
      return _PairedRemoteProbeState.online;
    } catch (_) {
      return _PairedRemoteProbeState.offline;
    }
  }

  Future<void> _deletePairedRemote(String name) async {
    setState(() => _busy = true);
    try {
      await _clients.runtimeRemoteLinkService.removePairedRemote(name: name);
      await _autoSync.refresh();
      await _refreshPairedRemoteStates();
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _setRemoteAutoSync(String name, bool enabled) async {
    final l10n = AppLocalizations.of(context)!;
    setState(() {
      _autoSyncTogglingRemoteNames.add(name);
      _connectionFailed = false;
    });
    try {
      await _autoSync.setRemoteEnabled(name, enabled);
      if (mounted) {
        setState(() {
          _connectionMessage = enabled
              ? l10n.settingsRuntimeAutoSyncEnabled
              : l10n.settingsRuntimeAutoSyncDisabled;
          _connectionFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeAutoSyncFailed(
            error.toString(),
          );
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() {
          _autoSyncTogglingRemoteNames.remove(name);
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

  /// Selects the local runtime as the destination for application requests.
  Future<void> _selectLocalRoute() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      await _clients.runtimeRemoteLinkService.selectLocalRoute();
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeSwitchedLocal;
          _connectionFailed = false;
        });
      }
      await _refreshPairedRemoteStates();
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

  /// Selects one paired remote runtime as the destination for application requests.
  Future<void> _selectPairedRemoteRoute(String name) async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      await _clients.runtimeRemoteLinkService.selectPairedRemoteRoute(
        name: name,
      );
      if (mounted) {
        setState(() {
          _connectionMessage = l10n.settingsRuntimeSwitchedRemote;
          _connectionFailed = false;
        });
      }
      await _refreshPairedRemoteStates();
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

  Future<void> _setDiscoverable(bool value) async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final config = await LinkAccessHostConfigStore.read();
      final server = LinkAccessHost.instance;
      if (value) {
        final next = _linkHostConfigForWrite(
          config.copyWith(
            discoveryEnabled: true,
            updatedAt: DateTime.now().millisecondsSinceEpoch,
          ),
        );
        await server.start(next);
        await LinkAccessHostConfigStore.write(next);
      } else {
        final next = _linkHostConfigForWrite(
          config.copyWith(
            discoveryEnabled: false,
            updatedAt: DateTime.now().millisecondsSinceEpoch,
          ),
        );
        if (config.webAccessEnabled) {
          await server.start(next);
          await LinkAccessHostConfigStore.write(next);
        } else {
          await server.stop(updateConfig: false);
          await LinkAccessHostConfigStore.write(next);
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
      _discoveredDevices = <generated.RuntimeRemoteDiscoveredDevice>[];
    });
    try {
      final devices = await _clients.runtimeRemoteLinkService
          .discoverPairedRemotes(timeoutMs: 2000);
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

  /// Filters local and already usable paired runtimes from one discovery result.
  Future<List<generated.RuntimeRemoteDiscoveredDevice>>
  _visibleDiscoveredDevices(
    List<generated.RuntimeRemoteDiscoveredDevice> devices,
  ) async {
    final localDeviceId = LinkAccessHost.instance.deviceId;
    final visibleDevices = <generated.RuntimeRemoteDiscoveredDevice>[];
    final checkedStates = <String, _PairedRemoteProbeState>{};
    for (final device in devices) {
      if (device.deviceId == localDeviceId) {
        continue;
      }
      final pairedEntries = _pairedRemoteSessions.entries
          .where((entry) => device.deviceId == entry.value.coreDeviceId)
          .toList(growable: false);
      if (pairedEntries.isEmpty) {
        visibleDevices.add(device);
        continue;
      }
      var anyOnline = false;
      for (final entry in pairedEntries) {
        final state = await _probePairedRemote(entry.key);
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

  /// Starts pairing with one runtime selected from the runtime-owned discovery result.
  Future<void> _pairDiscoveredRemote(
    generated.RuntimeRemoteDiscoveredDevice device,
  ) async {
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

  /// Refreshes settings after the runtime service persisted a completed pairing.
  Future<void> _saveRemotePairResult(_RemotePairResult _) async {
    if (!mounted) {
      return;
    }
    setState(() => _busy = true);
    try {
      if (mounted) {
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
    final children = <Widget>[
      _SectionCard(
        title: l10n.settingsRuntimeConnection,
        children: <Widget>[
          _CurrentDeviceLine(
            active: _isLocalRoute,
            busy: _busy,
            onSelect: _selectLocalRoute,
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
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
            sessions: _pairedRemoteSessions,
            busy: _busy,
            autoSyncEnabledNames: _autoSync.enabledRemoteNames,
            autoSyncTogglingNames: _autoSyncTogglingRemoteNames,
            states: _pairedRemoteStates,
            selectedRouteName: _currentRoute?.route.tag == 'remote'
                ? _currentRoute?.route.sessionName
                : null,
            onAutoSyncChanged: _setRemoteAutoSync,
            onSelectRoute: _selectPairedRemoteRoute,
            onDelete: _deletePairedRemote,
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

LinkAccessHostConfig _linkHostConfigForWrite(LinkAccessHostConfig config) {
  if (config.portMode == LinkAccessHostPortMode.automatic) {
    return config.copyWith(
      bindAddress: LinkAccessHostConfig.automaticBindAddress,
    );
  }
  return config;
}

enum _PairedRemoteProbeState { checking, online, offline }

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
  const _CurrentDeviceLine({
    required this.active,
    required this.busy,
    required this.onSelect,
  });

  final bool active;
  final bool busy;
  final VoidCallback onSelect;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Icon(Icons.devices_outlined, color: colorScheme.primary),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                active
                    ? l10n.settingsRuntimeUsingLocal
                    : l10n.settingsRuntimeLocalTitle,
                style: Theme.of(
                  context,
                ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
              ),
              const SizedBox(height: 4),
              Text(
                l10n.settingsRuntimeLocalDescription,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
        IconButton(
          tooltip: l10n.settingsRuntimeUseLocal,
          icon: Icon(
            active
                ? Icons.radio_button_checked_outlined
                : Icons.radio_button_unchecked_outlined,
          ),
          onPressed: active || busy ? null : onSelect,
        ),
      ],
    );
  }
}

class _RemoteSessionList extends StatelessWidget {
  const _RemoteSessionList({
    required this.sessions,
    required this.busy,
    required this.autoSyncEnabledNames,
    required this.autoSyncTogglingNames,
    required this.states,
    required this.selectedRouteName,
    required this.onAutoSyncChanged,
    required this.onSelectRoute,
    required this.onDelete,
  });

  final Map<String, generated.PairedRemoteSessionRecord> sessions;
  final bool busy;
  final Set<String> autoSyncEnabledNames;
  final Set<String> autoSyncTogglingNames;
  final Map<String, _PairedRemoteProbeState> states;
  final String? selectedRouteName;
  final void Function(String name, bool enabled) onAutoSyncChanged;
  final ValueChanged<String> onSelectRoute;
  final ValueChanged<String> onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final entries = sessions.entries.toList(growable: false);
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
            busy: busy,
            autoSyncEnabled: autoSyncEnabledNames.contains(entries[index].key),
            autoSyncToggling: autoSyncTogglingNames.contains(
              entries[index].key,
            ),
            state: states[entries[index].key],
            selected: selectedRouteName == entries[index].key,
            onAutoSyncChanged: (enabled) =>
                onAutoSyncChanged(entries[index].key, enabled),
            onSelect: () => onSelectRoute(entries[index].key),
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
    required this.busy,
    required this.autoSyncEnabled,
    required this.autoSyncToggling,
    required this.state,
    required this.selected,
    required this.onAutoSyncChanged,
    required this.onSelect,
    required this.onDelete,
  });

  final String name;
  final generated.PairedRemoteSessionRecord session;
  final bool busy;
  final bool autoSyncEnabled;
  final bool autoSyncToggling;
  final _PairedRemoteProbeState? state;
  final bool selected;
  final ValueChanged<bool> onAutoSyncChanged;
  final VoidCallback onSelect;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final probeState = state ?? _PairedRemoteProbeState.checking;
    final syncBusy = autoSyncToggling;
    final syncIcon = syncBusy
        ? const SizedBox(
            width: 18,
            height: 18,
            child: M3LoadingIndicator(size: 18),
          )
        : Icon(
            autoSyncEnabled
                ? Icons.sync_outlined
                : Icons.sync_disabled_outlined,
          );
    final syncTooltip = syncBusy
        ? l10n.settingsRuntimeSyncing
        : autoSyncEnabled
        ? l10n.settingsRuntimeAutoSyncDisable
        : l10n.settingsRuntimeAutoSyncEnable;
    final syncPressed = busy || syncBusy
        ? null
        : () => onAutoSyncChanged(!autoSyncEnabled);
    return ListTile(
      dense: true,
      contentPadding: EdgeInsets.zero,
      leading: _RemoteProbeIcon(state: probeState),
      title: Text(
        '${session.remoteDeviceInfo.platform}-${session.remoteDeviceInfo.model}',
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
          IconButton(
            tooltip: l10n.settingsRuntimeRemoteTitle,
            icon: Icon(
              selected
                  ? Icons.radio_button_checked_outlined
                  : Icons.radio_button_unchecked_outlined,
            ),
            onPressed: busy || selected ? null : onSelect,
          ),
          autoSyncEnabled
              ? IconButton.filledTonal(
                  tooltip: syncTooltip,
                  icon: syncIcon,
                  onPressed: syncPressed,
                )
              : IconButton(
                  tooltip: syncTooltip,
                  icon: syncIcon,
                  onPressed: syncPressed,
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
        name: _pairedRuntimeSessionName(pairing),
      );
      if (mounted) {
        Navigator.of(context).pop(
          _RemotePairResult(
            name: _pairedRuntimeSessionName(pairing),
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
        name: _pairedRuntimeSessionName(widget.pairing),
      );
      if (mounted) {
        Navigator.of(context).pop(
          _RemotePairResult(
            name: _pairedRuntimeSessionName(widget.pairing),
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

/// Builds one stable local session key for a completed remote pairing.
String _pairedRuntimeSessionName(RemotePairStartResult pairing) {
  return '${pairing.coreDeviceInfo.platform}-${pairing.coreDeviceInfo.model}-${pairing.coreDeviceId}';
}

class _RemotePairResult {
  const _RemotePairResult({required this.name, required this.session});

  final String name;
  final generated.PairedRemoteSessionRecord session;
}
