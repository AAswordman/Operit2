// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitFormStyles.dart';

/// Exposes the current device-to-identity state and administrator controls.
class NetworkControlPanel extends StatefulWidget {
  /// Creates the network identity panel over the generated runtime clients.
  const NetworkControlPanel({
    super.key,
    required this.clients,
    required this.onChanged,
  });

  final GeneratedCoreProxyClients clients;
  final Future<void> Function() onChanged;

  /// Creates the panel state.
  @override
  State<NetworkControlPanel> createState() => _NetworkControlPanelState();
}

/// Loads the current control state and topology for the panel.
class _NetworkControlPanelState extends State<NetworkControlPanel> {
  generated.NetworkControlState? _state;
  generated.RuntimeDeviceSpaceTopology? _topology;
  String? _error;

  /// Starts the first state load.
  @override
  void initState() {
    super.initState();
    unawaited(_reload());
  }

  /// Reads the current synchronized state and device projection.
  Future<void> _reload() async {
    try {
      final results = await Future.wait<Object>(<Future<Object>>[
        widget.clients.server.runtimeRemoteLinkService.deviceSpaceControl(),
        widget.clients.server.runtimeRemoteLinkService.deviceSpaceTopology(),
      ]);
      if (!mounted) {
        return;
      }
      setState(() {
        _state = results[0] as generated.NetworkControlState;
        _topology = results[1] as generated.RuntimeDeviceSpaceTopology;
        _error = null;
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = error.toString());
      }
    }
  }

  /// Builds the compact entry point for the current identity view.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final state = _state;
    final topology = _topology;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Row(
          children: <Widget>[
            const Icon(Icons.admin_panel_settings_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                l10n.settingsRuntimeNetworkControl,
                style: Theme.of(
                  context,
                ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
              ),
            ),
            IconButton(
              tooltip: MaterialLocalizations.of(
                context,
              ).refreshIndicatorSemanticLabel,
              onPressed: _reload,
              icon: const Icon(Icons.refresh_outlined),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Text(
          l10n.settingsRuntimeNetworkControlDescription,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 8),
        if (state == null || topology == null)
          const SizedBox(
            height: 36,
            child: Align(
              alignment: Alignment.centerLeft,
              child: M3LoadingIndicator(size: 18),
            ),
          )
        else
          OutlinedButton.icon(
            onPressed: !state.initialized ? null : _openManager,
            icon: const Icon(Icons.devices_outlined, size: 18),
            label: Text(
              '${topology.devices.length} ${l10n.settingsRuntimeControlDevice}',
            ),
          ),
        if (_error case final error?) ...<Widget>[
          const SizedBox(height: 8),
          Text(
            error,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: Theme.of(context).colorScheme.error,
            ),
          ),
        ],
      ],
    );
  }

  /// Opens the current-state dialog without exposing protocol records.
  Future<void> _openManager() async {
    final state = _state;
    final topology = _topology;
    if (state == null || topology == null) {
      return;
    }
    await _NetworkControlDialog.show(
      context,
      state: state,
      topology: topology,
      clients: widget.clients,
      onChanged: widget.onChanged,
    );
    await _reload();
  }
}

/// Shows current device identities and administrator-only identity controls.
class _NetworkControlDialog extends StatelessWidget {
  /// Creates the current-state dialog.
  const _NetworkControlDialog({
    required this.state,
    required this.topology,
    required this.clients,
    required this.onChanged,
  });

  final generated.NetworkControlState state;
  final generated.RuntimeDeviceSpaceTopology topology;
  final GeneratedCoreProxyClients clients;
  final Future<void> Function() onChanged;

  /// Opens the dialog in the enclosing navigator.
  static Future<void> show(
    BuildContext context, {
    required generated.NetworkControlState state,
    required generated.RuntimeDeviceSpaceTopology topology,
    required GeneratedCoreProxyClients clients,
    required Future<void> Function() onChanged,
  }) {
    return showDialog<void>(
      context: context,
      builder: (_) => _NetworkControlDialog(
        state: state,
        topology: topology,
        clients: clients,
        onChanged: onChanged,
      ),
    );
  }

  /// Builds the tabs visible to the current device identity.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final canManageIdentities = _hasCapability(
      topology,
      'network.identity.manage',
    );
    final canAssignIdentities = _hasCapability(
      topology,
      'network.identity.assign',
    );
    final canReadAudit = _hasCapability(topology, 'network.audit.read');
    final showIdentityTab = canManageIdentities || canAssignIdentities;
    final tabs = <Widget>[
      Tab(text: l10n.settingsRuntimeControlDevice),
      if (showIdentityTab)
        Tab(text: l10n.settingsRuntimeControlManageIdentities),
      if (canReadAudit) Tab(text: l10n.settingsRuntimeControlAudit),
    ];
    return Dialog(
      child: SizedBox(
        width: 760,
        height: 600,
        child: DefaultTabController(
          length: tabs.length,
          child: Column(
            children: <Widget>[
              Padding(
                padding: const EdgeInsets.fromLTRB(20, 12, 8, 0),
                child: Row(
                  children: <Widget>[
                    Expanded(
                      child: Text(
                        l10n.settingsRuntimeNetworkControl,
                        style: Theme.of(context).textTheme.titleMedium
                            ?.copyWith(fontWeight: FontWeight.w800),
                      ),
                    ),
                    IconButton(
                      tooltip: MaterialLocalizations.of(
                        context,
                      ).closeButtonTooltip,
                      onPressed: () => Navigator.of(context).pop(),
                      icon: const Icon(Icons.close_outlined),
                    ),
                  ],
                ),
              ),
              TabBar(tabs: tabs),
              Expanded(
                child: TabBarView(
                  children: <Widget>[
                    _statusTab(context, l10n),
                    if (showIdentityTab)
                      _identityTab(
                        context,
                        l10n,
                        canManageIdentities,
                        canAssignIdentities,
                      ),
                    if (canReadAudit) _auditTab(context, l10n),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Builds the all-device current-state view.
  Widget _statusTab(BuildContext context, AppLocalizations l10n) {
    final devices = <generated.RuntimeDeviceSpaceDevice>[
      ...topology.devices,
      ...topology.removedDevices,
    ];
    final directory = _DeviceDirectory(devices);
    return ListView(
      padding: const EdgeInsets.all(16),
      children: <Widget>[
        for (final device in devices)
          _deviceStatusCard(context, device, directory, l10n),
      ],
    );
  }

  /// Builds one device card around its current identity and effective abilities.
  Widget _deviceStatusCard(
    BuildContext context,
    generated.RuntimeDeviceSpaceDevice device,
    _DeviceDirectory directory,
    AppLocalizations l10n,
  ) {
    final removed = topology.removedDevices.any(
      (candidate) => candidate.deviceId == device.deviceId,
    );
    final identity = device.currentIdentity;
    final capabilityText = identity == null
        ? l10n.settingsRuntimeControlNoIdentity
        : _values(
            identity.capabilities,
          ).map((value) => _capabilityLabel(value, l10n)).join('、');
    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      child: ListTile(
        leading: Icon(
          removed || !device.online
              ? Icons.link_off_outlined
              : Icons.devices_outlined,
          color: removed || !device.online
              ? Theme.of(context).colorScheme.error
              : Theme.of(context).colorScheme.primary,
        ),
        title: Text(directory.label(device.deviceId)),
        subtitle: Text(
          '${l10n.settingsRuntimeControlDeviceId}: ${directory.id(device.deviceId)}\n'
          '${l10n.settingsRuntimeControlCurrentIdentity}: ${identity == null ? l10n.settingsRuntimeControlNoIdentity : _identityDisplayName(identity.displayName, l10n)}\n'
          '${l10n.settingsRuntimeControlCurrentCapabilities}: $capabilityText\n'
          '${removed ? l10n.settingsRuntimeControlRemoved : (device.online ? l10n.settingsRuntimeControlOnline : l10n.settingsRuntimeControlOffline)}',
        ),
        trailing: _deviceActions(context, device, removed, l10n),
      ),
    );
  }

  /// Builds administrator-only device membership actions.
  Widget? _deviceActions(
    BuildContext context,
    generated.RuntimeDeviceSpaceDevice device,
    bool removed,
    AppLocalizations l10n,
  ) {
    if (removed) {
      if (!_hasCapability(topology, 'network.members.join')) {
        return null;
      }
      return IconButton(
        tooltip: l10n.settingsRuntimeControlAdmitDevice,
        onPressed: () => _commit(
          context,
          () => clients.server.runtimeRemoteLinkService.admitDeviceSpaceMember(
            deviceId: device.deviceId,
          ),
        ),
        icon: const Icon(Icons.person_add_alt_1_outlined),
      );
    }
    final actions = <Widget>[
      if (device.currentIdentity != null &&
          _hasCapability(topology, 'network.identity.manage'))
        MenuItemButton(
          onPressed: () => _commit(
            context,
            () => clients.server.runtimeRemoteLinkService
                .clearDeviceSpaceIdentity(nodeId: device.deviceId),
          ),
          child: Text(l10n.settingsRuntimeControlClearIdentity),
        ),
      if (device.deviceId != topology.currentDeviceId &&
          _hasCapability(topology, 'network.connections.disconnect'))
        MenuItemButton(
          onPressed: () => _commit(
            context,
            () => clients.server.runtimeRemoteLinkService
                .disconnectDeviceSpaceNode(deviceId: device.deviceId),
          ),
          child: Text(l10n.settingsRuntimeControlDisconnectDevice),
        ),
      if (device.deviceId != topology.currentDeviceId &&
          _hasCapability(topology, 'network.members.remove'))
        MenuItemButton(
          onPressed: () => _commit(
            context,
            () => clients.server.runtimeRemoteLinkService
                .removeDeviceSpaceMember(deviceId: device.deviceId),
          ),
          child: Text(l10n.settingsRuntimeControlRemoveDevice),
        ),
    ];
    if (actions.isEmpty) {
      return null;
    }
    return MenuAnchor(
      builder: (context, controller, _) => IconButton(
        tooltip: l10n.settingsRuntimeControlDevice,
        onPressed: controller.open,
        icon: const Icon(Icons.more_vert_outlined),
      ),
      menuChildren: actions,
    );
  }

  /// Builds identity controls permitted by the current device identity.
  Widget _identityTab(
    BuildContext context,
    AppLocalizations l10n,
    bool canManageIdentities,
    bool canAssignIdentities,
  ) {
    final roles = state.roles.values.toList(growable: false);
    return ListView(
      padding: const EdgeInsets.all(16),
      children: <Widget>[
        if (canManageIdentities) ...<Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  l10n.settingsRuntimeControlIdentityDefinitions,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              FilledButton.icon(
                onPressed: () => _defineIdentity(context, l10n),
                icon: const Icon(Icons.add_outlined, size: 18),
                label: Text(l10n.settingsRuntimeControlAddRole),
              ),
            ],
          ),
          const SizedBox(height: 10),
          for (final role in roles) _identityCard(context, role, l10n),
        ],
        if (canManageIdentities && canAssignIdentities)
          const SizedBox(height: 20),
        if (canAssignIdentities)
          FilledButton.icon(
            onPressed: () => _assignIdentity(context, l10n),
            icon: const Icon(Icons.assignment_ind_outlined),
            label: Text(l10n.settingsRuntimeControlAssignIdentity),
          ),
      ],
    );
  }

  /// Builds one editable identity definition card.
  Widget _identityCard(
    BuildContext context,
    generated.NetworkControlRole role,
    AppLocalizations l10n,
  ) {
    final capabilities = _values(
      role.capabilities,
    ).map((value) => _capabilityLabel(value, l10n)).join('、');
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        leading: const Icon(Icons.badge_outlined),
        title: Text(_identityDisplayName(role.displayName, l10n)),
        subtitle: Text(
          '${l10n.settingsRuntimeControlCurrentCapabilities}: $capabilities',
        ),
      ),
    );
  }

  /// Builds the administrator audit view for authorization history.
  Widget _auditTab(BuildContext context, AppLocalizations l10n) {
    return FutureBuilder<List<generated.NetworkControlAuditRecord>>(
      future: clients.server.runtimeRemoteLinkService.deviceSpaceControlAudit(),
      builder: (context, snapshot) {
        if (!snapshot.hasData) {
          return const Center(child: M3LoadingIndicator(size: 20));
        }
        final records = snapshot.data!;
        if (records.isEmpty) {
          return Center(child: Text(l10n.settingsRuntimeControlNoAudit));
        }
        return ListView.builder(
          padding: const EdgeInsets.all(16),
          itemCount: records.length,
          itemBuilder: (context, index) {
            final record = records[index];
            final accepted = record.accepted;
            return Card(
              margin: const EdgeInsets.only(bottom: 8),
              child: ListTile(
                leading: Icon(
                  accepted ? Icons.verified_outlined : Icons.error_outline,
                  color: accepted
                      ? Theme.of(context).colorScheme.primary
                      : Theme.of(context).colorScheme.error,
                ),
                title: Text(record.summary),
                subtitle: Text(
                  '${accepted ? l10n.settingsRuntimeControlGranted : l10n.settingsRuntimeControlRevoked} · ${record.reason}',
                ),
              ),
            );
          },
        );
      },
    );
  }

  /// Opens the administrator identity definition editor.
  Future<void> _defineIdentity(
    BuildContext context,
    AppLocalizations l10n,
  ) async {
    final result = await _IdentityEditor.show(
      context,
      title: l10n.settingsRuntimeControlAddRole,
      choices: _capabilityChoices(l10n)
          .where((choice) => _hasCapability(topology, choice.value))
          .toList(growable: false),
    );
    if (result == null) {
      return;
    }
    if (!context.mounted) {
      return;
    }
    await _commit(
      context,
      () => clients.server.runtimeRemoteLinkService.defineDeviceSpaceRole(
        role: generated.NetworkControlRole(
          roleId: _generatedControlId('identity'),
          displayName: result.name,
          capabilities: result.capabilities,
        ),
      ),
    );
  }

  /// Opens the administrator assignment editor for one device identity.
  Future<void> _assignIdentity(
    BuildContext context,
    AppLocalizations l10n,
  ) async {
    final directory = _DeviceDirectory(topology.devices);
    final result = await _IdentityAssignmentEditor.show(
      context,
      title: l10n.settingsRuntimeControlAssignIdentity,
      devices: topology.devices
          .map(
            (device) => _Choice(
              label: directory.optionLabel(device.deviceId),
              value: device.deviceId,
            ),
          )
          .toList(growable: false),
      identities: state.roles.values
          .where(
            (role) => _values(
              role.capabilities,
            ).every((capability) => _hasCapability(topology, capability)),
          )
          .map(
            (role) => _Choice(
              label: _identityDisplayName(role.displayName, l10n),
              value: role.roleId,
            ),
          )
          .toList(growable: false),
    );
    if (result == null) {
      return;
    }
    if (!context.mounted) {
      return;
    }
    await _commit(
      context,
      () => clients.server.runtimeRemoteLinkService.setDeviceSpaceIdentity(
        assignment: generated.NetworkControlIdentityAssignment(
          nodeId: result.deviceId,
          roleId: result.identityId,
        ),
      ),
    );
  }

  /// Executes one administrator command and closes the stale snapshot dialog.
  Future<void> _commit(
    BuildContext context,
    Future<void> Function() command,
  ) async {
    try {
      await command();
      await onChanged();
      if (context.mounted) {
        Navigator.of(context).pop();
      }
    } catch (error) {
      if (context.mounted) {
        await showDialog<void>(
          context: context,
          builder: (errorContext) => AlertDialog(
            title: Text(
              MaterialLocalizations.of(errorContext).alertDialogLabel,
            ),
            content: Text(error.toString()),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(errorContext).pop(),
                child: Text(
                  MaterialLocalizations.of(errorContext).okButtonLabel,
                ),
              ),
            ],
          ),
        );
      }
    }
  }
}

/// Represents one human-facing dropdown option and its internal value.
class _Choice {
  /// Creates one choice.
  const _Choice({required this.label, required this.value});

  final String label;
  final String value;
}

/// Represents the submitted identity definition form.
class _IdentityDefinition {
  /// Creates one identity definition result.
  const _IdentityDefinition({required this.name, required this.capabilities});

  final String name;
  final List<String> capabilities;
}

/// Represents the submitted device identity assignment.
class _IdentityAssignment {
  /// Creates one identity assignment result.
  const _IdentityAssignment({required this.deviceId, required this.identityId});

  final String deviceId;
  final String identityId;
}

/// Edits an identity name and its human-readable capabilities.
class _IdentityEditor extends StatefulWidget {
  /// Creates the identity editor.
  const _IdentityEditor({required this.title, required this.choices});

  final String title;
  final List<_Choice> choices;

  /// Opens the editor and returns a validated definition.
  static Future<_IdentityDefinition?> show(
    BuildContext context, {
    required String title,
    required List<_Choice> choices,
  }) {
    return showDialog<_IdentityDefinition>(
      context: context,
      builder: (_) => _IdentityEditor(title: title, choices: choices),
    );
  }

  /// Creates the editor state.
  @override
  State<_IdentityEditor> createState() => _IdentityEditorState();
}

/// Owns the identity editor controls.
class _IdentityEditorState extends State<_IdentityEditor> {
  late final TextEditingController _name;
  late final Set<String> _selected;

  /// Allocates the editor controls.
  @override
  void initState() {
    super.initState();
    _name = TextEditingController();
    _selected = <String>{};
  }

  /// Releases the editor controls.
  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  /// Submits the identity definition when both name and capabilities exist.
  void _submit() {
    final name = _name.text.trim();
    if (name.isEmpty || _selected.isEmpty) {
      return;
    }
    Navigator.of(context).pop(
      _IdentityDefinition(
        name: name,
        capabilities: _selected.toList(growable: false),
      ),
    );
  }

  /// Builds the identity editor dialog.
  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              TextField(
                controller: _name,
                autofocus: true,
                decoration: InputDecoration(
                  labelText: AppLocalizations.of(
                    context,
                  )!.settingsRuntimeControlIdentityName,
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              for (final choice in widget.choices)
                CheckboxListTile(
                  dense: true,
                  value: _selected.contains(choice.value),
                  title: Text(choice.label),
                  onChanged: (value) => setState(() {
                    if (value == true) {
                      if (choice.value == '*') {
                        _selected
                          ..clear()
                          ..add(choice.value);
                      } else {
                        _selected
                          ..remove('*')
                          ..add(choice.value);
                      }
                    } else {
                      _selected.remove(choice.value);
                    }
                  }),
                ),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
        ),
        FilledButton(
          onPressed: _submit,
          child: Text(MaterialLocalizations.of(context).okButtonLabel),
        ),
      ],
    );
  }
}

/// Selects one device and one identity without exposing internal identifiers.
class _IdentityAssignmentEditor extends StatefulWidget {
  /// Creates the assignment editor.
  const _IdentityAssignmentEditor({
    required this.title,
    required this.devices,
    required this.identities,
  });

  final String title;
  final List<_Choice> devices;
  final List<_Choice> identities;

  /// Opens the assignment editor.
  static Future<_IdentityAssignment?> show(
    BuildContext context, {
    required String title,
    required List<_Choice> devices,
    required List<_Choice> identities,
  }) {
    return showDialog<_IdentityAssignment>(
      context: context,
      builder: (_) => _IdentityAssignmentEditor(
        title: title,
        devices: devices,
        identities: identities,
      ),
    );
  }

  /// Creates the assignment editor state.
  @override
  State<_IdentityAssignmentEditor> createState() =>
      _IdentityAssignmentEditorState();
}

/// Owns the assignment dropdown selections.
class _IdentityAssignmentEditorState extends State<_IdentityAssignmentEditor> {
  String? _deviceId;
  String? _identityId;

  /// Submits a complete device identity assignment.
  void _submit() {
    final deviceId = _deviceId;
    final identityId = _identityId;
    if (deviceId == null || identityId == null) {
      return;
    }
    Navigator.of(
      context,
    ).pop(_IdentityAssignment(deviceId: deviceId, identityId: identityId));
  }

  /// Builds the assignment dialog.
  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 420,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            OperitFormStyles.dropdownButtonFormField<String>(
              context,
              initialValue: _deviceId,
              isExpanded: true,
              decoration: InputDecoration(
                labelText: AppLocalizations.of(
                  context,
                )!.settingsRuntimeControlDevice,
                border: OutlineInputBorder(),
              ),
              items: widget.devices
                  .map(
                    (choice) => DropdownMenuItem<String>(
                      value: choice.value,
                      child: Text(
                        choice.label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  )
                  .toList(growable: false),
              onChanged: (value) => setState(() => _deviceId = value),
            ),
            const SizedBox(height: 12),
            OperitFormStyles.dropdownButtonFormField<String>(
              context,
              initialValue: _identityId,
              isExpanded: true,
              decoration: InputDecoration(
                labelText: AppLocalizations.of(
                  context,
                )!.settingsRuntimeControlIdentities,
                border: OutlineInputBorder(),
              ),
              items: widget.identities
                  .map(
                    (choice) => DropdownMenuItem<String>(
                      value: choice.value,
                      child: Text(
                        choice.label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  )
                  .toList(growable: false),
              onChanged: (value) => setState(() => _identityId = value),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
        ),
        FilledButton(
          onPressed: _submit,
          child: Text(MaterialLocalizations.of(context).okButtonLabel),
        ),
      ],
    );
  }
}

/// Maps device identifiers to readable labels while preserving duplicate names.
class _DeviceDirectory {
  /// Builds the directory from one device snapshot.
  _DeviceDirectory(Iterable<generated.RuntimeDeviceSpaceDevice> devices) {
    final counts = <String, int>{};
    final occurrences = <String, int>{};
    final values = devices.toList(growable: false);
    for (final device in values) {
      final base = _deviceBaseName(device);
      counts[base] = (counts[base] ?? 0) + 1;
    }
    for (final device in values) {
      final base = _deviceBaseName(device);
      final occurrence = (occurrences[base] ?? 0) + 1;
      occurrences[base] = occurrence;
      _labels[device.deviceId] = counts[base] == 1
          ? base
          : '$base · 设备 $occurrence';
      _ids[device.deviceId] = device.deviceId;
    }
  }

  final Map<String, String> _labels = <String, String>{};
  final Map<String, String> _ids = <String, String>{};

  /// Returns the readable label for one device.
  String label(String deviceId) => _labels[deviceId]!;

  /// Returns the stable runtime identifier shown beside one device name.
  String id(String deviceId) => _ids[deviceId]!;

  /// Returns a selection label that disambiguates duplicate device names.
  String optionLabel(String deviceId) => '${label(deviceId)} (${id(deviceId)})';
}

/// Builds a readable device label without protocol identifiers.
String _deviceBaseName(generated.RuntimeDeviceSpaceDevice device) {
  return <String>[
    device.deviceName,
    if (device.userName.trim().isNotEmpty) device.userName,
    if (device.platform.trim().isNotEmpty) device.platform,
    if (device.model.trim().isNotEmpty) device.model,
  ].join(' · ');
}

/// Returns whether the current device identity owns one capability.
bool _hasCapability(
  generated.RuntimeDeviceSpaceTopology topology,
  String capability,
) {
  final device = topology.devices.firstWhere(
    (candidate) => candidate.deviceId == topology.currentDeviceId,
  );
  final identity = device.currentIdentity;
  return identity != null &&
      (identity.capabilities.contains('*') ||
          identity.capabilities.contains(capability));
}

/// Converts built-in identity names into localized labels.
String _identityDisplayName(String name, AppLocalizations l10n) {
  return switch (name) {
    'Administrator' => l10n.settingsRuntimeControlRoleAdministrator,
    'User' => l10n.settingsRuntimeControlRoleUser,
    'Relay' => l10n.settingsRuntimeControlRoleRelay,
    'Storage' => l10n.settingsRuntimeControlRoleStorage,
    'Runner' => l10n.settingsRuntimeControlRoleRunner,
    'Auditor' => l10n.settingsRuntimeControlRoleAuditor,
    _ => name,
  };
}

/// Converts one protocol capability into a readable label.
String _capabilityLabel(String capability, AppLocalizations l10n) {
  return switch (capability) {
    '*' => l10n.settingsRuntimeControlCapabilityAll,
    'network.devices.view' => l10n.settingsRuntimeControlCapabilityViewDevices,
    'network.audit.read' => l10n.settingsRuntimeControlCapabilityAuditRead,
    'network.relay' => l10n.settingsRuntimeControlCapabilityNetworkRelay,
    'storage.provide' => l10n.settingsRuntimeControlCapabilityStorageProvide,
    'runtime.execute' => l10n.settingsRuntimeControlCapabilityRuntimeExecute,
    'network.user' => l10n.settingsRuntimeControlCapabilityNetworkUser,
    'network.identity.manage' => l10n.settingsRuntimeControlManageIdentities,
    'network.identity.assign' => l10n.settingsRuntimeControlAssignIdentity,
    'network.approval' => l10n.settingsRuntimeControlCapabilityApproval,
    _ => capability.replaceAll('.', ' · '),
  };
}

/// Returns the selectable identity capabilities exposed to administrators.
List<_Choice> _capabilityChoices(AppLocalizations l10n) {
  return <_Choice>[
    _Choice(label: l10n.settingsRuntimeControlCapabilityAll, value: '*'),
    _Choice(
      label: l10n.settingsRuntimeControlCapabilityViewDevices,
      value: 'network.devices.view',
    ),
    _Choice(
      label: l10n.settingsRuntimeControlCapabilityAuditRead,
      value: 'network.audit.read',
    ),
    _Choice(
      label: l10n.settingsRuntimeControlCapabilityNetworkRelay,
      value: 'network.relay',
    ),
    _Choice(
      label: l10n.settingsRuntimeControlCapabilityStorageProvide,
      value: 'storage.provide',
    ),
    _Choice(
      label: l10n.settingsRuntimeControlCapabilityRuntimeExecute,
      value: 'runtime.execute',
    ),
    _Choice(
      label: l10n.settingsRuntimeControlManageIdentities,
      value: 'network.identity.manage',
    ),
    _Choice(
      label: l10n.settingsRuntimeControlAssignIdentity,
      value: 'network.identity.assign',
    ),
    _Choice(
      label: l10n.settingsRuntimeControlCapabilityApproval,
      value: 'network.approval',
    ),
  ];
}

/// Converts a generated collection into stable string values.
List<String> _values(Object? values) {
  if (values is Iterable<Object?>) {
    return values.map((value) => value.toString()).toList(growable: false);
  }
  throw StateError('network identity capabilities must be iterable');
}

/// Generates an internal assignment identifier for the replicated command.
String _generatedControlId(String prefix) {
  return '$prefix-${DateTime.now().microsecondsSinceEpoch}';
}
