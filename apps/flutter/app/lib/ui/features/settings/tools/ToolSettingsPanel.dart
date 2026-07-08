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

class _ToolSettingsPanelState extends State<ToolSettingsPanel> {
  Future<_ToolSettingsData>? _future;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<_ToolSettingsData> _load() async {
    final host = await widget.clients.servicesRuntimeHostInfoService
        .runtimeHostDescriptor();
    return _ToolSettingsData(
      permissionMode: await widget.clients.permissionsToolPermissionSystem
          .getAiPermissionMode(),
      host: host,
      hostRequirements: await _HostAuthorizationBridge.requirements(
        host,
      ),
      mcpStartupTimeoutSeconds: await widget.clients.preferencesApiPreferences
          .getMcpStartupTimeoutSeconds(),
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
    final data = await _future!;
    await _HostAuthorizationBridge.request(data.host.id, requirement.id);
    _reload();
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
                  _modeFor(data.permissionMode).description,
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
              title: '系统授权',
              children: <Widget>[
                _HostAuthorizationList(
                  requirements: data.hostRequirements,
                  onRequest: _requestHostAuthorization,
                ),
              ],
            ),
            _SectionCard(
              title: '高级设置',
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
  });

  final core_proxy.AiPermissionMode permissionMode;
  final core_proxy.RuntimeHostDescriptor host;
  final List<_HostRequirement> hostRequirements;
  final int mcpStartupTimeoutSeconds;
}

enum _PermissionMode {
  readOnly(core_proxy.AiPermissionMode.readOnly),
  workspaceWrite(core_proxy.AiPermissionMode.workspaceWrite),
  full(core_proxy.AiPermissionMode.full);

  const _PermissionMode(this.permissionMode);

  final core_proxy.AiPermissionMode permissionMode;

  String get label {
    return switch (this) {
      _PermissionMode.readOnly => '只读',
      _PermissionMode.workspaceWrite => '读写',
      _PermissionMode.full => '完整权限',
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
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: <Widget>[
        for (final mode in _PermissionMode.values)
          _ModeChip(
            label: mode.label,
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
    if (requirements.isEmpty) {
      return const Text('当前设备没有需要用户处理的授权项。');
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
                Text(
                  requirement.title,
                  style: textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
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
                  _statusLabel(requirement.status),
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
            child: Text(_actionLabel(requirement)),
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
    required this.status,
    required this.action,
  });

  factory _HostRequirement.fromJson(Map<Object?, Object?> json) {
    return _HostRequirement(
      id: json['id'] as String,
      title: json['title'] as String,
      description: json['description'] as String,
      status: json['status'] as String,
      action: json['action'] as String,
    );
  }

  final String id;
  final String title;
  final String description;
  final String status;
  final String action;

  bool get canRequest =>
      status != 'Satisfied' &&
      (action == 'RuntimePermission' ||
          action == 'OpenSystemSettings' ||
          action == 'HostManaged');

  _HostRequirement withStatus(String status) {
    return _HostRequirement(
      id: id,
      title: title,
      description: description,
      status: status,
      action: action,
    );
  }
}

class _PermissionChain extends StatelessWidget {
  const _PermissionChain({required this.data});

  final _ToolSettingsData data;

  @override
  Widget build(BuildContext context) {
    final mode = _modeFor(data.permissionMode);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _ChainStep(
          index: '0',
          title: '应用运行隔离',
          value: '${data.host.isolation}',
        ),
        _ChainStep(
          index: '1',
          title: '系统授权',
          value: '${data.host.displayName} / ${data.host.platform}',
        ),
        _ChainStep(
          index: '2',
          title: 'AI 能力限制',
          value: '${mode.label}：${mode.description}',
        ),
        const _ChainStep(
          index: '3',
          title: '工具调用确认',
          value: '按 AI 直接调用的工具执行审批。',
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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _InfoRow(label: '当前运行环境', value: host.displayName),
        _InfoRow(label: '平台', value: '${host.platform}'),
        _InfoRow(label: '外层隔离', value: '${host.isolation}'),
        _InfoRow(label: '文件能力', value: host.fileSystemHost ? '已注册' : '未注册'),
        _InfoRow(label: '终端能力', value: host.terminalHost ? '已注册' : '未注册'),
        _InfoRow(label: '授权项', value: '${host.onboardingRequirements.length} 项'),
        _InfoRow(label: '结构化能力', value: '${host.structuredCapabilities.length} 项'),
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
          Expanded(
            child: Text(
              value,
              style: textTheme.bodyMedium,
            ),
          ),
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
    return host.onboardingRequirements.map((item) {
      final requirement = _HostRequirement.fromJson(
        Map<Object?, Object?>.from(item as Map),
      );
      final status = statusById[requirement.id] as String;
      return requirement.withStatus(status);
    }).toList(growable: false);
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

String _statusLabel(String status) {
  return switch (status) {
    'Satisfied' => '已授予',
    'Missing' => '未授予',
    'Unavailable' => '需要由系统处理',
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

String _actionLabel(_HostRequirement requirement) {
  if (requirement.status == 'Satisfied') {
    return '已授予';
  }
  return switch (requirement.action) {
    'RuntimePermission' => '授予',
    'OpenSystemSettings' => '打开设置',
    'HostManaged' => '授予',
    'None' => '不可操作',
    _ => '处理',
  };
}

class _ModeSummary {
  const _ModeSummary({
    required this.label,
    required this.description,
  });

  final String label;
  final String description;
}

_ModeSummary _modeFor(core_proxy.AiPermissionMode mode) {
  return switch (mode) {
    core_proxy.AiPermissionMode.readOnly => const _ModeSummary(
        label: '只读',
        description: 'AI 可以读取当前工作区，不能启动写入工具。',
      ),
    core_proxy.AiPermissionMode.workspaceWrite => const _ModeSummary(
        label: '读写',
        description: 'AI 可以读写当前工作区，应用内沙盒保持开启。',
      ),
    core_proxy.AiPermissionMode.full => const _ModeSummary(
        label: '完整权限',
        description: 'AI 可以读写当前工作区，并关闭应用内沙盒。',
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
