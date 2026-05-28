// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;

class AgentExtraSettingsPopup extends StatefulWidget {
  const AgentExtraSettingsPopup({
    super.key,
    required this.bridge,
    required this.onDismiss,
  });

  final OperitRuntimeBridge bridge;
  final VoidCallback onDismiss;

  @override
  State<AgentExtraSettingsPopup> createState() =>
      _AgentExtraSettingsPopupState();
}

class _AgentExtraSettingsPopupState extends State<AgentExtraSettingsPopup> {
  Future<_AgentExtraSettingsData>? _settingsFuture;
  bool _memoryExpanded = false;
  bool _disableExpanded = false;
  String? _infoTitle;
  String? _infoDescription;

  GeneratedCoreProxyClients get _clients =>
      GeneratedCoreProxyClients(widget.bridge);

  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
  }

  Future<_AgentExtraSettingsData> _loadSettings() async {
    await _clients.preferencesUserPreferencesManager.initializeIfNeeded(
      defaultProfileName: 'Operit',
    );
    final activeProfileId = await _clients.preferencesUserPreferencesManager
        .activeProfileId();
    final profileIds = await _clients.preferencesUserPreferencesManager
        .profileListFlowSnapshot();
    final profiles = <core_proxy.PreferenceProfile>[];
    for (final profileId in profileIds) {
      profiles.add(
        await _clients.preferencesUserPreferencesManager.getProfile(
          profileId: profileId,
        ),
      );
    }
    return _AgentExtraSettingsData(
      preferenceProfiles: profiles,
      currentProfileId: activeProfileId,
      disableStreamOutput: await _clients.preferencesApiPreferences
          .disableStreamOutputFlowSnapshot(),
    );
  }

  void _reloadSettings() {
    setState(() {
      _settingsFuture = _loadSettings();
    });
  }

  Future<void> _selectMemory(String profileId) async {
    await _clients.preferencesUserPreferencesManager.setActiveProfile(
      profileId: profileId,
    );
    widget.onDismiss();
  }

  Future<void> _toggleDisableStreamOutput(_AgentExtraSettingsData data) async {
    await _clients.preferencesApiPreferences.saveDisableStreamOutput(
      isDisabled: !data.disableStreamOutput,
    );
    _reloadSettings();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final popupContainerColor = colorScheme.surfaceContainer;
    return Material(
      color: Colors.transparent,
      child: Stack(
        clipBehavior: Clip.none,
        children: <Widget>[
          Card(
            margin: EdgeInsets.zero,
            color: popupContainerColor,
            elevation: 4,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(8),
            ),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 300, maxHeight: 420),
              child: FutureBuilder<_AgentExtraSettingsData>(
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
                        _MemorySelectorItem(
                          preferenceProfiles: data.preferenceProfiles,
                          currentProfileId: data.currentProfileId,
                          expanded: _memoryExpanded,
                          onExpandedChanged: (expanded) {
                            setState(() {
                              _memoryExpanded = expanded;
                            });
                          },
                          onSelectMemory: _selectMemory,
                          onManageClick: widget.onDismiss,
                          onInfoClick: () => _showInfo('记忆', '选择聊天使用的记忆配置。'),
                        ),
                        _DisableSettingsGroupItem(
                          expanded: _disableExpanded,
                          disableStreamOutput: data.disableStreamOutput,
                          onExpandedChanged: (expanded) {
                            setState(() {
                              _disableExpanded = expanded;
                            });
                          },
                          onToggleDisableStreamOutput: () =>
                              _toggleDisableStreamOutput(data),
                          onInfoClick: () => _showInfo('禁用设置', '管理输出、工具和偏好描述。'),
                          onDisableStreamOutputInfoClick: () =>
                              _showInfo('禁用流式输出', '关闭后回复将不再流式显示。'),
                        ),
                      ],
                    ),
                  );
                },
              ),
            ),
          ),
          if (_infoTitle != null && _infoDescription != null)
            Positioned(
              right: 0,
              bottom: 0,
              child: _InfoPopup(
                title: _infoTitle!,
                description: _infoDescription!,
                onDismiss: () {
                  setState(() {
                    _infoTitle = null;
                    _infoDescription = null;
                  });
                },
              ),
            ),
        ],
      ),
    );
  }

  void _showInfo(String title, String description) {
    setState(() {
      _infoTitle = title;
      _infoDescription = description;
    });
  }
}

class _AgentExtraSettingsData {
  const _AgentExtraSettingsData({
    required this.preferenceProfiles,
    required this.currentProfileId,
    required this.disableStreamOutput,
  });

  final List<core_proxy.PreferenceProfile> preferenceProfiles;
  final String currentProfileId;
  final bool disableStreamOutput;
}

class _MemorySelectorItem extends StatelessWidget {
  const _MemorySelectorItem({
    required this.preferenceProfiles,
    required this.currentProfileId,
    required this.expanded,
    required this.onExpandedChanged,
    required this.onSelectMemory,
    required this.onManageClick,
    required this.onInfoClick,
  });

  final List<core_proxy.PreferenceProfile> preferenceProfiles;
  final String currentProfileId;
  final bool expanded;
  final ValueChanged<bool> onExpandedChanged;
  final ValueChanged<String> onSelectMemory;
  final VoidCallback onManageClick;
  final VoidCallback onInfoClick;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final currentProfileName = preferenceProfiles
        .where((profile) => profile.id == currentProfileId)
        .single
        .name;
    return Column(
      children: <Widget>[
        _HeaderRow(
          icon: Icons.data_object_outlined,
          title: '记忆:',
          value: currentProfileName,
          expanded: expanded,
          onTap: () => onExpandedChanged(!expanded),
          onInfoClick: onInfoClick,
        ),
        if (expanded)
          ColoredBox(
            color: colorScheme.surfaceContainer,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Column(
                children: <Widget>[
                  for (var i = 0; i < preferenceProfiles.length; i++) ...[
                    _MemoryProfileRow(
                      profile: preferenceProfiles[i],
                      selected: preferenceProfiles[i].id == currentProfileId,
                      onTap: () => onSelectMemory(preferenceProfiles[i].id),
                    ),
                    if (i < preferenceProfiles.length - 1)
                      const SizedBox(height: 4),
                  ],
                  InkWell(
                    borderRadius: BorderRadius.circular(4),
                    onTap: onManageClick,
                    child: const SizedBox(
                      height: 30,
                      child: Center(
                        child: Text('管理配置', style: TextStyle(fontSize: 13)),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
      ],
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
    return InkWell(
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
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(
            fontSize: 13,
            color: selected ? colorScheme.primary : colorScheme.onSurface,
            fontWeight: selected ? FontWeight.bold : FontWeight.normal,
          ),
        ),
      ),
    );
  }
}

class _DisableSettingsGroupItem extends StatelessWidget {
  const _DisableSettingsGroupItem({
    required this.expanded,
    required this.disableStreamOutput,
    required this.onExpandedChanged,
    required this.onToggleDisableStreamOutput,
    required this.onInfoClick,
    required this.onDisableStreamOutputInfoClick,
  });

  final bool expanded;
  final bool disableStreamOutput;
  final ValueChanged<bool> onExpandedChanged;
  final VoidCallback onToggleDisableStreamOutput;
  final VoidCallback onInfoClick;
  final VoidCallback onDisableStreamOutputInfoClick;

  @override
  Widget build(BuildContext context) {
    final disabledCount = disableStreamOutput ? 1 : 0;
    return Column(
      children: <Widget>[
        _HeaderRow(
          icon: Icons.block_outlined,
          title: '禁用设置:',
          value: '$disabledCount/3',
          expanded: expanded,
          onTap: () => onExpandedChanged(!expanded),
          onInfoClick: onInfoClick,
        ),
        if (expanded)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: _SimpleToggleSettingItem(
              title: '禁用流式输出',
              icon: Icons.speed_outlined,
              isChecked: disableStreamOutput,
              onToggle: onToggleDisableStreamOutput,
              onInfoClick: onDisableStreamOutputInfoClick,
            ),
          ),
      ],
    );
  }
}

class _SimpleToggleSettingItem extends StatelessWidget {
  const _SimpleToggleSettingItem({
    required this.title,
    required this.icon,
    required this.isChecked,
    required this.onToggle,
    required this.onInfoClick,
  });

  final String title;
  final IconData icon;
  final bool isChecked;
  final VoidCallback onToggle;
  final VoidCallback onInfoClick;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return InkWell(
      onTap: onToggle,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Icon(
                icon,
                size: 16,
                color: isChecked
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
              ),
              _InfoIconButton(onPressed: onInfoClick),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  title,
                  style: const TextStyle(fontSize: 13),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Transform.scale(
                scale: 0.65,
                child: Switch(value: isChecked, onChanged: (_) => onToggle()),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _HeaderRow extends StatelessWidget {
  const _HeaderRow({
    required this.icon,
    required this.title,
    required this.value,
    required this.expanded,
    required this.onTap,
    required this.onInfoClick,
  });

  final IconData icon;
  final String title;
  final String value;
  final bool expanded;
  final VoidCallback onTap;
  final VoidCallback onInfoClick;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return InkWell(
      onTap: onTap,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Icon(
                icon,
                size: 16,
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
              ),
              _InfoIconButton(onPressed: onInfoClick),
              const SizedBox(width: 8),
              Text(title, style: const TextStyle(fontSize: 13)),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  value,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontSize: 13,
                    color: colorScheme.primary,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
              Icon(
                expanded ? Icons.keyboard_arrow_up : Icons.keyboard_arrow_down,
                size: 20,
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _InfoIconButton extends StatelessWidget {
  const _InfoIconButton({required this.onPressed});

  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 24,
      height: 24,
      child: IconButton(
        onPressed: onPressed,
        padding: EdgeInsets.zero,
        iconSize: 16,
        icon: Icon(
          Icons.info_outline,
          color: Theme.of(
            context,
          ).colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
        ),
      ),
    );
  }
}

class _InfoPopup extends StatelessWidget {
  const _InfoPopup({
    required this.title,
    required this.description,
    required this.onDismiss,
  });

  final String title;
  final String description;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      margin: EdgeInsets.zero,
      color: colorScheme.surfaceContainer,
      elevation: 6,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 260),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Expanded(
                    child: Text(
                      title,
                      textAlign: TextAlign.center,
                      style: const TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                  IconButton(
                    onPressed: onDismiss,
                    icon: const Icon(Icons.close, size: 18),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                description,
                style: TextStyle(
                  fontSize: 14,
                  height: 20 / 14,
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
