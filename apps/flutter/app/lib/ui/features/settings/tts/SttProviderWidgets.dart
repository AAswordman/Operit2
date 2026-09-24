// ignore_for_file: file_names

part of 'TtsSettingsPanel.dart';

class _SttProviderManager extends StatelessWidget {
  const _SttProviderManager({
    required this.configs,
    required this.currentConfigId,
    required this.onEdit,
    required this.onDelete,
    required this.onSetCurrent,
  });

  final List<core_proxy.SttConfig> configs;
  final String? currentConfigId;
  final Future<void> Function(core_proxy.SttConfig config) onEdit;
  final Future<void> Function(core_proxy.SttConfig config) onDelete;
  final Future<void> Function(String id) onSetCurrent;

  /// Builds the ordered STT provider configuration list.
  @override
  Widget build(BuildContext context) {
    if (configs.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Center(
          child: Text(
            '尚未配置语音识别供应商',
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      );
    }
    return Column(
      children: <Widget>[
        for (var index = 0; index < configs.length; index++) ...<Widget>[
          if (index > 0) const SizedBox(height: 6),
          _SttProviderTile(
            config: configs[index],
            current: configs[index].id == currentConfigId,
            onEdit: () => onEdit(configs[index]),
            onDelete: () => onDelete(configs[index]),
            onSetCurrent: () => onSetCurrent(configs[index].id),
          ),
        ],
      ],
    );
  }
}

class _SttProviderTile extends StatelessWidget {
  const _SttProviderTile({
    required this.config,
    required this.current,
    required this.onEdit,
    required this.onDelete,
    required this.onSetCurrent,
  });

  final core_proxy.SttConfig config;
  final bool current;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback onSetCurrent;

  /// Builds one editable STT provider row with current-selection controls.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final endpoint = config.endpoint.trim();
    final radius = BorderRadius.circular(12);
    final subtitle = <String>[
      config.providerType,
      if (endpoint.isNotEmpty) endpoint,
      if (config.model.trim().isNotEmpty) config.model.trim(),
    ].join(' · ');

    return Material(
      color: current
          ? colorScheme.primaryContainer.withValues(alpha: 0.16)
          : colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
      shape: RoundedRectangleBorder(
        borderRadius: radius,
        side: BorderSide(
          color: current
              ? colorScheme.primary.withValues(alpha: 0.45)
              : colorScheme.outlineVariant.withValues(alpha: 0.28),
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        borderRadius: radius,
        onTap: onEdit,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: Row(
            children: <Widget>[
              Icon(
                Icons.mic_outlined,
                size: 20,
                color: current
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Text(
                      config.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      subtitle,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              if (current)
                const SettingsActivePill(label: '全局当前')
              else
                SettingsSetActiveButton(
                  label: '设为全局',
                  onPressed: onSetCurrent,
                ),
              const SizedBox(width: 4),
              SettingsEntityIconButton(
                tooltip: '编辑',
                icon: Icons.edit_outlined,
                onPressed: onEdit,
              ),
              SettingsEntityIconButton(
                tooltip: current ? '当前配置不能删除' : '删除',
                icon: Icons.delete_outline,
                onPressed: current ? null : onDelete,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
