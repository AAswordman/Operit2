// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../common/CharacterAvatar.dart';
import '../../../../../../common/icons/MaterialIconNameResolver.dart';
import '../../../../viewmodel/ChatViewModel.dart';

class AgentInputMenuPopup extends StatefulWidget {
  const AgentInputMenuPopup({
    super.key,
    required this.viewModel,
    required this.currentChatId,
    required this.currentCharacterCardName,
    required this.currentCharacterCardAvatarUri,
    required this.onDismiss,
    this.leadingChildren = const <Widget>[],
  });

  final ChatViewModel viewModel;
  final String? currentChatId;
  final String? currentCharacterCardName;
  final String? currentCharacterCardAvatarUri;
  final VoidCallback onDismiss;
  final List<Widget> leadingChildren;

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
      version = await _clients.application
          .inputMenuToggleBridge()
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
    final inputMenuToggleBridge = _clients.application.inputMenuToggleBridge();
    _observedPluginChangeVersion = await inputMenuToggleBridge.changeVersion();
    final pluginToggles = await inputMenuToggleBridge
        .createToggleDefinitionsForFlutter(
          chatId: widget.currentChatId,
          featureStates: const <String, bool>{},
          runtime: 'main',
        );
    return _AgentInputMenuData(
      enableMemoryAutoUpdate: await _clients.preferencesApiPreferences
          .enableMemoryAutoUpdateFlow()
          .first,
      permissionMode: await _clients.permissionsToolPermissionSystem
          .getAiPermissionMode(),
      disableStreamOutput: await _clients.preferencesApiPreferences
          .disableStreamOutputFlow()
          .first,
      disableUserPreferenceDescription: await _clients.preferencesApiPreferences
          .disableUserPreferenceDescriptionFlow()
          .first,
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
    await _clients.permissionsToolPermissionSystem.saveAiPermissionMode(
      mode: mode.permissionMode,
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
    await _clients.application.inputMenuToggleBridge().triggerToggleForFlutter(
      toggleId: toggle.id,
      chatId: widget.currentChatId,
      runtime: 'main',
    );
    _reloadSettings();
  }

  /// Builds the input menu popup with optional leading entries.
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
                    _ChatSessionSummarySection(
                      viewModel: widget.viewModel,
                      currentCharacterCardName: widget.currentCharacterCardName,
                      currentCharacterCardAvatarUri:
                          widget.currentCharacterCardAvatarUri,
                      onDismiss: widget.onDismiss,
                    ),
                    const Divider(height: 1),
                    ...widget.leadingChildren,
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
                          icon: Icons.assignment_ind_outlined,
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
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 3,
                          ),
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

class _ChatSessionSummarySection extends StatefulWidget {
  const _ChatSessionSummarySection({
    required this.viewModel,
    required this.currentCharacterCardName,
    required this.currentCharacterCardAvatarUri,
    required this.onDismiss,
  });

  final ChatViewModel viewModel;
  final String? currentCharacterCardName;
  final String? currentCharacterCardAvatarUri;
  final VoidCallback onDismiss;

  /// Creates the state for the current chat summary section.
  @override
  State<_ChatSessionSummarySection> createState() =>
      _ChatSessionSummarySectionState();
}

class _ChatSessionSummarySectionState
    extends State<_ChatSessionSummarySection> {
  StreamSubscription<int>? _currentWindowSizeSubscription;
  StreamSubscription<int>? _inputTokenCountSubscription;
  StreamSubscription<int>? _outputTokenCountSubscription;
  Future<double>? _maxContextLengthFuture;
  int _currentWindowSize = 0;
  int _inputTokenCount = 0;
  int _outputTokenCount = 0;
  bool _statsExpanded = false;

  GeneratedCoreProxyClients get _clients => widget.viewModel.clients;

  /// Starts token statistic streams and loads the active model context size.
  @override
  void initState() {
    super.initState();
    _maxContextLengthFuture = _loadMaxContextLength();
    _subscribeToTokenStatistics();
  }

  /// Opens the character card selector above the chat surface.
  void _showCharacterCardSelector() {
    final dialogFuture = showDialog<void>(
      context: context,
      builder: (context) {
        return _CharacterCardSelectorDialog(viewModel: widget.viewModel);
      },
    );
    widget.onDismiss();
    unawaited(dialogFuture);
  }

  /// Loads the context limit of the currently selected chat model.
  Future<double> _loadMaxContextLength() async {
    final binding = await _clients.preferencesFunctionalConfigManager
        .getModelBindingForFunction(functionType: core_proxy.FunctionType.chat);
    final config = await _clients.preferencesModelConfigManager
        .getResolvedModelConfig(
          providerId: binding.providerId,
          modelId: binding.modelId,
        );
    return config.context.maxContextLength;
  }

  /// Subscribes to the runtime-owned token statistic streams.
  void _subscribeToTokenStatistics() {
    final holder = _clients.chatRuntimeHolderMain;
    _currentWindowSizeSubscription = holder.currentWindowSizeFlow().listen((
      value,
    ) {
      if (mounted) {
        setState(() {
          _currentWindowSize = value;
        });
      }
    });
    _inputTokenCountSubscription = holder.inputTokenCountFlow().listen((value) {
      if (mounted) {
        setState(() {
          _inputTokenCount = value;
        });
      }
    });
    _outputTokenCountSubscription = holder.outputTokenCountFlow().listen((
      value,
    ) {
      if (mounted) {
        setState(() {
          _outputTokenCount = value;
        });
      }
    });
  }

  /// Stops token statistic streams when the menu closes.
  @override
  void dispose() {
    unawaited(_currentWindowSizeSubscription?.cancel());
    unawaited(_inputTokenCountSubscription?.cancel());
    unawaited(_outputTokenCountSubscription?.cancel());
    super.dispose();
  }

  /// Builds the current character and token summary rows.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return FutureBuilder<double>(
      future: _maxContextLengthFuture,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
        }
        final maxContextLength = snapshot.data;
        final maxContextTokens = maxContextLength == null
            ? null
            : (maxContextLength * 1024).round();
        final contextUsagePercentage = maxContextTokens == null
            ? null
            : _contextUsagePercentage(maxContextTokens);
        final totalTokenCount = _inputTokenCount + _outputTokenCount;
        return Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            InkWell(
              onTap: _showCharacterCardSelector,
              child: ConstrainedBox(
                constraints: const BoxConstraints(minHeight: 48),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 6,
                  ),
                  child: Row(
                    children: <Widget>[
                      SizedBox(
                        width: 32,
                        height: 32,
                        child: ClipOval(
                          child: CharacterAvatarImage(
                            avatarUri: widget.currentCharacterCardAvatarUri,
                            fit: BoxFit.cover,
                          ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            Text(
                              '当前角色卡',
                              style: textTheme.labelSmall?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                            Text(
                              widget.currentCharacterCardName ?? '未绑定',
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: textTheme.bodySmall?.copyWith(
                                color: colorScheme.onSurface,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ],
                        ),
                      ),
                      Icon(
                        Icons.chevron_right,
                        size: 20,
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ],
                  ),
                ),
              ),
            ),
            InkWell(
              onTap: () {
                setState(() {
                  _statsExpanded = !_statsExpanded;
                });
              },
              child: ConstrainedBox(
                constraints: const BoxConstraints(minHeight: 40),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  child: Row(
                    children: <Widget>[
                      Icon(
                        Icons.data_usage_outlined,
                        size: 17,
                        color: colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 12),
                      Text('统计', style: textTheme.bodySmall),
                      const Spacer(),
                      Text(
                        contextUsagePercentage == null
                            ? '加载中...'
                            : '${contextUsagePercentage.toStringAsFixed(0)}%',
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.primary,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(width: 6),
                      Icon(
                        _statsExpanded
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
            if (_statsExpanded)
              ColoredBox(
                color: colorScheme.surface.withValues(alpha: 0.42),
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(40, 6, 12, 8),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      _ChatStatValueRow(
                        label: '上下文窗口',
                        value: _contextWindowLabel(
                          currentWindowSize: _currentWindowSize,
                          maxContextTokens: maxContextTokens,
                        ),
                      ),
                      _ChatStatValueRow(
                        label: '输入 Token',
                        value: _formatTokenCount(_inputTokenCount),
                      ),
                      _ChatStatValueRow(
                        label: '输出 Token',
                        value: _formatTokenCount(_outputTokenCount),
                      ),
                      _ChatStatValueRow(
                        label: '总 Token',
                        value: _formatTokenCount(totalTokenCount),
                        highlighted: true,
                      ),
                    ],
                  ),
                ),
              ),
          ],
        );
      },
    );
  }

  /// Calculates the current context usage percentage for the summary row.
  double _contextUsagePercentage(int maxContextTokens) {
    if (maxContextTokens <= 0) {
      return 0;
    }
    return (_currentWindowSize / maxContextTokens * 100).clamp(0, 999);
  }

  /// Formats the context window values used in the expanded statistics.
  String _contextWindowLabel({
    required int currentWindowSize,
    required int? maxContextTokens,
  }) {
    final maxTokens = maxContextTokens;
    if (maxTokens == null) {
      return '加载中...';
    }
    return '${_formatTokenCount(currentWindowSize)} / ${_formatTokenCount(maxTokens)}';
  }

  /// Formats token counts with compact thousands separators.
  String _formatTokenCount(int value) {
    return value.toString().replaceAllMapped(
      RegExp(r'(?<=\d)(?=(\d{3})+$)'),
      (match) => ',',
    );
  }
}

class _CharacterCardSelectorDialog extends StatefulWidget {
  const _CharacterCardSelectorDialog({required this.viewModel});

  final ChatViewModel viewModel;

  /// Creates the state for the character card selector dialog.
  @override
  State<_CharacterCardSelectorDialog> createState() =>
      _CharacterCardSelectorDialogState();
}

class _CharacterCardSelectorDialogState
    extends State<_CharacterCardSelectorDialog> {
  StreamSubscription<core_proxy.ActivePrompt>? _activePromptSubscription;
  Future<List<core_proxy.CharacterCard>>? _cardsFuture;
  core_proxy.ActivePrompt? _activePrompt;
  String? _switchingCharacterCardId;

  GeneratedCoreProxyClients get _clients => widget.viewModel.clients;

  /// Starts loading cards and observing the active prompt.
  @override
  void initState() {
    super.initState();
    _cardsFuture = _loadCharacterCards();
    _activePromptSubscription = _clients.preferencesActivePromptManager
        .activePromptFlow()
        .listen((prompt) {
          if (mounted) {
            setState(() {
              _activePrompt = prompt;
            });
          }
        });
    unawaited(_loadActivePrompt());
  }

  /// Loads the character cards shown in the selector dialog.
  Future<List<core_proxy.CharacterCard>> _loadCharacterCards() {
    return _clients.preferencesCharacterCardManager.getAllCharacterCards();
  }

  /// Reads the active prompt for the initial selection marker.
  Future<void> _loadActivePrompt() async {
    final prompt = await _clients.preferencesActivePromptManager
        .getActivePrompt();
    if (mounted) {
      setState(() {
        _activePrompt = prompt;
      });
    }
  }

  /// Switches the runtime to the selected character card and closes the dialog.
  Future<void> _selectCharacterCard(core_proxy.CharacterCard card) async {
    if (_switchingCharacterCardId != null) {
      return;
    }
    setState(() {
      _switchingCharacterCardId = card.id;
    });
    try {
      await _clients.chatRuntimeHolderMain.switchActiveCharacterCardTarget(
        characterCardId: card.id,
      );
      if (mounted) {
        Navigator.of(context).pop();
      }
    } finally {
      if (mounted) {
        setState(() {
          _switchingCharacterCardId = null;
        });
      }
    }
  }

  /// Returns the active card id when the runtime targets a character card.
  String? get _activeCharacterCardId {
    final prompt = _activePrompt;
    return prompt?.tag == 'CharacterCard' ? prompt?.id : null;
  }

  /// Stops the active prompt subscription when the dialog closes.
  @override
  void dispose() {
    unawaited(_activePromptSubscription?.cancel());
    super.dispose();
  }

  /// Builds the character card selector dialog.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Dialog(
      insetPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(
          maxWidth: 360,
          minHeight: 420,
          maxHeight: 420,
        ),
        child: FutureBuilder<List<core_proxy.CharacterCard>>(
          future: _cardsFuture,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
            }
            final cards = snapshot.data;
            if (cards == null) {
              return const SizedBox.expand(
                child: Center(
                  child: SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                ),
              );
            }
            return Column(
              children: <Widget>[
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 8, 8, 4),
                  child: Row(
                    children: <Widget>[
                      Expanded(
                        child: Text(
                          '切换角色卡',
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                      ),
                      Text(
                        '${cards.length} 个',
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(width: 2),
                      IconButton(
                        tooltip: '关闭',
                        onPressed: () => Navigator.of(context).pop(),
                        icon: const Icon(Icons.close, size: 18),
                        visualDensity: VisualDensity.compact,
                      ),
                    ],
                  ),
                ),
                const Divider(height: 1),
                if (cards.isEmpty)
                  Expanded(
                    child: Align(
                      alignment: Alignment.topLeft,
                      child: Padding(
                        padding: const EdgeInsets.fromLTRB(16, 18, 16, 20),
                        child: Text(
                          '暂无角色卡',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                    ),
                  )
                else
                  Expanded(
                    child: Scrollbar(
                      child: ListView.separated(
                        padding: const EdgeInsets.fromLTRB(8, 4, 8, 8),
                        itemCount: cards.length,
                        separatorBuilder: (context, index) => Divider(
                          height: 1,
                          color: colorScheme.outlineVariant.withValues(
                            alpha: 0.45,
                          ),
                        ),
                        itemBuilder: (context, index) {
                          final card = cards[index];
                          return _CharacterCardOption(
                            card: card,
                            active: card.id == _activeCharacterCardId,
                            switching: card.id == _switchingCharacterCardId,
                            enabled: _switchingCharacterCardId == null,
                            onTap: () => _selectCharacterCard(card),
                          );
                        },
                      ),
                    ),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _CharacterCardOption extends StatelessWidget {
  const _CharacterCardOption({
    required this.card,
    required this.active,
    required this.switching,
    required this.enabled,
    required this.onTap,
  });

  final core_proxy.CharacterCard card;
  final bool active;
  final bool switching;
  final bool enabled;
  final VoidCallback onTap;

  /// Builds one selectable character card row.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final titleColor = enabled
        ? colorScheme.onSurface
        : colorScheme.onSurfaceVariant.withValues(alpha: 0.65);
    final descriptionColor = enabled
        ? colorScheme.onSurfaceVariant
        : colorScheme.onSurfaceVariant.withValues(alpha: 0.5);
    return InkWell(
      onTap: enabled ? onTap : null,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(10, 7, 8, 7),
        child: Row(
          children: <Widget>[
            SizedBox(
              width: 28,
              height: 28,
              child: ClipOval(
                child: CharacterAvatarImage(
                  avatarUri: card.avatarUri,
                  fit: BoxFit.cover,
                ),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    card.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.bodySmall?.copyWith(
                      color: active ? colorScheme.primary : titleColor,
                      fontWeight: active ? FontWeight.w700 : FontWeight.w600,
                    ),
                  ),
                  if (card.description.isNotEmpty)
                    Text(
                      card.description,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.labelSmall?.copyWith(
                        color: descriptionColor,
                      ),
                    ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            SizedBox(
              width: 20,
              height: 20,
              child: switching
                  ? const Padding(
                      padding: EdgeInsets.all(2),
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : Icon(
                      active ? Icons.check : Icons.circle_outlined,
                      size: active ? 18 : 16,
                      color: active
                          ? colorScheme.primary
                          : colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.45,
                            ),
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ChatStatValueRow extends StatelessWidget {
  const _ChatStatValueRow({
    required this.label,
    required this.value,
    this.highlighted = false,
  });

  final String label;
  final String value;
  final bool highlighted;

  /// Builds one value row in the expanded chat statistics section.
  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(label, style: textTheme.labelSmall)),
          Text(
            value,
            style: textTheme.labelSmall?.copyWith(
              color: highlighted
                  ? colorScheme.primary
                  : colorScheme.onSurfaceVariant,
              fontWeight: highlighted ? FontWeight.w700 : FontWeight.normal,
            ),
          ),
        ],
      ),
    );
  }
}

class _AgentInputMenuData {
  const _AgentInputMenuData({
    required this.enableMemoryAutoUpdate,
    required this.permissionMode,
    required this.disableStreamOutput,
    required this.disableUserPreferenceDescription,
    required this.pluginToggles,
  });

  final bool enableMemoryAutoUpdate;
  final core_proxy.AiPermissionMode permissionMode;
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
    return switch (permissionMode) {
      core_proxy.AiPermissionMode.readOnly => _ToolPermissionMode.readOnly,
      core_proxy.AiPermissionMode.workspaceWrite =>
        _ToolPermissionMode.workspaceWrite,
      core_proxy.AiPermissionMode.full => _ToolPermissionMode.full,
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
  readOnly('只读', core_proxy.AiPermissionMode.readOnly),
  workspaceWrite('读写', core_proxy.AiPermissionMode.workspaceWrite),
  full('完整', core_proxy.AiPermissionMode.full);

  const _ToolPermissionMode(this.label, this.permissionMode);

  final String label;
  final core_proxy.AiPermissionMode permissionMode;
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

  /// Builds a menu section with a softly grouped expanded content area.
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
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 4, 8, 6),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(6),
              child: ColoredBox(
                color: colorScheme.surface.withValues(alpha: 0.42),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: children,
                ),
              ),
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

  /// Builds a compact switch row using the model row metrics.
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
    final resolvedIcon = iconName == null || iconName.isEmpty
        ? icon
        : MaterialIconNameResolver.resolveOrNull(iconName);
    if (resolvedIcon == null) {
      throw ArgumentError.value(
        materialIconName,
        'materialIconName',
        'Unknown Material icon name',
      );
    }
    return InkWell(
      onTap: enabled ? onTap : null,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Icon(resolvedIcon, size: 16, color: iconColor),
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
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(width: 8),
              Transform.scale(
                scale: 0.65,
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

class _PermissionModeSelector extends StatelessWidget {
  const _PermissionModeSelector({
    required this.selectedMode,
    required this.onSelected,
  });

  final _ToolPermissionMode selectedMode;
  final ValueChanged<_ToolPermissionMode> onSelected;

  /// Builds the compact three-way permission mode selector.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Row(
      children: <Widget>[
        for (final mode in _ToolPermissionMode.values) ...[
          Expanded(
            child: InkWell(
              borderRadius: BorderRadius.circular(6),
              onTap: () => onSelected(mode),
              child: Container(
                height: 30,
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
                  borderRadius: BorderRadius.circular(6),
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
