// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitFormStyles.dart';
import 'SpaceIdentityLabels.dart';

/// Shows the local identity and the administrator controls it unlocks.
///
/// The panel owns identities, assignment, and audit only: devices and their
/// commands live in the device section, so no device list is duplicated here.
class NetworkControlPanel extends StatefulWidget {
  /// Creates the identity and permission panel over the generated runtime clients.
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

/// Loads the synchronized control state visible to the current identity.
class _NetworkControlPanelState extends State<NetworkControlPanel> {
  generated.NetworkControlState? _state;
  generated.RuntimeDeviceSpaceTopology? _topology;
  List<generated.NetworkControlAuditRecord>? _audit;
  String? _error;

  /// Starts the first state load.
  @override
  void initState() {
    super.initState();
    unawaited(_reload());
  }

  /// Reloads the panel and the device projection it depends on.
  Future<void> _reloadSettings() async {
    await _reload();
    await widget.onChanged();
  }

  /// Reads the current synchronized state, projection, and allowed audit.
  Future<void> _reload() async {
    try {
      final results = await Future.wait<Object>(<Future<Object>>[
        widget.clients.server.runtimeRemoteLinkService.deviceSpaceControl(),
        widget.clients.server.runtimeRemoteLinkService.deviceSpaceTopology(),
      ]);
      final topology = results[1] as generated.RuntimeDeviceSpaceTopology;
      final audit = _hasCapability(topology, 'network.audit.read')
          ? await widget.clients.server.runtimeRemoteLinkService
                .deviceSpaceControlAudit()
          : null;
      if (!mounted) {
        return;
      }
      setState(() {
        _state = results[0] as generated.NetworkControlState;
        _topology = topology;
        _audit = audit;
        _error = null;
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = error.toString());
      }
    }
  }

  /// Builds the panel sections the current identity is allowed to see.
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
            Expanded(
              child: Text(
                l10n.settingsRuntimeNetworkControlDescription,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            IconButton(
              tooltip: MaterialLocalizations.of(
                context,
              ).refreshIndicatorSemanticLabel,
              onPressed: _reloadSettings,
              icon: const Icon(Icons.refresh_outlined),
            ),
          ],
        ),
        if (state == null || topology == null)
          const SizedBox(
            height: 36,
            child: Align(
              alignment: Alignment.centerLeft,
              child: M3LoadingIndicator(size: 18),
            ),
          )
        else
          ..._sections(context, l10n, state, topology),
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

  /// Builds the identity, assignment, and audit sections of the panel.
  List<Widget> _sections(
    BuildContext context,
    AppLocalizations l10n,
    generated.NetworkControlState state,
    generated.RuntimeDeviceSpaceTopology topology,
  ) {
    final canManageIdentities = _hasCapability(
      topology,
      'network.identity.manage',
    );
    final canAssignIdentities = _hasCapability(
      topology,
      'network.identity.assign',
    );
    final canReadAudit = _hasCapability(topology, 'network.audit.read');
    return <Widget>[
      _selfIdentity(l10n, topology),
      if (canManageIdentities) ..._identityDefinitions(context, l10n, state),
      if (canAssignIdentities) ...<Widget>[
        const SizedBox(height: 12),
        Align(
          alignment: Alignment.centerLeft,
          child: FilledButton.icon(
            onPressed: () => _assignIdentity(l10n),
            icon: const Icon(Icons.assignment_ind_outlined),
            label: Text(l10n.settingsRuntimeControlAssignIdentity),
          ),
        ),
      ],
      if (canReadAudit) ..._auditSection(context, l10n),
    ];
  }

  /// Shows what the current identity is and what it may do.
  ///
  /// This is the only identity information every member owns; devices show the
  /// same readout for their own identities in the device section.
  Widget _selfIdentity(
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceTopology topology,
  ) {
    final identity = _localIdentityOf(topology);
    final capabilities = identity == null
        ? const <String>[]
        : spaceCapabilityValues(identity.capabilities)
              .map((value) => spaceCapabilityLabel(value, l10n))
              .toList(growable: false);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text(
          '${l10n.settingsRuntimeControlCurrentIdentity}: ${identity == null ? l10n.settingsRuntimeControlNoIdentity : spaceIdentityDisplayName(identity.displayName, l10n)}',
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w700),
        ),
        if (capabilities.isNotEmpty) ...<Widget>[
          const SizedBox(height: 2),
          Text(
            '${l10n.settingsRuntimeControlCurrentCapabilities}: ${capabilities.join('、')}',
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ],
    );
  }

  /// Builds the identity definitions owned by administrators.
  List<Widget> _identityDefinitions(
    BuildContext context,
    AppLocalizations l10n,
    generated.NetworkControlState state,
  ) {
    return <Widget>[
      const SizedBox(height: 14),
      Row(
        children: <Widget>[
          Expanded(
            child: Text(
              l10n.settingsRuntimeControlIdentityDefinitions,
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
            ),
          ),
          FilledButton.tonalIcon(
            onPressed: () => _defineIdentity(l10n),
            icon: const Icon(Icons.add_outlined, size: 18),
            label: Text(l10n.settingsRuntimeControlAddRole),
          ),
        ],
      ),
      const SizedBox(height: 8),
      for (final role in state.roles.values) _identityCard(context, role, l10n),
    ];
  }

  /// Builds one identity definition card.
  Widget _identityCard(
    BuildContext context,
    generated.NetworkControlRole role,
    AppLocalizations l10n,
  ) {
    final capabilities = spaceCapabilityValues(
      role.capabilities,
    ).map((value) => spaceCapabilityLabel(value, l10n)).join('、');
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        leading: const Icon(Icons.badge_outlined),
        title: Text(spaceIdentityDisplayName(role.displayName, l10n)),
        subtitle: Text(
          '${l10n.settingsRuntimeControlCurrentCapabilities}: $capabilities',
        ),
      ),
    );
  }

  /// Builds the administrator audit view for authorization history.
  List<Widget> _auditSection(BuildContext context, AppLocalizations l10n) {
    final records = _audit;
    return <Widget>[
      const SizedBox(height: 14),
      Text(
        l10n.settingsRuntimeControlAudit,
        style: Theme.of(
          context,
        ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
      ),
      const SizedBox(height: 8),
      if (records == null)
        const SizedBox(
          height: 24,
          child: Align(
            alignment: Alignment.centerLeft,
            child: M3LoadingIndicator(size: 18),
          ),
        )
      else if (records.isEmpty)
        Text(
          l10n.settingsRuntimeControlNoAudit,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        )
      else
        for (final record in records) _auditCard(context, l10n, record),
    ];
  }

  /// Builds one authorization history entry.
  Widget _auditCard(
    BuildContext context,
    AppLocalizations l10n,
    generated.NetworkControlAuditRecord record,
  ) {
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
  }

  /// Opens the administrator identity definition editor.
  Future<void> _defineIdentity(AppLocalizations l10n) async {
    final topology = _topology;
    if (topology == null) {
      return;
    }
    final result = await _IdentityEditor.show(
      context,
      title: l10n.settingsRuntimeControlAddRole,
      choices: spaceCapabilityChoices(l10n)
          .where((choice) => _hasCapability(topology, choice.value))
          .toList(growable: false),
    );
    if (result == null) {
      return;
    }
    await _runCommand(
      () => widget.clients.server.runtimeRemoteLinkService
          .defineDeviceSpaceRole(
            role: generated.NetworkControlRole(
              roleId: _generatedControlId('identity'),
              displayName: result.name,
              capabilities: result.capabilities,
            ),
          ),
    );
  }

  /// Opens the administrator assignment editor for one device identity.
  Future<void> _assignIdentity(AppLocalizations l10n) async {
    final state = _state;
    final topology = _topology;
    if (state == null || topology == null) {
      return;
    }
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
            (role) => spaceCapabilityValues(
              role.capabilities,
            ).every((capability) => _hasCapability(topology, capability)),
          )
          .map(
            (role) => _Choice(
              label: spaceIdentityDisplayName(role.displayName, l10n),
              value: role.roleId,
            ),
          )
          .toList(growable: false),
    );
    if (result == null) {
      return;
    }
    await _runCommand(
      () => widget.clients.server.runtimeRemoteLinkService
          .setDeviceSpaceIdentity(
            assignment: generated.NetworkControlIdentityAssignment(
              nodeId: result.deviceId,
              roleId: result.identityId,
            ),
          ),
      feedback: l10n.settingsRuntimeControlIdentityAssignment,
    );
  }

  /// Runs one administrator command and refreshes the synchronized state.
  Future<void> _runCommand(
    Future<void> Function() command, {
    String? feedback,
  }) async {
    try {
      await command();
      await _reload();
      await widget.onChanged();
      if (mounted && feedback != null) {
        _showFeedback(feedback);
      }
    } catch (error) {
      if (mounted) {
        _showFeedback(error.toString(), failed: true);
      }
    }
  }

  /// Surfaces one outcome with the same feedback channel for every command.
  void _showFeedback(String message, {bool failed = false}) {
    final scheme = Theme.of(context).colorScheme;
    ScaffoldMessenger.maybeOf(context)?.showSnackBar(
      SnackBar(
        content: Text(
          message,
          style: failed ? TextStyle(color: scheme.onErrorContainer) : null,
        ),
        backgroundColor: failed ? scheme.errorContainer : null,
      ),
    );
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
  final List<SpaceCapabilityChoice> choices;

  /// Opens the editor and returns a validated definition.
  static Future<_IdentityDefinition?> show(
    BuildContext context, {
    required String title,
    required List<SpaceCapabilityChoice> choices,
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

/// Returns the identity of the current device, when the Space assigned one.
generated.RuntimeDeviceSpaceIdentity? _localIdentityOf(
  generated.RuntimeDeviceSpaceTopology topology,
) {
  for (final device in topology.devices) {
    if (device.deviceId == topology.currentDeviceId) {
      return device.currentIdentity;
    }
  }
  return null;
}

/// Returns whether the current device identity owns one capability.
bool _hasCapability(
  generated.RuntimeDeviceSpaceTopology topology,
  String capability,
) {
  final identity = _localIdentityOf(topology);
  return identity != null &&
      (identity.capabilities.contains('*') ||
          identity.capabilities.contains(capability));
}

/// Generates an internal assignment identifier for the replicated command.
String _generatedControlId(String prefix) {
  return '$prefix-${DateTime.now().microsecondsSinceEpoch}';
}
