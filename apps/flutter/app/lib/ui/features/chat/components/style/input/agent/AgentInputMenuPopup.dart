// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../viewmodel/ChatViewModel.dart';

class AgentInputMenuPopup extends StatefulWidget {
  const AgentInputMenuPopup({
    super.key,
    required this.viewModel,
    required this.currentChatId,
    required this.onDismiss,
  });

  final ChatViewModel viewModel;
  final String? currentChatId;
  final VoidCallback onDismiss;

  @override
  State<AgentInputMenuPopup> createState() => _AgentInputMenuPopupState();
}

class _AgentInputMenuPopupState extends State<AgentInputMenuPopup> {
  Future<_AgentInputMenuData>? _settingsFuture;
  Timer? _pluginChangeTimer;
  int? _observedPluginChangeVersion;
  bool _checkingPluginChangeVersion = false;
  bool _memoryExpanded = false;
  bool _toolsExpanded = false;
  bool _behaviorExpanded = false;
  bool _pluginsExpanded = false;

  GeneratedCoreProxyClients get _clients => widget.viewModel.clients;

  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
    _startPluginChangeObserver();
  }

  @override
  void dispose() {
    _pluginChangeTimer?.cancel();
    super.dispose();
  }

  void _startPluginChangeObserver() {
    _pluginChangeTimer = Timer.periodic(const Duration(milliseconds: 250), (_) {
      _checkPluginChangeVersion();
    });
  }

  Future<void> _checkPluginChangeVersion() async {
    if (_checkingPluginChangeVersion) {
      return;
    }
    _checkingPluginChangeVersion = true;
    final int version;
    try {
      version = await _clients.pluginsToolpkgToolPkgInputMenuToggleBridge
          .changeVersion();
    } finally {
      _checkingPluginChangeVersion = false;
    }
    if (!mounted) {
      return;
    }
    final observed = _observedPluginChangeVersion;
    _observedPluginChangeVersion = version;
    if (observed != null && observed != version) {
      _reloadSettings();
    }
  }

  Future<_AgentInputMenuData> _loadSettings() async {
    _observedPluginChangeVersion = await _clients
        .pluginsToolpkgToolPkgInputMenuToggleBridge
        .changeVersion();
    final pluginToggles = await _clients
        .pluginsToolpkgToolPkgInputMenuToggleBridge
        .createToggleDefinitionsForFlutter(
          chatId: widget.currentChatId,
          featureStates: const <String, bool>{},
          runtime: 'main',
        );
    return _AgentInputMenuData(
      enableMemoryAutoUpdate: await _clients.preferencesApiPreferences
          .enableMemoryAutoUpdateFlowSnapshot(),
      enableTools: await _clients.preferencesApiPreferences
          .enableToolsFlowSnapshot(),
      permissionLevel: await _clients.permissionsToolPermissionSystem
          .getMasterSwitch(),
      disableStreamOutput: await _clients.preferencesApiPreferences
          .disableStreamOutputFlowSnapshot(),
      disableUserPreferenceDescription: await _clients.preferencesApiPreferences
          .disableUserPreferenceDescriptionFlowSnapshot(),
      pluginToggles: pluginToggles,
    );
  }

  void _reloadSettings() {
    setState(() {
      _settingsFuture = _loadSettings();
    });
  }

  Future<void> _setUserMarkdownEnabled(
    _AgentInputMenuData data,
    bool enabled,
  ) async {
    await _clients.preferencesApiPreferences
        .saveDisableUserPreferenceDescription(isDisabled: !enabled);
    _reloadSettings();
  }

  Future<void> _toggleMemoryAutoUpdate(_AgentInputMenuData data) async {
    await _clients.preferencesApiPreferences.saveEnableMemoryAutoUpdate(
      isEnabled: !data.enableMemoryAutoUpdate,
    );
    _reloadSettings();
  }

  Future<void> _setPermissionMode(_ToolPermissionMode mode) async {
    await _clients.preferencesApiPreferences.saveEnableTools(
      isEnabled: mode.enableTools,
    );
    await _clients.permissionsToolPermissionSystem.saveMasterSwitch(
      level: mode.permissionLevel,
    );
    _reloadSettings();
  }

  Future<void> _toggleDisableStreamOutput(_AgentInputMenuData data) async {
    await _clients.preferencesApiPreferences.saveDisableStreamOutput(
      isDisabled: !data.disableStreamOutput,
    );
    _reloadSettings();
  }

  Future<void> _togglePlugin(
    core_proxy.InputMenuToggleDefinitionSnapshot toggle,
  ) async {
    await _clients.pluginsToolpkgToolPkgInputMenuToggleBridge
        .triggerToggleForFlutter(
          toggleId: toggle.id,
          chatId: widget.currentChatId,
          runtime: 'main',
        );
    _reloadSettings();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: Colors.transparent,
      child: Card(
        margin: EdgeInsets.zero,
        color: colorScheme.surfaceContainer,
        elevation: 4,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 300, maxHeight: 420),
          child: FutureBuilder<_AgentInputMenuData>(
            future: _settingsFuture,
            builder: (context, snapshot) {
              if (snapshot.hasError) {
                Error.throwWithStackTrace(
                  snapshot.error!,
                  snapshot.stackTrace!,
                );
              }
              final data = snapshot.data;
              if (data == null) {
                return const SizedBox(
                  width: 300,
                  height: 96,
                  child: Center(child: CircularProgressIndicator()),
                );
              }
              return SingleChildScrollView(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    _MenuSection(
                      icon: Icons.data_object_outlined,
                      title: '记忆',
                      value: data.memorySummary,
                      expanded: _memoryExpanded,
                      onTap: () {
                        setState(() {
                          _memoryExpanded = !_memoryExpanded;
                        });
                      },
                      children: <Widget>[
                        _SwitchRow(
                          icon: Icons.account_circle_outlined,
                          title: '提供用户资料',
                          value: data.disableUserPreferenceDescription
                              ? '关'
                              : '开',
                          checked: !data.disableUserPreferenceDescription,
                          onTap: () => _setUserMarkdownEnabled(
                            data,
                            data.disableUserPreferenceDescription,
                          ),
                        ),
                        _SwitchRow(
                          icon: data.enableMemoryAutoUpdate
                              ? Icons.save
                              : Icons.save_outlined,
                          title: '自动更新记忆库',
                          value: data.enableMemoryAutoUpdate ? '开' : '关',
                          checked: data.enableMemoryAutoUpdate,
                          onTap: () => _toggleMemoryAutoUpdate(data),
                        ),
                      ],
                    ),
                    _MenuSection(
                      icon: Icons.security_outlined,
                      title: '工具',
                      value: data.toolPermissionMode.label,
                      expanded: _toolsExpanded,
                      onTap: () {
                        setState(() {
                          _toolsExpanded = !_toolsExpanded;
                        });
                      },
                      children: <Widget>[
                        Padding(
                          padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
                          child: _PermissionModeSelector(
                            selectedMode: data.toolPermissionMode,
                            onSelected: _setPermissionMode,
                          ),
                        ),
                      ],
                    ),
                    _MenuSection(
                      icon: Icons.bolt_outlined,
                      title: '行为',
                      value: data.disableStreamOutput ? '非流式' : '流式',
                      expanded: _behaviorExpanded,
                      onTap: () {
                        setState(() {
                          _behaviorExpanded = !_behaviorExpanded;
                        });
                      },
                      children: <Widget>[
                        _SwitchRow(
                          icon: Icons.speed_outlined,
                          title: '流式输出',
                          value: data.disableStreamOutput ? '关' : '开',
                          checked: !data.disableStreamOutput,
                          onTap: () => _toggleDisableStreamOutput(data),
                        ),
                      ],
                    ),
                    if (data.pluginToggles.isNotEmpty)
                      _MenuSection(
                        icon: Icons.extension_outlined,
                        title: '插件',
                        value: data.pluginSummary,
                        expanded: _pluginsExpanded,
                        onTap: () {
                          setState(() {
                            _pluginsExpanded = !_pluginsExpanded;
                          });
                        },
                        children: <Widget>[
                          for (final toggle in data.pluginToggles)
                            _SwitchRow(
                              icon: Icons.hub,
                              materialIconName: toggle.icon,
                              title: toggle.title ?? toggle.id,
                              value: toggle.isChecked ? '开' : '关',
                              checked: toggle.isChecked,
                              enabled: toggle.isEnabled,
                              onTap: () => _togglePlugin(toggle),
                            ),
                        ],
                      ),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

class _AgentInputMenuData {
  const _AgentInputMenuData({
    required this.enableMemoryAutoUpdate,
    required this.enableTools,
    required this.permissionLevel,
    required this.disableStreamOutput,
    required this.disableUserPreferenceDescription,
    required this.pluginToggles,
  });

  final bool enableMemoryAutoUpdate;
  final bool enableTools;
  final core_proxy.PermissionLevel permissionLevel;
  final bool disableStreamOutput;
  final bool disableUserPreferenceDescription;
  final List<core_proxy.InputMenuToggleDefinitionSnapshot> pluginToggles;

  String get memorySummary {
    return switch ((disableUserPreferenceDescription, enableMemoryAutoUpdate)) {
      (true, false) => '关',
      (false, false) => '用户资料',
      (true, true) => '记忆库更新',
      (false, true) => '用户资料 · 记忆库更新',
    };
  }

  _ToolPermissionMode get toolPermissionMode {
    if (!enableTools) {
      return _ToolPermissionMode.forbid;
    }
    return switch (permissionLevel) {
      core_proxy.PermissionLevel.allow => _ToolPermissionMode.allow,
      core_proxy.PermissionLevel.ask => _ToolPermissionMode.ask,
      core_proxy.PermissionLevel.forbid => _ToolPermissionMode.forbid,
    };
  }

  String get pluginSummary {
    final enabledCount = pluginToggles
        .where((toggle) => toggle.isChecked)
        .length;
    return '$enabledCount/${pluginToggles.length}';
  }
}

enum _ToolPermissionMode {
  forbid('禁用', false, core_proxy.PermissionLevel.forbid),
  ask('询问', true, core_proxy.PermissionLevel.ask),
  allow('允许', true, core_proxy.PermissionLevel.allow);

  const _ToolPermissionMode(this.label, this.enableTools, this.permissionLevel);

  final String label;
  final bool enableTools;
  final core_proxy.PermissionLevel permissionLevel;
}

class _MenuSection extends StatelessWidget {
  const _MenuSection({
    required this.icon,
    required this.title,
    required this.value,
    required this.expanded,
    required this.onTap,
    required this.children,
  });

  final IconData icon;
  final String title;
  final String value;
  final bool expanded;
  final VoidCallback onTap;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        InkWell(
          onTap: onTap,
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 40),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                children: <Widget>[
                  Icon(icon, size: 17, color: colorScheme.onSurfaceVariant),
                  const SizedBox(width: 12),
                  Text(title, style: textTheme.bodySmall),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      value,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.end,
                      style: textTheme.bodySmall!.copyWith(
                        color: colorScheme.primary,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 6),
                  Icon(
                    expanded
                        ? Icons.keyboard_arrow_up
                        : Icons.keyboard_arrow_down,
                    size: 20,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),
        ),
        if (expanded)
          ColoredBox(
            color: colorScheme.surface.withValues(alpha: 0.42),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Column(mainAxisSize: MainAxisSize.min, children: children),
            ),
          ),
      ],
    );
  }
}

class _SwitchRow extends StatelessWidget {
  const _SwitchRow({
    required this.icon,
    this.materialIconName,
    required this.title,
    required this.value,
    required this.checked,
    this.enabled = true,
    required this.onTap,
  });

  final IconData icon;
  final String? materialIconName;
  final String title;
  final String value;
  final bool checked;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final iconColor = !enabled
        ? colorScheme.onSurfaceVariant.withValues(alpha: 0.45)
        : checked
        ? colorScheme.primary
        : colorScheme.onSurfaceVariant;
    final iconName = materialIconName?.trim();
    return InkWell(
      onTap: enabled ? onTap : null,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              iconName == null || iconName.isEmpty
                  ? Icon(icon, size: 16, color: iconColor)
                  : _MaterialIconLigature(
                      iconName: iconName,
                      size: 16,
                      color: iconColor,
                    ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall!.copyWith(
                    color: enabled
                        ? colorScheme.onSurface
                        : colorScheme.onSurfaceVariant.withValues(alpha: 0.65),
                  ),
                ),
              ),
              Text(
                value,
                style: textTheme.bodySmall!.copyWith(
                  color: enabled
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant.withValues(alpha: 0.65),
                ),
              ),
              const SizedBox(width: 8),
              Transform.scale(
                scale: 0.66,
                child: Switch(
                  value: checked,
                  onChanged: enabled ? (_) => onTap() : null,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MaterialIconLigature extends StatelessWidget {
  const _MaterialIconLigature({
    required this.iconName,
    required this.size,
    required this.color,
  });

  final String iconName;
  final double size;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return SizedBox.square(
      dimension: size,
      child: Center(
        child: FittedBox(
          fit: BoxFit.contain,
          child: Text(
            _materialIconLigatureName(iconName),
            overflow: TextOverflow.clip,
            softWrap: false,
            style: TextStyle(
              fontFamily: 'MaterialIcons',
              height: 1,
              color: color,
            ),
          ),
        ),
      ),
    );
  }
}

String _materialIconLigatureName(String iconName) {
  final buffer = StringBuffer();
  var wroteSeparator = false;
  var previousWasLowerOrDigit = false;

  for (final codeUnit in iconName.trim().codeUnits) {
    final isUpper = codeUnit >= 65 && codeUnit <= 90;
    final isLower = codeUnit >= 97 && codeUnit <= 122;
    final isDigit = codeUnit >= 48 && codeUnit <= 57;

    if (isUpper || isLower || isDigit) {
      if (isUpper && previousWasLowerOrDigit && !wroteSeparator) {
        buffer.write('_');
      }
      buffer.writeCharCode(isUpper ? codeUnit + 32 : codeUnit);
      wroteSeparator = false;
      previousWasLowerOrDigit = isLower || isDigit;
      continue;
    }

    if (!wroteSeparator && buffer.isNotEmpty) {
      buffer.write('_');
      wroteSeparator = true;
      previousWasLowerOrDigit = false;
    }
  }

  var result = buffer.toString();
  while (result.endsWith('_')) {
    result = result.substring(0, result.length - 1);
  }
  return _materialIconNameHasStyle(result) ? result : '${result}_baseline';
}

bool _materialIconNameHasStyle(String iconName) {
  return iconName.endsWith('_baseline') ||
      iconName.endsWith('_outlined') ||
      iconName.endsWith('_rounded') ||
      iconName.endsWith('_sharp');
}

class _PermissionModeSelector extends StatelessWidget {
  const _PermissionModeSelector({
    required this.selectedMode,
    required this.onSelected,
  });

  final _ToolPermissionMode selectedMode;
  final ValueChanged<_ToolPermissionMode> onSelected;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Row(
      children: <Widget>[
        for (final mode in _ToolPermissionMode.values) ...[
          Expanded(
            child: InkWell(
              borderRadius: BorderRadius.circular(8),
              onTap: () => onSelected(mode),
              child: Container(
                height: 34,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: mode == selectedMode
                      ? colorScheme.primaryContainer
                      : Colors.transparent,
                  border: Border.all(
                    color: mode == selectedMode
                        ? colorScheme.primary
                        : colorScheme.outline.withValues(alpha: 0.35),
                  ),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Text(
                  mode.label,
                  style: textTheme.bodySmall!.copyWith(
                    color: mode == selectedMode
                        ? colorScheme.onPrimaryContainer
                        : colorScheme.onSurface,
                    fontWeight: mode == selectedMode
                        ? FontWeight.w600
                        : FontWeight.normal,
                  ),
                ),
              ),
            ),
          ),
          if (mode != _ToolPermissionMode.values.last) const SizedBox(width: 6),
        ],
      ],
    );
  }
}
