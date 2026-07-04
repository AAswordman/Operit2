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
    final apiPreferences = widget.clients.preferencesApiPreferences;
    final permissionSystem = widget.clients.permissionsToolPermissionSystem;
    final toolHandler = widget.clients.permissionsAiToolHandler;
    await toolHandler.registerDefaultTools();
    final toolNames =
        (await toolHandler.getAllToolNames())
            .where((toolName) => !_hiddenToolNames.contains(toolName))
            .toList(growable: false)
          ..sort();
    return _ToolSettingsData(
      enableTools: await apiPreferences.enableToolsFlowSnapshot(),
      permissionLevel: await permissionSystem.getMasterSwitch(),
      toolNames: toolNames,
      overrides: await permissionSystem.getToolPermissionOverrides(),
      mcpStartupTimeoutSeconds: await apiPreferences
          .getMcpStartupTimeoutSeconds(),
    );
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _setPermissionMode(_PermissionMode mode) async {
    await widget.clients.preferencesApiPreferences.saveEnableTools(
      isEnabled: mode.enableTools,
    );
    await widget.clients.permissionsToolPermissionSystem.saveMasterSwitch(
      level: mode.level,
    );
    _reload();
  }

  Future<void> _clearToolPermission(String toolName) async {
    await widget.clients.permissionsToolPermissionSystem.clearToolPermission(
      toolName: toolName,
    );
    _reload();
  }

  Future<void> _saveToolPermission(
    String toolName,
    core_proxy.PermissionLevel level,
  ) async {
    await widget.clients.permissionsToolPermissionSystem.saveToolPermission(
      toolName: toolName,
      level: level,
    );
    _reload();
  }

  Future<void> _openToolSelector(
    _ToolSettingsData data,
    core_proxy.PermissionLevel level,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final toolNames = await _ToolSelectorDialog.show(
      context: context,
      title: level == core_proxy.PermissionLevel.allow
          ? l10n.settingsToolsAddAllowTool
          : l10n.settingsToolsAddForbidTool,
      tools: data.toolNames,
      selectedTools: data.toolsForLevel(level),
    );
    if (toolNames == null || toolNames.isEmpty) {
      return;
    }
    for (final toolName in toolNames) {
      await widget.clients.permissionsToolPermissionSystem.saveToolPermission(
        toolName: toolName,
        level: level,
      );
    }
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
                _PermissionModeSelector(
                  selectedLevel: data.permissionLevel,
                  enableTools: data.enableTools,
                  onSelected: _setPermissionMode,
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsToolsToolGroups,
              children: <Widget>[
                Text(
                  l10n.settingsToolsToolGroupsDescription,
                  style: TextStyle(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 12),
                _ToolPermissionGroup(
                  title: l10n.settingsToolsAlwaysAllow,
                  description: l10n.settingsToolsAlwaysAllowDescription,
                  level: core_proxy.PermissionLevel.allow,
                  tools: data.toolsForLevel(core_proxy.PermissionLevel.allow),
                  allToolCount: data.toolNames.length,
                  onAdd: () =>
                      _openToolSelector(data, core_proxy.PermissionLevel.allow),
                  onRemove: _clearToolPermission,
                ),
                const SizedBox(height: 12),
                _ToolPermissionGroup(
                  title: l10n.settingsToolsAlwaysForbid,
                  description: l10n.settingsToolsAlwaysForbidDescription,
                  level: core_proxy.PermissionLevel.forbid,
                  tools: data.toolsForLevel(core_proxy.PermissionLevel.forbid),
                  allToolCount: data.toolNames.length,
                  onAdd: () => _openToolSelector(
                    data,
                    core_proxy.PermissionLevel.forbid,
                  ),
                  onRemove: _clearToolPermission,
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsToolsMcpStartupTimeout,
              children: <Widget>[
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
              ],
            ),
            _SectionCard(
              title: l10n.settingsToolsOverrides,
              initiallyExpanded: false,
              children: <Widget>[
                if (data.overrides.isEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 12),
                    child: Text(l10n.noPermissionRecords),
                  ),
                for (final entry in data.overrides.entries)
                  ListTile(
                    contentPadding: EdgeInsets.zero,
                    dense: true,
                    visualDensity: VisualDensity.compact,
                    title: Text(entry.key),
                    subtitle: Text(_permissionLevelName(entry.value)),
                    trailing: TextButton(
                      onPressed: () => _clearToolPermission(entry.key),
                      child: Text(l10n.clear),
                    ),
                  ),
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
    required this.enableTools,
    required this.permissionLevel,
    required this.toolNames,
    required this.overrides,
    required this.mcpStartupTimeoutSeconds,
  });

  final bool enableTools;
  final core_proxy.PermissionLevel permissionLevel;
  final List<String> toolNames;
  final Map<String, core_proxy.PermissionLevel> overrides;
  final int mcpStartupTimeoutSeconds;

  List<String> toolsForLevel(core_proxy.PermissionLevel level) {
    return overrides.entries
        .where((entry) => entry.value == level)
        .map((entry) => entry.key)
        .toList(growable: false)
      ..sort();
  }
}

enum _PermissionMode {
  allow(core_proxy.PermissionLevel.allow, true),
  ask(core_proxy.PermissionLevel.ask, true),
  forbid(core_proxy.PermissionLevel.forbid, false);

  const _PermissionMode(this.level, this.enableTools);

  final core_proxy.PermissionLevel level;
  final bool enableTools;
}

class _PermissionModeSelector extends StatelessWidget {
  const _PermissionModeSelector({
    required this.selectedLevel,
    required this.enableTools,
    required this.onSelected,
  });

  final core_proxy.PermissionLevel selectedLevel;
  final bool enableTools;
  final ValueChanged<_PermissionMode> onSelected;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final selected = enableTools
        ? selectedLevel
        : core_proxy.PermissionLevel.forbid;
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: <Widget>[
        _ModeChip(
          label: l10n.allow,
          selected: selected == core_proxy.PermissionLevel.allow,
          onTap: () => onSelected(_PermissionMode.allow),
        ),
        _ModeChip(
          label: l10n.settingsToolsAsk,
          selected: selected == core_proxy.PermissionLevel.ask,
          onTap: () => onSelected(_PermissionMode.ask),
        ),
        _ModeChip(
          label: l10n.deny,
          selected: selected == core_proxy.PermissionLevel.forbid,
          onTap: () => onSelected(_PermissionMode.forbid),
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

class _ToolPermissionGroup extends StatelessWidget {
  const _ToolPermissionGroup({
    required this.title,
    required this.description,
    required this.level,
    required this.tools,
    required this.allToolCount,
    required this.onAdd,
    required this.onRemove,
  });

  final String title;
  final String description;
  final core_proxy.PermissionLevel level;
  final List<String> tools;
  final int allToolCount;
  final VoidCallback onAdd;
  final ValueChanged<String> onRemove;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final color = level == core_proxy.PermissionLevel.allow
        ? colorScheme.primary
        : colorScheme.error;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: color.withValues(alpha: 0.28)),
        borderRadius: BorderRadius.circular(16),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(Icons.circle, size: 12, color: color),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        title,
                        style: const TextStyle(fontWeight: FontWeight.w800),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        description,
                        style: TextStyle(color: colorScheme.onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
                TextButton.icon(
                  onPressed: allToolCount == 0 ? null : onAdd,
                  icon: const Icon(Icons.add),
                  label: Text(l10n.settingsToolsAddTool),
                ),
              ],
            ),
            const SizedBox(height: 10),
            if (tools.isEmpty)
              Text(
                l10n.settingsToolsNoToolsInGroup,
                style: TextStyle(color: colorScheme.onSurfaceVariant),
              )
            else
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: <Widget>[
                  for (final toolName in tools)
                    InputChip(
                      label: Text(toolName),
                      onDeleted: () => onRemove(toolName),
                    ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}

class _ToolSelectorDialog extends StatefulWidget {
  const _ToolSelectorDialog({
    required this.title,
    required this.tools,
    required this.selectedTools,
  });

  final String title;
  final List<String> tools;
  final List<String> selectedTools;

  static Future<List<String>?> show({
    required BuildContext context,
    required String title,
    required List<String> tools,
    required List<String> selectedTools,
  }) {
    return showDialog<List<String>>(
      context: context,
      builder: (context) => _ToolSelectorDialog(
        title: title,
        tools: tools,
        selectedTools: selectedTools,
      ),
    );
  }

  @override
  State<_ToolSelectorDialog> createState() => _ToolSelectorDialogState();
}

class _ToolSelectorDialogState extends State<_ToolSelectorDialog> {
  final TextEditingController _searchController = TextEditingController();
  final Set<String> _pendingTools = <String>{};

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final query = _searchController.text.trim().toLowerCase();
    final selected = widget.selectedTools.toSet();
    final tools = widget.tools
        .where((tool) => query.isEmpty || tool.toLowerCase().contains(query))
        .toList(growable: false);
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 520,
        height: 520,
        child: Column(
          children: <Widget>[
            TextField(
              controller: _searchController,
              autofocus: true,
              decoration: InputDecoration(
                prefixIcon: const Icon(Icons.search),
                labelText: l10n.settingsToolsSearchTools,
              ),
              onChanged: (_) => setState(() {}),
            ),
            const SizedBox(height: 12),
            Expanded(
              child: ListView.builder(
                itemCount: tools.length,
                itemBuilder: (context, index) {
                  final toolName = tools[index];
                  final isSelected = selected.contains(toolName);
                  final pending = _pendingTools.contains(toolName);
                  return CheckboxListTile(
                    contentPadding: EdgeInsets.zero,
                    dense: true,
                    visualDensity: VisualDensity.compact,
                    value: isSelected || pending,
                    title: Text(toolName),
                    controlAffinity: ListTileControlAffinity.leading,
                    enabled: !isSelected,
                    onChanged: isSelected
                        ? null
                        : (value) {
                            setState(() {
                              if (value == true) {
                                _pendingTools.add(toolName);
                              } else {
                                _pendingTools.remove(toolName);
                              }
                            });
                          },
                  );
                },
              ),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _pendingTools.isEmpty
              ? null
              : () => Navigator.of(context).pop(_pendingTools.toList()),
          child: Text(l10n.ok),
        ),
      ],
    );
  }
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

String _permissionLevelName(core_proxy.PermissionLevel level) {
  return level.value;
}

const Set<String> _hiddenToolNames = <String>{
  'package_proxy',
  'proxy',
  'search',
};
