// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../../common/icons/MaterialIconNameResolver.dart';
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
  UserPreferencesManager get _userPreferencesManager =>
      UserPreferencesManager(clients: _clients);

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
    await _userPreferencesManager.initializeIfNeeded(
      defaultProfileName: 'Operit',
    );
    final activeProfileId = await _userPreferencesManager.activeProfileId();
    final profileIds = await _userPreferencesManager.profileListFlowSnapshot();
    final profiles = <core_proxy.PreferenceProfile>[];
    for (final profileId in profileIds) {
      profiles.add(
        await _userPreferencesManager.getProfile(profileId: profileId),
      );
    }
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
      preferenceProfiles: profiles,
      currentProfileId: activeProfileId,
      enableMemoryAutoUpdate: await _clients.preferencesApiPreferences
          .enableMemoryAutoUpdateFlowSnapshot(),
      enableTools: await _clients.preferencesApiPreferences
          .enableToolsFlowSnapshot(),
      permissionLevel: _permissionLevelName(
        await _clients.permissionsToolPermissionSystem.getMasterSwitch(),
      ),
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

  Future<void> _selectMemory(String profileId) async {
    await _userPreferencesManager.setActiveProfile(profileId: profileId);
    await _clients.preferencesApiPreferences
        .saveDisableUserPreferenceDescription(isDisabled: false);
    _reloadSettings();
  }

  Future<void> _setMemoryProfileEnabled(
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
                          title: '当前档案',
                          value: data.disableUserPreferenceDescription
                              ? '关'
                              : data.currentProfileName,
                          checked: !data.disableUserPreferenceDescription,
                          onTap: () => _setMemoryProfileEnabled(
                            data,
                            data.disableUserPreferenceDescription,
                          ),
                        ),
                        if (!data.disableUserPreferenceDescription)
                          for (final profile in data.preferenceProfiles)
                            _MemoryProfileRow(
                              profile: profile,
                              selected: profile.id == data.currentProfileId,
                              onTap: () => _selectMemory(profile.id),
                            ),
                        _SwitchRow(
                          icon: data.enableMemoryAutoUpdate
                              ? Icons.save
                              : Icons.save_outlined,
                          title: '自动整理',
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
                              icon: _pluginToggleIcon(toggle),
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
    required this.preferenceProfiles,
    required this.currentProfileId,
    required this.enableMemoryAutoUpdate,
    required this.enableTools,
    required this.permissionLevel,
    required this.disableStreamOutput,
    required this.disableUserPreferenceDescription,
    required this.pluginToggles,
  });

  final List<core_proxy.PreferenceProfile> preferenceProfiles;
  final String currentProfileId;
  final bool enableMemoryAutoUpdate;
  final bool enableTools;
  final String permissionLevel;
  final bool disableStreamOutput;
  final bool disableUserPreferenceDescription;
  final List<core_proxy.InputMenuToggleDefinitionSnapshot> pluginToggles;

  String get currentProfileName {
    return preferenceProfiles
        .singleWhere((profile) => profile.id == currentProfileId)
        .name;
  }

  String get memorySummary {
    return disableUserPreferenceDescription ? '关' : currentProfileName;
  }

  _ToolPermissionMode get toolPermissionMode {
    if (!enableTools) {
      return _ToolPermissionMode.forbid;
    }
    return switch (permissionLevel) {
      'ALLOW' => _ToolPermissionMode.allow,
      'FORBID' => _ToolPermissionMode.forbid,
      _ => _ToolPermissionMode.ask,
    };
  }

  String get pluginSummary {
    final enabledCount = pluginToggles
        .where((toggle) => toggle.isChecked)
        .length;
    return '$enabledCount/${pluginToggles.length}';
  }
}

IconData _pluginToggleIcon(
  core_proxy.InputMenuToggleDefinitionSnapshot toggle,
) {
  return MaterialIconNameResolver.resolveOrDefault(toggle.icon, Icons.hub);
}

enum _ToolPermissionMode {
  forbid('禁用', false, 'FORBID'),
  ask('询问', true, 'ASK'),
  allow('允许', true, 'ALLOW');

  const _ToolPermissionMode(this.label, this.enableTools, this.permissionLevel);

  final String label;
  final bool enableTools;
  final String permissionLevel;
}

String _permissionLevelName(Object? value) {
  return value.toString().split('.').last;
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
                  Text(title, style: const TextStyle(fontSize: 13)),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      value,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.end,
                      style: TextStyle(
                        fontSize: 13,
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
    required this.title,
    required this.value,
    required this.checked,
    this.enabled = true,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String value;
  final bool checked;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return InkWell(
      onTap: enabled ? onTap : null,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Icon(
                icon,
                size: 16,
                color: !enabled
                    ? colorScheme.onSurfaceVariant.withValues(alpha: 0.45)
                    : checked
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontSize: 13,
                    color: enabled
                        ? colorScheme.onSurface
                        : colorScheme.onSurfaceVariant.withValues(alpha: 0.65),
                  ),
                ),
              ),
              Text(
                value,
                style: TextStyle(
                  fontSize: 13,
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

class _MemoryProfileRow extends StatelessWidget {
  const _MemoryProfileRow({
    required this.profile,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.PreferenceProfile profile;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
      child: InkWell(
        borderRadius: BorderRadius.circular(4),
        onTap: onTap,
        child: Container(
          width: double.infinity,
          decoration: BoxDecoration(
            color: selected
                ? colorScheme.primary.withValues(alpha: 0.10)
                : Colors.transparent,
            borderRadius: BorderRadius.circular(4),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
          child: Text(
            profile.name,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              fontSize: 13,
              color: selected ? colorScheme.primary : colorScheme.onSurface,
              fontWeight: selected ? FontWeight.w600 : FontWeight.normal,
            ),
          ),
        ),
      ),
    );
  }
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
                  style: TextStyle(
                    fontSize: 13,
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
