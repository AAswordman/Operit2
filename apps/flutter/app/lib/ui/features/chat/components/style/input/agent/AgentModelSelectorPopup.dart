// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';
import 'package:operit2/ui/features/settings/models/SettingsModels.dart';

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

  /// Creates the mutable state for the standalone model selector popup.
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

  /// Initializes model selector settings loading.
  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
  }

  /// Loads model, context, and thinking settings for the popup.
  Future<_AgentModelSelectorData> _loadSettings() {
    return _loadAgentModelSelectorData(_clients);
  }

  /// Refreshes the popup data after a setting changes.
  void _reloadSettings() {
    setState(() {
      _settingsFuture = _loadSettings();
    });
  }

  /// Applies a provider model as the chat model.
  Future<void> _selectModel(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    if (_isDisallowedChatModel(model.id)) {
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

  /// Toggles the thinking mode preference.
  Future<void> _toggleThinking(_AgentModelSelectorData data) async {
    await _clients.preferencesApiPreferences.updateThinkingSettings(
      enableThinkingMode: !data.enableThinkingMode,
      thinkingQualityLevel: null,
    );
    _reloadSettings();
  }

  /// Stores the selected thinking quality level.
  Future<void> _updateThinkingQuality(int level) async {
    await _clients.preferencesApiPreferences.updateThinkingSettings(
      enableThinkingMode: null,
      thinkingQualityLevel: level,
    );
    _reloadSettings();
  }

  /// Navigates to the model settings screen.
  void _openModelSettings() {
    widget.onDismiss();
    final entry = ScreenRouteRegistry.toEntry(
      screen: const SettingsScreenRoute(category: SettingsCategory.model),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  /// Toggles Max Context mode for the active chat model.
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

  /// Builds the model selector popup card.
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
                          showInfoButton: false,
                        ),
                        _MaxContextSettingItem(
                          enabled:
                              data.currentConfig.context.enableMaxContextMode,
                          onToggle: () => _toggleMaxContext(data),
                          onInfoClick: () => _showInfo(
                            'Max模式',
                            'Max Mode（超大上下文模式）开启后将使用 ${_formatContextLength(data.currentConfig.context.maxContextLength)}k 上下文窗口，关闭则使用 ${_formatContextLength(data.currentConfig.context.maxContextLength * 0.4)}k。',
                          ),
                          showInfoButton: false,
                        ),
                        _ModelSelectorItem(
                          popupContainerColor: popupContainerColor,
                          providers: data.providers,
                          currentBinding: data.currentBinding,
                          expanded: true,
                          expandedProviderId: _expandedProviderId,
                          onExpandedChanged: (_) {},
                          onExpandedProviderChanged: (providerId) {
                            setState(() {
                              _expandedProviderId = providerId;
                            });
                          },
                          onSelectModel: _selectModel,
                          onManageClick: _openModelSettings,
                          onInfoClick: () => _showInfo(
                            '模型配置',
                            '在这里选择一个已经配置好的模型，或者点击下方的管理配置去新建或修改模型',
                          ),
                          showChevron: false,
                          showInfoButton: false,
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

  /// Shows an information bubble near the selector.
  void _showInfo(String title, String description) {
    setState(() {
      _infoTitle = title;
      _infoDescription = description;
    });
  }
}

class AgentModelMenuSection extends StatefulWidget {
  const AgentModelMenuSection({
    super.key,
    required this.viewModel,
    required this.onDismiss,
  });

  final ChatViewModel viewModel;
  final VoidCallback onDismiss;

  /// Creates the mutable state for the embedded model menu section.
  @override
  State<AgentModelMenuSection> createState() => _AgentModelMenuSectionState();
}

class _AgentModelMenuSectionState extends State<AgentModelMenuSection> {
  Future<_AgentModelSelectorData>? _settingsFuture;
  String? _expandedProviderId;
  bool _modelSectionExpanded = false;
  bool _modelDropdownExpanded = false;

  GeneratedCoreProxyClients get _clients => widget.viewModel.clients;

  /// Loads the current model selector data when the menu section mounts.
  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
  }

  /// Loads model selector state for the embedded input menu section.
  Future<_AgentModelSelectorData> _loadSettings() {
    return _loadAgentModelSelectorData(_clients);
  }

  /// Refreshes the embedded menu data after changing a model setting.
  void _reloadSettings() {
    setState(() {
      _settingsFuture = _loadSettings();
    });
  }

  /// Selects one configured model for chat from the embedded menu.
  Future<void> _selectModel(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    if (_isDisallowedChatModel(model.id)) {
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
    if (!mounted) {
      return;
    }
    setState(() {
      _modelDropdownExpanded = false;
      _expandedProviderId = null;
      _settingsFuture = _loadSettings();
    });
  }

  /// Toggles thinking mode from the embedded menu.
  Future<void> _toggleThinking(_AgentModelSelectorData data) async {
    await _clients.preferencesApiPreferences.updateThinkingSettings(
      enableThinkingMode: !data.enableThinkingMode,
      thinkingQualityLevel: null,
    );
    _reloadSettings();
  }

  /// Stores thinking quality from the embedded menu.
  Future<void> _updateThinkingQuality(int level) async {
    await _clients.preferencesApiPreferences.updateThinkingSettings(
      enableThinkingMode: null,
      thinkingQualityLevel: level,
    );
    _reloadSettings();
  }

  /// Toggles Max Context mode from the embedded menu.
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

  /// Navigates to the model settings screen from the embedded menu.
  void _openModelSettings() {
    widget.onDismiss();
    final entry = ScreenRouteRegistry.toEntry(
      screen: const SettingsScreenRoute(category: SettingsCategory.model),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  /// Builds the embedded model selector row and its inline list.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return FutureBuilder<_AgentModelSelectorData>(
      future: _settingsFuture,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
        }
        final data = snapshot.data;
        if (data == null) {
          return _SettingsHeaderRow(
            icon: Icons.data_object_outlined,
            title: '模型',
            value: '加载中...',
            expanded: _modelSectionExpanded,
            onTap: () {
              setState(() {
                _modelSectionExpanded = !_modelSectionExpanded;
              });
            },
            onInfoClick: () {},
            showInfoButton: false,
          );
        }
        final embeddedPanelColor = colorScheme.surface.withValues(alpha: 0.42);
        return Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            _SettingsHeaderRow(
              icon: Icons.data_object_outlined,
              title: '模型',
              value: data.currentBinding.modelId,
              expanded: _modelSectionExpanded,
              onTap: () {
                setState(() {
                  _modelSectionExpanded = !_modelSectionExpanded;
                  if (_modelSectionExpanded == false) {
                    _modelDropdownExpanded = false;
                    _expandedProviderId = null;
                  }
                });
              },
              onInfoClick: () {},
              showInfoButton: false,
            ),
            if (_modelSectionExpanded)
              Padding(
                padding: const EdgeInsets.fromLTRB(12, 4, 8, 6),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(6),
                  child: ColoredBox(
                    color: embeddedPanelColor,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        _ModelSelectorItem(
                          popupContainerColor: embeddedPanelColor,
                          headerIcon: Icons.tune_outlined,
                          headerTitle: '选择模型',
                          providers: data.providers,
                          currentBinding: data.currentBinding,
                          expanded: _modelDropdownExpanded,
                          expandedProviderId: _expandedProviderId,
                          onExpandedChanged: (expanded) {
                            setState(() {
                              _modelDropdownExpanded = expanded;
                            });
                          },
                          onExpandedProviderChanged: (providerId) {
                            setState(() {
                              _expandedProviderId = providerId;
                            });
                          },
                          onSelectModel: _selectModel,
                          onManageClick: _openModelSettings,
                          onInfoClick: () {},
                          showInfoButton: false,
                        ),
                        const SizedBox(height: 2),
                        _ThinkingSettingsItem(
                          popupContainerColor: embeddedPanelColor,
                          data: data,
                          onToggleThinkingMode: () => _toggleThinking(data),
                          onThinkingQualityChanged: _updateThinkingQuality,
                          onInfoClick: () {},
                          onThinkingModeInfoClick: () {},
                          onThinkingQualityInfoClick: () {},
                          showInfoButton: false,
                        ),
                        const SizedBox(height: 2),
                        _MaxContextSettingItem(
                          enabled:
                              data.currentConfig.context.enableMaxContextMode,
                          onToggle: () => _toggleMaxContext(data),
                          onInfoClick: () {},
                          showInfoButton: false,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
          ],
        );
      },
    );
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
    this.showInfoButton = true,
  });

  final Color popupContainerColor;
  final _AgentModelSelectorData data;
  final VoidCallback onToggleThinkingMode;
  final ValueChanged<int> onThinkingQualityChanged;
  final VoidCallback onInfoClick;
  final VoidCallback onThinkingModeInfoClick;
  final VoidCallback onThinkingQualityInfoClick;
  final bool showInfoButton;

  /// Creates the mutable state for thinking controls.
  @override
  State<_ThinkingSettingsItem> createState() => _ThinkingSettingsItemState();
}

class _ThinkingSettingsItemState extends State<_ThinkingSettingsItem> {
  bool _expanded = false;
  late double _sliderValue = widget.data.thinkingQualityLevel.toDouble();

  /// Syncs the slider when loaded thinking settings change.
  @override
  void didUpdateWidget(covariant _ThinkingSettingsItem oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.data.thinkingQualityLevel !=
        widget.data.thinkingQualityLevel) {
      _sliderValue = widget.data.thinkingQualityLevel.toDouble();
    }
  }

  /// Builds the expandable thinking settings block.
  @override
  Widget build(BuildContext context) {
    final thinkingTypeText = widget.data.enableThinkingMode ? 'mode' : 'off';
    return Column(
      children: <Widget>[
        _SettingsHeaderRow(
          icon: Icons.psychology,
          title: '思考:',
          value: thinkingTypeText,
          expanded: _expanded,
          onTap: () => setState(() => _expanded = !_expanded),
          onInfoClick: widget.onInfoClick,
          showInfoButton: widget.showInfoButton,
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
                    highlightWhenChecked: true,
                    onToggle: widget.onToggleThinkingMode,
                    onInfoClick: widget.onThinkingModeInfoClick,
                    showInfoButton: widget.showInfoButton,
                  ),
                  if (widget.data.enableThinkingMode)
                    _ThinkingQualitySettingRow(
                      value: _sliderValue,
                      onChanged: (value) {
                        setState(() {
                          _sliderValue = value;
                        });
                      },
                      onChangeEnd: (value) {
                        widget.onThinkingQualityChanged(value.round());
                      },
                      onInfoClick: widget.onThinkingQualityInfoClick,
                      showInfoButton: widget.showInfoButton,
                    ),
                ],
              ),
            ),
          ),
      ],
    );
  }
}

class _ThinkingQualitySettingRow extends StatelessWidget {
  const _ThinkingQualitySettingRow({
    required this.value,
    required this.onChanged,
    required this.onChangeEnd,
    required this.onInfoClick,
    required this.showInfoButton,
  });

  final double value;
  final ValueChanged<double> onChanged;
  final ValueChanged<double> onChangeEnd;
  final VoidCallback onInfoClick;
  final bool showInfoButton;

  /// Builds the thinking quality row and slider as one peer setting.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final selectedLevel = value.round();
    return Column(
      children: <Widget>[
        ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 36),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Row(
              children: <Widget>[
                Icon(
                  Icons.speed_outlined,
                  size: 16,
                  color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                ),
                if (showInfoButton) _InfoIconButton(onPressed: onInfoClick),
                const SizedBox(width: 12),
                Text(
                  '思考程度',
                  style: textTheme.bodySmall,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                const Spacer(),
                Text(
                  selectedLevel.toString(),
                  style: textTheme.bodySmall!.copyWith(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(40, 0, 12, 6),
          child: Column(
            children: <Widget>[
              SliderTheme(
                data: SliderTheme.of(context).copyWith(
                  trackHeight: 16,
                  thumbShape: const RoundSliderThumbShape(
                    enabledThumbRadius: 6,
                    elevation: 0,
                    pressedElevation: 0,
                  ),
                  overlayShape: const RoundSliderOverlayShape(
                    overlayRadius: 10,
                  ),
                  tickMarkShape: const RoundSliderTickMarkShape(
                    tickMarkRadius: 2,
                  ),
                  thumbColor: colorScheme.onSurfaceVariant,
                  activeTrackColor: colorScheme.primary.withValues(alpha: 0.72),
                  inactiveTrackColor: colorScheme.surfaceContainerHighest,
                  activeTickMarkColor: colorScheme.onSurfaceVariant,
                  inactiveTickMarkColor: colorScheme.outlineVariant,
                ),
                child: SizedBox(
                  height: 36,
                  child: Slider(
                    value: value,
                    min: 1,
                    max: 4,
                    divisions: 3,
                    onChanged: onChanged,
                    onChangeEnd: onChangeEnd,
                  ),
                ),
              ),
              Row(
                children: <Widget>[
                  for (var level = 1; level <= 4; level++)
                    Expanded(
                      child: Text(
                        '$level',
                        textAlign: TextAlign.center,
                        style: textTheme.labelSmall?.copyWith(
                          color: selectedLevel == level
                              ? colorScheme.primary
                              : colorScheme.onSurfaceVariant,
                          fontWeight: selectedLevel == level
                              ? FontWeight.w700
                              : FontWeight.normal,
                        ),
                      ),
                    ),
                ],
              ),
            ],
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
    this.showInfoButton = true,
  });

  final bool enabled;
  final VoidCallback onToggle;
  final VoidCallback onInfoClick;
  final bool showInfoButton;

  /// Builds the Max Context switch row.
  @override
  Widget build(BuildContext context) {
    return _SwitchSettingRow(
      icon: Icons.whatshot,
      title: 'Max模式',
      checked: enabled,
      onToggle: onToggle,
      onInfoClick: onInfoClick,
      showInfoButton: showInfoButton,
    );
  }
}

class _ModelSelectorItem extends StatelessWidget {
  const _ModelSelectorItem({
    required this.popupContainerColor,
    this.headerIcon = Icons.data_object_outlined,
    this.headerTitle = '模型:',
    required this.providers,
    required this.currentBinding,
    required this.expanded,
    required this.expandedProviderId,
    required this.onExpandedChanged,
    required this.onExpandedProviderChanged,
    required this.onSelectModel,
    required this.onManageClick,
    required this.onInfoClick,
    this.showChevron = true,
    this.showInfoButton = true,
  });

  final Color popupContainerColor;
  final IconData headerIcon;
  final String headerTitle;
  final List<core_proxy.ProviderProfile> providers;
  final core_proxy.FunctionModelBinding currentBinding;
  final bool expanded;
  final String? expandedProviderId;
  final ValueChanged<bool> onExpandedChanged;
  final ValueChanged<String?> onExpandedProviderChanged;
  final void Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onSelectModel;
  final VoidCallback onManageClick;
  final VoidCallback onInfoClick;
  final bool showChevron;
  final bool showInfoButton;

  /// Builds the model selector header and inline provider list.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      children: <Widget>[
        _SettingsHeaderRow(
          icon: headerIcon,
          title: headerTitle,
          value: currentBinding.modelId,
          expanded: expanded,
          onTap: () => onExpandedChanged(!expanded),
          onInfoClick: onInfoClick,
          showChevron: showChevron,
          showInfoButton: showInfoButton,
        ),
        if (expanded)
          ColoredBox(
            color: popupContainerColor,
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Column(
                children: <Widget>[
                  if (providers.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 12),
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
                  const SizedBox(height: 4),
                  Align(
                    alignment: Alignment.center,
                    child: InkWell(
                      mouseCursor: SystemMouseCursors.click,
                      borderRadius: BorderRadius.circular(4),
                      hoverColor: colorScheme.primary.withValues(alpha: 0.08),
                      splashColor: colorScheme.primary.withValues(alpha: 0.12),
                      highlightColor: colorScheme.primary.withValues(
                        alpha: 0.06,
                      ),
                      onTap: onManageClick,
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 4,
                        ),
                        child: Text(
                          '管理配置',
                          style: textTheme.bodySmall?.copyWith(
                            color: colorScheme.primary,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
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

  /// Builds one provider row and its model choices.
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

  /// Builds one selectable model name row.
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
    this.showInfoButton = true,
  });

  final IconData icon;
  final String title;
  final String value;
  final bool expanded;
  final VoidCallback onTap;
  final VoidCallback onInfoClick;
  final bool showChevron;
  final bool showInfoButton;

  /// Builds a settings header row with a value and optional chevron.
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
              if (showInfoButton) _InfoIconButton(onPressed: onInfoClick),
              const SizedBox(width: 12),
              Text(title, style: textTheme.bodySmall),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  value,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  textAlign: TextAlign.end,
                  style: textTheme.bodySmall!.copyWith(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w600,
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
    this.highlightWhenChecked = false,
    this.showInfoButton = true,
  });

  final IconData icon;
  final String title;
  final bool checked;
  final VoidCallback onToggle;
  final VoidCallback onInfoClick;
  final bool highlightWhenChecked;
  final bool showInfoButton;

  /// Builds a switch row inside a settings block.
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
              if (showInfoButton) _InfoIconButton(onPressed: onInfoClick),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  title,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall?.copyWith(
                    color: checked && highlightWhenChecked
                        ? colorScheme.primary
                        : colorScheme.onSurface,
                    fontWeight: checked && highlightWhenChecked
                        ? FontWeight.w700
                        : FontWeight.normal,
                  ),
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

  /// Builds the compact information icon button.
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

  /// Builds the floating information card.
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

/// Loads model selector data shared by popup and embedded menu rendering.
Future<_AgentModelSelectorData> _loadAgentModelSelectorData(
  GeneratedCoreProxyClients clients,
) async {
  final binding = await clients.preferencesFunctionalConfigManager
      .getModelBindingForFunction(functionType: core_proxy.FunctionType.chat);
  final config = await clients.preferencesModelConfigManager
      .getResolvedModelConfig(
        providerId: binding.providerId,
        modelId: binding.modelId,
      );
  return _AgentModelSelectorData(
    providers: await clients.preferencesModelConfigManager
        .getProviderProfiles(),
    currentBinding: binding,
    currentConfig: config,
    enableThinkingMode: await clients.preferencesApiPreferences
        .enableThinkingModeFlow()
        .first,
    thinkingQualityLevel: await clients.preferencesApiPreferences
        .thinkingQualityLevelFlow()
        .first,
  );
}

/// Returns whether the model is reserved for UI control instead of chat.
bool _isDisallowedChatModel(String modelId) {
  return modelId.toLowerCase().contains('autoglm');
}

/// Formats context length in thousands for menu copy.
String _formatContextLength(double value) {
  if (value % 1 == 0) {
    return value.toInt().toString();
  }
  return value.toStringAsFixed(1);
}
