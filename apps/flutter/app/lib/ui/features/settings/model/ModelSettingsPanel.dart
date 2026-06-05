// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';

class ModelSettingsPanel extends StatefulWidget {
  const ModelSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<ModelSettingsPanel> createState() => _ModelSettingsPanelState();
}

class _ModelSettingsPanelState extends State<ModelSettingsPanel> {
  Future<_ModelSettingsData>? _future;
  core_proxy.ModelConnectionTestReport? _connectionTestReport;
  bool _isTestingConnection = false;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<_ModelSettingsData> _load() async {
    final modelManager = widget.clients.preferencesModelConfigManager;
    final functionManager = widget.clients.preferencesFunctionalConfigManager;
    final apiPreferences = widget.clients.preferencesApiPreferences;
    await modelManager.initializeIfNeeded();
    await functionManager.initializeIfNeeded();
    final chatMapping = await functionManager.getConfigMappingForFunction(
      functionType: 'CHAT',
    );
    final currentConfig = await modelManager.getModelConfig(
      configId: chatMapping.configId,
    );
    final summaries = await modelManager.getAllConfigSummaries();
    final configDataById = <String, core_proxy.ModelConfigData>{};
    for (final summary in summaries) {
      configDataById[summary.id] = await modelManager.getModelConfig(
        configId: summary.id,
      );
    }
    return _ModelSettingsData(
      summaries: summaries,
      chatMapping: chatMapping,
      currentConfig: currentConfig,
      functionMappings: await functionManager
          .functionConfigMappingWithIndexFlowSnapshot(),
      configDataById: configDataById,
      enableThinkingMode: await apiPreferences.enableThinkingModeFlowSnapshot(),
      thinkingQualityLevel: await apiPreferences
          .thinkingQualityLevelFlowSnapshot(),
      disableStreamOutput: await apiPreferences
          .disableStreamOutputFlowSnapshot(),
      maxImageHistoryUserTurns: await apiPreferences
          .maxImageHistoryUserTurnsFlowSnapshot(),
      maxMediaHistoryUserTurns: await apiPreferences
          .maxMediaHistoryUserTurnsFlowSnapshot(),
    );
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _selectChatModel(core_proxy.ModelConfigSummary summary) async {
    final l10n = AppLocalizations.of(context)!;
    final modelName = _modelNameAt(summary.modelName, summary.modelIndex);
    if (modelName.toLowerCase().contains('autoglm')) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsModelChatAutoGlmWarning)),
      );
      return;
    }
    await widget.clients.preferencesFunctionalConfigManager
        .setConfigForFunctionWithIndex(
          functionType: 'CHAT',
          configId: summary.id,
          modelIndex: summary.modelIndex,
        );
    _reload();
  }

  Future<void> _selectFunctionModel(
    String functionType,
    _ModelSettingsData data,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    final selection = await _FunctionModelSelectorDialog.show(
      context: context,
      functionType: functionType,
      summaries: data.summaries,
      currentMapping: data.functionMappings[functionType]!,
    );
    if (selection == null) {
      return;
    }
    if (!mounted) {
      return;
    }
    final summary = data.summaryForConfigId(selection.configId);
    final modelName = _modelNameAt(summary.modelName, selection.modelIndex);
    if (functionType == 'CHAT' && modelName.toLowerCase().contains('autoglm')) {
      messenger.showSnackBar(
        SnackBar(content: Text(l10n.settingsModelChatAutoGlmWarning)),
      );
      return;
    }
    await widget.clients.preferencesFunctionalConfigManager
        .setConfigForFunctionWithIndex(
          functionType: functionType,
          configId: selection.configId,
          modelIndex: selection.modelIndex,
        );
    if (!mounted) {
      return;
    }
    _reload();
  }

  Future<void> _resetFunctionMappings() async {
    await widget.clients.preferencesFunctionalConfigManager
        .resetAllFunctionConfigs();
    _reload();
  }

  Future<void> _createModelConfig() async {
    final l10n = AppLocalizations.of(context)!;
    final result = await _ModelConfigEditorDialog.show(
      context: context,
      title: l10n.settingsModelCreateProfile,
    );
    if (result == null) {
      return;
    }
    final manager = widget.clients.preferencesModelConfigManager;
    final configId = await manager.createConfig(name: result.name);
    await _saveModelConfig(configId, result);
  }

  Future<void> _editModelConfig(core_proxy.ModelConfigSummary summary) async {
    final l10n = AppLocalizations.of(context)!;
    final manager = widget.clients.preferencesModelConfigManager;
    final config = await manager.getModelConfig(configId: summary.id);
    if (!mounted) {
      return;
    }
    final result = await _ModelConfigEditorDialog.show(
      context: context,
      title: l10n.settingsModelEditProfile,
      config: config,
    );
    if (result == null) {
      return;
    }
    await _saveModelConfig(config.id, result);
  }

  Future<void> _saveModelConfig(
    String configId,
    _ModelConfigEditResult result,
  ) async {
    final manager = widget.clients.preferencesModelConfigManager;
    await manager.updateConfigBase(configId: configId, name: result.name);
    await manager.updateModelConfig(
      configId: configId,
      apiKey: result.apiKey,
      apiEndpoint: result.apiEndpoint,
      modelName: result.modelName,
    );
    await manager.updateApiSettingsFull(
      configId: configId,
      apiKey: result.apiKey,
      apiEndpoint: result.apiEndpoint,
      modelName: result.modelName,
      apiProviderType: result.apiProviderType,
      apiProviderTypeId: result.apiProviderTypeId,
      mnnForwardType: result.mnnForwardType,
      mnnThreadCount: result.mnnThreadCount,
      llamaThreadCount: result.llamaThreadCount,
      llamaContextSize: result.llamaContextSize,
      llamaGpuLayers: result.llamaGpuLayers,
      enableDirectImageProcessing: result.enableDirectImageProcessing,
      enableDirectAudioProcessing: result.enableDirectAudioProcessing,
      enableDirectVideoProcessing: result.enableDirectVideoProcessing,
      enableGoogleSearch: result.enableGoogleSearch,
      enableToolCall: result.enableToolCall,
    );
    await manager.updateRequestQueueSettings(
      configId: configId,
      requestLimitPerMinute: result.requestLimitPerMinute,
      maxConcurrentRequests: result.maxConcurrentRequests,
    );
    await manager.updateContextSettings(
      configId: configId,
      contextLength: result.contextLength,
      maxContextLength: result.maxContextLength,
      enableMaxContextMode: result.enableMaxContextMode,
    );
    await manager.updateSummarySettings(
      configId: configId,
      enableSummary: result.enableSummary,
      summaryTokenThreshold: result.summaryTokenThreshold,
      enableSummaryByMessageCount: result.enableSummaryByMessageCount,
      summaryMessageCountThreshold: result.summaryMessageCountThreshold,
    );
    await manager.updateCustomHeaders(
      configId: configId,
      customHeaders: result.customHeaders,
    );
    await manager.updateParameters(
      configId: configId,
      parameters: _standardModelParameterPayload(result),
    );
    await manager.updateCustomParameters(
      configId: configId,
      parametersJson: result.customParameters,
    );
    await manager.updateApiKeyPoolSettings(
      configId: configId,
      useMultipleApiKeys: result.useMultipleApiKeys,
      apiKeyPool: result.apiKeyPool,
    );
    _reload();
  }

  Future<void> _deleteModelConfig(core_proxy.ModelConfigSummary summary) async {
    await widget.clients.preferencesModelConfigManager.deleteConfig(
      configId: summary.id,
    );
    _reload();
  }

  Future<void> _toggleThinking(_ModelSettingsData data) async {
    await widget.clients.preferencesApiPreferences.updateThinkingSettings(
      enableThinkingMode: !data.enableThinkingMode,
      thinkingQualityLevel: null,
    );
    _reload();
  }

  Future<void> _toggleStream(_ModelSettingsData data) async {
    await widget.clients.preferencesApiPreferences.saveDisableStreamOutput(
      isDisabled: !data.disableStreamOutput,
    );
    _reload();
  }

  Future<void> _toggleToolCall(_ModelSettingsData data) async {
    await widget.clients.preferencesModelConfigManager.updateToolCall(
      configId: data.currentConfig.id,
      enableToolCall: !data.currentConfig.enableToolCall,
    );
    _reload();
  }

  Future<void> _toggleDirectImage(_ModelSettingsData data) async {
    await widget.clients.preferencesModelConfigManager
        .updateDirectImageProcessing(
          configId: data.currentConfig.id,
          enableDirectImageProcessing:
              !data.currentConfig.enableDirectImageProcessing,
        );
    _reload();
  }

  Future<void> _toggleDirectAudio(_ModelSettingsData data) async {
    await widget.clients.preferencesModelConfigManager
        .updateDirectAudioProcessing(
          configId: data.currentConfig.id,
          enableDirectAudioProcessing:
              !data.currentConfig.enableDirectAudioProcessing,
        );
    _reload();
  }

  Future<void> _toggleDirectVideo(_ModelSettingsData data) async {
    await widget.clients.preferencesModelConfigManager
        .updateDirectVideoProcessing(
          configId: data.currentConfig.id,
          enableDirectVideoProcessing:
              !data.currentConfig.enableDirectVideoProcessing,
        );
    _reload();
  }

  Future<void> _toggleGoogleSearch(_ModelSettingsData data) async {
    await widget.clients.preferencesModelConfigManager.updateGoogleSearch(
      configId: data.currentConfig.id,
      enableGoogleSearch: !data.currentConfig.enableGoogleSearch,
    );
    _reload();
  }

  Future<void> _testConnection(_ModelSettingsData data) async {
    final l10n = AppLocalizations.of(context)!;
    setState(() {
      _isTestingConnection = true;
    });
    try {
      final report = await widget.clients.preferencesModelConfigManager
          .testModelConfigConnection(
            configId: data.currentConfig.id,
            modelIndex: data.chatMapping.modelIndex,
          );
      if (!mounted) {
        return;
      }
      await _applyConnectionTestCapabilities(data, report);
      if (!mounted) {
        return;
      }
      setState(() {
        _connectionTestReport = report;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsModelConnectionTestError('$error')),
        ),
      );
    } finally {
      if (mounted) {
        setState(() {
          _isTestingConnection = false;
        });
      }
    }
  }

  Future<void> _applyConnectionTestCapabilities(
    _ModelSettingsData data,
    core_proxy.ModelConnectionTestReport report,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    final chatPassed = _connectionTestSucceeded(report, 'CHAT');
    if (!chatPassed) {
      messenger.showSnackBar(
        SnackBar(content: Text(l10n.settingsModelCapabilitiesNeedChat)),
      );
      return;
    }
    final manager = widget.clients.preferencesModelConfigManager;
    await manager.updateToolCall(
      configId: data.currentConfig.id,
      enableToolCall: _connectionTestSucceeded(report, 'TOOL_CALL'),
    );
    await manager.updateDirectImageProcessing(
      configId: data.currentConfig.id,
      enableDirectImageProcessing: _connectionTestSucceeded(report, 'IMAGE'),
    );
    await manager.updateDirectAudioProcessing(
      configId: data.currentConfig.id,
      enableDirectAudioProcessing: _connectionTestSucceeded(report, 'AUDIO'),
    );
    await manager.updateDirectVideoProcessing(
      configId: data.currentConfig.id,
      enableDirectVideoProcessing: _connectionTestSucceeded(report, 'VIDEO'),
    );
    if (!mounted) {
      return;
    }
    messenger.showSnackBar(
      SnackBar(content: Text(l10n.settingsModelCapabilitiesApplied)),
    );
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<_ModelSettingsData>(
      future: _future,
      builder: (context, snapshot) {
        final data = snapshot.data;
        if (data == null) {
          return const M3LoadingPane();
        }
        return ListView(
          padding: const EdgeInsets.fromLTRB(28, 24, 28, 36),
          children: <Widget>[
            _SettingsHero(
              icon: Icons.hub_outlined,
              title: l10n.settingsCategoryModelTitle,
              description: l10n.settingsCategoryModelDescription,
            ),
            _SectionCard(
              title: l10n.settingsModelCurrentSection,
              children: <Widget>[
                _InfoLine(
                  label: l10n.settingsModelCurrentChatModel,
                  value:
                      '${data.currentConfig.name} · ${_modelNameAt(data.currentConfig.modelName, data.chatMapping.modelIndex)}',
                ),
                _SwitchLine(
                  title: l10n.settingsChatThinkingMode,
                  subtitle: l10n.settingsChatThinkingModeDescription,
                  value: data.enableThinkingMode,
                  onChanged: (_) => _toggleThinking(data),
                ),
                _SwitchLine(
                  title: l10n.settingsChatStreamOutput,
                  subtitle: l10n.settingsChatStreamOutputDescription,
                  value: !data.disableStreamOutput,
                  onChanged: (_) => _toggleStream(data),
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsModelConnectionTestSection,
              action: FilledButton.icon(
                onPressed: _isTestingConnection
                    ? null
                    : () => _testConnection(data),
                icon: const Icon(Icons.network_check_outlined),
                label: Text(l10n.settingsModelRunConnectionTest),
              ),
              children: <Widget>[
                if (_isTestingConnection) ...<Widget>[
                  Row(
                    children: <Widget>[
                      const M3LoadingIndicator(size: 24),
                      const SizedBox(width: 12),
                      Expanded(child: Text(l10n.settingsModelTestingConnection)),
                    ],
                  ),
                  const SizedBox(height: 12),
                ],
                if (_connectionTestReport case final report?) ...<Widget>[
                  _InfoLine(
                    label: l10n.settingsModelTestedModel,
                    value: report.testedModelName,
                  ),
                  _InfoLine(
                    label: l10n.settingsModelConnectionTestSection,
                    value: report.success
                        ? l10n.settingsModelConnectionTestPassed
                        : l10n.settingsModelConnectionTestFailed,
                  ),
                  for (final item in report.items)
                    _ConnectionTestItemTile(item: item),
                ],
              ],
            ),
            _SectionCard(
              title: l10n.settingsModelProfilesSection,
              action: FilledButton.icon(
                onPressed: _createModelConfig,
                icon: const Icon(Icons.add),
                label: Text(l10n.create),
              ),
              children: <Widget>[
                for (final summary in data.summaries)
                  _ModelSummaryTile(
                    summary: summary,
                    selected: summary.id == data.chatMapping.configId,
                    deletable:
                        summary.id != data.chatMapping.configId &&
                        data.summaries.length > 1,
                    onTap: () => _selectChatModel(summary),
                    onEdit: () => _editModelConfig(summary),
                    onDelete: () => _deleteModelConfig(summary),
                  ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsModelFunctionMappingsSection,
              initiallyExpanded: false,
              action: TextButton.icon(
                onPressed: _resetFunctionMappings,
                icon: const Icon(Icons.refresh),
                label: Text(l10n.settingsModelFunctionMappingsReset),
              ),
              children: <Widget>[
                Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    l10n.settingsModelFunctionMappingsDescription,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                const SizedBox(height: 8),
                for (final functionType in _functionTypes)
                  _FunctionMappingTile(
                    functionType: functionType,
                    mapping: data.functionMappings[functionType]!,
                    summary: data.summaryForConfigId(
                      data.functionMappings[functionType]!.configId,
                    ),
                    config:
                        data.configDataById[data
                            .functionMappings[functionType]!
                            .configId]!,
                    onTap: () => _selectFunctionModel(functionType, data),
                  ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsAdvanced,
              initiallyExpanded: false,
              children: <Widget>[
                _SwitchLine(
                  title: l10n.settingsModelToolCall,
                  subtitle: l10n.settingsModelToolCallDescription,
                  value: data.currentConfig.enableToolCall,
                  onChanged: (_) => _toggleToolCall(data),
                ),
                _SwitchLine(
                  title: l10n.settingsModelDirectImage,
                  subtitle: l10n.settingsModelDirectImageDescription,
                  value: data.currentConfig.enableDirectImageProcessing,
                  onChanged: (_) => _toggleDirectImage(data),
                ),
                _SwitchLine(
                  title: l10n.settingsModelDirectAudio,
                  subtitle: l10n.settingsModelDirectAudioDescription,
                  value: data.currentConfig.enableDirectAudioProcessing,
                  onChanged: (_) => _toggleDirectAudio(data),
                ),
                _SwitchLine(
                  title: l10n.settingsModelDirectVideo,
                  subtitle: l10n.settingsModelDirectVideoDescription,
                  value: data.currentConfig.enableDirectVideoProcessing,
                  onChanged: (_) => _toggleDirectVideo(data),
                ),
                _SwitchLine(
                  title: l10n.settingsModelGoogleSearch,
                  subtitle: l10n.settingsModelGoogleSearchDescription,
                  value: data.currentConfig.enableGoogleSearch,
                  onChanged: (_) => _toggleGoogleSearch(data),
                ),
                _InfoLine(
                  label: l10n.settingsModelContext,
                  value:
                      '${data.currentConfig.contextLength.toStringAsFixed(0)}k / ${data.currentConfig.maxContextLength.toStringAsFixed(0)}k',
                ),
                _InfoLine(
                  label: l10n.settingsModelSummary,
                  value: data.currentConfig.enableSummary
                      ? l10n.enable
                      : l10n.disable,
                ),
                _InfoLine(
                  label: l10n.settingsModelMediaHistory,
                  value:
                      '${data.maxImageHistoryUserTurns} / ${data.maxMediaHistoryUserTurns}',
                ),
              ],
            ),
          ],
        );
      },
    );
  }
}

class _ModelSettingsData {
  const _ModelSettingsData({
    required this.summaries,
    required this.chatMapping,
    required this.currentConfig,
    required this.functionMappings,
    required this.configDataById,
    required this.enableThinkingMode,
    required this.thinkingQualityLevel,
    required this.disableStreamOutput,
    required this.maxImageHistoryUserTurns,
    required this.maxMediaHistoryUserTurns,
  });

  final List<core_proxy.ModelConfigSummary> summaries;
  final core_proxy.FunctionConfigMapping chatMapping;
  final core_proxy.ModelConfigData currentConfig;
  final Map<Object?, core_proxy.FunctionConfigMapping> functionMappings;
  final Map<String, core_proxy.ModelConfigData> configDataById;
  final bool enableThinkingMode;
  final int thinkingQualityLevel;
  final bool disableStreamOutput;
  final int maxImageHistoryUserTurns;
  final int maxMediaHistoryUserTurns;

  core_proxy.ModelConfigSummary summaryForConfigId(String configId) {
    return summaries.firstWhere((summary) => summary.id == configId);
  }
}

class _ModelConfigEditResult {
  const _ModelConfigEditResult({
    required this.name,
    required this.apiKey,
    required this.apiEndpoint,
    required this.modelName,
    required this.apiProviderType,
    required this.apiProviderTypeId,
    required this.mnnForwardType,
    required this.mnnThreadCount,
    required this.llamaThreadCount,
    required this.llamaContextSize,
    required this.llamaGpuLayers,
    required this.enableDirectImageProcessing,
    required this.enableDirectAudioProcessing,
    required this.enableDirectVideoProcessing,
    required this.enableGoogleSearch,
    required this.enableToolCall,
    required this.requestLimitPerMinute,
    required this.maxConcurrentRequests,
    required this.contextLength,
    required this.maxContextLength,
    required this.enableMaxContextMode,
    required this.enableSummary,
    required this.summaryTokenThreshold,
    required this.enableSummaryByMessageCount,
    required this.summaryMessageCountThreshold,
    required this.customHeaders,
    required this.customParameters,
    required this.useMultipleApiKeys,
    required this.apiKeyPool,
    required this.maxTokensEnabled,
    required this.temperatureEnabled,
    required this.topPEnabled,
    required this.topKEnabled,
    required this.presencePenaltyEnabled,
    required this.frequencyPenaltyEnabled,
    required this.repetitionPenaltyEnabled,
    required this.maxTokens,
    required this.temperature,
    required this.topP,
    required this.topK,
    required this.presencePenalty,
    required this.frequencyPenalty,
    required this.repetitionPenalty,
  });

  final String name;
  final String apiKey;
  final String apiEndpoint;
  final String modelName;
  final Object? apiProviderType;
  final String apiProviderTypeId;
  final int mnnForwardType;
  final int mnnThreadCount;
  final int llamaThreadCount;
  final int llamaContextSize;
  final int llamaGpuLayers;
  final bool enableDirectImageProcessing;
  final bool enableDirectAudioProcessing;
  final bool enableDirectVideoProcessing;
  final bool enableGoogleSearch;
  final bool enableToolCall;
  final int requestLimitPerMinute;
  final int maxConcurrentRequests;
  final double contextLength;
  final double maxContextLength;
  final bool enableMaxContextMode;
  final bool enableSummary;
  final double summaryTokenThreshold;
  final bool enableSummaryByMessageCount;
  final int summaryMessageCountThreshold;
  final String customHeaders;
  final String customParameters;
  final bool useMultipleApiKeys;
  final List<core_proxy.ApiKeyInfo> apiKeyPool;
  final bool maxTokensEnabled;
  final bool temperatureEnabled;
  final bool topPEnabled;
  final bool topKEnabled;
  final bool presencePenaltyEnabled;
  final bool frequencyPenaltyEnabled;
  final bool repetitionPenaltyEnabled;
  final int maxTokens;
  final double temperature;
  final double topP;
  final int topK;
  final double presencePenalty;
  final double frequencyPenalty;
  final double repetitionPenalty;
}

class _ModelConfigEditorDialog extends StatefulWidget {
  const _ModelConfigEditorDialog({required this.title, this.config});

  final String title;
  final core_proxy.ModelConfigData? config;

  static Future<_ModelConfigEditResult?> show({
    required BuildContext context,
    required String title,
    core_proxy.ModelConfigData? config,
  }) {
    return showDialog<_ModelConfigEditResult>(
      context: context,
      builder: (context) =>
          _ModelConfigEditorDialog(title: title, config: config),
    );
  }

  @override
  State<_ModelConfigEditorDialog> createState() =>
      _ModelConfigEditorDialogState();
}

class _ModelConfigEditorDialogState extends State<_ModelConfigEditorDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameController;
  late final TextEditingController _apiKeyController;
  late final TextEditingController _apiEndpointController;
  late final TextEditingController _modelNameController;
  late final TextEditingController _providerIdController;
  late final TextEditingController _requestLimitController;
  late final TextEditingController _maxConcurrentController;
  late final TextEditingController _contextLengthController;
  late final TextEditingController _maxContextLengthController;
  late final TextEditingController _summaryThresholdController;
  late final TextEditingController _summaryCountController;
  late final TextEditingController _customHeadersController;
  late final TextEditingController _customParametersController;
  late final TextEditingController _maxTokensController;
  late final TextEditingController _temperatureController;
  late final TextEditingController _topPController;
  late final TextEditingController _topKController;
  late final TextEditingController _presencePenaltyController;
  late final TextEditingController _frequencyPenaltyController;
  late final TextEditingController _repetitionPenaltyController;
  late bool _useMultipleApiKeys;
  late List<core_proxy.ApiKeyInfo> _apiKeyPool;
  bool _maxTokensEnabled = false;
  bool _temperatureEnabled = false;
  bool _topPEnabled = false;
  bool _topKEnabled = false;
  bool _presencePenaltyEnabled = false;
  bool _frequencyPenaltyEnabled = false;
  bool _repetitionPenaltyEnabled = false;
  bool _enableToolCall = false;
  bool _enableDirectImage = false;
  bool _enableDirectAudio = false;
  bool _enableDirectVideo = false;
  bool _enableGoogleSearch = false;
  bool _enableMaxContextMode = false;
  bool _enableSummary = false;
  bool _enableSummaryByMessageCount = false;

  @override
  void initState() {
    super.initState();
    final config = widget.config;
    _nameController = TextEditingController(text: config?.name ?? '');
    _apiKeyController = TextEditingController(text: config?.apiKey ?? '');
    _apiEndpointController = TextEditingController(
      text: config?.apiEndpoint ?? '',
    );
    _modelNameController = TextEditingController(text: config?.modelName ?? '');
    _providerIdController = TextEditingController(
      text: config?.apiProviderTypeId ?? '',
    );
    _requestLimitController = TextEditingController(
      text: (config?.requestLimitPerMinute ?? 0).toString(),
    );
    _maxConcurrentController = TextEditingController(
      text: (config?.maxConcurrentRequests ?? 1).toString(),
    );
    _contextLengthController = TextEditingController(
      text: (config?.contextLength ?? 32).toStringAsFixed(0),
    );
    _maxContextLengthController = TextEditingController(
      text: (config?.maxContextLength ?? 32).toStringAsFixed(0),
    );
    _summaryThresholdController = TextEditingController(
      text: (config?.summaryTokenThreshold ?? 30).toStringAsFixed(0),
    );
    _summaryCountController = TextEditingController(
      text: (config?.summaryMessageCountThreshold ?? 20).toString(),
    );
    _customHeadersController = TextEditingController(
      text: config?.customHeaders ?? '',
    );
    _customParametersController = TextEditingController(
      text: config?.customParameters ?? '',
    );
    _maxTokensController = TextEditingController(
      text: (config?.maxTokens ?? 4096).toString(),
    );
    _temperatureController = TextEditingController(
      text: (config?.temperature ?? 1.0).toString(),
    );
    _topPController = TextEditingController(
      text: (config?.topP ?? 1.0).toString(),
    );
    _topKController = TextEditingController(
      text: (config?.topK ?? 0).toString(),
    );
    _presencePenaltyController = TextEditingController(
      text: (config?.presencePenalty ?? 0.0).toString(),
    );
    _frequencyPenaltyController = TextEditingController(
      text: (config?.frequencyPenalty ?? 0.0).toString(),
    );
    _repetitionPenaltyController = TextEditingController(
      text: (config?.repetitionPenalty ?? 1.0).toString(),
    );
    _useMultipleApiKeys = config?.useMultipleApiKeys ?? false;
    _apiKeyPool = _apiKeyPoolFromRaw(config?.apiKeyPool ?? const <Object?>[]);
    _maxTokensEnabled = config?.maxTokensEnabled ?? false;
    _temperatureEnabled = config?.temperatureEnabled ?? false;
    _topPEnabled = config?.topPEnabled ?? false;
    _topKEnabled = config?.topKEnabled ?? false;
    _presencePenaltyEnabled = config?.presencePenaltyEnabled ?? false;
    _frequencyPenaltyEnabled = config?.frequencyPenaltyEnabled ?? false;
    _repetitionPenaltyEnabled = config?.repetitionPenaltyEnabled ?? false;
    _enableToolCall = config?.enableToolCall ?? false;
    _enableDirectImage = config?.enableDirectImageProcessing ?? false;
    _enableDirectAudio = config?.enableDirectAudioProcessing ?? false;
    _enableDirectVideo = config?.enableDirectVideoProcessing ?? false;
    _enableGoogleSearch = config?.enableGoogleSearch ?? false;
    _enableMaxContextMode = config?.enableMaxContextMode ?? false;
    _enableSummary = config?.enableSummary ?? false;
    _enableSummaryByMessageCount = config?.enableSummaryByMessageCount ?? false;
  }

  @override
  void dispose() {
    _nameController.dispose();
    _apiKeyController.dispose();
    _apiEndpointController.dispose();
    _modelNameController.dispose();
    _providerIdController.dispose();
    _requestLimitController.dispose();
    _maxConcurrentController.dispose();
    _contextLengthController.dispose();
    _maxContextLengthController.dispose();
    _summaryThresholdController.dispose();
    _summaryCountController.dispose();
    _customHeadersController.dispose();
    _customParametersController.dispose();
    _maxTokensController.dispose();
    _temperatureController.dispose();
    _topPController.dispose();
    _topKController.dispose();
    _presencePenaltyController.dispose();
    _frequencyPenaltyController.dispose();
    _repetitionPenaltyController.dispose();
    super.dispose();
  }

  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    final config = widget.config;
    Navigator.of(context).pop(
      _ModelConfigEditResult(
        name: _nameController.text.trim(),
        apiKey: _apiKeyController.text,
        apiEndpoint: _apiEndpointController.text.trim(),
        modelName: _modelNameController.text.trim(),
        apiProviderType: config?.apiProviderType,
        apiProviderTypeId: _providerIdController.text.trim(),
        mnnForwardType: config?.mnnForwardType ?? 0,
        mnnThreadCount: config?.mnnThreadCount ?? 4,
        llamaThreadCount: config?.llamaThreadCount ?? 4,
        llamaContextSize: config?.llamaContextSize ?? 4096,
        llamaGpuLayers: config?.llamaGpuLayers ?? 0,
        enableDirectImageProcessing: _enableDirectImage,
        enableDirectAudioProcessing: _enableDirectAudio,
        enableDirectVideoProcessing: _enableDirectVideo,
        enableGoogleSearch: _enableGoogleSearch,
        enableToolCall: _enableToolCall,
        requestLimitPerMinute: int.parse(_requestLimitController.text),
        maxConcurrentRequests: int.parse(_maxConcurrentController.text),
        contextLength: double.parse(_contextLengthController.text),
        maxContextLength: double.parse(_maxContextLengthController.text),
        enableMaxContextMode: _enableMaxContextMode,
        enableSummary: _enableSummary,
        summaryTokenThreshold: double.parse(_summaryThresholdController.text),
        enableSummaryByMessageCount: _enableSummaryByMessageCount,
        summaryMessageCountThreshold: int.parse(_summaryCountController.text),
        customHeaders: _customHeadersController.text,
        customParameters: _customParametersController.text,
        useMultipleApiKeys: _useMultipleApiKeys,
        apiKeyPool: List<core_proxy.ApiKeyInfo>.from(_apiKeyPool),
        maxTokensEnabled: _maxTokensEnabled,
        temperatureEnabled: _temperatureEnabled,
        topPEnabled: _topPEnabled,
        topKEnabled: _topKEnabled,
        presencePenaltyEnabled: _presencePenaltyEnabled,
        frequencyPenaltyEnabled: _frequencyPenaltyEnabled,
        repetitionPenaltyEnabled: _repetitionPenaltyEnabled,
        maxTokens: int.parse(_maxTokensController.text),
        temperature: double.parse(_temperatureController.text),
        topP: double.parse(_topPController.text),
        topK: int.parse(_topKController.text),
        presencePenalty: double.parse(_presencePenaltyController.text),
        frequencyPenalty: double.parse(_frequencyPenaltyController.text),
        repetitionPenalty: double.parse(_repetitionPenaltyController.text),
      ),
    );
  }

  Future<void> _editApiKey({core_proxy.ApiKeyInfo? keyInfo}) async {
    final l10n = AppLocalizations.of(context)!;
    final edited = await _ApiKeyEditorDialog.show(
      context: context,
      title: keyInfo == null
          ? l10n.settingsModelAddApiKey
          : l10n.settingsModelEditApiKey,
      keyInfo: keyInfo,
    );
    if (edited == null) {
      return;
    }
    setState(() {
      if (keyInfo == null) {
        _apiKeyPool.add(edited);
      } else {
        _apiKeyPool = _apiKeyPool
            .map((item) => item.id == keyInfo.id ? edited : item)
            .toList(growable: false);
      }
    });
  }

  void _toggleApiKey(core_proxy.ApiKeyInfo keyInfo, bool enabled) {
    setState(() {
      _apiKeyPool = _apiKeyPool
          .map(
            (item) => item.id == keyInfo.id
                ? _apiKeyInfoWith(item, isEnabled: enabled)
                : item,
          )
          .toList(growable: false);
    });
  }

  void _deleteApiKey(core_proxy.ApiKeyInfo keyInfo) {
    setState(() {
      _apiKeyPool = _apiKeyPool
          .where((item) => item.id != keyInfo.id)
          .toList(growable: false);
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 640,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                _DialogTextField(
                  controller: _nameController,
                  label: l10n.settingsModelProfileName,
                  requiredField: true,
                ),
                _DialogTextField(
                  controller: _apiEndpointController,
                  label: l10n.settingsModelApiEndpoint,
                  requiredField: true,
                ),
                _DialogTextField(
                  controller: _modelNameController,
                  label: l10n.settingsModelModelNames,
                  requiredField: true,
                ),
                _DialogTextField(
                  controller: _apiKeyController,
                  label: l10n.settingsModelApiKey,
                  obscureText: true,
                ),
                ExpansionTile(
                  title: Text(l10n.settingsAdvanced),
                  tilePadding: EdgeInsets.zero,
                  childrenPadding: EdgeInsets.zero,
                  children: <Widget>[
                    _DialogTextField(
                      controller: _providerIdController,
                      label: l10n.settingsModelProviderId,
                    ),
                    SwitchListTile(
                      contentPadding: EdgeInsets.zero,
                      title: Text(l10n.settingsModelApiKeyPool),
                      subtitle: Text(l10n.settingsModelApiKeyPoolDescription),
                      value: _useMultipleApiKeys,
                      onChanged: (value) =>
                          setState(() => _useMultipleApiKeys = value),
                    ),
                    if (_useMultipleApiKeys)
                      _ApiKeyPoolEditor(
                        apiKeyPool: _apiKeyPool,
                        onAdd: () => _editApiKey(),
                        onEdit: (keyInfo) => _editApiKey(keyInfo: keyInfo),
                        onToggle: _toggleApiKey,
                        onDelete: _deleteApiKey,
                      ),
                    _ModelParameterControl(
                      title: l10n.settingsModelMaxTokens,
                      subtitle: l10n.settingsModelMaxTokensDescription,
                      controller: _maxTokensController,
                      enabled: _maxTokensEnabled,
                      integerOnly: true,
                      onEnabledChanged: (value) =>
                          setState(() => _maxTokensEnabled = value),
                    ),
                    _ModelParameterControl(
                      title: l10n.settingsModelTemperature,
                      subtitle: l10n.settingsModelTemperatureDescription,
                      controller: _temperatureController,
                      enabled: _temperatureEnabled,
                      onEnabledChanged: (value) =>
                          setState(() => _temperatureEnabled = value),
                    ),
                    _ModelParameterControl(
                      title: l10n.settingsModelTopP,
                      subtitle: l10n.settingsModelTopPDescription,
                      controller: _topPController,
                      enabled: _topPEnabled,
                      onEnabledChanged: (value) =>
                          setState(() => _topPEnabled = value),
                    ),
                    _ModelParameterControl(
                      title: l10n.settingsModelTopK,
                      subtitle: l10n.settingsModelTopKDescription,
                      controller: _topKController,
                      enabled: _topKEnabled,
                      integerOnly: true,
                      onEnabledChanged: (value) =>
                          setState(() => _topKEnabled = value),
                    ),
                    _ModelParameterControl(
                      title: l10n.settingsModelPresencePenalty,
                      subtitle: l10n.settingsModelPresencePenaltyDescription,
                      controller: _presencePenaltyController,
                      enabled: _presencePenaltyEnabled,
                      onEnabledChanged: (value) =>
                          setState(() => _presencePenaltyEnabled = value),
                    ),
                    _ModelParameterControl(
                      title: l10n.settingsModelFrequencyPenalty,
                      subtitle: l10n.settingsModelFrequencyPenaltyDescription,
                      controller: _frequencyPenaltyController,
                      enabled: _frequencyPenaltyEnabled,
                      onEnabledChanged: (value) =>
                          setState(() => _frequencyPenaltyEnabled = value),
                    ),
                    _ModelParameterControl(
                      title: l10n.settingsModelRepetitionPenalty,
                      subtitle: l10n.settingsModelRepetitionPenaltyDescription,
                      controller: _repetitionPenaltyController,
                      enabled: _repetitionPenaltyEnabled,
                      onEnabledChanged: (value) =>
                          setState(() => _repetitionPenaltyEnabled = value),
                    ),
                    _SwitchLine(
                      title: l10n.settingsModelToolCall,
                      subtitle: l10n.settingsModelToolCallDescription,
                      value: _enableToolCall,
                      onChanged: (value) =>
                          setState(() => _enableToolCall = value),
                    ),
                    _SwitchLine(
                      title: l10n.settingsModelDirectImage,
                      subtitle: l10n.settingsModelDirectImageDescription,
                      value: _enableDirectImage,
                      onChanged: (value) =>
                          setState(() => _enableDirectImage = value),
                    ),
                    _SwitchLine(
                      title: l10n.settingsModelDirectAudio,
                      subtitle: l10n.settingsModelDirectAudioDescription,
                      value: _enableDirectAudio,
                      onChanged: (value) =>
                          setState(() => _enableDirectAudio = value),
                    ),
                    _SwitchLine(
                      title: l10n.settingsModelDirectVideo,
                      subtitle: l10n.settingsModelDirectVideoDescription,
                      value: _enableDirectVideo,
                      onChanged: (value) =>
                          setState(() => _enableDirectVideo = value),
                    ),
                    _SwitchLine(
                      title: l10n.settingsModelGoogleSearch,
                      subtitle: l10n.settingsModelGoogleSearchDescription,
                      value: _enableGoogleSearch,
                      onChanged: (value) =>
                          setState(() => _enableGoogleSearch = value),
                    ),
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: _DialogTextField(
                            controller: _requestLimitController,
                            label: l10n.settingsModelRequestLimit,
                            numberOnly: true,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: _DialogTextField(
                            controller: _maxConcurrentController,
                            label: l10n.settingsModelMaxConcurrent,
                            numberOnly: true,
                          ),
                        ),
                      ],
                    ),
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: _DialogTextField(
                            controller: _contextLengthController,
                            label: l10n.settingsModelContextLength,
                            numberOnly: true,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: _DialogTextField(
                            controller: _maxContextLengthController,
                            label: l10n.settingsModelMaxContextLength,
                            numberOnly: true,
                          ),
                        ),
                      ],
                    ),
                    SwitchListTile(
                      contentPadding: EdgeInsets.zero,
                      title: Text(l10n.settingsModelMaxContextMode),
                      value: _enableMaxContextMode,
                      onChanged: (value) =>
                          setState(() => _enableMaxContextMode = value),
                    ),
                    SwitchListTile(
                      contentPadding: EdgeInsets.zero,
                      title: Text(l10n.settingsModelSummary),
                      value: _enableSummary,
                      onChanged: (value) =>
                          setState(() => _enableSummary = value),
                    ),
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: _DialogTextField(
                            controller: _summaryThresholdController,
                            label: l10n.settingsModelSummaryThreshold,
                            numberOnly: true,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: _DialogTextField(
                            controller: _summaryCountController,
                            label: l10n.settingsModelSummaryMessageCount,
                            numberOnly: true,
                          ),
                        ),
                      ],
                    ),
                    SwitchListTile(
                      contentPadding: EdgeInsets.zero,
                      title: Text(l10n.settingsModelSummaryByMessageCount),
                      value: _enableSummaryByMessageCount,
                      onChanged: (value) =>
                          setState(() => _enableSummaryByMessageCount = value),
                    ),
                    _DialogTextField(
                      controller: _customHeadersController,
                      label: l10n.settingsModelCustomHeaders,
                      maxLines: 4,
                    ),
                    _DialogTextField(
                      controller: _customParametersController,
                      label: l10n.settingsModelCustomParameters,
                      maxLines: 4,
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
    );
  }
}

class _ApiKeyPoolEditor extends StatelessWidget {
  const _ApiKeyPoolEditor({
    required this.apiKeyPool,
    required this.onAdd,
    required this.onEdit,
    required this.onToggle,
    required this.onDelete,
  });

  final List<core_proxy.ApiKeyInfo> apiKeyPool;
  final VoidCallback onAdd;
  final ValueChanged<core_proxy.ApiKeyInfo> onEdit;
  final void Function(core_proxy.ApiKeyInfo keyInfo, bool enabled) onToggle;
  final ValueChanged<core_proxy.ApiKeyInfo> onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: DecoratedBox(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: colorScheme.outlineVariant),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Expanded(
                    child: Text(
                      l10n.settingsModelApiKeyPoolCount(apiKeyPool.length),
                      style: const TextStyle(fontWeight: FontWeight.w800),
                    ),
                  ),
                  TextButton.icon(
                    onPressed: onAdd,
                    icon: const Icon(Icons.add),
                    label: Text(l10n.settingsModelAddApiKey),
                  ),
                ],
              ),
              if (apiKeyPool.isEmpty)
                Padding(
                  padding: const EdgeInsets.only(top: 4, bottom: 8),
                  child: Text(
                    l10n.settingsModelApiKeyPoolEmpty,
                    style: TextStyle(color: colorScheme.onSurfaceVariant),
                  ),
                )
              else
                for (final keyInfo in apiKeyPool)
                  _ApiKeyTile(
                    keyInfo: keyInfo,
                    onEdit: () => onEdit(keyInfo),
                    onToggle: (enabled) => onToggle(keyInfo, enabled),
                    onDelete: () => onDelete(keyInfo),
                  ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ApiKeyTile extends StatelessWidget {
  const _ApiKeyTile({
    required this.keyInfo,
    required this.onEdit,
    required this.onToggle,
    required this.onDelete,
  });

  final core_proxy.ApiKeyInfo keyInfo;
  final VoidCallback onEdit;
  final ValueChanged<bool> onToggle;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Switch(value: keyInfo.isEnabled, onChanged: onToggle),
      title: Text(keyInfo.name),
      subtitle: Text(
        '${_maskApiKey(keyInfo.key)} · ${_apiKeyStatusText(keyInfo.availabilityStatus)}',
      ),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          IconButton(
            tooltip: l10n.edit,
            onPressed: onEdit,
            icon: const Icon(Icons.edit_outlined),
          ),
          IconButton(
            tooltip: l10n.delete,
            onPressed: onDelete,
            icon: const Icon(Icons.delete_outline),
          ),
        ],
      ),
    );
  }
}

class _ApiKeyEditorDialog extends StatefulWidget {
  const _ApiKeyEditorDialog({required this.title, this.keyInfo});

  final String title;
  final core_proxy.ApiKeyInfo? keyInfo;

  static Future<core_proxy.ApiKeyInfo?> show({
    required BuildContext context,
    required String title,
    core_proxy.ApiKeyInfo? keyInfo,
  }) {
    return showDialog<core_proxy.ApiKeyInfo>(
      context: context,
      builder: (context) => _ApiKeyEditorDialog(title: title, keyInfo: keyInfo),
    );
  }

  @override
  State<_ApiKeyEditorDialog> createState() => _ApiKeyEditorDialogState();
}

class _ApiKeyEditorDialogState extends State<_ApiKeyEditorDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameController;
  late final TextEditingController _keyController;
  late bool _isEnabled;

  @override
  void initState() {
    super.initState();
    final keyInfo = widget.keyInfo;
    _nameController = TextEditingController(text: keyInfo?.name ?? '');
    _keyController = TextEditingController(text: keyInfo?.key ?? '');
    _isEnabled = keyInfo?.isEnabled ?? true;
  }

  @override
  void dispose() {
    _nameController.dispose();
    _keyController.dispose();
    super.dispose();
  }

  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    final keyInfo = widget.keyInfo;
    Navigator.of(context).pop(
      core_proxy.ApiKeyInfo(
        id: keyInfo?.id ?? _newApiKeyId(),
        key: _keyController.text.trim(),
        name: _nameController.text.trim(),
        isEnabled: _isEnabled,
        availabilityStatus: keyInfo?.availabilityStatus ?? 'UNTESTED',
        usageCount: keyInfo?.usageCount ?? 0,
        lastUsed: keyInfo?.lastUsed ?? 0,
        errorCount: keyInfo?.errorCount ?? 0,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 520,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                _DialogTextField(
                  controller: _nameController,
                  label: l10n.settingsModelApiKeyName,
                  requiredField: true,
                ),
                _DialogTextField(
                  controller: _keyController,
                  label: l10n.settingsModelApiKey,
                  requiredField: true,
                  obscureText: true,
                ),
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  title: Text(l10n.settingsModelApiKeyEnabled),
                  value: _isEnabled,
                  onChanged: (value) => setState(() => _isEnabled = value),
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
    );
  }
}

class _ModelParameterControl extends StatelessWidget {
  const _ModelParameterControl({
    required this.title,
    required this.subtitle,
    required this.controller,
    required this.enabled,
    required this.onEnabledChanged,
    this.integerOnly = false,
  });

  final String title;
  final String subtitle;
  final TextEditingController controller;
  final bool enabled;
  final ValueChanged<bool> onEnabledChanged;
  final bool integerOnly;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: DecoratedBox(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(14),
          border: Border.all(
            color: Theme.of(context).colorScheme.outlineVariant,
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 0),
          child: Column(
            children: <Widget>[
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: Text(title),
                subtitle: Text(subtitle),
                value: enabled,
                onChanged: onEnabledChanged,
              ),
              _DialogTextField(
                controller: controller,
                label: title,
                requiredField: true,
                numberOnly: integerOnly,
                decimalNumber: !integerOnly,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _DialogTextField extends StatelessWidget {
  const _DialogTextField({
    required this.controller,
    required this.label,
    this.requiredField = false,
    this.obscureText = false,
    this.numberOnly = false,
    this.decimalNumber = false,
    this.maxLines = 1,
  });

  final TextEditingController controller;
  final String label;
  final bool requiredField;
  final bool obscureText;
  final bool numberOnly;
  final bool decimalNumber;
  final int maxLines;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: TextFormField(
        controller: controller,
        obscureText: obscureText,
        maxLines: obscureText ? 1 : maxLines,
        keyboardType: numberOnly
            ? TextInputType.number
            : decimalNumber
            ? const TextInputType.numberWithOptions(decimal: true, signed: true)
            : TextInputType.text,
        inputFormatters: numberOnly
            ? <TextInputFormatter>[FilteringTextInputFormatter.digitsOnly]
            : decimalNumber
            ? <TextInputFormatter>[
                FilteringTextInputFormatter.allow(RegExp(r'[-0-9.]')),
              ]
            : null,
        decoration: InputDecoration(labelText: label),
        validator: (value) {
          final text = value?.trim() ?? '';
          if (requiredField && text.isEmpty) {
            return label;
          }
          if (numberOnly && text.isEmpty) {
            return label;
          }
          if (decimalNumber && double.tryParse(text) == null) {
            return label;
          }
          return null;
        },
      ),
    );
  }
}

const List<String> _functionTypes = <String>[
  'CHAT',
  'SUMMARY',
  'MEMORY',
  'UI_CONTROLLER',
  'TRANSLATION',
  'GREP',
  'ROLE_RESPONSE_PLANNER',
  'IMAGE_RECOGNITION',
  'AUDIO_RECOGNITION',
  'VIDEO_RECOGNITION',
];

class _IndexedModelName {
  const _IndexedModelName({required this.index, required this.name});

  final int index;
  final String name;
}

List<_IndexedModelName> _indexedModelNames(String raw) {
  final names = raw
      .split(',')
      .map((item) => item.trim())
      .where((item) => item.isNotEmpty)
      .toList(growable: false);
  return <_IndexedModelName>[
    for (var index = 0; index < names.length; index++)
      _IndexedModelName(index: index, name: names[index]),
  ];
}

String _modelNameAt(String raw, int index) {
  final names = raw
      .split(',')
      .map((item) => item.trim())
      .where((item) => item.isNotEmpty)
      .toList(growable: false);
  if (names.isEmpty) {
    return raw;
  }
  return names[index.clamp(0, names.length - 1)];
}

String _functionTypeTitle(AppLocalizations l10n, String functionType) {
  return switch (functionType) {
    'CHAT' => l10n.settingsModelFunctionChat,
    'SUMMARY' => l10n.settingsModelFunctionSummary,
    'MEMORY' => l10n.settingsModelFunctionMemory,
    'UI_CONTROLLER' => l10n.settingsModelFunctionUiController,
    'TRANSLATION' => l10n.settingsModelFunctionTranslation,
    'GREP' => l10n.settingsModelFunctionGrep,
    'ROLE_RESPONSE_PLANNER' => l10n.settingsModelFunctionRoleResponsePlanner,
    'IMAGE_RECOGNITION' => l10n.settingsModelFunctionImageRecognition,
    'AUDIO_RECOGNITION' => l10n.settingsModelFunctionAudioRecognition,
    'VIDEO_RECOGNITION' => l10n.settingsModelFunctionVideoRecognition,
    _ => functionType,
  };
}

String _functionTypeDescription(AppLocalizations l10n, String functionType) {
  return switch (functionType) {
    'CHAT' => l10n.settingsModelFunctionChatDescription,
    'SUMMARY' => l10n.settingsModelFunctionSummaryDescription,
    'MEMORY' => l10n.settingsModelFunctionMemoryDescription,
    'UI_CONTROLLER' => l10n.settingsModelFunctionUiControllerDescription,
    'TRANSLATION' => l10n.settingsModelFunctionTranslationDescription,
    'GREP' => l10n.settingsModelFunctionGrepDescription,
    'ROLE_RESPONSE_PLANNER' =>
      l10n.settingsModelFunctionRoleResponsePlannerDescription,
    'IMAGE_RECOGNITION' =>
      l10n.settingsModelFunctionImageRecognitionDescription,
    'AUDIO_RECOGNITION' =>
      l10n.settingsModelFunctionAudioRecognitionDescription,
    'VIDEO_RECOGNITION' =>
      l10n.settingsModelFunctionVideoRecognitionDescription,
    _ => functionType,
  };
}

String? _functionMappingWarning(
  AppLocalizations l10n,
  String functionType,
  core_proxy.ModelConfigData config,
) {
  return switch (functionType) {
    'IMAGE_RECOGNITION' when !config.enableDirectImageProcessing =>
      l10n.settingsModelFunctionImageUnsupported,
    'AUDIO_RECOGNITION' when !config.enableDirectAudioProcessing =>
      l10n.settingsModelFunctionAudioUnsupported,
    'VIDEO_RECOGNITION' when !config.enableDirectVideoProcessing =>
      l10n.settingsModelFunctionVideoUnsupported,
    _ => null,
  };
}

List<Object?> _standardModelParameterPayload(_ModelConfigEditResult result) {
  return <Object?>[
    _modelParameterJson(
      id: 'max_tokens',
      name: 'Max tokens',
      apiName: 'max_tokens',
      description: 'Maximum number of tokens to generate in one response',
      defaultValue: 4096,
      currentValue: result.maxTokens,
      isEnabled: result.maxTokensEnabled,
      valueType: 'INT',
      minValue: 1,
      maxValue: null,
      category: 'GENERATION',
    ),
    _modelParameterJson(
      id: 'temperature',
      name: 'Temperature',
      apiName: 'temperature',
      description:
          'Controls randomness: lower is more deterministic, higher is more random',
      defaultValue: 1.0,
      currentValue: result.temperature,
      isEnabled: result.temperatureEnabled,
      valueType: 'FLOAT',
      minValue: 0.0,
      maxValue: 2.0,
      category: 'CREATIVITY',
    ),
    _modelParameterJson(
      id: 'top_p',
      name: 'Top-p sampling',
      apiName: 'top_p',
      description:
          'Alternative to temperature: consider only tokens within cumulative probability top-p',
      defaultValue: 1.0,
      currentValue: result.topP,
      isEnabled: result.topPEnabled,
      valueType: 'FLOAT',
      minValue: 0.0,
      maxValue: 1.0,
      category: 'CREATIVITY',
    ),
    _modelParameterJson(
      id: 'top_k',
      name: 'Top-k sampling',
      apiName: 'top_k',
      description: 'Consider only the top-k tokens by probability. 0 disables',
      defaultValue: 0,
      currentValue: result.topK,
      isEnabled: result.topKEnabled,
      valueType: 'INT',
      minValue: 0,
      maxValue: 100,
      category: 'CREATIVITY',
    ),
    _modelParameterJson(
      id: 'presence_penalty',
      name: 'Presence penalty',
      apiName: 'presence_penalty',
      description:
          'Encourages new topics: higher values reduce repetition of existing tokens',
      defaultValue: 0.0,
      currentValue: result.presencePenalty,
      isEnabled: result.presencePenaltyEnabled,
      valueType: 'FLOAT',
      minValue: -2.0,
      maxValue: 2.0,
      category: 'REPETITION',
    ),
    _modelParameterJson(
      id: 'frequency_penalty',
      name: 'Frequency penalty',
      apiName: 'frequency_penalty',
      description:
          'Reduces repetition: higher values penalize tokens based on frequency',
      defaultValue: 0.0,
      currentValue: result.frequencyPenalty,
      isEnabled: result.frequencyPenaltyEnabled,
      valueType: 'FLOAT',
      minValue: -2.0,
      maxValue: 2.0,
      category: 'REPETITION',
    ),
    _modelParameterJson(
      id: 'repetition_penalty',
      name: 'Repetition penalty',
      apiName: 'repetition_penalty',
      description:
          'Further reduces repetition: 1.0 means no penalty; values > 1.0 discourage repetition',
      defaultValue: 1.0,
      currentValue: result.repetitionPenalty,
      isEnabled: result.repetitionPenaltyEnabled,
      valueType: 'FLOAT',
      minValue: 0.0,
      maxValue: 2.0,
      category: 'REPETITION',
    ),
  ];
}

Map<String, Object?> _modelParameterJson({
  required String id,
  required String name,
  required String apiName,
  required String description,
  required Object defaultValue,
  required Object currentValue,
  required bool isEnabled,
  required String valueType,
  required Object? minValue,
  required Object? maxValue,
  required String category,
}) {
  return <String, Object?>{
    'id': id,
    'name': name,
    'apiName': apiName,
    'description': description,
    'defaultValue': defaultValue,
    'currentValue': currentValue,
    'isEnabled': isEnabled,
    'valueType': valueType,
    'minValue': minValue,
    'maxValue': maxValue,
    'category': category,
    'isCustom': false,
  };
}

List<core_proxy.ApiKeyInfo> _apiKeyPoolFromRaw(List<Object?> rawItems) {
  return rawItems
      .map(
        (item) => core_proxy.ApiKeyInfo.fromJson(
          Map<String, Object?>.from(item as Map),
        ),
      )
      .toList(growable: false);
}

core_proxy.ApiKeyInfo _apiKeyInfoWith(
  core_proxy.ApiKeyInfo keyInfo, {
  bool? isEnabled,
}) {
  return core_proxy.ApiKeyInfo(
    id: keyInfo.id,
    key: keyInfo.key,
    name: keyInfo.name,
    isEnabled: isEnabled ?? keyInfo.isEnabled,
    availabilityStatus: keyInfo.availabilityStatus,
    usageCount: keyInfo.usageCount,
    lastUsed: keyInfo.lastUsed,
    errorCount: keyInfo.errorCount,
  );
}

String _newApiKeyId() {
  return 'api-key-${DateTime.now().microsecondsSinceEpoch}';
}

String _maskApiKey(String key) {
  if (key.length <= 8) {
    return '****';
  }
  return '${key.substring(0, 4)}****${key.substring(key.length - 4)}';
}

String _apiKeyStatusText(Object? status) {
  final text = '$status';
  final match = RegExp(r'[A-Z_]+').firstMatch(text);
  return match?.group(0) ?? text;
}

class _ModelSummaryTile extends StatelessWidget {
  const _ModelSummaryTile({
    required this.summary,
    required this.selected,
    required this.deletable,
    required this.onTap,
    required this.onEdit,
    required this.onDelete,
  });

  final core_proxy.ModelConfigSummary summary;
  final bool selected;
  final bool deletable;
  final VoidCallback onTap;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(
        selected ? Icons.radio_button_checked : Icons.radio_button_unchecked,
        color: selected ? colorScheme.primary : colorScheme.onSurfaceVariant,
      ),
      title: Text(summary.name),
      subtitle: Text('${summary.apiEndpoint}\n${summary.modelName}'),
      isThreeLine: true,
      onTap: onTap,
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          IconButton(
            tooltip: l10n.edit,
            onPressed: onEdit,
            icon: const Icon(Icons.edit_outlined),
          ),
          IconButton(
            tooltip: l10n.delete,
            onPressed: deletable ? onDelete : null,
            icon: const Icon(Icons.delete_outline),
          ),
        ],
      ),
    );
  }
}

class _FunctionMappingTile extends StatelessWidget {
  const _FunctionMappingTile({
    required this.functionType,
    required this.mapping,
    required this.summary,
    required this.config,
    required this.onTap,
  });

  final String functionType;
  final core_proxy.FunctionConfigMapping mapping;
  final core_proxy.ModelConfigSummary summary;
  final core_proxy.ModelConfigData config;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final warning = _functionMappingWarning(l10n, functionType, config);
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text(_functionTypeTitle(l10n, functionType)),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(_functionTypeDescription(l10n, functionType)),
          const SizedBox(height: 4),
          Text(
            l10n.settingsModelFunctionMappingsCurrent(
              summary.name,
              _modelNameAt(summary.modelName, mapping.modelIndex),
            ),
            style: TextStyle(
              color: colorScheme.primary,
              fontWeight: FontWeight.w700,
            ),
          ),
          if (warning != null) ...<Widget>[
            const SizedBox(height: 4),
            Row(
              children: <Widget>[
                Icon(
                  Icons.warning_amber_outlined,
                  size: 16,
                  color: colorScheme.error,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    warning,
                    style: TextStyle(color: colorScheme.error),
                  ),
                ),
              ],
            ),
          ],
        ],
      ),
      trailing: TextButton(
        onPressed: onTap,
        child: Text(l10n.settingsModelFunctionMappingsChange),
      ),
      onTap: onTap,
    );
  }
}

class _FunctionModelSelection {
  const _FunctionModelSelection({
    required this.configId,
    required this.modelIndex,
  });

  final String configId;
  final int modelIndex;
}

class _FunctionModelSelectorDialog extends StatelessWidget {
  const _FunctionModelSelectorDialog({
    required this.functionType,
    required this.summaries,
    required this.currentMapping,
  });

  final String functionType;
  final List<core_proxy.ModelConfigSummary> summaries;
  final core_proxy.FunctionConfigMapping currentMapping;

  static Future<_FunctionModelSelection?> show({
    required BuildContext context,
    required String functionType,
    required List<core_proxy.ModelConfigSummary> summaries,
    required core_proxy.FunctionConfigMapping currentMapping,
  }) {
    return showDialog<_FunctionModelSelection>(
      context: context,
      builder: (context) => _FunctionModelSelectorDialog(
        functionType: functionType,
        summaries: summaries,
        currentMapping: currentMapping,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(
        l10n.settingsModelFunctionMappingsSelect(
          _functionTypeTitle(l10n, functionType),
        ),
      ),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              for (final summary in summaries)
                for (final indexedModel in _indexedModelNames(
                  summary.modelName,
                ))
                  _FunctionModelOptionTile(
                    summary: summary,
                    modelName: indexedModel.name,
                    modelIndex: indexedModel.index,
                    selected:
                        summary.id == currentMapping.configId &&
                        indexedModel.index == currentMapping.modelIndex,
                    onTap: () {
                      Navigator.of(context).pop(
                        _FunctionModelSelection(
                          configId: summary.id,
                          modelIndex: indexedModel.index,
                        ),
                      );
                    },
                  ),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
    );
  }
}

class _FunctionModelOptionTile extends StatelessWidget {
  const _FunctionModelOptionTile({
    required this.summary,
    required this.modelName,
    required this.modelIndex,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.ModelConfigSummary summary;
  final String modelName;
  final int modelIndex;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(
        selected ? Icons.check_circle : Icons.circle_outlined,
        color: selected ? colorScheme.primary : colorScheme.onSurfaceVariant,
      ),
      title: Text(summary.name),
      subtitle: Text(modelName),
      trailing: Text('#${modelIndex + 1}'),
      onTap: onTap,
    );
  }
}

class _ConnectionTestItemTile extends StatelessWidget {
  const _ConnectionTestItemTile({required this.item});

  final core_proxy.ModelConnectionTestItem item;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final color = item.success ? Colors.green : colorScheme.error;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(
        item.success ? Icons.check_circle_outline : Icons.error_outline,
        color: color,
      ),
      title: Text(_connectionTestTypeLabel(l10n, item.type)),
      subtitle: item.error == null ? null : Text(item.error!),
      trailing: Text(
        item.success
            ? l10n.settingsModelConnectionTestPassed
            : l10n.settingsModelConnectionTestFailed,
        style: TextStyle(color: color, fontWeight: FontWeight.w700),
      ),
    );
  }
}

String _connectionTestTypeLabel(AppLocalizations l10n, Object? type) {
  return switch (type) {
    'CHAT' => l10n.settingsModelTestItemChat,
    'TOOL_CALL' => l10n.settingsModelTestItemToolCall,
    'IMAGE' => l10n.settingsModelTestItemImage,
    'AUDIO' => l10n.settingsModelTestItemAudio,
    'VIDEO' => l10n.settingsModelTestItemVideo,
    _ => l10n.settingsModelTestItemUnknown,
  };
}

bool _connectionTestSucceeded(
  core_proxy.ModelConnectionTestReport report,
  String type,
) {
  return report.items.any((item) => item.type == type && item.success);
}

class _SettingsHero extends StatelessWidget {
  const _SettingsHero({
    required this.icon,
    required this.title,
    required this.description,
  });

  final IconData icon;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 18),
      child: Row(
        children: <Widget>[
          CircleAvatar(
            radius: 24,
            backgroundColor: colorScheme.primaryContainer,
            child: Icon(icon, color: colorScheme.onPrimaryContainer),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  title,
                  style: theme.textTheme.headlineSmall?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  description,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
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
    final radius = BorderRadius.circular(18);
    return Padding(
      padding: const EdgeInsets.only(bottom: 14),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: ExpansionTile(
          initiallyExpanded: initiallyExpanded,
          shape: RoundedRectangleBorder(borderRadius: radius),
          collapsedShape: RoundedRectangleBorder(borderRadius: radius),
          title: Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  title,
                  style: const TextStyle(fontWeight: FontWeight.w800),
                ),
              ),
              ?action,
            ],
          ),
          childrenPadding: const EdgeInsets.fromLTRB(18, 0, 18, 16),
          children: children,
        ),
      ),
    );
  }
}

class _InfoLine extends StatelessWidget {
  const _InfoLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 9),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(label)),
          const SizedBox(width: 12),
          Flexible(
            child: Text(
              value,
              textAlign: TextAlign.end,
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            ),
          ),
        ],
      ),
    );
  }
}

class _SwitchLine extends StatelessWidget {
  const _SwitchLine({
    required this.title,
    required this.subtitle,
    required this.value,
    required this.onChanged,
  });

  final String title;
  final String subtitle;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SwitchListTile(
      contentPadding: EdgeInsets.zero,
      title: Text(title),
      subtitle: Text(
        subtitle,
        style: TextStyle(color: colorScheme.onSurfaceVariant),
      ),
      value: value,
      onChanged: onChanged,
    );
  }
}
