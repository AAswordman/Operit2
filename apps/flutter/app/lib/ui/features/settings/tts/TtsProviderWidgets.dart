// ignore_for_file: file_names

part of 'TtsSettingsPanel.dart';

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    required this.children,
    this.action,
    this.initiallyExpanded = true,
  });

  final String title;
  final List<Widget> children;
  final Widget? action;
  final bool initiallyExpanded;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Material(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        shape: RoundedRectangleBorder(
          borderRadius: radius,
          side: BorderSide(
            color: colorScheme.outlineVariant.withValues(alpha: 0.18),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: OperitGlassSurface(
          color: Colors.transparent,
          borderRadius: radius,
          material: true,
          clip: false,
          child: ExpansionTile(
            initiallyExpanded: initiallyExpanded,
            tilePadding: const EdgeInsets.symmetric(horizontal: 14),
            childrenPadding: const EdgeInsets.fromLTRB(14, 0, 14, 12),
            shape: RoundedRectangleBorder(borderRadius: radius),
            collapsedShape: RoundedRectangleBorder(borderRadius: radius),
            title: Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    title,
                    style: SettingsControlStyles.sectionTitleTextStyle(
                      context,
                    ),
                  ),
                ),
                ?action,
              ],
            ),
            children: children,
          ),
        ),
      ),
    );
  }
}

class _TtsProviderManager extends StatelessWidget {
  const _TtsProviderManager({
    required this.groups,
    required this.currentConfigId,
    required this.testingConfigId,
    required this.expandedProviderKeys,
    required this.onToggleProviderExpanded,
    required this.onAddVoice,
    required this.onEditProvider,
    required this.onEditConfig,
    required this.onTestConfig,
    required this.onSetCurrent,
  });

  final List<_TtsProviderGroup> groups;
  final String currentConfigId;
  final String? testingConfigId;
  final Set<String> expandedProviderKeys;
  final ValueChanged<String> onToggleProviderExpanded;
  final void Function(_TtsProviderGroup group) onAddVoice;
  final void Function(_TtsProviderGroup group) onEditProvider;
  final void Function(core_proxy.TtsConfig config) onEditConfig;
  final Future<void> Function(core_proxy.TtsConfig config) onTestConfig;
  final Future<void> Function(String id) onSetCurrent;

  @override
  Widget build(BuildContext context) {
    if (groups.isEmpty) {
      return const SizedBox.shrink();
    }
    return Column(
      children: <Widget>[
        for (final group in groups)
          _TtsProviderGroupTile(
            group: group,
            currentConfigId: currentConfigId,
            testingConfigId: testingConfigId,
            expanded: expandedProviderKeys.contains(group.key),
            onToggleExpanded: () => onToggleProviderExpanded(group.key),
            onAddVoice: () => onAddVoice(group),
            onEditProvider: () => onEditProvider(group),
            onEditConfig: onEditConfig,
            onTestConfig: onTestConfig,
            onSetCurrent: onSetCurrent,
          ),
      ],
    );
  }
}

class _TtsProviderGroupTile extends StatelessWidget {
  const _TtsProviderGroupTile({
    required this.group,
    required this.currentConfigId,
    required this.testingConfigId,
    required this.expanded,
    required this.onToggleExpanded,
    required this.onAddVoice,
    required this.onEditProvider,
    required this.onEditConfig,
    required this.onTestConfig,
    required this.onSetCurrent,
  });

  final _TtsProviderGroup group;
  final String currentConfigId;
  final String? testingConfigId;
  final bool expanded;
  final VoidCallback onToggleExpanded;
  final VoidCallback onAddVoice;
  final VoidCallback onEditProvider;
  final void Function(core_proxy.TtsConfig config) onEditConfig;
  final Future<void> Function(core_proxy.TtsConfig config) onTestConfig;
  final Future<void> Function(String id) onSetCurrent;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final currentProvider = group.configs.any(
      (config) => config.id == currentConfigId,
    );
    final radius = BorderRadius.circular(8);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Material(
            color: expanded
                ? colorScheme.surfaceContainerHighest.withValues(alpha: 0.34)
                : Colors.transparent,
            borderRadius: radius,
            child: InkWell(
              borderRadius: radius,
              onTap: onToggleExpanded,
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 8,
                ),
                child: Row(
                  children: <Widget>[
                    Icon(
                      expanded
                          ? Icons.keyboard_arrow_down
                          : Icons.chevron_right,
                      size: 18,
                      color: colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 6),
                    Container(
                      width: 8,
                      height: 8,
                      decoration: BoxDecoration(
                        color: currentProvider
                            ? colorScheme.primary
                            : colorScheme.outlineVariant.withValues(alpha: 0.7),
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          Text(
                            group.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.titleSmall
                                ?.copyWith(fontWeight: FontWeight.w700),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            '${group.providerType} · ${group.configs.length}',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.bodySmall!
                                .copyWith(color: colorScheme.onSurfaceVariant),
                          ),
                        ],
                      ),
                    ),
                    if (expanded) ...<Widget>[
                      TextButton.icon(
                        onPressed: onAddVoice,
                        style: SettingsControlStyles.sectionTextButton(),
                        icon: const Icon(Icons.playlist_add, size: 18),
                        label: const Text('添加'),
                      ),
                      SettingsEntityIconButton(
                        tooltip: '编辑供应商',
                        icon: Icons.edit_outlined,
                        onPressed: onEditProvider,
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
          if (expanded)
            Padding(
              padding: const EdgeInsets.only(left: 24, top: 6),
              child: IntrinsicHeight(
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    Container(
                      width: 1,
                      margin: const EdgeInsets.only(top: 2, bottom: 8),
                      color: colorScheme.outlineVariant.withValues(alpha: 0.34),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: _TtsVoiceList(
                        configs: group.configs,
                        currentConfigId: currentConfigId,
                        testingConfigId: testingConfigId,
                        onEditConfig: onEditConfig,
                        onTestConfig: onTestConfig,
                        onSetCurrent: onSetCurrent,
                      ),
                    ),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _TtsVoiceList extends StatelessWidget {
  const _TtsVoiceList({
    required this.configs,
    required this.currentConfigId,
    required this.testingConfigId,
    required this.onEditConfig,
    required this.onTestConfig,
    required this.onSetCurrent,
  });

  final List<core_proxy.TtsConfig> configs;
  final String currentConfigId;
  final String? testingConfigId;
  final void Function(core_proxy.TtsConfig config) onEditConfig;
  final Future<void> Function(core_proxy.TtsConfig config) onTestConfig;
  final Future<void> Function(String id) onSetCurrent;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (var index = 0; index < configs.length; index++) ...<Widget>[
          if (index > 0) const SizedBox(height: 4),
          _TtsVoiceTile(
            config: configs[index],
            current: configs[index].id == currentConfigId,
            testing: configs[index].id == testingConfigId,
            onEdit: () => onEditConfig(configs[index]),
            onTest: () => onTestConfig(configs[index]),
            onSetCurrent: () {
              onSetCurrent(configs[index].id);
            },
          ),
        ],
      ],
    );
  }
}

class _TtsVoiceTile extends StatelessWidget {
  const _TtsVoiceTile({
    required this.config,
    required this.current,
    required this.testing,
    required this.onSetCurrent,
    required this.onEdit,
    required this.onTest,
  });

  final core_proxy.TtsConfig config;
  final bool current;
  final bool testing;
  final VoidCallback onSetCurrent;
  final VoidCallback onEdit;
  final Future<void> Function() onTest;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: current
          ? colorScheme.primaryContainer.withValues(alpha: 0.24)
          : Colors.transparent,
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: onEdit,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          child: Row(
            children: <Widget>[
              SizedBox(
                width: 16,
                height: 16,
                child: Icon(
                  Icons.graphic_eq_outlined,
                  size: 16,
                  color: current
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      _ttsConfigModelVoiceText(config),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 3),
                    _TtsVoiceMeta(config: config),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              if (testing)
                const SizedBox.square(
                  dimension: 24,
                  child: Center(child: M3LoadingIndicator(size: 24)),
                )
              else
                SettingsEntityIconButton(
                  tooltip: '试听',
                  icon: Icons.volume_up_outlined,
                  onPressed: () {
                    onTest();
                  },
                ),
              const SizedBox(width: 4),
              if (current)
                const SettingsActivePill(label: '全局当前')
              else
                SettingsSetActiveButton(
                  label: '设为全局',
                  onPressed: onSetCurrent,
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _TtsVoiceMeta extends StatelessWidget {
  const _TtsVoiceMeta({required this.config});

  final core_proxy.TtsConfig config;

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.onSurfaceVariant;
    return Wrap(
      spacing: 8,
      runSpacing: 4,
      children: <Widget>[
        _TtsMetaIcon(
          icon: Icons.audiotrack_outlined,
          tooltip: config.responseFormat,
          color: color,
        ),
        _TtsMetaIcon(
          icon: Icons.speed_outlined,
          tooltip: config.speed.toStringAsFixed(2),
          color: color,
        ),
        _TtsMetaIcon(
          icon: Icons.http_outlined,
          tooltip: config.httpMethod,
          color: color,
        ),
      ],
    );
  }
}

class _TtsMetaIcon extends StatelessWidget {
  const _TtsMetaIcon({
    required this.icon,
    required this.tooltip,
    required this.color,
  });

  final IconData icon;
  final String tooltip;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 15, color: color),
          const SizedBox(width: 3),
          Text(
            tooltip,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: color,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}
