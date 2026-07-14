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
        padding: const EdgeInsets.symmetric(vertical: 10),
        child: Text(
          '尚未配置语音识别供应商',
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
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
    return Material(
      color: current
          ? colorScheme.primaryContainer.withValues(alpha: 0.22)
          : colorScheme.surfaceContainerLow.withValues(alpha: 0.24),
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: onEdit,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 8, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Icon(
                    Icons.mic_outlined,
                    size: 19,
                    color: current
                        ? colorScheme.primary
                        : colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      config.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  if (current) const SettingsActivePill(label: '全局当前'),
                ],
              ),
              const SizedBox(height: 5),
              Padding(
                padding: const EdgeInsets.only(left: 29),
                child: Text(
                  endpoint.isEmpty
                      ? config.providerType
                      : '${config.providerType} · $endpoint',
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
              const SizedBox(height: 3),
              Padding(
                padding: const EdgeInsets.only(left: 29),
                child: Row(
                  children: <Widget>[
                    Icon(
                      Icons.model_training_outlined,
                      size: 15,
                      color: colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 5),
                    Expanded(
                      child: Text(
                        config.model,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.labelMedium
                            ?.copyWith(color: colorScheme.onSurfaceVariant),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 6),
              Align(
                alignment: Alignment.centerRight,
                child: Wrap(
                  spacing: 4,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: <Widget>[
                    SettingsEntityIconButton(
                      tooltip: '编辑 STT 供应商',
                      icon: Icons.edit_outlined,
                      onPressed: onEdit,
                    ),
                    SettingsEntityIconButton(
                      tooltip: current ? '当前配置不能删除' : '删除 STT 供应商',
                      icon: Icons.delete_outline,
                      onPressed: current ? null : onDelete,
                    ),
                    if (!current)
                      SettingsSetActiveButton(
                        label: '设为全局',
                        onPressed: onSetCurrent,
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
