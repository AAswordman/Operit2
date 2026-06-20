// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../viewmodel/ChatViewModel.dart';

class AgentModelSelectorPopup extends StatefulWidget {
  const AgentModelSelectorPopup({
    super.key,
    required this.viewModel,
    required this.onDismiss,
    required this.onModelChanged,
  });

  final ChatViewModel viewModel;
  final VoidCallback onDismiss;
  final ValueChanged<String> onModelChanged;

  @override
  State<AgentModelSelectorPopup> createState() =>
      _AgentModelSelectorPopupState();
}

class _AgentModelSelectorPopupState extends State<AgentModelSelectorPopup> {
  Future<_AgentModelSelectorData>? _settingsFuture;
  String? _expandedProviderId;
  String? _infoTitle;
  String? _infoDescription;

  GeneratedCoreProxyClients get _clients => widget.viewModel.clients;

  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
  }

  Future<_AgentModelSelectorData> _loadSettings() async {
    await _clients.preferencesModelConfigManager.initializeIfNeeded();
    await _clients.preferencesFunctionalConfigManager.initializeIfNeeded();
    final binding = await _clients.preferencesFunctionalConfigManager
        .getModelBindingForFunction(functionType: core_proxy.FunctionType.chat);
    final config = await _clients.preferencesModelConfigManager
        .getResolvedModelConfig(
          providerId: binding.providerId,
          modelId: binding.modelId,
        );
    return _AgentModelSelectorData(
      providers: await _clients.preferencesModelConfigManager
          .getProviderProfiles(),
      currentBinding: binding,
      currentConfig: config,
      enableThinkingMode: await _clients.preferencesApiPreferences
          .enableThinkingModeFlowSnapshot(),
      thinkingQualityLevel: await _clients.preferencesApiPreferences
          .thinkingQualityLevelFlowSnapshot(),
    );
  }

  void _reloadSettings() {
    setState(() {
      _settingsFuture = _loadSettings();
    });
  }

  Future<void> _selectModel(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    if (model.id.toLowerCase().contains('autoglm')) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('禁止使用autoglm作为对话主模型。对话模型和ui控制模型是分离的，请选择任意一个别的聪明的大模型。'),
        ),
      );
      return;
    }
    await _clients.preferencesFunctionalConfigManager.setModelForFunction(
      functionType: core_proxy.FunctionType.chat,
      providerId: provider.id,
      modelId: model.id,
    );
    widget.onModelChanged(model.id);
    widget.onDismiss();
  }

  Future<void> _toggleThinking(_AgentModelSelectorData data) async {
    await _clients.preferencesApiPreferences.updateThinkingSettings(
      enableThinkingMode: !data.enableThinkingMode,
      thinkingQualityLevel: null,
    );
    _reloadSettings();
  }

  Future<void> _updateThinkingQuality(int level) async {
    await _clients.preferencesApiPreferences.updateThinkingSettings(
      enableThinkingMode: null,
      thinkingQualityLevel: level,
    );
    _reloadSettings();
  }

  Future<void> _toggleMaxContext(_AgentModelSelectorData data) async {
    final config = data.currentConfig;
    await _clients.preferencesModelConfigManager.updateContextForModel(
      providerId: config.providerId,
      modelId: config.modelId,
      context: core_proxy.ModelContextSpec(
        maxContextLength: config.context.maxContextLength,
        enableMaxContextMode: !config.context.enableMaxContextMode,
      ),
    );
    _reloadSettings();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
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
              child: FutureBuilder<_AgentModelSelectorData>(
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
                        _ThinkingSettingsItem(
                          popupContainerColor: popupContainerColor,
                          data: data,
                          onToggleThinkingMode: () => _toggleThinking(data),
                          onThinkingQualityChanged: _updateThinkingQuality,
                          onInfoClick: () => _showInfo('思考设置', '管理思考模式'),
                          onThinkingModeInfoClick: () => _showInfo(
                            '思考模式',
                            '目前支持Gemini、Qwen3、Claude、豆包、NVIDIA、硅基流动和MNN本地模型，能够启用内置的思考。',
                          ),
                          onThinkingQualityInfoClick: () => _showInfo(
                            '思考质量',
                            '仅在思考模式下生效，共 4 挡，数值越高思考越深，1 为自动。',
                          ),
                        ),
                        _MaxContextSettingItem(
                          enabled:
                              data.currentConfig.context.enableMaxContextMode,
                          onToggle: () => _toggleMaxContext(data),
                          onInfoClick: () => _showInfo(
                            'Max模式',
                            'Max Mode（超大上下文模式）开启后将使用 ${_formatContextLength(data.currentConfig.context.maxContextLength)}k 上下文窗口，关闭则使用 ${_formatContextLength(data.currentConfig.context.maxContextLength * 0.4)}k。',
                          ),
                        ),
                        _ModelSelectorItem(
                          popupContainerColor: popupContainerColor,
                          providers: data.providers,
                          currentBinding: data.currentBinding,
                          expandedProviderId: _expandedProviderId,
                          onExpandedProviderChanged: (providerId) {
                            setState(() {
                              _expandedProviderId = providerId;
                            });
                          },
                          onSelectModel: _selectModel,
                          onManageClick: widget.onDismiss,
                          onInfoClick: () => _showInfo(
                            '模型配置',
                            '在这里选择一个已经配置好的模型，或者点击下方的管理配置去新建或修改模型',
                          ),
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

class _AgentModelSelectorData {
  const _AgentModelSelectorData({
    required this.providers,
    required this.currentBinding,
    required this.currentConfig,
    required this.enableThinkingMode,
    required this.thinkingQualityLevel,
  });

  final List<core_proxy.ProviderProfile> providers;
  final core_proxy.FunctionModelBinding currentBinding;
  final core_proxy.ResolvedModelConfig currentConfig;
  final bool enableThinkingMode;
  final int thinkingQualityLevel;
}

class _ThinkingSettingsItem extends StatefulWidget {
  const _ThinkingSettingsItem({
    required this.popupContainerColor,
    required this.data,
    required this.onToggleThinkingMode,
    required this.onThinkingQualityChanged,
    required this.onInfoClick,
    required this.onThinkingModeInfoClick,
    required this.onThinkingQualityInfoClick,
  });

  final Color popupContainerColor;
  final _AgentModelSelectorData data;
  final VoidCallback onToggleThinkingMode;
  final ValueChanged<int> onThinkingQualityChanged;
  final VoidCallback onInfoClick;
  final VoidCallback onThinkingModeInfoClick;
  final VoidCallback onThinkingQualityInfoClick;

  @override
  State<_ThinkingSettingsItem> createState() => _ThinkingSettingsItemState();
}

class _ThinkingSettingsItemState extends State<_ThinkingSettingsItem> {
  bool _expanded = false;
  late double _sliderValue = widget.data.thinkingQualityLevel.toDouble();

  @override
  void didUpdateWidget(covariant _ThinkingSettingsItem oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sliderValue = widget.data.thinkingQualityLevel.toDouble();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final thinkingTypeText = widget.data.enableThinkingMode ? 'mode' : 'off';
    return Column(
      children: <Widget>[
        _SettingsHeaderRow(
          icon: Icons.psychology,
          title: '思考设置:',
          value: thinkingTypeText,
          expanded: _expanded,
          onTap: () => setState(() => _expanded = !_expanded),
          onInfoClick: widget.onInfoClick,
        ),
        if (_expanded)
          ColoredBox(
            color: widget.popupContainerColor,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Column(
                children: <Widget>[
                  _SwitchSettingRow(
                    icon: widget.data.enableThinkingMode
                        ? Icons.psychology
                        : Icons.psychology_outlined,
                    title: '思考模式',
                    checked: widget.data.enableThinkingMode,
                    onToggle: widget.onToggleThinkingMode,
                    onInfoClick: widget.onThinkingModeInfoClick,
                  ),
                  if (widget.data.enableThinkingMode)
                    Padding(
                      padding: const EdgeInsets.fromLTRB(28, 4, 8, 8),
                      child: Column(
                        children: <Widget>[
                          Row(
                            children: <Widget>[
                              Icon(
                                Icons.speed_outlined,
                                size: 16,
                                color: colorScheme.onSurfaceVariant.withValues(
                                  alpha: 0.7,
                                ),
                              ),
                              _InfoIconButton(
                                onPressed: widget.onThinkingQualityInfoClick,
                              ),
                              const Text('思考质量'),
                              const Spacer(),
                              Text(
                                _sliderValue.round().toString(),
                                style: textTheme.bodySmall!.copyWith(
                                  color: colorScheme.primary,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                            ],
                          ),
                          Slider(
                            value: _sliderValue,
                            min: 1,
                            max: 4,
                            divisions: 3,
                            onChanged: (value) {
                              setState(() {
                                _sliderValue = value;
                              });
                            },
                            onChangeEnd: (value) {
                              widget.onThinkingQualityChanged(value.round());
                            },
                          ),
                        ],
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

class _MaxContextSettingItem extends StatelessWidget {
  const _MaxContextSettingItem({
    required this.enabled,
    required this.onToggle,
    required this.onInfoClick,
  });

  final bool enabled;
  final VoidCallback onToggle;
  final VoidCallback onInfoClick;

  @override
  Widget build(BuildContext context) {
    return _SwitchSettingRow(
      icon: Icons.whatshot,
      title: 'Max模式',
      checked: enabled,
      onToggle: onToggle,
      onInfoClick: onInfoClick,
    );
  }
}

class _ModelSelectorItem extends StatelessWidget {
  const _ModelSelectorItem({
    required this.popupContainerColor,
    required this.providers,
    required this.currentBinding,
    required this.expandedProviderId,
    required this.onExpandedProviderChanged,
    required this.onSelectModel,
    required this.onManageClick,
    required this.onInfoClick,
  });

  final Color popupContainerColor;
  final List<core_proxy.ProviderProfile> providers;
  final core_proxy.FunctionModelBinding currentBinding;
  final String? expandedProviderId;
  final ValueChanged<String?> onExpandedProviderChanged;
  final void Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onSelectModel;
  final VoidCallback onManageClick;
  final VoidCallback onInfoClick;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      children: <Widget>[
        _SettingsHeaderRow(
          icon: Icons.data_object_outlined,
          title: '模型:',
          value: currentBinding.modelId,
          expanded: true,
          onTap: () {},
          onInfoClick: onInfoClick,
          showChevron: false,
        ),
        ColoredBox(
          color: popupContainerColor,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Column(
              children: <Widget>[
                if (providers.isEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 4,
                    ),
                    child: Text(
                      '没有可用的模型',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                for (var i = 0; i < providers.length; i++) ...[
                  _ModelProviderRow(
                    provider: providers[i],
                    selected:
                        providers[i].id == currentBinding.providerId &&
                        providers[i].models.any(
                          (model) => model.id == currentBinding.modelId,
                        ),
                    selectedProviderId: currentBinding.providerId,
                    selectedModelId: currentBinding.modelId,
                    expanded: expandedProviderId == providers[i].id,
                    onExpandedChanged: onExpandedProviderChanged,
                    onSelectModel: onSelectModel,
                  ),
                  if (i < providers.length - 1) const SizedBox(height: 4),
                ],
                InkWell(
                  borderRadius: BorderRadius.circular(4),
                  onTap: onManageClick,
                  child: SizedBox(
                    height: 30,
                    child: Center(
                      child: Text('管理配置', style: textTheme.bodySmall),
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

class _ModelProviderRow extends StatelessWidget {
  const _ModelProviderRow({
    required this.provider,
    required this.selected,
    required this.selectedProviderId,
    required this.selectedModelId,
    required this.expanded,
    required this.onExpandedChanged,
    required this.onSelectModel,
  });

  final core_proxy.ProviderProfile provider;
  final bool selected;
  final String selectedProviderId;
  final String selectedModelId;
  final bool expanded;
  final ValueChanged<String?> onExpandedChanged;
  final void Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onSelectModel;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final models = provider.models;
    final hasMultipleModels = models.length > 1;
    return Column(
      children: <Widget>[
        InkWell(
          borderRadius: BorderRadius.circular(4),
          onTap: () {
            if (hasMultipleModels) {
              onExpandedChanged(expanded ? null : provider.id);
            } else if (models.isNotEmpty) {
              onSelectModel(provider, models.first);
            } else {
              onExpandedChanged(expanded ? null : provider.id);
            }
          },
          child: Container(
            decoration: BoxDecoration(
              color: selected
                  ? colorScheme.primary.withValues(alpha: 0.10)
                  : Colors.transparent,
              borderRadius: BorderRadius.circular(4),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            child: Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    provider.name,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.bodySmall!.copyWith(
                      color: selected
                          ? colorScheme.primary
                          : colorScheme.onSurface,
                      fontWeight: selected
                          ? FontWeight.bold
                          : FontWeight.normal,
                    ),
                  ),
                ),
                const SizedBox(width: 4),
                if (hasMultipleModels) ...[
                  Text(
                    '${models.length}个模型',
                    style: textTheme.labelSmall!.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                  Icon(
                    expanded
                        ? Icons.keyboard_arrow_up
                        : Icons.keyboard_arrow_down,
                    size: 16,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ] else
                  Flexible(
                    child: Text(
                      provider.models.isEmpty
                          ? provider.providerTypeId
                          : provider.models.first.id,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.labelSmall!.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
        if (hasMultipleModels && expanded)
          ColoredBox(
            color: colorScheme.surfaceContainer,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 4, 8, 4),
              child: Column(
                children: <Widget>[
                  for (var index = 0; index < models.length; index++) ...[
                    _ModelNameRow(
                      modelName: models[index].id,
                      selected:
                          provider.id == selectedProviderId &&
                          models[index].id == selectedModelId,
                      onTap: () => onSelectModel(provider, models[index]),
                    ),
                    if (index < models.length - 1) const SizedBox(height: 2),
                  ],
                ],
              ),
            ),
          ),
      ],
    );
  }
}

class _ModelNameRow extends StatelessWidget {
  const _ModelNameRow({
    required this.modelName,
    required this.selected,
    required this.onTap,
  });

  final String modelName;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return InkWell(
      borderRadius: BorderRadius.circular(4),
      onTap: onTap,
      child: Container(
        width: double.infinity,
        decoration: BoxDecoration(
          color: selected
              ? colorScheme.primary.withValues(alpha: 0.15)
              : Colors.transparent,
          borderRadius: BorderRadius.circular(4),
        ),
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Text(
          modelName,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
          style: textTheme.bodySmall!.copyWith(
            color: selected ? colorScheme.primary : colorScheme.onSurface,
            fontWeight: selected ? FontWeight.bold : FontWeight.normal,
          ),
        ),
      ),
    );
  }
}

class _SettingsHeaderRow extends StatelessWidget {
  const _SettingsHeaderRow({
    required this.icon,
    required this.title,
    required this.value,
    required this.expanded,
    required this.onTap,
    required this.onInfoClick,
    this.showChevron = true,
  });

  final IconData icon;
  final String title;
  final String value;
  final bool expanded;
  final VoidCallback onTap;
  final VoidCallback onInfoClick;
  final bool showChevron;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
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
              Text(title, style: textTheme.bodySmall),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  value,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall!.copyWith(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
              if (showChevron)
                Icon(
                  expanded
                      ? Icons.keyboard_arrow_up
                      : Icons.keyboard_arrow_down,
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

class _SwitchSettingRow extends StatelessWidget {
  const _SwitchSettingRow({
    required this.icon,
    required this.title,
    required this.checked,
    required this.onToggle,
    required this.onInfoClick,
  });

  final IconData icon;
  final String title;
  final bool checked;
  final VoidCallback onToggle;
  final VoidCallback onInfoClick;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
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
                color: checked
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
              ),
              _InfoIconButton(onPressed: onInfoClick),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  title,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall,
                ),
              ),
              Transform.scale(
                scale: 0.65,
                child: Switch(value: checked, onChanged: (_) => onToggle()),
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
    final textTheme = Theme.of(context).textTheme;
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
                      style: textTheme.titleMedium!.copyWith(
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
                style: textTheme.bodyMedium!.copyWith(
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

String _formatContextLength(double value) {
  if (value % 1 == 0) {
    return value.toInt().toString();
  }
  return value.toStringAsFixed(1);
}
