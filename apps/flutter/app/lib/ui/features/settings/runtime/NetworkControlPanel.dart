// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../components/SettingsControlStyles.dart';
import 'SpaceAuditLabels.dart';
import 'SpaceIdentityLabels.dart';

/// Shows the local identity and the administrator controls it unlocks.
///
/// The panel owns identity definitions and audit only: devices and their
/// commands - identity assignment included - live in the device section, so
/// no device list is duplicated here.
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
        if (state == null || topology == null)
          Row(
            children: <Widget>[
              if (_error == null)
                const Expanded(
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: M3LoadingIndicator(size: 18),
                  ),
                )
              else
                const Spacer(),
              IconButton(
                tooltip: MaterialLocalizations.of(
                  context,
                ).refreshIndicatorSemanticLabel,
                style: SettingsControlStyles.entityIconButton(),
                onPressed: _reloadSettings,
                icon: const Icon(Icons.refresh_outlined),
              ),
            ],
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
    final canReadAudit = _hasCapability(topology, 'network.audit.read');
    return <Widget>[
      _selfIdentity(l10n, topology),
      if (canManageIdentities)
        _CollapsibleSection(
          title: l10n.settingsRuntimeControlIdentityDefinitions,
          summary: l10n.settingsRuntimeControlIdentityCount(state.roles.length),
          action: TextButton.icon(
            style: SettingsControlStyles.sectionTextButton(),
            onPressed: () => _defineIdentity(l10n),
            icon: const Icon(Icons.add_outlined, size: 16),
            label: Text(l10n.settingsRuntimeControlAddRole),
          ),
          child: _FlatList(
            children: <Widget>[
              for (final role in state.roles.values)
                _identityRow(context, role, l10n),
            ],
          ),
        ),
      if (canReadAudit)
        _CollapsibleSection(
          title: l10n.settingsRuntimeControlAudit,
          summary: l10n.settingsRuntimeControlAuditCount(_audit?.length ?? 0),
          child: _auditRecords(context, l10n, topology),
        ),
    ];
  }

  /// Gives the local role one clear headline, with individually readable grants.
  Widget _selfIdentity(
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceTopology topology,
  ) {
    final theme = Theme.of(context);
    final identity = _localIdentityOf(topology);
    final grants = identity == null
        ? const <String>[]
        : spaceCapabilityValues(identity.capabilities);
    final allPermissions = grants.contains('*');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Row(
          key: const ValueKey('space-local-identity-header'),
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Expanded(
              child: Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 8,
                runSpacing: 4,
                children: <Widget>[
                  Icon(
                    Icons.verified_user_outlined,
                    size: 18,
                    color: theme.colorScheme.primary,
                  ),
                  Text(
                    l10n.settingsRuntimeControlLocalIdentity,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  Text(
                    identity == null
                        ? l10n.settingsRuntimeControlNoIdentity
                        : spaceIdentityDisplayName(identity.displayName, l10n),
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  if (allPermissions)
                    _SmallLabel(
                      label: l10n.settingsRuntimeControlCapabilityAll,
                    ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            IconButton(
              key: const ValueKey('space-identity-refresh'),
              tooltip: MaterialLocalizations.of(
                context,
              ).refreshIndicatorSemanticLabel,
              style: SettingsControlStyles.entityIconButton(),
              onPressed: _reloadSettings,
              icon: const Icon(Icons.refresh_outlined),
            ),
          ],
        ),
        if (grants.isNotEmpty && !allPermissions) ...<Widget>[
          const SizedBox(height: 6),
          _CapabilityLabels(capabilities: grants),
        ],
      ],
    );
  }

  /// Uses one flat row instead of an oversized elevated card per role.
  Widget _identityRow(
    BuildContext context,
    generated.NetworkControlRole role,
    AppLocalizations l10n,
  ) {
    final topology = _topology;
    final editable =
        topology != null &&
        _hasCapability(topology, 'network.identity.manage') &&
        !_builtinRoleIds.contains(role.roleId);
    final actions = editable
        ? Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              IconButton(
                tooltip: l10n.settingsRuntimeControlEditRole,
                style: SettingsControlStyles.entityIconButton(),
                icon: const Icon(Icons.edit_outlined, size: 20),
                onPressed: () => _editIdentity(role, l10n),
              ),
              IconButton(
                tooltip: l10n.settingsRuntimeControlDeleteRole,
                style: SettingsControlStyles.entityIconButton(),
                icon: const Icon(Icons.delete_outline, size: 20),
                onPressed: () => _deleteIdentity(role, l10n),
              ),
            ],
          )
        : null;
    return Padding(
      key: ValueKey('space-role-${role.roleId}'),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final name = Text(
            spaceIdentityDisplayName(role.displayName, l10n),
            style: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
          );
          final capabilities = _CapabilityLabels(
            capabilities: spaceCapabilityValues(role.capabilities),
          );
          final grantCount = spaceCapabilityValues(role.capabilities).length;
          final textScale = MediaQuery.textScalerOf(context).scale(12) / 12;
          final minimumWidth =
              (grantCount <= 1 ? 240 : 320) * textScale + (editable ? 64 : 0);
          if (constraints.maxWidth >= minimumWidth) {
            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                SizedBox(width: 96 * textScale, child: name),
                const SizedBox(width: 12),
                Expanded(child: capabilities),
                ?actions,
              ],
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Expanded(child: name),
                  ?actions,
                ],
              ),
              const SizedBox(height: 6),
              capabilities,
            ],
          );
        },
      ),
    );
  }

  /// Opens the identity editor pre-filled for updating one existing role.
  Future<void> _editIdentity(
    generated.NetworkControlRole role,
    AppLocalizations l10n,
  ) async {
    final topology = _topology;
    if (topology == null) {
      return;
    }
    final result = await _IdentityEditor.show(
      context,
      title: l10n.settingsRuntimeControlEditRole,
      choices: spaceCapabilityChoices(l10n)
          .where((choice) => _hasCapability(topology, choice.value))
          .toList(growable: false),
      initialName: role.displayName,
      initialCapabilities: spaceCapabilityValues(role.capabilities),
    );
    if (result == null) {
      return;
    }
    await _runCommand(
      () =>
          widget.clients.server.runtimeRemoteLinkService.updateDeviceSpaceRole(
            role: generated.NetworkControlRole(
              roleId: role.roleId,
              displayName: result.name,
              capabilities: result.capabilities,
            ),
          ),
      feedback: l10n.settingsRuntimeControlEditRole,
    );
  }

  /// Confirms and removes one custom role.
  Future<void> _deleteIdentity(
    generated.NetworkControlRole role,
    AppLocalizations l10n,
  ) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(
          l10n.settingsRuntimeControlDeleteRoleTitle(
            spaceIdentityDisplayName(role.displayName, l10n),
          ),
        ),
        content: Text(l10n.settingsRuntimeControlDeleteRoleMessage),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(
              MaterialLocalizations.of(dialogContext).cancelButtonLabel,
            ),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(dialogContext).colorScheme.error,
            ),
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(MaterialLocalizations.of(dialogContext).okButtonLabel),
          ),
        ],
      ),
    );
    if (confirmed != true) {
      return;
    }
    await _runCommand(
      () => widget.clients.server.runtimeRemoteLinkService
          .removeDeviceSpaceRole(roleId: role.roleId),
      feedback: l10n.settingsRuntimeControlDeleteRole,
    );
  }

  /// Keeps history newest-first and bounded, rather than growing the whole page.
  Widget _auditRecords(
    BuildContext context,
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceTopology topology,
  ) {
    final records = _audit;
    if (records == null) {
      return const SizedBox(
        height: 24,
        child: Align(
          alignment: Alignment.centerLeft,
          child: M3LoadingIndicator(size: 18),
        ),
      );
    }
    if (records.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 14),
        child: Row(
          children: <Widget>[
            Icon(
              Icons.history_outlined,
              size: 18,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                l10n.settingsRuntimeControlNoAudit,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ],
        ),
      );
    }
    final sorted = records.toList()
      ..sort((a, b) => b.recordedAt.compareTo(a.recordedAt));
    return Container(
      decoration: _listDecoration(context),
      clipBehavior: Clip.antiAlias,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxHeight: 320),
        child: ListView.separated(
          primary: false,
          shrinkWrap: true,
          padding: EdgeInsets.zero,
          itemCount: sorted.length,
          separatorBuilder: (context, index) => _listDivider(context),
          itemBuilder: (context, index) =>
              _auditRow(context, l10n, sorted[index], topology),
        ),
      ),
    );
  }

  /// Leads with the action and result; protocol details are available on demand.
  Widget _auditRow(
    BuildContext context,
    AppLocalizations l10n,
    generated.NetworkControlAuditRecord record,
    generated.RuntimeDeviceSpaceTopology topology,
  ) {
    final theme = Theme.of(context);
    final summary = spaceAuditSummary(
      record.summary,
      l10n,
      identityNames:
          _state?.roles.values.map((role) => role.displayName) ??
          const <String>[],
    );
    final result = record.accepted
        ? l10n.settingsRuntimeControlAuditAccepted
        : l10n.settingsRuntimeControlAuditRejected;
    final resultColor = record.accepted
        ? theme.colorScheme.primary
        : theme.colorScheme.error;
    final time = _auditTime(record.recordedAt);
    return InkWell(
      key: ValueKey('space-audit-${record.commandId}'),
      onTap: () => _showAuditDetails(l10n, record, topology),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: Icon(
                record.accepted
                    ? Icons.check_circle_outline
                    : Icons.block_outlined,
                size: 18,
                color: resultColor,
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: <Widget>[
                      Text(
                        summary.action,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      _SmallLabel(label: result, color: resultColor),
                    ],
                  ),
                  const SizedBox(height: 3),
                  Wrap(
                    spacing: 10,
                    runSpacing: 3,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: <Widget>[
                      if (summary.target.isNotEmpty)
                        Text(
                          summary.target,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      Text(
                        time,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            Icon(
              Icons.chevron_right,
              size: 18,
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ],
        ),
      ),
    );
  }

  /// Preserves the complete diagnostic record without putting it in the overview.
  Future<void> _showAuditDetails(
    AppLocalizations l10n,
    generated.NetworkControlAuditRecord record,
    generated.RuntimeDeviceSpaceTopology topology,
  ) async {
    final summary = spaceAuditSummary(
      record.summary,
      l10n,
      identityNames:
          _state?.roles.values.map((role) => role.displayName) ??
          const <String>[],
    );
    String deviceLabel(String id) {
      for (final device in topology.devices) {
        if (device.deviceId == id) return device.deviceName;
      }
      return id;
    }

    await showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(summary.action),
        content: SizedBox(
          width: 420,
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                if (summary.target.isNotEmpty) Text(summary.target),
                const SizedBox(height: 12),
                Text(
                  record.accepted
                      ? l10n.settingsRuntimeControlAuditAccepted
                      : l10n.settingsRuntimeControlAuditRejected,
                ),
                Text(_auditTime(record.recordedAt)),
                Text(
                  '${l10n.settingsRuntimeControlAuditIssuer}: ${deviceLabel(record.issuerNodeId)}',
                ),
                const Divider(height: 24),
                Text(
                  l10n.settingsRuntimeControlAuditDetails,
                  style: Theme.of(context).textTheme.labelMedium,
                ),
                const SizedBox(height: 6),
                SelectableText(
                  const JsonEncoder.withIndent('  ').convert(record.toJson()),
                ),
              ],
            ),
          ),
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(MaterialLocalizations.of(context).closeButtonLabel),
          ),
        ],
      ),
    );
  }

  /// Shows an unambiguous local date without exposing epoch timestamps.
  String _auditTime(int milliseconds) {
    final time = DateTime.fromMillisecondsSinceEpoch(milliseconds);
    final localizations = MaterialLocalizations.of(context);
    return '${localizations.formatCompactDate(time)} · ${localizations.formatTimeOfDay(TimeOfDay.fromDateTime(time), alwaysUse24HourFormat: true)}';
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
      () =>
          widget.clients.server.runtimeRemoteLinkService.defineDeviceSpaceRole(
            role: generated.NetworkControlRole(
              roleId: _generatedControlId('identity'),
              displayName: result.name,
              capabilities: result.capabilities,
            ),
          ),
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

/// A one-line counted disclosure, with its action outside the toggle target.
class _CollapsibleSection extends StatelessWidget {
  const _CollapsibleSection({
    required this.title,
    required this.summary,
    required this.child,
    this.action,
  });

  final String title;
  final String summary;
  final Widget child;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _listDivider(context),
          Theme(
            data: theme.copyWith(dividerColor: Colors.transparent),
            child: ExpansionTile(
              tilePadding: EdgeInsets.zero,
              childrenPadding: const EdgeInsets.only(bottom: 4),
              shape: const Border(),
              collapsedShape: const Border(),
              dense: true,
              minTileHeight: 48,
              controlAffinity: ListTileControlAffinity.leading,
              title: Wrap(
                spacing: 8,
                runSpacing: 4,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: <Widget>[
                  Text(
                    title,
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  _SmallLabel(label: summary),
                ],
              ),
              trailing: action ?? const SizedBox.shrink(),
              children: <Widget>[child],
            ),
          ),
        ],
      ),
    );
  }
}

/// Small permission tokens, not repeated prose or oversized interactive chips.
class _CapabilityLabels extends StatelessWidget {
  const _CapabilityLabels({required this.capabilities});
  final List<String> capabilities;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final values = capabilities.contains('*')
        ? const <String>['*']
        : capabilities;
    return Wrap(
      spacing: 5,
      runSpacing: 5,
      children: <Widget>[
        for (final value in values)
          _SmallLabel(label: spaceCapabilityLabel(value, l10n)),
      ],
    );
  }
}

class _SmallLabel extends StatelessWidget {
  const _SmallLabel({required this.label, this.color});
  final String label;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final foreground = color ?? theme.colorScheme.onSurfaceVariant;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
      decoration: BoxDecoration(
        color:
            color?.withValues(alpha: 0.08) ??
            theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(5),
      ),
      child: Text(
        label,
        style: theme.textTheme.labelSmall?.copyWith(color: foreground),
      ),
    );
  }
}

/// A shared, low-contrast outline around the list, without per-item elevation.
class _FlatList extends StatelessWidget {
  const _FlatList({required this.children});
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: _listDecoration(context),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          for (var index = 0; index < children.length; index++) ...<Widget>[
            if (index > 0) _listDivider(context),
            children[index],
          ],
        ],
      ),
    );
  }
}

BoxDecoration _listDecoration(BuildContext context) => BoxDecoration(
  border: Border.all(
    color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.5),
  ),
  borderRadius: BorderRadius.circular(8),
);

Widget _listDivider(BuildContext context) => Divider(
  height: 1,
  thickness: 1,
  color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.35),
);

/// Represents the submitted identity definition form.
class _IdentityDefinition {
  /// Creates one identity definition result.
  const _IdentityDefinition({required this.name, required this.capabilities});

  final String name;
  final List<String> capabilities;
}

/// Edits an identity name and its human-readable capabilities.
class _IdentityEditor extends StatefulWidget {
  /// Creates the identity editor.
  const _IdentityEditor({
    required this.title,
    required this.choices,
    this.initialName,
    this.initialCapabilities,
  });

  final String title;
  final List<SpaceCapabilityChoice> choices;
  final String? initialName;
  final List<String>? initialCapabilities;

  /// Opens the editor and returns a validated definition.
  static Future<_IdentityDefinition?> show(
    BuildContext context, {
    required String title,
    required List<SpaceCapabilityChoice> choices,
    String? initialName,
    List<String>? initialCapabilities,
  }) {
    return showDialog<_IdentityDefinition>(
      context: context,
      builder: (_) => _IdentityEditor(
        title: title,
        choices: choices,
        initialName: initialName,
        initialCapabilities: initialCapabilities,
      ),
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
    _name = TextEditingController(text: widget.initialName ?? '');
    // Capabilities the current administrator cannot see stay selected so an
    // edit never silently narrows a role the issuer holds no capability for;
    // the replicated command still rejects unauthorized capability grants.
    _selected = <String>{...?widget.initialCapabilities};
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

/// Role ids seeded by the Space bootstrap; built-in roles cannot be edited or removed.
const _builtinRoleIds = <String>{'admin', 'user', 'relay', 'edge'};

/// Generates an internal assignment identifier for the replicated command.
String _generatedControlId(String prefix) {
  return '$prefix-${DateTime.now().microsecondsSinceEpoch}';
}
