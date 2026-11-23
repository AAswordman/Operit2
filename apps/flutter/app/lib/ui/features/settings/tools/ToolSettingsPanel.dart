// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class ToolSettingsPanel extends StatefulWidget {
  const ToolSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<ToolSettingsPanel> createState() => _ToolSettingsPanelState();
}

class _ToolSettingsPanelState extends State<ToolSettingsPanel>
    with WidgetsBindingObserver {
  Future<_ToolSettingsData>? _future;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _future = _load();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      _reload();
    }
  }

  Future<_ToolSettingsData> _load() async {
    final host = await widget.clients.servicesRuntimeHostInfoService
        .runtimeHostDescriptor();
    return _ToolSettingsData(
      permissionMode: await widget.clients.permissionsToolPermissionSystem
          .getAiPermissionMode(),
      host: host,
      hostRequirements: await _HostAuthorizationBridge.requirements(host),
      mcpStartupTimeoutSeconds: await widget.clients.preferencesApiPreferences
          .getMcpStartupTimeoutSeconds(),
      toolPkgPreHookTimeoutSeconds: await widget
          .clients
          .preferencesApiPreferences
          .getToolPkgPreHookTimeoutSeconds(),
    );
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _setPermissionMode(_PermissionMode mode) async {
    await widget.clients.permissionsToolPermissionSystem.saveAiPermissionMode(
      mode: mode.permissionMode,
    );
    _reload();
  }

  Future<void> _requestHostAuthorization(_HostRequirement requirement) async {
    final l10n = AppLocalizations.of(context)!;
    try {
      final data = await _future!;
      await _HostAuthorizationBridge.request(data.host.id, requirement.id);
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(l10n.settingsToolsAuthorizationFailed(error.toString()))));
      }
    } finally {
      if (mounted) {
        _reload();
      }
    }
  }

  Future<void> _editMcpStartupTimeout(_ToolSettingsData data) async {
    final l10n = AppLocalizations.of(context)!;
    final seconds = await _NumberInputDialog.show(
      context: context,
      title: l10n.settingsToolsMcpStartupTimeout,
      label: l10n.settingsToolsMcpStartupTimeoutSeconds,
      initialValue: data.mcpStartupTimeoutSeconds,
    );
    if (seconds == null) {
      return;
    }
    await widget.clients.preferencesApiPreferences.saveMcpStartupTimeoutSeconds(
      seconds: seconds,
    );
    _reload();
  }

  /// Edits the total deadline shared by one ToolPkg pre-hook chain.
  Future<void> _editToolPkgPreHookTimeout(_ToolSettingsData data) async {
    final l10n = AppLocalizations.of(context)!;
    final seconds = await _NumberInputDialog.show(
      context: context,
      title: l10n.settingsToolsToolPkgPreHookTimeout,
      label: l10n.settingsToolsToolPkgPreHookTimeoutSeconds,
      initialValue: data.toolPkgPreHookTimeoutSeconds,
    );
    if (seconds == null) {
      return;
    }
    await widget.clients.preferencesApiPreferences
        .saveToolPkgPreHookTimeoutSeconds(seconds: seconds);
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<_ToolSettingsData>(
      future: _future,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
        }
        final data = snapshot.data;
        if (data == null) {
          return const M3LoadingPane();
        }
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: <Widget>[
            _SectionCard(
              title: l10n.settingsToolsPermissionMode,
              children: <Widget>[
                Text(
                  _modeFor(data.permissionMode, l10n).description,
                  style: TextStyle(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 12),
                _PermissionModeSelector(
                  selectedMode: data.permissionMode,
                  onSelected: _setPermissionMode,
                ),
              ],
            ),
            _SectionCard(
              title: l10n.onboardingPermissions,
              children: <Widget>[
                _HostAuthorizationList(
                  requirements: data.hostRequirements,
                  onRequest: _requestHostAuthorization,
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsToolsAdvanced,
              initiallyExpanded: false,
              children: <Widget>[
                _PermissionChain(data: data),
                const Divider(height: 24),
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                  visualDensity: VisualDensity.compact,
                  leading: const Icon(Icons.timer_outlined),
                  title: Text(l10n.settingsToolsMcpStartupTimeout),
                  subtitle: Text(
                    l10n.settingsToolsMcpDescription(
                      data.mcpStartupTimeoutSeconds,
                    ),
                  ),
                  trailing: TextButton(
                    onPressed: () => _editMcpStartupTimeout(data),
                    child: Text(l10n.edit),
                  ),
                ),
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                  visualDensity: VisualDensity.compact,
                  leading: const Icon(Icons.timer_outlined),
                  title: Text(l10n.settingsToolsToolPkgPreHookTimeout),
                  subtitle: Text(
                    l10n.settingsToolsToolPkgPreHookDescription(
                      data.toolPkgPreHookTimeoutSeconds,
                    ),
                  ),
                  trailing: TextButton(
                    onPressed: () => _editToolPkgPreHookTimeout(data),
                    child: Text(l10n.edit),
                  ),
                ),
                const Divider(height: 24),
                _AdvancedHostSummary(host: data.host),
              ],
            ),
          ],
        );
      },
    );
  }
}

class _ToolSettingsData {
  const _ToolSettingsData({
    required this.permissionMode,
    required this.host,
    required this.hostRequirements,
    required this.mcpStartupTimeoutSeconds,
    required this.toolPkgPreHookTimeoutSeconds,
  });

  final core_proxy.AiPermissionMode permissionMode;
  final core_proxy.RuntimeHostDescriptor host;
  final List<_HostRequirement> hostRequirements;
  final int mcpStartupTimeoutSeconds;
  final int toolPkgPreHookTimeoutSeconds;
}

enum _PermissionMode {
  readOnly(core_proxy.AiPermissionMode.readOnly),
  workspaceWrite(core_proxy.AiPermissionMode.workspaceWrite),
  full(core_proxy.AiPermissionMode.full);

  const _PermissionMode(this.permissionMode);

  final core_proxy.AiPermissionMode permissionMode;

  String label(AppLocalizations l10n) {
    return switch (this) {
      _PermissionMode.readOnly => l10n.settingsToolsReadOnly,
      _PermissionMode.workspaceWrite => l10n.settingsToolsReadWrite,
      _PermissionMode.full => l10n.settingsToolsFullAccess,
    };
  }
}

class _PermissionModeSelector extends StatelessWidget {
  const _PermissionModeSelector({
    required this.selectedMode,
    required this.onSelected,
  });

  final core_proxy.AiPermissionMode selectedMode;
  final ValueChanged<_PermissionMode> onSelected;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: <Widget>[
        for (final mode in _PermissionMode.values)
          _ModeChip(
            label: mode.label(l10n),
            selected: selectedMode == mode.permissionMode,
            onTap: () => onSelected(mode),
          ),
      ],
    );
  }
}

class _ModeChip extends StatelessWidget {
  const _ModeChip({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ChoiceChip(
      label: Text(label),
      selected: selected,
      onSelected: (_) => onTap(),
    );
  }
}

class _HostAuthorizationList extends StatelessWidget {
  const _HostAuthorizationList({
    required this.requirements,
    required this.onRequest,
  });

  final List<_HostRequirement> requirements;
  final ValueChanged<_HostRequirement> onRequest;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    if (requirements.isEmpty) {
      return Text(l10n.onboardingNoPermissions);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (final requirement in requirements)
          _HostAuthorizationTile(
            requirement: requirement,
            onRequest: () => onRequest(requirement),
          ),
      ],
    );
  }
}

class _HostAuthorizationTile extends StatelessWidget {
  const _HostAuthorizationTile({
    required this.requirement,
    required this.onRequest,
  });

  final _HostRequirement requirement;
  final VoidCallback onRequest;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: <Widget>[
          Icon(
            _statusIcon(requirement.status),
            color: _statusColor(requirement.status, colorScheme),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Row(
                  children: <Widget>[
                    Expanded(
                      child: Text(
                        requirement.title,
                        style: textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w800,
                        ),
                      ),
                    ),
                    if (!requirement.isRequired) ...<Widget>[
                      const SizedBox(width: 8),
                      Text(
                        l10n.onboardingOptional,
                        style: textTheme.labelMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  requirement.description,
                  style: textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  _statusLabel(requirement.status, l10n),
                  style: textTheme.labelMedium?.copyWith(
                    color: _statusColor(requirement.status, colorScheme),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          FilledButton.tonal(
            onPressed: requirement.canRequest ? onRequest : null,
            child: Text(_actionLabel(requirement, l10n)),
          ),
        ],
      ),
    );
  }
}

class _HostRequirement {
  const _HostRequirement({
    required this.id,
    required this.title,
    required this.description,
    required this.isRequired,
    required this.status,
    required this.action,
  });

  /// Creates a display requirement from the generated runtime host descriptor.
  factory _HostRequirement.fromHostRequirement(
    core_proxy.HostOnboardingRequirement requirement,
  ) {
    return _HostRequirement(
      id: requirement.id,
      title: requirement.title,
      description: requirement.description,
      isRequired: requirement.isRequired,
      status: requirement.status.value,
      action: requirement.action.value,
    );
  }

  final String id;
  final String title;
  final String description;
  final bool isRequired;
  final String status;
  final String action;

  bool get canRequest =>
      action == 'OpenSystemSettings' ||
      (status != 'Satisfied' &&
          status != 'Unavailable' &&
          (action == 'RuntimePermission' || action == 'HostManaged'));

  _HostRequirement withStatus(String status) {
    return _HostRequirement(
      id: id,
      title: title,
      description: description,
      isRequired: isRequired,
      status: status,
      action: action,
    );
  }
}

/// Uses the Host's actual process signature, independently of AI approval mode.
String _hostIsolationDescription(core_proxy.RuntimeHostDescriptor host, AppLocalizations l10n) {
  if (host.platform == core_proxy.HostPlatform.macos) {
    return switch (host.isolation) {
      core_proxy.HostIsolation.osAppSandbox => l10n.settingsToolsSandboxed,
      core_proxy.HostIsolation.none => l10n.settingsToolsUnsandboxed,
      _ => host.isolation.value,
    };
  }
  return host.isolation.value;
}

class _PermissionChain extends StatelessWidget {
  const _PermissionChain({required this.data});

  final _ToolSettingsData data;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final mode = _modeFor(data.permissionMode, l10n);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _ChainStep(
          index: '0',
          title: l10n.settingsToolsIsolation,
          value: _hostIsolationDescription(data.host, l10n),
        ),
        _ChainStep(
          index: '1',
          title: l10n.onboardingPermissions,
          value: '${data.host.displayName} / ${data.host.platform}',
        ),
        _ChainStep(
          index: '2',
          title: l10n.settingsToolsRestrictions,
          value: '${mode.label}：${mode.description}',
        ),
        _ChainStep(
          index: '3',
          title: l10n.settingsToolsConfirmCalls,
          value: l10n.settingsToolsConfirmDirectCalls,
        ),
      ],
    );
  }
}

class _AdvancedHostSummary extends StatelessWidget {
  const _AdvancedHostSummary({required this.host});

  final core_proxy.RuntimeHostDescriptor host;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _InfoRow(label: l10n.settingsToolsRuntimeEnvironment, value: host.displayName),
        _InfoRow(label: l10n.settingsToolsPlatform, value: '${host.platform}'),
        _InfoRow(label: l10n.settingsToolsOuterIsolation, value: _hostIsolationDescription(host, l10n)),
        _InfoRow(label: l10n.settingsToolsFileCapability, value: host.fileSystemHost ? l10n.settingsToolsRegistered : l10n.settingsToolsUnregistered),
        _InfoRow(label: l10n.settingsToolsTerminalCapability, value: host.terminalHost ? l10n.settingsToolsRegistered : l10n.settingsToolsUnregistered),
        _InfoRow(
          label: l10n.settingsToolsAuthorizations,
          value: l10n.settingsToolsItemCount(host.onboardingRequirements.length),
        ),
        _InfoRow(
          label: l10n.settingsToolsStructuredCapabilities,
          value: l10n.settingsToolsItemCount(host.structuredCapabilities.length),
        ),
      ],
    );
  }
}

class _ChainStep extends StatelessWidget {
  const _ChainStep({
    required this.index,
    required this.title,
    required this.value,
  });

  final String index;
  final String title;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          CircleAvatar(
            radius: 13,
            backgroundColor: colorScheme.primaryContainer,
            child: Text(index, style: textTheme.labelMedium),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  title,
                  style: textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  value,
                  style: textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Row(
        children: <Widget>[
          SizedBox(
            width: 96,
            child: Text(
              label,
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(child: Text(value, style: textTheme.bodyMedium)),
        ],
      ),
    );
  }
}

class _HostAuthorizationBridge {
  static const MethodChannel _channel = MethodChannel('operit/runtime');

  static Future<List<_HostRequirement>> requirements(
    core_proxy.RuntimeHostDescriptor host,
  ) async {
    if (host.onboardingRequirements.isEmpty) {
      return const <_HostRequirement>[];
    }
    final statusById = await _requirementStatus(host.id);
    return host.onboardingRequirements
        .map((item) {
          final requirement = _HostRequirement.fromHostRequirement(item);
          final status = statusById[requirement.id] as String;
          return requirement.withStatus(status);
        })
        .toList(growable: false);
  }

  static Future<void> request(String hostId, String requirementId) {
    return _channel.invokeMethod<void>(
      'hostOnboardingRequestPermission',
      <String, Object?>{'hostId': hostId, 'requirementId': requirementId},
    );
  }

  static Future<Map<String, String>> _requirementStatus(String hostId) async {
    final result = await _channel.invokeMapMethod<Object?, Object?>(
      'hostOnboardingPermissionSnapshot',
      <String, Object?>{'hostId': hostId},
    );
    if (result == null) {
      throw StateError('host onboarding permission snapshot is empty');
    }
    return result.map((key, value) {
      final item = Map<Object?, Object?>.from(value as Map);
      return MapEntry(key as String, item['status'] as String);
    });
  }
}

String _statusLabel(String status, AppLocalizations l10n) {
  return switch (status) {
    'Satisfied' => l10n.onboardingPermissionGranted,
    'Missing' => l10n.settingsToolsNotGranted,
    'Unavailable' => l10n.settingsToolsSystemRequired,
    _ => status,
  };
}

IconData _statusIcon(String status) {
  return switch (status) {
    'Satisfied' => Icons.check_circle_rounded,
    'Missing' => Icons.error_outline_rounded,
    'Unavailable' => Icons.info_outline_rounded,
    _ => Icons.help_outline_rounded,
  };
}

Color _statusColor(String status, ColorScheme colorScheme) {
  return switch (status) {
    'Satisfied' => colorScheme.primary,
    'Missing' => colorScheme.error,
    'Unavailable' => colorScheme.tertiary,
    _ => colorScheme.onSurfaceVariant,
  };
}

String _actionLabel(_HostRequirement requirement, AppLocalizations l10n) {
  if (requirement.action == 'OpenSystemSettings') {
    return l10n.settingsToolsOpenSettings;
  }
  if (requirement.status == 'Satisfied') {
    return l10n.onboardingPermissionGranted;
  }
  return switch (requirement.action) {
    'RuntimePermission' => l10n.onboardingGrantPermission,
    'OpenSystemSettings' => l10n.settingsToolsOpenSettings,
    'HostManaged' => l10n.onboardingGrantPermission,
    'None' => l10n.settingsToolsUnavailable,
    _ => l10n.settingsToolsHandle,
  };
}

class _ModeSummary {
  const _ModeSummary({required this.label, required this.description});

  final String label;
  final String description;
}

_ModeSummary _modeFor(core_proxy.AiPermissionMode mode, AppLocalizations l10n) {
  return switch (mode) {
    core_proxy.AiPermissionMode.readOnly => _ModeSummary(
      label: l10n.settingsToolsReadOnly,
      description: l10n.settingsToolsReadOnlyDescription,
    ),
    core_proxy.AiPermissionMode.workspaceWrite => _ModeSummary(
      label: l10n.settingsToolsReadWrite,
      description: l10n.settingsToolsReadWriteDescription,
    ),
    core_proxy.AiPermissionMode.full => _ModeSummary(
      label: l10n.settingsToolsFullAccess,
      description: l10n.settingsToolsFullAccessDescription,
    ),
  };
}

class _NumberInputDialog extends StatefulWidget {
  const _NumberInputDialog({
    required this.title,
    required this.label,
    required this.initialValue,
  });

  final String title;
  final String label;
  final int initialValue;

  static Future<int?> show({
    required BuildContext context,
    required String title,
    required String label,
    required int initialValue,
  }) {
    return showDialog<int>(
      context: context,
      builder: (context) => _NumberInputDialog(
        title: title,
        label: label,
        initialValue: initialValue,
      ),
    );
  }

  @override
  State<_NumberInputDialog> createState() => _NumberInputDialogState();
}

class _NumberInputDialogState extends State<_NumberInputDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _controller = TextEditingController(
    text: widget.initialValue.toString(),
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(widget.title),
      content: Form(
        key: _formKey,
        child: TextFormField(
          controller: _controller,
          autofocus: true,
          keyboardType: TextInputType.number,
          inputFormatters: <TextInputFormatter>[
            FilteringTextInputFormatter.digitsOnly,
          ],
          decoration: InputDecoration(labelText: widget.label),
          validator: (value) {
            final text = value?.trim() ?? '';
            if (text.isEmpty) {
              return widget.label;
            }
            return null;
          },
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () {
            if (!_formKey.currentState!.validate()) {
              return;
            }
            Navigator.of(context).pop(int.parse(_controller.text.trim()));
          },
          child: Text(l10n.save),
        ),
      ],
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    required this.children,
    this.initiallyExpanded = true,
  });

  final String title;
  final List<Widget> children;
  final bool initiallyExpanded;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: ExpansionTile(
          initiallyExpanded: initiallyExpanded,
          tilePadding: const EdgeInsets.symmetric(horizontal: 14),
          childrenPadding: const EdgeInsets.fromLTRB(14, 0, 14, 12),
          shape: RoundedRectangleBorder(borderRadius: radius),
          collapsedShape: RoundedRectangleBorder(borderRadius: radius),
          title: Text(
            title,
            style: SettingsControlStyles.sectionTitleTextStyle(context),
          ),
          children: children,
        ),
      ),
    );
  }
}
