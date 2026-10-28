// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitFormStyles.dart';

/// Keeps connection settings and their single save action in the same dialog.
class PeerListenerSettingsDialog extends StatefulWidget {
  const PeerListenerSettingsDialog({
    super.key,
    required this.clients,
    this.onBusyChanged,
  });

  final GeneratedCoreProxyClients clients;
  final ValueChanged<bool>? onBusyChanged;

  @override
  State<PeerListenerSettingsDialog> createState() =>
      _PeerListenerSettingsDialogState();
}

class _PeerListenerSettingsDialogState
    extends State<PeerListenerSettingsDialog> {
  final _formKey = GlobalKey<_PeerListenerSettingsState>();
  bool _busy = true;

  void _busyChanged(bool busy) {
    if (!mounted) return;
    setState(() => _busy = busy);
    widget.onBusyChanged?.call(busy);
  }

  Future<void> _save() async {
    final saved = await _formKey.currentState?._apply() ?? false;
    if (mounted && saved) Navigator.pop(context);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return PopScope(
      canPop: !_busy,
      child: AlertDialog(
        title: Text(l10n.deviceSpaceConnectionSettings),
        content: SizedBox(
          width: 440,
          child: SingleChildScrollView(
            child: PeerListenerSettings(
              key: _formKey,
              clients: widget.clients,
              onBusyChanged: _busyChanged,
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: !_busy && _formKey.currentState?._loaded == true
                ? _save
                : null,
            child: Text(l10n.save),
          ),
        ],
      ),
    );
  }
}

/// Manages node-local discovery, and explicit listener binding.
class PeerListenerSettings extends StatefulWidget {
  /// Creates the node-local connection settings panel.
  const PeerListenerSettings({
    super.key,
    required this.clients,
    this.enabled = true,
    this.onBusyChanged,
  });

  final GeneratedCoreProxyClients clients;
  final bool enabled;
  final ValueChanged<bool>? onBusyChanged;

  /// Creates state that owns the listener settings form.
  @override
  State<PeerListenerSettings> createState() => _PeerListenerSettingsState();
}

class _PeerListenerSettingsState extends State<PeerListenerSettings> {
  final _bindHost = TextEditingController();
  final _bindPort = TextEditingController();
  Set<PeerTransport> _transports = {};
  PeerListenerCapabilities? _capabilities;
  PeerHostPortMode? _portMode;
  bool _discoverable = false;
  bool _busy = true;
  bool _loaded = false;
  String? _error;

  /// Loads the node-local listener configuration when the panel opens.
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _load();
    });
  }

  /// Releases the address text controllers.
  @override
  void dispose() {
    _bindHost.dispose();
    _bindPort.dispose();
    super.dispose();
  }

  /// Loads the persisted listener configuration into the settings form.
  Future<void> _load() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    widget.onBusyChanged?.call(true);
    try {
      final config = await widget.clients.server.runtimeRemoteLinkService
          .localHostConfig();
      if (config == null) {
        throw StateError('Runtime listener preferences were not initialized');
      }
      final capabilities = await widget.clients.server.runtimeRemoteLinkService
          .listenerCapabilities();
      if (!mounted) return;
      setState(() {
        _capabilities = capabilities;
        _updateFields(config);
        _loaded = true;
      });
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      if (mounted) {
        setState(() => _busy = false);
        widget.onBusyChanged?.call(false);
      }
    }
  }

  /// Persists listener settings, reporting success only after listeners restart.
  Future<bool> _apply() async {
    if (_busy || !_loaded || !widget.enabled) return false;
    final l10n = AppLocalizations.of(context)!;
    final discoverable = _discoverable;
    final activeTransports = _transports.where(_supports).toList();
    final activeDiscovery =
        discoverable && _capabilities!.discoveryAdvertisement;
    if (activeTransports.isEmpty && activeDiscovery) {
      setState(() => _error = l10n.settingsPeerDiscoveryNeedsTransport);
      return false;
    }
    // TCP and the shared HTTP/WS server currently bind the same configured port.
    if (activeTransports.contains(PeerTransport.tcp) &&
        (activeTransports.contains(PeerTransport.http) ||
            activeTransports.contains(PeerTransport.webSocket))) {
      setState(() => _error = l10n.settingsPeerPortConflict);
      return false;
    }
    final host = _bindHost.text.trim().replaceAll(RegExp(r'^\[|\]$'), '');
    final automatic = _portMode == PeerHostPortMode.automatic;
    final portText = _bindPort.text.trim();
    final port = int.tryParse(portText);
    if (host.isEmpty) {
      setState(() => _error = l10n.settingsPeerAddressRequired);
      return false;
    }
    if (port == null || port < (automatic ? 0 : 1) || port > 65535) {
      setState(() => _error = l10n.settingsPeerPortRequired);
      return false;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    widget.onBusyChanged?.call(true);
    final service = widget.clients.server.runtimeRemoteLinkService;
    try {
      final config = PeerHostConfig(
        bindAddress: _composeBindAddress(host, portText),
        transports: _transports.toList(),
        discoveryEnabled: discoverable,
        portMode: _portMode!,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
      await service.stopListening();
      await service.saveLocalHostConfig(config: config);
      if (mounted) setState(() => _updateFields(config));
      if (activeTransports.isNotEmpty) {
        await service.startListening(transports: activeTransports);
      }
      final applied = await service.localHostConfig();
      if (applied == null) {
        throw StateError('Listener configuration is missing after save');
      }
      if (!mounted) return false;
      setState(() => _updateFields(applied));
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.settingsPeerApplied)));
      return true;
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
      return false;
    } finally {
      if (mounted) {
        setState(() => _busy = false);
        widget.onBusyChanged?.call(false);
      }
    }
  }

  /// Updates form values from the configuration persisted by the runtime.
  void _updateFields(PeerHostConfig config) {
    final parts = _splitBindAddress(config.bindAddress);
    _bindHost.text = parts.host;
    _bindPort.text = parts.port;
    _transports = config.transports.toSet();
    _portMode = config.portMode;
    _discoverable = config.discoveryEnabled;
  }

  /// Splits the persisted SocketAddr into independently editable host and port fields.
  ({String host, String port}) _splitBindAddress(String address) {
    final value = address.trim();
    if (value.startsWith('[')) {
      final close = value.indexOf(']');
      if (close > 0 && close + 1 < value.length && value[close + 1] == ':') {
        return (
          host: value.substring(1, close),
          port: value.substring(close + 2),
        );
      }
    }
    final separator = value.lastIndexOf(':');
    if (separator > 0 && value.indexOf(':') == separator) {
      return (
        host: value.substring(0, separator),
        port: value.substring(separator + 1),
      );
    }
    return (host: value.replaceFirst(RegExp(r'^\[|\]$'), ''), port: '');
  }

  /// Reassembles the Host-compatible SocketAddr after form validation.
  String _composeBindAddress(String host, String port) {
    return host.contains(':') ? '[$host]:$port' : '$host:$port';
  }

  /// Checks one inbound transport against the loaded Host capability declaration.
  bool _supports(PeerTransport transport) {
    final capabilities = _capabilities;
    return capabilities != null && capabilities.transports.contains(transport);
  }

  /// Builds authentication controls and the explicit listener settings form.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final enabled = widget.enabled && _loaded && !_busy;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          dense: true,
          visualDensity: VisualDensity.compact,
          title: Text(l10n.settingsRuntimeEnableDiscovery),
          subtitle: Text(
            _loaded && !_capabilities!.discoveryAdvertisement
                ? l10n.settingsPeerDiscoveryUnavailable
                : l10n.settingsRuntimeEnableDiscoveryDescription,
          ),
          value: _discoverable && _capabilities?.discoveryAdvertisement == true,
          onChanged: enabled && _capabilities!.discoveryAdvertisement
              ? (value) => setState(() => _discoverable = value)
              : null,
        ),
        ExpansionTile(
          dense: true,
          tilePadding: EdgeInsets.zero,
          shape: const Border(),
          collapsedShape: const Border(),
          leading: const Icon(Icons.tune_outlined, size: 20),
          title: Text(l10n.settingsPeerAdvanced),
          children: [
            Padding(
              padding: const EdgeInsets.only(top: 8, bottom: 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    l10n.settingsPeerTransportHelp,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      for (final entry in {
                        PeerTransport.tcp: 'TCP',
                        PeerTransport.bluetooth: 'Bluetooth',
                        PeerTransport.serial: 'Serial',
                        PeerTransport.http: 'HTTP',
                        PeerTransport.webSocket: 'WebSocket',
                      }.entries)
                        FilterChip(
                          visualDensity: VisualDensity.compact,
                          label: Text(entry.value),
                          tooltip: _supports(entry.key)
                              ? null
                              : l10n.settingsPeerTransportUnavailable(
                                  entry.value,
                                ),
                          selected: _transports.contains(entry.key),
                          onSelected: enabled && _supports(entry.key)
                              ? (selected) => setState(() {
                                  if (selected) {
                                    _transports.add(entry.key);
                                  } else {
                                    _transports.remove(entry.key);
                                  }
                                })
                              : null,
                        ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  OperitFormStyles.dropdownButtonFormField<PeerHostPortMode>(
                    context,
                    initialValue: _portMode,
                    isExpanded: true,
                    items: [
                      DropdownMenuItem(
                        value: PeerHostPortMode.automatic,
                        child: Text(l10n.settingsPeerPortModeAutomatic),
                      ),
                      DropdownMenuItem(
                        value: PeerHostPortMode.fixed,
                        child: Text(l10n.settingsPeerPortModeFixed),
                      ),
                    ],
                    onChanged: enabled
                        ? (value) {
                            if (value != null) {
                              setState(() {
                                _portMode = value;
                                if (value == PeerHostPortMode.automatic) {
                                  _bindPort.text = '0';
                                }
                              });
                            }
                          }
                        : null,
                    decoration: InputDecoration(
                      labelText: l10n.settingsPeerPortMode,
                      helperText: l10n.settingsPeerPortModeHelp,
                      helperMaxLines: 3,
                      isDense: true,
                      border: const OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 12),
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        flex: 3,
                        child: TextField(
                          key: const ValueKey('peer-bind-host'),
                          controller: _bindHost,
                          enabled: enabled,
                          decoration: InputDecoration(
                            labelText: l10n.settingsPeerBindAddress,
                            helperText: l10n.settingsPeerBindAddressHelp,
                            helperMaxLines: 2,
                            isDense: true,
                            border: const OutlineInputBorder(),
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        flex: 2,
                        child: TextField(
                          key: const ValueKey('peer-bind-port'),
                          controller: _bindPort,
                          enabled:
                              enabled &&
                              _portMode != PeerHostPortMode.automatic,
                          keyboardType: TextInputType.number,
                          inputFormatters: [
                            FilteringTextInputFormatter.digitsOnly,
                          ],
                          decoration: InputDecoration(
                            labelText: l10n.settingsPeerBindPort,
                            helperText: _portMode == PeerHostPortMode.automatic
                                ? l10n.settingsPeerBindPortAutomaticHelp
                                : l10n.settingsPeerBindPortHelp,
                            helperMaxLines: 2,
                            isDense: true,
                            border: const OutlineInputBorder(),
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
        if (_busy) const LinearProgressIndicator(),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.all(16),
            child: SelectableText(
              _error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        if (!_loaded && !_busy)
          TextButton(onPressed: _load, child: Text(l10n.retry)),
      ],
    );
  }
}
