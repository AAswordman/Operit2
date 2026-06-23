// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

const String _defaultSystemTtsConfigId = 'system_tts_default';

class TtsSettingsPanel extends StatefulWidget {
  const TtsSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<TtsSettingsPanel> createState() => _TtsSettingsPanelState();
}

class _TtsSettingsPanelState extends State<TtsSettingsPanel> {
  Future<_TtsSettingsData>? _future;
  final Set<String> _expandedProviderKeys = <String>{};
  bool _providerExpansionInitialized = false;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  void _reload() {
    setState(() {
      _future = _loadData();
    });
  }

  Future<_TtsSettingsData> _loadData() async {
    final manager = widget.clients.preferencesTtsConfigManager;
    final configs = await manager.getAllTtsConfigs();
    final currentConfigId = await manager.getCurrentTtsConfigId();
    final characterCards = await widget.clients.preferencesCharacterCardManager
        .getAllCharacterCards();
    final characterBoundConfigIds = characterCards
        .map((card) => card.ttsConfigId?.trim())
        .whereType<String>()
        .where((id) => id.isNotEmpty)
        .toSet();
    return _TtsSettingsData(
      configs: configs,
      currentConfigId: currentConfigId,
      characterBoundConfigIds: characterBoundConfigIds,
    );
  }

  Future<void> _setCurrentTtsConfigId(String id) async {
    await widget.clients.preferencesTtsConfigManager.setCurrentTtsConfigId(
      id: id,
    );
    _reload();
  }

  Future<void> _save(core_proxy.TtsConfig config) async {
    final manager = widget.clients.preferencesTtsConfigManager;
    if (config.id.isEmpty) {
      await manager.createTtsConfig(config: config);
    } else {
      await manager.updateTtsConfig(config: config);
    }
    _reload();
  }

  Future<void> _delete(
    core_proxy.TtsConfig config,
    String currentConfigId,
    Set<String> characterBoundConfigIds,
  ) async {
    if (_ttsConfigDeleteBlockedReason(
      config,
      currentConfigId,
      characterBoundConfigIds,
    )
        case final reason?) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(reason)),
      );
      return;
    }
    try {
      await widget.clients.preferencesTtsConfigManager.deleteTtsConfig(
        id: config.id,
      );
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('删除 TTS 配置失败：$error')),
      );
    }
  }

  Future<void> _createProviderConfig() async {
    final edited = await _TtsConfigDialog.show(
      context: context,
    );
    if (edited == null) {
      return;
    }
    await _save(edited);
  }

  Future<void> _openVoiceEditor(
    core_proxy.TtsConfig config,
    String currentConfigId,
    Set<String> characterBoundConfigIds,
  ) async {
    final result = await _TtsVoiceConfigDialog.show(
      context: context,
      config: config,
      deleteBlockedReason: _ttsConfigDeleteBlockedReason(
        config,
        currentConfigId,
        characterBoundConfigIds,
      ),
    );
    if (result == null) {
      return;
    }
    switch (result) {
      case _TtsVoiceEditSaved(:final config):
        await _save(config);
      case _TtsVoiceEditDeleted():
        await _delete(config, currentConfigId, characterBoundConfigIds);
    }
  }

  Future<void> _addProviderVoice(_TtsProviderGroup group) async {
    final messenger = ScaffoldMessenger.of(context);
    final providerConfig = group.configs.first;
    try {
      final voices = await widget.clients.preferencesTtsConfigManager
          .getAvailableTtsVoices(providerConfigId: providerConfig.id);
      if (!mounted) {
        return;
      }
      final selection = await _AvailableTtsVoiceDialog.show(
        context: context,
        voices: voices,
      );
      if (selection == null) {
        return;
      }
      switch (selection) {
        case _AvailableTtsVoicePicked(:final voice):
          await widget.clients.preferencesTtsConfigManager
              .addTtsVoiceFromAvailable(
                providerConfigId: providerConfig.id,
                model: voice.model,
                voice: voice.voice,
              );
          _reload();
        case _AvailableTtsVoiceCustom():
          if (!mounted) {
            return;
          }
          await _createCustomTtsVoice(providerConfig);
      }
    } catch (error) {
      messenger.showSnackBar(SnackBar(content: Text('$error')));
    }
  }

  Future<void> _createCustomTtsVoice(core_proxy.TtsConfig providerConfig) async {
    final result = await _CustomTtsVoiceDialog.show(
      context: context,
      requireModel: providerConfig.providerType == _TtsProviderTypes.openAiCompatible,
      requireVoice: providerConfig.providerType == _TtsProviderTypes.openAiCompatible,
    );
    if (result == null) {
      return;
    }
    if (!mounted) {
      return;
    }
    await widget.clients.preferencesTtsConfigManager.createCustomTtsVoice(
      providerConfigId: providerConfig.id,
      model: result.model,
      voice: result.voice,
    );
    _reload();
  }

  Future<void> _editProvider(_TtsProviderGroup group) async {
    final edited = await _TtsProviderDialog.show(
      context: context,
      config: group.configs.first,
    );
    if (edited == null) {
      return;
    }
    final manager = widget.clients.preferencesTtsConfigManager;
    final now = DateTime.now().millisecondsSinceEpoch;
    for (final config in group.configs) {
      await manager.updateTtsConfig(
        config: core_proxy.TtsConfig(
          id: config.id,
          name: edited.name,
          providerType: edited.providerType,
          endpoint: edited.endpoint,
          apiKey: edited.apiKey,
          model: config.model,
          voice: config.voice,
          responseFormat: config.responseFormat,
          speed: config.speed,
          httpMethod: edited.httpMethod,
          requestBody: edited.requestBody,
          contentType: edited.contentType,
          headers: edited.headers,
          responsePipeline: edited.responsePipeline,
          createdAt: config.createdAt,
          updatedAt: now,
        ),
      );
    }
    _providerExpansionInitialized = false;
    _expandedProviderKeys.clear();
    _reload();
  }

  void _initializeProviderExpansion(
    List<_TtsProviderGroup> groups,
    String currentConfigId,
  ) {
    if (_providerExpansionInitialized) {
      return;
    }
    _providerExpansionInitialized = true;
    for (final group in groups) {
      for (final config in group.configs) {
        if (config.id == currentConfigId) {
          _expandedProviderKeys.add(group.key);
        }
      }
    }
  }

  void _toggleProviderExpanded(String key) {
    setState(() {
      if (_expandedProviderKeys.contains(key)) {
        _expandedProviderKeys.remove(key);
      } else {
        _expandedProviderKeys.add(key);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<_TtsSettingsData>(
      future: _future,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(child: M3LoadingIndicator());
        }
        if (snapshot.hasError) {
          return Center(child: Text('TTS 配置加载失败：${snapshot.error}'));
        }
        final data = snapshot.data!;
        final groups = _ttsProviderGroups(data.configs);
        _initializeProviderExpansion(groups, data.currentConfigId);
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: <Widget>[
            _SectionCard(
              title: '供应商',
              action: FilledButton.icon(
                onPressed: () => _createProviderConfig(),
                style: SettingsControlStyles.sectionFilledButton(),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('创建'),
              ),
              children: <Widget>[
                _TtsProviderManager(
                  groups: groups,
                  currentConfigId: data.currentConfigId,
                  expandedProviderKeys: _expandedProviderKeys,
                  onToggleProviderExpanded: _toggleProviderExpanded,
                  onAddVoice: _addProviderVoice,
                  onEditProvider: _editProvider,
                  onEditConfig: (config) => _openVoiceEditor(
                    config,
                    data.currentConfigId,
                    data.characterBoundConfigIds,
                  ),
                  onSetCurrent: _setCurrentTtsConfigId,
                ),
              ],
            ),
          ],
        );
      },
    );
  }
}

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
    required this.expandedProviderKeys,
    required this.onToggleProviderExpanded,
    required this.onAddVoice,
    required this.onEditProvider,
    required this.onEditConfig,
    required this.onSetCurrent,
  });

  final List<_TtsProviderGroup> groups;
  final String currentConfigId;
  final Set<String> expandedProviderKeys;
  final ValueChanged<String> onToggleProviderExpanded;
  final void Function(_TtsProviderGroup group) onAddVoice;
  final void Function(_TtsProviderGroup group) onEditProvider;
  final void Function(core_proxy.TtsConfig config) onEditConfig;
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
            expanded: expandedProviderKeys.contains(group.key),
            onToggleExpanded: () => onToggleProviderExpanded(group.key),
            onAddVoice: () => onAddVoice(group),
            onEditProvider: () => onEditProvider(group),
            onEditConfig: onEditConfig,
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
    required this.expanded,
    required this.onToggleExpanded,
    required this.onAddVoice,
    required this.onEditProvider,
    required this.onEditConfig,
    required this.onSetCurrent,
  });

  final _TtsProviderGroup group;
  final String currentConfigId;
  final bool expanded;
  final VoidCallback onToggleExpanded;
  final VoidCallback onAddVoice;
  final VoidCallback onEditProvider;
  final void Function(core_proxy.TtsConfig config) onEditConfig;
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
                        onEditConfig: onEditConfig,
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
    required this.onEditConfig,
    required this.onSetCurrent,
  });

  final List<core_proxy.TtsConfig> configs;
  final String currentConfigId;
  final void Function(core_proxy.TtsConfig config) onEditConfig;
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
            onEdit: () => onEditConfig(configs[index]),
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
    required this.onSetCurrent,
    required this.onEdit,
  });

  final core_proxy.TtsConfig config;
  final bool current;
  final VoidCallback onSetCurrent;
  final VoidCallback onEdit;

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

class _TtsProviderEditValues {
  const _TtsProviderEditValues({
    required this.providerType,
    required this.name,
    required this.endpoint,
    required this.apiKey,
    required this.httpMethod,
    required this.contentType,
    required this.requestBody,
    required this.headers,
    required this.responsePipeline,
  });

  final String providerType;
  final String name;
  final String endpoint;
  final String apiKey;
  final String httpMethod;
  final String contentType;
  final String requestBody;
  final List<core_proxy.TtsHttpHeader> headers;
  final List<core_proxy.TtsHttpResponsePipelineStep> responsePipeline;
}

class _TtsProviderDialog extends StatefulWidget {
  const _TtsProviderDialog({required this.config});

  final core_proxy.TtsConfig config;

  static Future<_TtsProviderEditValues?> show({
    required BuildContext context,
    required core_proxy.TtsConfig config,
  }) {
    return showDialog<_TtsProviderEditValues>(
      context: context,
      builder: (context) => _TtsProviderDialog(config: config),
    );
  }

  @override
  State<_TtsProviderDialog> createState() => _TtsProviderDialogState();
}

class _TtsProviderDialogState extends State<_TtsProviderDialog> {
  final _formKey = GlobalKey<FormState>();
  late String _providerType;
  late final TextEditingController _nameController;
  late final TextEditingController _endpointController;
  late final TextEditingController _apiKeyController;
  late final TextEditingController _httpMethodController;
  late final TextEditingController _contentTypeController;
  late final TextEditingController _requestBodyController;
  late final TextEditingController _headersController;
  late final TextEditingController _responsePipelineController;

  @override
  void initState() {
    super.initState();
    final config = widget.config;
    _providerType = config.providerType;
    _nameController = TextEditingController(text: config.name);
    _endpointController = TextEditingController(text: config.endpoint);
    _apiKeyController = TextEditingController(text: config.apiKey);
    _httpMethodController = TextEditingController(text: config.httpMethod);
    _contentTypeController = TextEditingController(text: config.contentType);
    _requestBodyController = TextEditingController(text: config.requestBody);
    _headersController = TextEditingController(
      text: _encodeJsonList(config.headers),
    );
    _responsePipelineController = TextEditingController(
      text: _encodeJsonList(config.responsePipeline),
    );
  }

  @override
  void dispose() {
    _nameController.dispose();
    _endpointController.dispose();
    _apiKeyController.dispose();
    _httpMethodController.dispose();
    _contentTypeController.dispose();
    _requestBodyController.dispose();
    _headersController.dispose();
    _responsePipelineController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final isHttpTts = _providerType == _TtsProviderTypes.http;
    return AlertDialog(
      title: const Text('编辑 TTS 供应商'),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                DropdownButtonFormField<String>(
                  value: _providerType,
                  decoration: const InputDecoration(labelText: '供应商类型'),
                  items: const <DropdownMenuItem<String>>[
                    DropdownMenuItem(
                      value: _TtsProviderTypes.system,
                      child: Text('系统 TTS'),
                    ),
                    DropdownMenuItem(
                      value: _TtsProviderTypes.http,
                      child: Text('HTTP TTS'),
                    ),
                    DropdownMenuItem(
                      value: _TtsProviderTypes.openAiCompatible,
                      child: Text('OpenAI 兼容'),
                    ),
                  ],
                  onChanged: (value) {
                    if (value == null) {
                      return;
                    }
                    setState(() {
                      _providerType = value;
                      if (value == _TtsProviderTypes.http) {
                        _httpMethodController.text = 'POST';
                        _contentTypeController.text = 'application/json';
                      }
                    });
                  },
                ),
                const SizedBox(height: 10),
                _field(_nameController, '供应商名称', requiredField: true),
                if (_providerType == _TtsProviderTypes.openAiCompatible) ...[
                  _field(_endpointController, 'Endpoint', requiredField: true),
                  _field(_apiKeyController, 'API Key', obscureText: true),
                ] else if (_providerType == _TtsProviderTypes.http) ...[
                  _field(_endpointController, 'URL 模板', requiredField: true),
                  _field(_apiKeyController, 'API Key'),
                  _field(
                    _httpMethodController,
                    'HTTP 方法',
                    requiredField: true,
                  ),
                  _field(_contentTypeController, 'Content-Type'),
                  _field(
                    _requestBodyController,
                    '请求体模板',
                    requiredField:
                        isHttpTts &&
                        _httpMethodController.text.trim().toUpperCase() == 'POST',
                    minLines: 4,
                  ),
                  _field(_headersController, 'Headers JSON', minLines: 3),
                  _field(_responsePipelineController, '响应管道 JSON', minLines: 4),
                ],
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _submit,
          child: const Text('保存'),
        ),
      ],
    );
  }

  Widget _field(
    TextEditingController controller,
    String label, {
    bool obscureText = false,
    bool requiredField = false,
    int minLines = 1,
  }) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: TextFormField(
        controller: controller,
        decoration: InputDecoration(labelText: label),
        obscureText: obscureText,
        minLines: minLines,
        maxLines: obscureText ? 1 : (minLines == 1 ? 1 : 8),
        validator: (value) {
          final text = value?.trim() ?? '';
          if (requiredField && text.isEmpty) {
            return '$label不能为空';
          }
          return null;
        },
      ),
    );
  }

  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    late final List<core_proxy.TtsHttpHeader> headers;
    late final List<core_proxy.TtsHttpResponsePipelineStep> responsePipeline;
    try {
      headers = _providerType == _TtsProviderTypes.http
          ? _decodeHeaders(_headersController.text.trim(), 'Headers JSON')
          : const <core_proxy.TtsHttpHeader>[];
      responsePipeline = _providerType == _TtsProviderTypes.http
          ? _decodePipeline(
              _responsePipelineController.text.trim(),
              '响应管道 JSON',
            )
          : const <core_proxy.TtsHttpResponsePipelineStep>[];
    } catch (error) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('$error')),
      );
      return;
    }
    Navigator.of(context).pop(
      _TtsProviderEditValues(
        providerType: _providerType,
        name: _nameController.text.trim(),
        endpoint: _endpointController.text.trim(),
        apiKey: _apiKeyController.text.trim(),
        httpMethod: _httpMethodController.text.trim(),
        contentType: _contentTypeController.text.trim(),
        requestBody: _requestBodyController.text.trim(),
        headers: headers,
        responsePipeline: responsePipeline,
      ),
    );
  }
}

sealed class _AvailableTtsVoiceSelection {
  const _AvailableTtsVoiceSelection();
}

class _AvailableTtsVoicePicked extends _AvailableTtsVoiceSelection {
  const _AvailableTtsVoicePicked(this.voice);

  final core_proxy.AvailableTtsVoice voice;
}

class _AvailableTtsVoiceCustom extends _AvailableTtsVoiceSelection {
  const _AvailableTtsVoiceCustom();
}

class _AvailableTtsVoiceDialog extends StatefulWidget {
  const _AvailableTtsVoiceDialog({required this.voices});

  final List<core_proxy.AvailableTtsVoice> voices;

  static Future<_AvailableTtsVoiceSelection?> show({
    required BuildContext context,
    required List<core_proxy.AvailableTtsVoice> voices,
  }) {
    return showDialog<_AvailableTtsVoiceSelection>(
      context: context,
      builder: (context) => _AvailableTtsVoiceDialog(voices: voices),
    );
  }

  @override
  State<_AvailableTtsVoiceDialog> createState() =>
      _AvailableTtsVoiceDialogState();
}

class _AvailableTtsVoiceDialogState extends State<_AvailableTtsVoiceDialog> {
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  List<core_proxy.AvailableTtsVoice> _filteredVoices() {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) {
      return widget.voices;
    }
    return widget.voices
        .where((voice) {
          final text =
              '${_availableTtsVoiceTitle(voice)} ${_availableTtsVoiceSubtitle(voice)}'
                  .toLowerCase();
          return text.contains(query);
        })
        .toList(growable: false);
  }

  @override
  Widget build(BuildContext context) {
    final filteredVoices = _filteredVoices();
    return AlertDialog(
      title: const Text('添加 TTS 音色'),
      content: SizedBox(
        width: 520,
        height: 500,
        child: Column(
          children: <Widget>[
            TextField(
              controller: _searchController,
              decoration: const InputDecoration(
                prefixIcon: Icon(Icons.search),
                labelText: '搜索',
              ),
              onChanged: (_) => setState(() {}),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: ListView(
                children: <Widget>[
                  for (final voice in filteredVoices)
                    Material(
                      type: MaterialType.transparency,
                      child: ListTile(
                        dense: true,
                        visualDensity: VisualDensity.compact,
                        contentPadding: EdgeInsets.zero,
                        title: Text(_availableTtsVoiceTitle(voice)),
                        subtitle: Text(_availableTtsVoiceSubtitle(voice)),
                        onTap: () => Navigator.of(
                          context,
                        ).pop(_AvailableTtsVoicePicked(voice)),
                      ),
                    ),
                  Material(
                    type: MaterialType.transparency,
                    child: ListTile(
                      dense: true,
                      visualDensity: VisualDensity.compact,
                      contentPadding: EdgeInsets.zero,
                      leading: const Icon(Icons.add),
                      title: const Text('自定义音色'),
                      subtitle: const Text('手动填写模型和音色'),
                      onTap: () => Navigator.of(
                        context,
                      ).pop(const _AvailableTtsVoiceCustom()),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
      ],
    );
  }
}

class _CustomTtsVoiceValues {
  const _CustomTtsVoiceValues({required this.model, required this.voice});

  final String model;
  final String voice;
}

class _CustomTtsVoiceDialog extends StatefulWidget {
  const _CustomTtsVoiceDialog({
    required this.requireModel,
    required this.requireVoice,
  });

  final bool requireModel;
  final bool requireVoice;

  static Future<_CustomTtsVoiceValues?> show({
    required BuildContext context,
    required bool requireModel,
    required bool requireVoice,
  }) {
    return showDialog<_CustomTtsVoiceValues>(
      context: context,
      builder: (context) => _CustomTtsVoiceDialog(
        requireModel: requireModel,
        requireVoice: requireVoice,
      ),
    );
  }

  @override
  State<_CustomTtsVoiceDialog> createState() => _CustomTtsVoiceDialogState();
}

class _CustomTtsVoiceDialogState extends State<_CustomTtsVoiceDialog> {
  final _formKey = GlobalKey<FormState>();
  final _modelController = TextEditingController();
  final _voiceController = TextEditingController();
  String? _pairError;

  @override
  void dispose() {
    _modelController.dispose();
    _voiceController.dispose();
    super.dispose();
  }

  void _save() {
    setState(() {
      _pairError = null;
    });
    if (!_formKey.currentState!.validate()) {
      return;
    }
    final model = _modelController.text.trim();
    final voice = _voiceController.text.trim();
    if (model.isEmpty && voice.isEmpty) {
      setState(() {
        _pairError = '模型和音色至少填写一项';
      });
      return;
    }
    Navigator.of(context).pop(_CustomTtsVoiceValues(model: model, voice: voice));
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('自定义 TTS 音色'),
      content: SizedBox(
        width: 420,
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              TextFormField(
                controller: _modelController,
                decoration: const InputDecoration(labelText: '模型'),
                validator: (value) {
                  final text = value?.trim() ?? '';
                  if (widget.requireModel && text.isEmpty) {
                    return '模型不能为空';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 10),
              TextFormField(
                controller: _voiceController,
                decoration: InputDecoration(
                  labelText: '音色',
                  errorText: _pairError,
                ),
                validator: (value) {
                  final text = value?.trim() ?? '';
                  if (widget.requireVoice && text.isEmpty) {
                    return '音色不能为空';
                  }
                  return null;
                },
              ),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(onPressed: _save, child: const Text('保存')),
      ],
    );
  }
}

sealed class _TtsVoiceEditResult {
  const _TtsVoiceEditResult();
}

class _TtsVoiceEditSaved extends _TtsVoiceEditResult {
  const _TtsVoiceEditSaved(this.config);

  final core_proxy.TtsConfig config;
}

class _TtsVoiceEditDeleted extends _TtsVoiceEditResult {
  const _TtsVoiceEditDeleted();
}

class _TtsVoiceConfigDialog extends StatefulWidget {
  const _TtsVoiceConfigDialog({
    required this.config,
    required this.deleteBlockedReason,
  });

  final core_proxy.TtsConfig config;
  final String? deleteBlockedReason;

  static Future<_TtsVoiceEditResult?> show({
    required BuildContext context,
    required core_proxy.TtsConfig config,
    required String? deleteBlockedReason,
  }) {
    return showDialog<_TtsVoiceEditResult>(
      context: context,
      builder: (context) => _TtsVoiceConfigDialog(
        config: config,
        deleteBlockedReason: deleteBlockedReason,
      ),
    );
  }

  @override
  State<_TtsVoiceConfigDialog> createState() => _TtsVoiceConfigDialogState();
}

class _TtsVoiceConfigDialogState extends State<_TtsVoiceConfigDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _modelController;
  late final TextEditingController _voiceController;
  late final TextEditingController _formatController;
  late final TextEditingController _speedController;

  @override
  void initState() {
    super.initState();
    final config = widget.config;
    _modelController = TextEditingController(text: config.model);
    _voiceController = TextEditingController(text: config.voice);
    _formatController = TextEditingController(text: config.responseFormat);
    _speedController = TextEditingController(text: '${config.speed}');
  }

  @override
  void dispose() {
    _modelController.dispose();
    _voiceController.dispose();
    _formatController.dispose();
    _speedController.dispose();
    super.dispose();
  }

  bool get _requiresModel =>
      widget.config.providerType == _TtsProviderTypes.openAiCompatible;

  bool get _requiresVoice =>
      widget.config.providerType == _TtsProviderTypes.openAiCompatible;

  String get _modelLabel {
    return switch (widget.config.providerType) {
      _TtsProviderTypes.system => '语言标签（可选，例如 zh-CN）',
      _TtsProviderTypes.http => '模型 / Locale',
      _ => '模型',
    };
  }

  String get _voiceLabel {
    return switch (widget.config.providerType) {
      _TtsProviderTypes.system => '系统声音名称（可选）',
      _ => '音色',
    };
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('编辑 TTS 音色'),
      content: SizedBox(
        width: 420,
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              _field(
                _modelController,
                _modelLabel,
                requiredField: _requiresModel,
              ),
              _field(
                _voiceController,
                _voiceLabel,
                requiredField: _requiresVoice,
              ),
              _field(_formatController, '音频格式', requiredField: true),
              _field(
                _speedController,
                '速度',
                requiredField: true,
                numberField: true,
              ),
              if (widget.deleteBlockedReason case final reason?) ...<Widget>[
                const SizedBox(height: 2),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    reason,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        if (widget.deleteBlockedReason == null)
          TextButton.icon(
            onPressed: () =>
                Navigator.of(context).pop(const _TtsVoiceEditDeleted()),
            icon: const Icon(Icons.delete_outline),
            label: const Text('删除'),
          ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(onPressed: _submit, child: const Text('保存')),
      ],
    );
  }

  Widget _field(
    TextEditingController controller,
    String label, {
    bool requiredField = false,
    bool numberField = false,
  }) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: TextFormField(
        controller: controller,
        decoration: InputDecoration(labelText: label),
        validator: (value) {
          final text = value?.trim() ?? '';
          if (requiredField && text.isEmpty) {
            return '$label不能为空';
          }
          if (numberField) {
            final parsed = double.tryParse(text);
            if (parsed == null || parsed <= 0) {
              return '$label必须为正数';
            }
          }
          return null;
        },
      ),
    );
  }

  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    final current = widget.config;
    Navigator.of(context).pop(
      _TtsVoiceEditSaved(
        core_proxy.TtsConfig(
          id: current.id,
          name: current.name,
          providerType: current.providerType,
          endpoint: current.endpoint,
          apiKey: current.apiKey,
          model: _modelController.text.trim(),
          voice: _voiceController.text.trim(),
          responseFormat: _formatController.text.trim(),
          speed: double.parse(_speedController.text.trim()),
          httpMethod: current.httpMethod,
          requestBody: current.requestBody,
          contentType: current.contentType,
          headers: current.headers,
          responsePipeline: current.responsePipeline,
          createdAt: current.createdAt,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
      ),
    );
  }
}

class _TtsConfigDialog extends StatefulWidget {
  const _TtsConfigDialog();

  static Future<core_proxy.TtsConfig?> show({
    required BuildContext context,
  }) {
    return showDialog<core_proxy.TtsConfig>(
      context: context,
      builder: (context) => const _TtsConfigDialog(),
    );
  }

  @override
  State<_TtsConfigDialog> createState() => _TtsConfigDialogState();
}

class _TtsConfigDialogState extends State<_TtsConfigDialog> {
  final _formKey = GlobalKey<FormState>();
  late String _providerType;
  late final TextEditingController _nameController;
  late final TextEditingController _endpointController;
  late final TextEditingController _apiKeyController;
  late final TextEditingController _modelController;
  late final TextEditingController _voiceController;
  late final TextEditingController _formatController;
  late final TextEditingController _speedController;
  late final TextEditingController _httpMethodController;
  late final TextEditingController _contentTypeController;
  late final TextEditingController _requestBodyController;
  late final TextEditingController _headersController;
  late final TextEditingController _responsePipelineController;

  @override
  void initState() {
    super.initState();
    _providerType = _TtsProviderTypes.system;
    _nameController = TextEditingController(text: '系统 TTS');
    _endpointController = TextEditingController(
      text: 'https://api.openai.com/v1/audio/speech',
    );
    _apiKeyController = TextEditingController();
    _modelController = TextEditingController();
    _voiceController = TextEditingController();
    _formatController = TextEditingController(text: 'wav');
    _speedController = TextEditingController(text: '1.0');
    _httpMethodController = TextEditingController(text: 'POST');
    _contentTypeController = TextEditingController(
      text: 'application/json',
    );
    _requestBodyController = TextEditingController(
      text:
          '{"model":"{model}","voice":"{voice}","input":"{text}","response_format":"{responseFormat}","speed":{speed}}',
    );
    _headersController = TextEditingController(text: '[]');
    _responsePipelineController = TextEditingController(text: '[]');
  }

  @override
  void dispose() {
    _nameController.dispose();
    _endpointController.dispose();
    _apiKeyController.dispose();
    _modelController.dispose();
    _voiceController.dispose();
    _formatController.dispose();
    _speedController.dispose();
    _httpMethodController.dispose();
    _contentTypeController.dispose();
    _requestBodyController.dispose();
    _headersController.dispose();
    _responsePipelineController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final isHttpTts = _providerType == _TtsProviderTypes.http;
    return AlertDialog(
      title: const Text('新建 TTS 供应商'),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                DropdownButtonFormField<String>(
                  value: _providerType,
                  decoration: const InputDecoration(labelText: '供应商类型'),
                  items: const <DropdownMenuItem<String>>[
                    DropdownMenuItem(
                      value: _TtsProviderTypes.system,
                      child: Text('系统 TTS'),
                    ),
                    DropdownMenuItem(
                      value: _TtsProviderTypes.http,
                      child: Text('HTTP TTS'),
                    ),
                    DropdownMenuItem(
                      value: _TtsProviderTypes.openAiCompatible,
                      child: Text('OpenAI 兼容'),
                    ),
                  ],
                  onChanged: (value) {
                    if (value == null) {
                      return;
                    }
                    setState(() {
                      _providerType = value;
                      if (value == _TtsProviderTypes.system) {
                        _formatController.text = 'wav';
                      }
                      if (value == _TtsProviderTypes.http) {
                        _httpMethodController.text = 'POST';
                        _contentTypeController.text = 'application/json';
                      }
                    });
                  },
                ),
                const SizedBox(height: 10),
                _field(_nameController, '供应商名称', requiredField: true),
                if (_providerType == _TtsProviderTypes.openAiCompatible) ...[
                  _field(_endpointController, 'Endpoint', requiredField: true),
                  _field(_apiKeyController, 'API Key', obscureText: true),
                  _field(_modelController, '模型', requiredField: true),
                  _field(_voiceController, '音色', requiredField: true),
                ] else if (_providerType == _TtsProviderTypes.http) ...[
                  _field(_endpointController, 'URL 模板', requiredField: true),
                  _field(_apiKeyController, 'API Key'),
                  _field(_modelController, '模型 / Locale'),
                  _field(_voiceController, '音色'),
                  _field(
                    _httpMethodController,
                    'HTTP 方法',
                    requiredField: true,
                    onChanged: (_) => setState(() {}),
                  ),
                  _field(_contentTypeController, 'Content-Type'),
                  _field(
                    _requestBodyController,
                    '请求体模板',
                    requiredField: isHttpTts && _httpMethodController.text.trim().toUpperCase() == 'POST',
                    minLines: 4,
                  ),
                  _field(_headersController, 'Headers JSON', minLines: 3),
                  _field(_responsePipelineController, '响应管道 JSON', minLines: 4),
                ] else ...[
                  _field(_modelController, '语言标签（可选，例如 zh-CN）'),
                  _field(_voiceController, '系统声音名称（可选）'),
                ],
                _field(_formatController, '音频格式', requiredField: true),
                _field(
                  _speedController,
                  '速度',
                  requiredField: true,
                  numberField: true,
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _submit,
          child: const Text('保存'),
        ),
      ],
    );
  }

  Widget _field(
    TextEditingController controller,
    String label, {
    bool obscureText = false,
    bool requiredField = false,
    bool numberField = false,
    int minLines = 1,
    ValueChanged<String>? onChanged,
  }) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: TextFormField(
        controller: controller,
        decoration: InputDecoration(labelText: label),
        obscureText: obscureText,
        minLines: minLines,
        maxLines: obscureText ? 1 : (minLines == 1 ? 1 : 8),
        onChanged: onChanged,
        validator: (value) {
          final text = value?.trim() ?? '';
          if (requiredField && text.isEmpty) {
            return '$label不能为空';
          }
          if (numberField) {
            final parsed = double.tryParse(text);
            if (parsed == null || parsed <= 0) {
              return '$label必须为正数';
            }
          }
          return null;
        },
      ),
    );
  }

  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    final isHttpTts = _providerType == _TtsProviderTypes.http;
    late final List<core_proxy.TtsHttpHeader> headers;
    late final List<core_proxy.TtsHttpResponsePipelineStep> responsePipeline;
    try {
      headers = isHttpTts
          ? _decodeHeaders(_headersController.text.trim(), 'Headers JSON')
          : const <core_proxy.TtsHttpHeader>[];
      responsePipeline = isHttpTts
          ? _decodePipeline(
              _responsePipelineController.text.trim(),
              '响应管道 JSON',
            )
          : const <core_proxy.TtsHttpResponsePipelineStep>[];
    } catch (error) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('$error')),
      );
      return;
    }
    final speed = double.parse(_speedController.text.trim());
    final now = DateTime.now().millisecondsSinceEpoch;
    Navigator.of(context).pop(
      core_proxy.TtsConfig(
        id: '',
        name: _nameController.text.trim(),
        providerType: _providerType,
        endpoint: _endpointController.text.trim(),
        apiKey: _apiKeyController.text.trim(),
        model: _modelController.text.trim(),
        voice: _voiceController.text.trim(),
        responseFormat: _formatController.text.trim(),
        speed: speed,
        httpMethod: isHttpTts ? _httpMethodController.text.trim() : 'POST',
        requestBody: isHttpTts ? _requestBodyController.text.trim() : '',
        contentType: isHttpTts
            ? _contentTypeController.text.trim()
            : 'application/json',
        headers: headers,
        responsePipeline: responsePipeline,
        createdAt: now,
        updatedAt: now,
      ),
    );
  }
}

class _TtsSettingsData {
  const _TtsSettingsData({
    required this.configs,
    required this.currentConfigId,
    required this.characterBoundConfigIds,
  });

  final List<core_proxy.TtsConfig> configs;
  final String currentConfigId;
  final Set<String> characterBoundConfigIds;
}

class _TtsProviderGroup {
  const _TtsProviderGroup({
    required this.key,
    required this.title,
    required this.providerType,
    required this.configs,
  });

  final String key;
  final String title;
  final String providerType;
  final List<core_proxy.TtsConfig> configs;
}

class _TtsProviderTypes {
  const _TtsProviderTypes._();

  static const String system = 'SYSTEM_TTS';
  static const String http = 'HTTP_TTS';
  static const String openAiCompatible = 'OPENAI_COMPATIBLE';
}

List<_TtsProviderGroup> _ttsProviderGroups(List<core_proxy.TtsConfig> configs) {
  final byKey = <String, List<core_proxy.TtsConfig>>{};
  for (final config in configs) {
    final key = _ttsProviderGroupKey(config);
    byKey.putIfAbsent(key, () => <core_proxy.TtsConfig>[]).add(config);
  }
  final groups = <_TtsProviderGroup>[];
  for (final entry in byKey.entries) {
    final first = entry.value.first;
    groups.add(
      _TtsProviderGroup(
        key: entry.key,
        title: first.name,
        providerType: _ttsConfigProviderTypeText(first),
        configs: entry.value..sort(_compareTtsConfig),
      ),
    );
  }
  groups.sort((left, right) => left.title.toLowerCase().compareTo(right.title.toLowerCase()));
  return groups;
}

String _ttsProviderGroupKey(core_proxy.TtsConfig config) {
  return '${config.providerType}\u001F${config.endpoint}\u001F${config.name}';
}

int _compareTtsConfig(core_proxy.TtsConfig left, core_proxy.TtsConfig right) {
  return _ttsConfigModelVoiceText(left)
      .toLowerCase()
      .compareTo(_ttsConfigModelVoiceText(right).toLowerCase());
}

String _ttsConfigProviderTypeText(core_proxy.TtsConfig config) {
  final endpoint = config.endpoint.trim();
  if (endpoint.isEmpty) {
    return config.providerType;
  }
  return '${config.providerType} · $endpoint';
}

String _ttsConfigModelVoiceText(core_proxy.TtsConfig config) {
  final model = config.model.trim();
  final voice = config.voice.trim();
  if (model.isEmpty && voice.isEmpty) {
    return '系统默认音色';
  }
  if (model.isEmpty) {
    return voice;
  }
  if (voice.isEmpty) {
    return model;
  }
  return '$model · $voice';
}

String? _ttsConfigDeleteBlockedReason(
  core_proxy.TtsConfig config,
  String currentConfigId,
  Set<String> characterBoundConfigIds,
) {
  final id = config.id.trim();
  if (id == _defaultSystemTtsConfigId) {
    return '系统默认 TTS 配置不能删除';
  }
  if (id == currentConfigId.trim()) {
    return '当前正在使用的 TTS 配置不能删除';
  }
  if (characterBoundConfigIds.contains(id)) {
    return '该 TTS 配置正在被角色卡使用，不能删除';
  }
  return null;
}

String _availableTtsVoiceTitle(core_proxy.AvailableTtsVoice voice) {
  final displayName = voice.displayName.trim();
  if (displayName.isNotEmpty) {
    return displayName;
  }
  final model = voice.model.trim();
  final voiceName = voice.voice.trim();
  if (model.isEmpty && voiceName.isEmpty) {
    return '系统默认音色';
  }
  if (model.isEmpty) {
    return voiceName;
  }
  if (voiceName.isEmpty) {
    return model;
  }
  return '$model · $voiceName';
}

String _availableTtsVoiceSubtitle(core_proxy.AvailableTtsVoice voice) {
  final labels = <String>[];
  final model = voice.model.trim();
  final voiceName = voice.voice.trim();
  if (model.isNotEmpty) {
    labels.add(model);
  }
  if (voiceName.isNotEmpty) {
    labels.add(voiceName);
  }
  final responseFormat = voice.responseFormat.trim();
  if (responseFormat.isNotEmpty) {
    labels.add(responseFormat);
  }
  labels.add(voice.speed.toStringAsFixed(2));
  final description = voice.description.trim();
  if (description.isNotEmpty) {
    labels.add(description);
  }
  return labels.join(' · ');
}

List<core_proxy.TtsHttpHeader> _decodeHeaders(String raw, String label) {
  final decoded = _decodeJsonList(raw, label);
  return decoded
      .map((item) => core_proxy.TtsHttpHeader.fromJson(item))
      .toList(growable: false);
}

List<core_proxy.TtsHttpResponsePipelineStep> _decodePipeline(
  String raw,
  String label,
) {
  final decoded = _decodeJsonList(raw, label);
  return decoded
      .map((item) {
        final headersValue = item['headers'];
        final headers = headersValue == null
            ? const <core_proxy.TtsHttpHeader>[]
            : (headersValue as List<Object?>)
                  .map(
                    (header) => core_proxy.TtsHttpHeader.fromJson(
                      header as Map<String, Object?>,
                    ),
                  )
                  .toList(growable: false);
        final stepType = (item['stepType'] as String?) ?? (item['type'] as String);
        return core_proxy.TtsHttpResponsePipelineStep(
          stepType: stepType,
          path: item['path'] as String,
          headers: headers,
        );
      })
      .toList(growable: false);
}

List<Map<String, Object?>> _decodeJsonList(String raw, String label) {
  final text = raw.isEmpty ? '[]' : raw;
  final decoded = jsonDecode(text);
  if (decoded is! List<Object?>) {
    throw FormatException('$label 必须是数组');
  }
  return decoded
      .map((item) {
        if (item is! Map<String, Object?>) {
          throw FormatException('$label 每一项必须是对象');
        }
        return item;
      })
      .toList(growable: false);
}

String _encodeJsonList(List<Object> value) {
  return const JsonEncoder.withIndent('  ')
      .convert(value.map((item) => (item as dynamic).toJson()).toList());
}
