// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../chat/tts/TtsPlaybackController.dart';
import '../components/SettingsControlStyles.dart';

part 'TtsProviderWidgets.dart';
part 'TtsProviderDialogs.dart';
part 'TtsSettingsModels.dart';
part 'SttProviderWidgets.dart';
part 'SttProviderDialogs.dart';

const String _ttsTestText = '你好，我是 Operit 的语音试听。';

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
  String? _testingTtsConfigId;
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
    final ttsManager = widget.clients.preferencesTtsConfigManager;
    final ttsConfigs = await ttsManager.getAllTtsConfigs();
    final currentTtsConfigId = await ttsManager.getCurrentTtsConfigId();
    final ttsProviderCatalogEntries = await ttsManager
        .getProviderCatalogEntries();
    if (ttsProviderCatalogEntries.isEmpty) {
      throw StateError('TTS provider catalog is empty');
    }
    final sttManager = widget.clients.preferencesSttConfigManager;
    final sttConfigs = await sttManager.getAllSttConfigs();
    final currentSttConfigId = await sttManager.getSelectedSttConfigId();
    final sttProviderCatalogEntries = await sttManager
        .getProviderCatalogEntries();
    if (sttProviderCatalogEntries.isEmpty) {
      throw StateError('STT provider catalog is empty');
    }
    final characterCards = await widget.clients.preferencesCharacterCardManager
        .getAllCharacterCards();
    final characterBoundConfigIds = characterCards
        .map((card) => card.ttsConfigId?.trim())
        .whereType<String>()
        .where((id) => id.isNotEmpty)
        .toSet();
    return _TtsSettingsData(
      configs: ttsConfigs,
      currentConfigId: currentTtsConfigId,
      providerCatalogEntries: ttsProviderCatalogEntries,
      characterBoundConfigIds: characterBoundConfigIds,
      sttConfigs: sttConfigs,
      currentSttConfigId: currentSttConfigId,
      sttProviderCatalogEntries: sttProviderCatalogEntries,
    );
  }

  Future<void> _setCurrentTtsConfigId(String id) async {
    await widget.clients.preferencesTtsConfigManager.setCurrentTtsConfigId(
      id: id,
    );
    _reload();
  }

  /// Selects the global STT provider configuration.
  Future<void> _setCurrentSttConfigId(String id) async {
    await _runSttOperation(() async {
      await widget.clients.preferencesSttConfigManager.setCurrentSttConfigId(
        id: id,
      );
      _reload();
    });
  }

  /// Creates or updates one STT provider configuration.
  Future<void> _saveSttConfig(core_proxy.SttConfig config) async {
    await _runSttOperation(() async {
      final manager = widget.clients.preferencesSttConfigManager;
      if (config.id.isEmpty) {
        await manager.createSttConfig(config: config);
      } else {
        await manager.updateSttConfig(config: config);
      }
      _reload();
    });
  }

  /// Opens the STT provider creation dialog.
  Future<void> _createSttProviderConfig(
    List<core_proxy.SttProviderCatalogEntry> providerCatalogEntries,
  ) async {
    final config = await _SttConfigDialog.show(
      context: context,
      config: null,
      providerCatalogEntries: providerCatalogEntries,
      loadModels: (providerTypeId) => widget.clients.preferencesSttConfigManager
          .getAvailableSttModels(providerTypeId: providerTypeId),
    );
    if (config == null) {
      return;
    }
    await _saveSttConfig(config);
  }

  /// Opens the STT provider editor for one persisted configuration.
  Future<void> _editSttProviderConfig(
    core_proxy.SttConfig config,
    List<core_proxy.SttProviderCatalogEntry> providerCatalogEntries,
  ) async {
    final edited = await _SttConfigDialog.show(
      context: context,
      config: config,
      providerCatalogEntries: providerCatalogEntries,
      loadModels: (providerTypeId) => widget.clients.preferencesSttConfigManager
          .getAvailableSttModels(providerTypeId: providerTypeId),
    );
    if (edited == null) {
      return;
    }
    await _saveSttConfig(edited);
  }

  /// Confirms and deletes one inactive STT provider configuration.
  Future<void> _deleteSttProviderConfig(
    core_proxy.SttConfig config,
    String? currentConfigId,
  ) async {
    if (config.id == currentConfigId) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('当前正在使用的 STT 配置不能删除')));
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('删除 STT 供应商'),
        content: Text('删除“${config.name}”及其识别配置？'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton.tonalIcon(
            onPressed: () => Navigator.of(context).pop(true),
            icon: const Icon(Icons.delete_outline),
            label: const Text('删除'),
          ),
        ],
      ),
    );
    if (confirmed != true) {
      return;
    }
    await _runSttOperation(() async {
      await widget.clients.preferencesSttConfigManager.deleteSttConfig(
        id: config.id,
      );
      _reload();
    });
  }

  /// Runs one STT mutation and reports its exact runtime error.
  Future<void> _runSttOperation(Future<void> Function() operation) async {
    try {
      await operation();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('STT 操作失败：$error')));
    }
  }

  Future<void> _testTtsConfig(core_proxy.TtsConfig config) async {
    final configId = config.id.trim();
    if (configId.isEmpty) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('TTS 配置 ID 为空，不能试听')));
      return;
    }
    setState(() {
      _testingTtsConfigId = configId;
    });
    try {
      await TtsPlaybackController.instance.speakWithConfig(
        bridge: widget.clients.bridge,
        ttsConfigId: configId,
        text: _ttsTestText,
        title: '试听 · ${_ttsConfigModelVoiceText(config)}',
        interrupt: true,
      );
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('已开始播放 TTS 试听')));
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('TTS 试听失败：$error')));
    } finally {
      if (mounted && _testingTtsConfigId == configId) {
        setState(() {
          _testingTtsConfigId = null;
        });
      }
    }
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
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(reason)));
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
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('删除 TTS 配置失败：$error')));
    }
  }

  Future<void> _createProviderConfig(
    List<core_proxy.TtsProviderCatalogEntry> providerCatalogEntries,
  ) async {
    final edited = await _TtsConfigDialog.show(
      context: context,
      providerCatalogEntries: providerCatalogEntries,
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
      onTest: () => _testTtsConfig(config),
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
      final existingVoiceKeys = group.configs
          .map((config) => _ttsVoiceKey(config.model, config.voice))
          .toSet();
      final selectableVoices = voices
          .where(
            (voice) => !existingVoiceKeys.contains(
              _ttsVoiceKey(voice.model, voice.voice),
            ),
          )
          .toList(growable: false);
      if (!mounted) {
        return;
      }
      final selection = await _AvailableTtsVoiceDialog.show(
        context: context,
        voices: selectableVoices,
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

  String _ttsVoiceKey(String model, String voice) {
    return '${model.trim()}\u0000${voice.trim()}';
  }

  Future<void> _createCustomTtsVoice(
    core_proxy.TtsConfig providerConfig,
  ) async {
    final result = await _CustomTtsVoiceDialog.show(
      context: context,
      requireModel: _ttsConfigUsesPlaceholder(providerConfig, 'model'),
      requireVoice: _ttsConfigUsesPlaceholder(providerConfig, 'voice'),
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

  Future<void> _editProvider(
    _TtsProviderGroup group,
    List<core_proxy.TtsProviderCatalogEntry> providerCatalogEntries,
  ) async {
    final edited = await _TtsProviderDialog.show(
      context: context,
      config: group.configs.first,
      providerCatalogEntries: providerCatalogEntries,
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
          return Center(child: Text('语音配置加载失败：${snapshot.error}'));
        }
        final data = snapshot.data!;
        final groups = _ttsProviderGroups(data.configs);
        _initializeProviderExpansion(groups, data.currentConfigId);
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: <Widget>[
            _SectionCard(
              title: 'TTS 供应商',
              action: FilledButton.icon(
                onPressed: () =>
                    _createProviderConfig(data.providerCatalogEntries),
                style: SettingsControlStyles.sectionFilledButton(),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('创建'),
              ),
              children: <Widget>[
                _TtsProviderManager(
                  groups: groups,
                  currentConfigId: data.currentConfigId,
                  testingConfigId: _testingTtsConfigId,
                  expandedProviderKeys: _expandedProviderKeys,
                  onToggleProviderExpanded: _toggleProviderExpanded,
                  onAddVoice: _addProviderVoice,
                  onEditProvider: (group) =>
                      _editProvider(group, data.providerCatalogEntries),
                  onEditConfig: (config) => _openVoiceEditor(
                    config,
                    data.currentConfigId,
                    data.characterBoundConfigIds,
                  ),
                  onTestConfig: _testTtsConfig,
                  onSetCurrent: _setCurrentTtsConfigId,
                ),
              ],
            ),
            _SectionCard(
              title: 'STT 供应商',
              action: FilledButton.icon(
                onPressed: () =>
                    _createSttProviderConfig(data.sttProviderCatalogEntries),
                style: SettingsControlStyles.sectionFilledButton(),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('创建'),
              ),
              children: <Widget>[
                _SttProviderManager(
                  configs: data.sttConfigs,
                  currentConfigId: data.currentSttConfigId,
                  onEdit: (config) => _editSttProviderConfig(
                    config,
                    data.sttProviderCatalogEntries,
                  ),
                  onDelete: (config) =>
                      _deleteSttProviderConfig(config, data.currentSttConfigId),
                  onSetCurrent: _setCurrentSttConfigId,
                ),
              ],
            ),
          ],
        );
      },
    );
  }
}
