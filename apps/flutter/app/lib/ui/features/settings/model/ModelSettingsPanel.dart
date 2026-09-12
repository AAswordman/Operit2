// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/CommonNetworkErrorView.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitFormStyles.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class ModelSettingsPanel extends StatefulWidget {
  const ModelSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  /// Creates the state object that owns model settings loading.
  @override
  ModelSettingsPanelState createState() => ModelSettingsPanelState();
}

class ModelSettingsPanelState extends State<ModelSettingsPanel> {
  Future<ModelSettingsData>? _future;
  String? _testingModelKey;
  final Set<String> _expandedProviderIds = <String>{};
  bool _providerExpansionInitialized = false;

  @override
  void initState() {
    super.initState();
    _future = load();
  }

  /// Loads the complete model settings snapshot from the runtime.
  Future<ModelSettingsData> load() async {
    final modelManager = widget.clients.preferencesModelConfigManager;
    final functionManager = widget.clients.preferencesFunctionalConfigManager;
    final apiPreferences = widget.clients.preferencesApiPreferences;
    final chatBinding = await functionManager.getModelBindingForFunction(
      functionType: core_proxy.FunctionType.chat,
    );
    final data = ModelSettingsData(
      providers: await modelManager.getProviderProfiles(),
      summaries: await modelManager.getAllModelSummaries(),
      chatBinding: chatBinding,
      currentConfig: await modelManager.getResolvedModelConfig(
        providerId: chatBinding.providerId,
        modelId: chatBinding.modelId,
      ),
      functionBindings: await functionManager.functionModelBindingFlow().first,
      maxImageHistoryUserTurns: await apiPreferences
          .maxImageHistoryUserTurnsFlow()
          .first,
      maxMediaHistoryUserTurns: await apiPreferences
          .maxMediaHistoryUserTurnsFlow()
          .first,
    );
    if (!_providerExpansionInitialized) {
      _providerExpansionInitialized = true;
      for (final provider in data.providers) {
        if (provider.id == data.chatBinding.providerId) {
          _expandedProviderIds.add(provider.id);
          break;
        }
      }
    }
    return data;
  }

  /// Returns the load operation started when the panel was initialized.
  Future<ModelSettingsData> get loadFuture => _future!;

  void _reload() {
    setState(() {
      _future = load();
    });
  }

  Future<void> _selectChatModel(String providerId, String modelId) async {
    final l10n = AppLocalizations.of(context)!;
    if (modelId.toLowerCase().contains('autoglm')) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsModelChatAutoGlmWarning)),
      );
      return;
    }
    await widget.clients.preferencesFunctionalConfigManager.setModelForFunction(
      functionType: core_proxy.FunctionType.chat,
      providerId: providerId,
      modelId: modelId,
    );
    _reload();
  }

  Future<void> _selectFunctionModel(
    core_proxy.FunctionType functionType,
    ModelSettingsData data,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final selected = await _FunctionModelSelectorDialog.show(
      context: context,
      functionType: functionType,
      summaries: data.summaries,
      currentBinding: data.functionBindings[functionType]!,
    );
    if (selected == null) {
      return;
    }
    if (functionType == core_proxy.FunctionType.chat &&
        selected.modelId.toLowerCase().contains('autoglm')) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsModelChatAutoGlmWarning)),
      );
      return;
    }
    await widget.clients.preferencesFunctionalConfigManager.setModelForFunction(
      functionType: functionType,
      providerId: selected.providerId,
      modelId: selected.modelId,
    );
    _reload();
  }

  Future<void> _createProvider() async {
    final catalogEntries = await widget.clients.preferencesModelConfigManager
        .getProviderCatalogEntries();
    if (!mounted) {
      return;
    }
    final result = await _ProviderEditorDialog.show(
      context: context,
      catalogEntries: catalogEntries,
    );
    if (result == null || result is! _ProviderEditSaveResult) {
      return;
    }
    final providerId = await widget.clients.preferencesModelConfigManager
        .createProvider(
          name: result.name,
          providerTypeId: result.providerTypeId,
          endpoint: result.endpoint,
        );
    final provider = await widget.clients.preferencesModelConfigManager
        .getProviderProfile(providerId: providerId);
    await widget.clients.preferencesModelConfigManager.updateProviderProfile(
      provider: core_proxy.ProviderProfile(
        id: provider.id,
        name: provider.name,
        providerTypeId: provider.providerTypeId,
        providerType: provider.providerType,
        endpoint: provider.endpoint,
        apiKey: result.apiKey,
        useMultipleApiKeys: provider.useMultipleApiKeys,
        apiKeyPool: provider.apiKeyPool,
        currentKeyIndex: provider.currentKeyIndex,
        keyRotationMode: provider.keyRotationMode,
        customHeaders: result.customHeaders,
        requestLimitPerMinute: result.requestLimitPerMinute,
        maxConcurrentRequests: result.maxConcurrentRequests,
        thinkingConfigurations:
            result.thinkingConfigurations ?? provider.thinkingConfigurations,
        thinkingOptionId: provider.thinkingOptionId,
        models: provider.models,
      ),
    );
    _expandedProviderIds.add(providerId);
    _reload();
  }

  Future<void> _editProvider(core_proxy.ProviderProfile provider) async {
    final catalogEntries = await widget.clients.preferencesModelConfigManager
        .getProviderCatalogEntries();
    if (!mounted) {
      return;
    }
    final result = await _ProviderEditorDialog.show(
      context: context,
      catalogEntries: catalogEntries,
      provider: provider,
    );
    if (result == null) {
      return;
    }
    if (result is _ProviderEditDeleteResult) {
      await _deleteProvider(provider);
      return;
    }
    final saveResult = result as _ProviderEditSaveResult;
    await widget.clients.preferencesModelConfigManager.updateProviderProfile(
      provider: core_proxy.ProviderProfile(
        id: provider.id,
        name: saveResult.name,
        providerTypeId: provider.providerTypeId,
        providerType: provider.providerType,
        endpoint: saveResult.endpoint,
        apiKey: saveResult.apiKey,
        useMultipleApiKeys: provider.useMultipleApiKeys,
        apiKeyPool: provider.apiKeyPool,
        currentKeyIndex: provider.currentKeyIndex,
        keyRotationMode: provider.keyRotationMode,
        customHeaders: saveResult.customHeaders,
        requestLimitPerMinute: saveResult.requestLimitPerMinute,
        maxConcurrentRequests: saveResult.maxConcurrentRequests,
        thinkingConfigurations:
            saveResult.thinkingConfigurations ??
            provider.thinkingConfigurations,
        thinkingOptionId: provider.thinkingOptionId,
        models: provider.models,
      ),
    );
    _reload();
  }

  Future<void> _deleteProvider(core_proxy.ProviderProfile provider) async {
    final bindings = await widget.clients.preferencesFunctionalConfigManager
        .functionModelBindingFlow()
        .first;
    final boundFunctions = _boundFunctionTypesForProvider(
      bindings,
      provider.id,
    );
    if (boundFunctions.isNotEmpty) {
      if (!mounted) {
        return;
      }
      await _DeleteProviderBlockedDialog.show(
        context: context,
        functionTypes: boundFunctions,
      );
      return;
    }
    if (!mounted) {
      return;
    }
    final confirmed = await _DeleteProviderConfirmDialog.show(
      context: context,
      providerName: provider.name,
      modelCount: provider.models.length,
    );
    if (confirmed != true) {
      return;
    }
    await widget.clients.preferencesModelConfigManager.deleteProvider(
      providerId: provider.id,
    );
    _expandedProviderIds.remove(provider.id);
    _reload();
  }

  void _toggleProviderExpanded(String providerId) {
    setState(() {
      if (_expandedProviderIds.contains(providerId)) {
        _expandedProviderIds.remove(providerId);
      } else {
        _expandedProviderIds.add(providerId);
      }
    });
  }

  /// Adds a provider model from the remote catalog or a user-entered model ID.
  Future<void> _addProviderModel(core_proxy.ProviderProfile provider) async {
    final List<core_proxy.AvailableProviderModel> availableModels;
    try {
      availableModels = await widget.clients.preferencesModelConfigManager
          .getAvailableProviderModels(providerId: provider.id);
    } on CoreLinkError catch (error) {
      if (!mounted) {
        return;
      }
      final action = await _AddProviderModelErrorDialog.show(
        context: context,
        errorDetails: core_proxy.CoreProxyErrorDetails.fromCoreLinkError(error),
        providerName: provider.name,
        showCustomAction: true,
      );
      if (action == _AddProviderModelErrorAction.custom && mounted) {
        try {
          await _createCustomProviderModel(provider);
        } on CoreLinkError catch (error) {
          if (!mounted) {
            return;
          }
          await _AddProviderModelErrorDialog.show(
            context: context,
            errorDetails: core_proxy.CoreProxyErrorDetails.fromCoreLinkError(
              error,
            ),
            providerName: provider.name,
          );
        }
      }
      return;
    }

    final existingModelIds = provider.models.map((model) => model.id).toSet();
    final selectableModels = availableModels
        .where((model) => !existingModelIds.contains(model.modelId))
        .toList(growable: false);
    if (!mounted) {
      return;
    }
    final selection = await _AvailableModelDialog.show(
      context: context,
      models: selectableModels,
    );
    if (selection == null) {
      return;
    }
    try {
      switch (selection) {
        case _AvailableModelsPicked(:final models):
          for (final model in models) {
            await widget.clients.preferencesModelConfigManager
                .addProviderModelFromAvailable(
                  providerId: provider.id,
                  modelId: model.modelId,
                );
          }
          _reload();
        case _AvailableModelCustom():
          if (!mounted) {
            return;
          }
          await _createCustomProviderModel(provider);
      }
    } on CoreLinkError catch (error) {
      if (!mounted) {
        return;
      }
      _reload();
      await _AddProviderModelErrorDialog.show(
        context: context,
        errorDetails: core_proxy.CoreProxyErrorDetails.fromCoreLinkError(error),
        providerName: provider.name,
      );
    }
  }

  /// Prompts for and creates a provider model with a user-entered model ID.
  Future<void> _createCustomProviderModel(
    core_proxy.ProviderProfile provider,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final modelId = await _TextInputDialog.show(
      context: context,
      title: l10n.settingsModelCustomModel,
      label: l10n.settingsModelModelId,
      validator: (value) {
        final normalizedModelId = value!.trim();
        final isDuplicate = provider.models.any(
          (model) => model.id == normalizedModelId,
        );
        return isDuplicate ? l10n.settingsModelDuplicateModelId : null;
      },
    );
    if (modelId == null) {
      return;
    }
    await widget.clients.preferencesModelConfigManager.createProviderModel(
      providerId: provider.id,
      modelId: modelId,
    );
    _reload();
  }

  Future<void> _deleteModel(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    final bindings = await widget.clients.preferencesFunctionalConfigManager
        .functionModelBindingFlow()
        .first;
    final boundFunctions = _boundFunctionTypesForModel(
      bindings,
      provider.id,
      model.id,
    );
    if (boundFunctions.isNotEmpty) {
      if (!mounted) {
        return;
      }
      await _DeleteModelBlockedDialog.show(
        context: context,
        functionTypes: boundFunctions,
      );
      return;
    }
    await widget.clients.preferencesModelConfigManager.deleteModel(
      providerId: provider.id,
      modelId: model.id,
    );
    _reload();
  }

  Future<void> _editModelSettings(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    final config = await widget.clients.preferencesModelConfigManager
        .getResolvedModelConfig(providerId: provider.id, modelId: model.id);
    if (!mounted) {
      return;
    }
    final result = await _ModelSettingsEditorDialog.show(
      context: context,
      providerName: provider.name,
      modelId: model.id,
      initialCapabilities: config.capabilities,
      initialBuiltinTools: config.builtinTools,
      initialContext: config.context,
      initialSummary: config.summary,
      onTest: () => _testModelConnection(provider, model),
    );
    if (result == null || !mounted) {
      return;
    }
    final _ModelSettingsChange changed;
    switch (result) {
      case _ModelSettingsDeleteRequested():
        await _deleteModel(provider, model);
        return;
      case _ModelSettingsSaved(:final change):
        changed = change;
    }
    if (changed.capabilities != config.capabilities) {
      await widget.clients.preferencesModelConfigManager
          .updateCapabilitiesForModel(
            providerId: provider.id,
            modelId: model.id,
            capabilities: changed.capabilities,
          );
    }
    if (changed.builtinTools != config.builtinTools) {
      await widget.clients.preferencesModelConfigManager
          .updateBuiltinToolsForModel(
            providerId: provider.id,
            modelId: model.id,
            builtinTools: changed.builtinTools,
          );
    }
    if (changed.context != config.context) {
      await widget.clients.preferencesModelConfigManager.updateContextForModel(
        providerId: provider.id,
        modelId: model.id,
        context: changed.context,
      );
    }
    if (changed.summary != config.summary) {
      await widget.clients.preferencesModelConfigManager.updateSummaryForModel(
        providerId: provider.id,
        modelId: model.id,
        summary: changed.summary,
      );
    }
    if (!mounted) {
      return;
    }
    _reload();
  }

  Future<core_proxy.ModelConnectionTestReport?> _testModelConnection(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final testKey = _modelTestKey(provider.id, model.id);
    setState(() {
      _testingModelKey = testKey;
    });
    try {
      final report = await widget.clients.application.testModelConnection(
        providerId: provider.id,
        modelId: model.id,
      );
      if (!mounted) {
        return report;
      }
      await _applyConnectionTestCapabilities(
        providerId: provider.id,
        modelId: model.id,
        report: report,
      );
      if (!mounted) {
        return report;
      }
      await _ConnectionTestReportDialog.show(context: context, report: report);
      return report;
    } catch (error) {
      if (!mounted) {
        return null;
      }
      await _ConnectionTestErrorDialog.show(
        context: context,
        message: l10n.settingsModelConnectionTestError('$error'),
      );
      return null;
    } finally {
      if (mounted && _testingModelKey == testKey) {
        setState(() {
          _testingModelKey = null;
        });
      }
    }
  }

  Future<void> _applyConnectionTestCapabilities({
    required String providerId,
    required String modelId,
    required core_proxy.ModelConnectionTestReport report,
  }) async {
    final chatPassed = _connectionTestSucceeded(
      report,
      core_proxy.ModelConnectionTestType.chat,
    );
    if (!chatPassed) {
      return;
    }
    await widget.clients.preferencesModelConfigManager
        .updateCapabilitiesForModel(
          providerId: providerId,
          modelId: modelId,
          capabilities: core_proxy.ModelCapabilities(
            directImage: _connectionTestSucceeded(
              report,
              core_proxy.ModelConnectionTestType.image,
            ),
            directAudio: _connectionTestSucceeded(
              report,
              core_proxy.ModelConnectionTestType.audio,
            ),
            directVideo: _connectionTestSucceeded(
              report,
              core_proxy.ModelConnectionTestType.video,
            ),
            toolCall: _connectionTestSucceeded(
              report,
              core_proxy.ModelConnectionTestType.toolCall,
            ),
          ),
        );
    if (!mounted) {
      return;
    }
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<ModelSettingsData>(
      future: _future,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
        }
        final data = snapshot.data;
        if (data == null) {
          return const M3LoadingPane();
        }
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: <Widget>[
            _SectionCard(
              title: l10n.settingsModelProvidersSection,
              action: FilledButton.icon(
                onPressed: _createProvider,
                style: SettingsControlStyles.sectionFilledButton(),
                icon: const Icon(Icons.add, size: 18),
                label: Text(l10n.create),
              ),
              children: <Widget>[
                _ProviderModelManager(
                  providers: data.providers,
                  summaries: data.summaries,
                  chatBinding: data.chatBinding,
                  expandedProviderIds: _expandedProviderIds,
                  onToggleProviderExpanded: _toggleProviderExpanded,
                  onEditProvider: _editProvider,
                  onAddModel: _addProviderModel,
                  onSelectModel: _selectChatModel,
                  testingModelKey: _testingModelKey,
                  onEditModelSettings: _editModelSettings,
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsModelFunctionMappingsSection,
              initiallyExpanded: false,
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
                    binding: data.functionBindings[functionType]!,
                    summary: data.summaryForBinding(
                      data.functionBindings[functionType]!,
                    ),
                    onTap: () => _selectFunctionModel(functionType, data),
                  ),
              ],
            ),
          ],
        );
      },
    );
  }
}

class ModelSettingsData {
  /// Creates an immutable model settings snapshot.
  const ModelSettingsData({
    required this.providers,
    required this.summaries,
    required this.chatBinding,
    required this.currentConfig,
    required this.functionBindings,
    required this.maxImageHistoryUserTurns,
    required this.maxMediaHistoryUserTurns,
  });

  final List<core_proxy.ProviderProfile> providers;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding chatBinding;
  final core_proxy.ResolvedModelConfig currentConfig;
  final Map<core_proxy.FunctionType, core_proxy.FunctionModelBinding>
  functionBindings;
  final int maxImageHistoryUserTurns;
  final int maxMediaHistoryUserTurns;

  /// Finds the model summary assigned to the provided function binding.
  core_proxy.ProviderModelSummary? summaryForBinding(
    core_proxy.FunctionModelBinding binding,
  ) {
    for (final summary in summaries) {
      if (summary.providerId == binding.providerId &&
          summary.modelId == binding.modelId) {
        return summary;
      }
    }
    return null;
  }
}

sealed class _ProviderEditResult {
  const _ProviderEditResult();
}

class _ProviderEditSaveResult extends _ProviderEditResult {
  const _ProviderEditSaveResult({
    required this.name,
    required this.providerTypeId,
    required this.endpoint,
    required this.apiKey,
    required this.customHeaders,
    required this.requestLimitPerMinute,
    required this.maxConcurrentRequests,
    required this.thinkingConfigurations,
  });

  final String name;
  final String providerTypeId;
  final String endpoint;
  final String apiKey;
  final String customHeaders;
  final int requestLimitPerMinute;
  final int maxConcurrentRequests;
  final String? thinkingConfigurations;
}

class _ProviderEditDeleteResult extends _ProviderEditResult {
  const _ProviderEditDeleteResult();
}

class _ProviderEditorDialog extends StatefulWidget {
  const _ProviderEditorDialog({required this.catalogEntries, this.provider});

  final List<core_proxy.ProviderCatalogEntry> catalogEntries;
  final core_proxy.ProviderProfile? provider;

  static Future<_ProviderEditResult?> show({
    required BuildContext context,
    required List<core_proxy.ProviderCatalogEntry> catalogEntries,
    core_proxy.ProviderProfile? provider,
  }) {
    return showDialog<_ProviderEditResult>(
      context: context,
      builder: (context) => _ProviderEditorDialog(
        catalogEntries: catalogEntries,
        provider: provider,
      ),
    );
  }

  @override
  State<_ProviderEditorDialog> createState() => _ProviderEditorDialogState();
}

class _ProviderEditorDialogState extends State<_ProviderEditorDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameController;
  late final TextEditingController _endpointController;
  late final TextEditingController _apiKeyController;
  late final TextEditingController _customHeadersController;
  late final TextEditingController _requestLimitController;
  late final TextEditingController _maxConcurrentController;
  late List<_ThinkingRuleEditor> _thinkingRules;
  String? _thinkingConfigError;
  bool _thinkingRulesChanged = false;
  String? _selectedProviderTypeId;

  @override
  void initState() {
    super.initState();
    final provider = widget.provider;
    _nameController = TextEditingController(text: provider?.name ?? '');
    _endpointController = TextEditingController(text: provider?.endpoint ?? '');
    _apiKeyController = TextEditingController(text: provider?.apiKey ?? '');
    _customHeadersController = TextEditingController(
      text: provider?.customHeaders ?? '{}',
    );
    _requestLimitController = TextEditingController(
      text: (provider?.requestLimitPerMinute ?? 0).toString(),
    );
    _maxConcurrentController = TextEditingController(
      text: (provider?.maxConcurrentRequests ?? 1).toString(),
    );
    if (provider != null) {
      _selectedProviderTypeId = provider.providerTypeId;
      try {
        _thinkingRules = _parseThinkingRuleEditors(
          provider.thinkingConfigurations,
        );
      } on FormatException catch (error) {
        _thinkingRules = <_ThinkingRuleEditor>[];
        _thinkingConfigError = error.message;
      }
    } else {
      _thinkingRules = <_ThinkingRuleEditor>[];
      _selectedProviderTypeId = _catalogDeepseek()?.providerTypeId;
      final catalog = _selectedCatalog();
      if (catalog != null) {
        _endpointController.text = catalog.defaultEndpoint;
      }
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _endpointController.dispose();
    _apiKeyController.dispose();
    _customHeadersController.dispose();
    _requestLimitController.dispose();
    _maxConcurrentController.dispose();
    super.dispose();
  }

  core_proxy.ProviderCatalogEntry? _catalogDeepseek() {
    for (final entry in widget.catalogEntries) {
      if (entry.providerTypeId == 'DEEPSEEK') {
        return entry;
      }
    }
    return null;
  }

  /// Finds the provider catalog entry for a provider type identifier.
  core_proxy.ProviderCatalogEntry? _catalogByProviderTypeId(
    String? providerTypeId,
  ) {
    if (providerTypeId == null) {
      return null;
    }
    for (final entry in widget.catalogEntries) {
      if (entry.providerTypeId == providerTypeId) {
        return entry;
      }
    }
    return null;
  }

  /// Returns the catalog entry for the selected provider type.
  core_proxy.ProviderCatalogEntry? _selectedCatalog() {
    return _catalogByProviderTypeId(_selectedProviderTypeId);
  }

  /// Returns selectable endpoints declared for the selected provider type.
  List<core_proxy.ProviderEndpointOption> get _selectedEndpointOptions {
    final catalog = _selectedCatalog();
    if (catalog == null) {
      return const <core_proxy.ProviderEndpointOption>[];
    }
    return catalog.endpointOptions;
  }

  /// Applies a provider type selection and its catalog endpoint.
  void _onProviderTypeChanged(String? providerTypeId) {
    final catalog = _catalogByProviderTypeId(providerTypeId);
    setState(() {
      _selectedProviderTypeId = providerTypeId;
      if (catalog != null) {
        _endpointController.text = catalog.defaultEndpoint;
      }
    });
  }

  /// Opens the endpoint selector for providers with declared options.
  Future<void> _showEndpointOptionsDialog() async {
    final options = _selectedEndpointOptions;
    if (options.isEmpty) {
      return;
    }
    final selectedEndpoint = _endpointController.text.trim();
    await showDialog<void>(
      context: context,
      builder: (dialogContext) {
        final l10n = AppLocalizations.of(dialogContext)!;
        return AlertDialog(
          title: Text(l10n.settingsModelApiEndpoint),
          content: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: SizedBox(
              width: 520,
              height: (options.length * 64.0).clamp(64.0, 360.0),
              child: ListView.separated(
                itemCount: options.length,
                separatorBuilder: (context, index) => const Divider(height: 1),
                itemBuilder: (context, index) {
                  final option = options[index];
                  final selected = option.endpoint == selectedEndpoint;
                  return ListTile(
                    selected: selected,
                    leading: selected
                        ? const Icon(Icons.check_rounded)
                        : const SizedBox(width: 24),
                    title: Text(option.endpoint),
                    subtitle: option.label == option.endpoint
                        ? null
                        : Text(option.label),
                    onTap: () {
                      setState(() {
                        _endpointController.text = option.endpoint;
                      });
                      Navigator.of(dialogContext).pop();
                    },
                  );
                },
              ),
            ),
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: Text(l10n.cancel),
            ),
          ],
        );
      },
    );
  }

  /// Replaces the editable thinking rule list after a UI change.
  void _setThinkingRules(List<_ThinkingRuleEditor> rules) {
    setState(() {
      _thinkingRules = rules;
      _thinkingRulesChanged = true;
      _thinkingConfigError = null;
    });
  }

  /// Serializes provider thinking rules and reports validation problems.
  String? _thinkingConfigurationsForSave() {
    final provider = widget.provider;
    if (provider == null) {
      return null;
    }
    if (_thinkingConfigError != null && !_thinkingRulesChanged) {
      return null;
    }
    try {
      return _serializeThinkingRuleEditors(_thinkingRules);
    } on FormatException catch (error) {
      setState(() {
        _thinkingConfigError = error.message;
      });
      return null;
    }
  }

  /// Saves provider settings from the dialog form.
  void _save() {
    if (!_formKey.currentState!.validate() || _selectedProviderTypeId == null) {
      return;
    }
    final thinkingConfigurations = _thinkingConfigurationsForSave();
    if (widget.provider != null && thinkingConfigurations == null) {
      return;
    }
    Navigator.of(context).pop(
      _ProviderEditSaveResult(
        name: _nameController.text.trim(),
        providerTypeId: _selectedProviderTypeId!,
        endpoint: _endpointController.text.trim(),
        apiKey: _apiKeyController.text,
        customHeaders: _customHeadersController.text,
        requestLimitPerMinute: int.parse(_requestLimitController.text),
        maxConcurrentRequests: int.parse(_maxConcurrentController.text),
        thinkingConfigurations: thinkingConfigurations,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final editing = widget.provider != null;
    final endpointOptions = _selectedEndpointOptions;
    return AlertDialog(
      title: Text(
        editing
            ? l10n.settingsModelEditProvider
            : l10n.settingsModelCreateProvider,
      ),
      content: SizedBox(
        width: 680,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                _DialogTextField(
                  controller: _nameController,
                  label: l10n.settingsModelProfileName,
                  requiredField: true,
                ),
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: OperitFormStyles.dropdownButtonFormField<String>(
                    context,
                    isExpanded: true,
                    initialValue: _selectedProviderTypeId,
                    style: OperitFormStyles.dropdownTextStyle(context),
                    decoration: InputDecoration(
                      labelText: l10n.settingsModelProviderType,
                    ),
                    items: widget.catalogEntries
                        .map(
                          (entry) => DropdownMenuItem<String>(
                            value: entry.providerTypeId,
                            child: Text(_providerCatalogLabel(l10n, entry)),
                          ),
                        )
                        .toList(growable: false),
                    onChanged: editing ? null : _onProviderTypeChanged,
                    validator: (value) {
                      if (value == null || value.isEmpty) {
                        return l10n.settingsModelProviderType;
                      }
                      return null;
                    },
                  ),
                ),
                _DialogTextField(
                  controller: _endpointController,
                  label: l10n.settingsModelApiEndpoint,
                  requiredField: true,
                  keyboardType: TextInputType.url,
                  inputFormatters: <TextInputFormatter>[
                    FilteringTextInputFormatter.deny(RegExp(r'\s')),
                  ],
                  suffixIcon: endpointOptions.isEmpty
                      ? null
                      : IconButton(
                          tooltip: l10n.settingsModelApiEndpoint,
                          icon: const Icon(Icons.arrow_drop_down_rounded),
                          onPressed: _showEndpointOptionsDialog,
                        ),
                ),
                _DialogTextField(
                  controller: _apiKeyController,
                  label: l10n.settingsModelApiKey,
                  obscureText: true,
                ),
                Theme(
                  data: Theme.of(context).copyWith(
                    dividerColor: Colors.transparent,
                    dividerTheme: const DividerThemeData(
                      color: Colors.transparent,
                    ),
                  ),
                  child: ExpansionTile(
                    tilePadding: EdgeInsets.zero,
                    childrenPadding: const EdgeInsets.only(top: 8),
                    shape: const Border(),
                    collapsedShape: const Border(),
                    title: Text(
                      l10n.settingsAdvanced,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    initiallyExpanded: false,
                    children: <Widget>[
                      _DialogTextField(
                        controller: _customHeadersController,
                        label: l10n.settingsModelCustomHeaders,
                        maxLines: 4,
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
                      if (editing) ...<Widget>[
                        const SizedBox(height: 8),
                        _ThinkingRulesEditor(
                          rules: _thinkingRules,
                          error: _thinkingConfigError,
                          onChanged: _setThinkingRules,
                        ),
                      ],
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        if (editing)
          TextButton.icon(
            onPressed: () =>
                Navigator.of(context).pop(const _ProviderEditDeleteResult()),
            icon: const Icon(Icons.delete_outline),
            label: Text(l10n.delete),
          ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
    );
  }
}

class _ThinkingRulesEditor extends StatelessWidget {
  /// Creates the provider-scoped thinking rules editor.
  const _ThinkingRulesEditor({
    required this.rules,
    required this.error,
    required this.onChanged,
  });

  final List<_ThinkingRuleEditor> rules;
  final String? error;
  final ValueChanged<List<_ThinkingRuleEditor>> onChanged;

  /// Opens a focused editor for one thinking rule.
  Future<void> _showRuleEditor(BuildContext context, int? index) async {
    var draft = index == null ? _newThinkingRule() : rules[index];
    final saved = await showDialog<_ThinkingRuleEditor>(
      context: context,
      builder: (dialogContext) {
        return StatefulBuilder(
          builder: (context, setDialogState) {
            final title = index == null
                ? '新建思考配置'
                : _thinkingRulePreviewTitle(draft);
            return AlertDialog(
              title: Text(title),
              content: SizedBox(
                width: 620,
                child: SingleChildScrollView(
                  child: _ThinkingRuleForm(
                    rule: draft,
                    onChanged: (rule) => setDialogState(() {
                      draft = rule;
                    }),
                  ),
                ),
              ),
              actions: <Widget>[
                if (index != null)
                  TextButton.icon(
                    style: TextButton.styleFrom(
                      foregroundColor: Theme.of(context).colorScheme.error,
                    ),
                    onPressed: () {
                      Navigator.of(dialogContext).pop();
                      onChanged(_removeAt<_ThinkingRuleEditor>(rules, index));
                    },
                    icon: const Icon(Icons.delete_outline),
                    label: const Text('删除'),
                  ),
                TextButton(
                  onPressed: () => Navigator.of(dialogContext).pop(),
                  child: const Text('取消'),
                ),
                FilledButton(
                  onPressed: () => Navigator.of(dialogContext).pop(draft),
                  child: const Text('保存'),
                ),
              ],
            );
          },
        );
      },
    );
    if (saved == null) {
      return;
    }
    if (index == null) {
      onChanged(<_ThinkingRuleEditor>[...rules, saved]);
      return;
    }
    onChanged(_replaceAt<_ThinkingRuleEditor>(rules, index, saved));
  }

  /// Builds the thinking rules list and rule management controls.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final controlSummary = _thinkingControlSummary(rules);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Icon(Icons.psychology_outlined, color: colorScheme.primary),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '思考配置',
                    style: textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  Text(
                    '${rules.length} 条规则 · $controlSummary',
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            TextButton.icon(
              onPressed: () => _showRuleEditor(context, null),
              icon: const Icon(Icons.add, size: 18),
              label: const Text('添加规则'),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Text(
          '点击规则编辑；顺序靠前的规则先匹配。这里的配置只属于当前供应商。',
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
        if (error != null) ...<Widget>[
          const SizedBox(height: 8),
          Text(
            error!,
            style: textTheme.bodySmall?.copyWith(color: colorScheme.error),
          ),
        ],
        const SizedBox(height: 8),
        if (rules.isEmpty)
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.4),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Text(
              '当前供应商没有思考规则，点击右上角添加规则。',
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          )
        else
          for (var index = 0; index < rules.length; index++) ...<Widget>[
            _ThinkingRuleCard(
              index: index,
              totalCount: rules.length,
              rule: rules[index],
              onTap: () => _showRuleEditor(context, index),
              onMoveUp: index == 0
                  ? null
                  : () => onChanged(
                      _moveAt<_ThinkingRuleEditor>(rules, index, index - 1),
                    ),
              onMoveDown: index == rules.length - 1
                  ? null
                  : () => onChanged(
                      _moveAt<_ThinkingRuleEditor>(rules, index, index + 1),
                    ),
            ),
            const SizedBox(height: 8),
          ],
      ],
    );
  }
}

class _ThinkingRuleCard extends StatelessWidget {
  /// Creates one compact thinking rule preview card.
  const _ThinkingRuleCard({
    required this.index,
    required this.totalCount,
    required this.rule,
    required this.onTap,
    required this.onMoveUp,
    required this.onMoveDown,
  });

  final int index;
  final int totalCount;
  final _ThinkingRuleEditor rule;
  final VoidCallback onTap;
  final VoidCallback? onMoveUp;
  final VoidCallback? onMoveDown;

  /// Builds the card preview and rule ordering controls.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final optionCount = rule.control == 'levels' ? rule.options.length : 0;
    final optionText = rule.control == 'levels' ? ' · $optionCount 档' : '';
    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.35),
      borderRadius: BorderRadius.circular(10),
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          child: Row(
            children: <Widget>[
              CircleAvatar(
                radius: 15,
                backgroundColor: colorScheme.primaryContainer,
                child: Text(
                  '${index + 1}',
                  style: textTheme.labelMedium?.copyWith(
                    color: colorScheme.onPrimaryContainer,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      _thinkingRulePreviewTitle(rule),
                      style: textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      '${_thinkingControlLabel(rule.control)}$optionText · ${_thinkingRulePathSummary(rule)}',
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
              IconButton(
                tooltip: index == 0 ? '已经是第一条' : '上移',
                onPressed: onMoveUp,
                icon: const Icon(Icons.keyboard_arrow_up),
              ),
              IconButton(
                tooltip: index >= totalCount - 1 ? '已经是最后一条' : '下移',
                onPressed: onMoveDown,
                icon: const Icon(Icons.keyboard_arrow_down),
              ),
              Icon(Icons.chevron_right, color: colorScheme.onSurfaceVariant),
            ],
          ),
        ),
      ),
    );
  }
}

class _ThinkingRuleForm extends StatelessWidget {
  /// Creates the form fields for one thinking rule.
  const _ThinkingRuleForm({required this.rule, required this.onChanged});

  final _ThinkingRuleEditor rule;
  final ValueChanged<_ThinkingRuleEditor> onChanged;

  /// Builds the editable fields for one thinking rule.
  @override
  Widget build(BuildContext context) {
    final actionSummary =
        '开启 ${rule.enableActions.length} 条 · 关闭 ${rule.disableActions.length} 条';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Expanded(
              child: _ThinkingChoiceField(
                label: '控件类型',
                value: rule.control,
                choices: const <MapEntry<String, String>>[
                  MapEntry<String, String>('levels', '多档位滑块'),
                  MapEntry<String, String>('toggle_only', '仅开关'),
                  MapEntry<String, String>('unsupported', '不支持思考'),
                ],
                onChanged: (value) => onChanged(
                  rule.copyWith(
                    control: value,
                    options: value == 'levels'
                        ? rule.options
                        : const <_ThinkingOptionEditor>[],
                  ),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: _ThinkingSwitchRow(
                title: '始终开启思考',
                subtitle: '命中后写入开启动作',
                value: rule.requiredValue,
                onChanged: (value) =>
                    onChanged(rule.copyWith(requiredValue: value)),
              ),
            ),
          ],
        ),
        const SizedBox(height: 10),
        _ThinkingMatchEditor(rule: rule, onChanged: onChanged),
        const SizedBox(height: 10),
        _ThinkingInlineTextField(
          label: '默认请求路径',
          value: rule.defaultPath,
          hint: 'reasoning_effort 或 thinking.type',
          onChanged: (value) => onChanged(rule.copyWith(defaultPath: value)),
        ),
        const SizedBox(height: 2),
        _ThinkingCollapsibleEditor(
          title: '开启 / 关闭时写入',
          subtitle: actionSummary,
          initiallyExpanded: false,
          child: Column(
            children: <Widget>[
              _ThinkingActionListEditor(
                title: '开启时写入',
                actions: rule.enableActions,
                onChanged: (actions) =>
                    onChanged(rule.copyWith(enableActions: actions)),
              ),
              const SizedBox(height: 8),
              _ThinkingActionListEditor(
                title: '关闭时写入',
                actions: rule.disableActions,
                onChanged: (actions) =>
                    onChanged(rule.copyWith(disableActions: actions)),
              ),
            ],
          ),
        ),
        if (rule.control == 'levels') ...<Widget>[
          const SizedBox(height: 8),
          _ThinkingCollapsibleEditor(
            title: '滑块档位',
            subtitle: '${rule.options.length} 个档位，决定滑块长度',
            initiallyExpanded: false,
            child: _ThinkingOptionListEditor(
              options: rule.options,
              defaultPath: rule.defaultPath,
              onChanged: (options) =>
                  onChanged(rule.copyWith(options: options)),
            ),
          ),
        ],
      ],
    );
  }
}

class _ThinkingChoiceField extends StatelessWidget {
  /// Creates a compact dropdown-like choice field.
  const _ThinkingChoiceField({
    required this.label,
    required this.value,
    required this.choices,
    required this.onChanged,
  });

  final String label;
  final String value;
  final List<MapEntry<String, String>> choices;
  final ValueChanged<String> onChanged;

  /// Builds a popup menu backed by readable choice labels.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final display = choices
        .where((choice) => choice.key == value)
        .map((choice) => choice.value)
        .first;
    return PopupMenuButton<String>(
      initialValue: value,
      onSelected: onChanged,
      itemBuilder: (context) => <PopupMenuEntry<String>>[
        for (final choice in choices)
          PopupMenuItem<String>(value: choice.key, child: Text(choice.value)),
      ],
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.45),
          borderRadius: BorderRadius.circular(10),
        ),
        child: Row(
          children: <Widget>[
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    label,
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    display,
                    style: textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            Icon(
              Icons.arrow_drop_down_rounded,
              color: colorScheme.onSurfaceVariant,
            ),
          ],
        ),
      ),
    );
  }
}

class _ThinkingSwitchRow extends StatelessWidget {
  /// Creates a compact switch row for thinking rule flags.
  const _ThinkingSwitchRow({
    required this.title,
    required this.subtitle,
    required this.value,
    required this.onChanged,
  });

  final String title;
  final String subtitle;
  final bool value;
  final ValueChanged<bool> onChanged;

  /// Builds the switch row surface.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  title,
                  style: textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  subtitle,
                  style: textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          Switch(value: value, onChanged: onChanged),
        ],
      ),
    );
  }
}

class _ThinkingCollapsibleEditor extends StatelessWidget {
  /// Creates a collapsible group for less frequently edited fields.
  const _ThinkingCollapsibleEditor({
    required this.title,
    required this.subtitle,
    required this.initiallyExpanded,
    required this.child,
  });

  final String title;
  final String subtitle;
  final bool initiallyExpanded;
  final Widget child;

  /// Builds the collapsible editor section.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.30),
      borderRadius: BorderRadius.circular(10),
      child: ExpansionTile(
        initiallyExpanded: initiallyExpanded,
        tilePadding: const EdgeInsets.symmetric(horizontal: 12),
        childrenPadding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        collapsedShape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
        ),
        title: Text(
          title,
          style: textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
        ),
        subtitle: Text(
          subtitle,
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
        children: <Widget>[child],
      ),
    );
  }
}

class _ThinkingMatchEditor extends StatelessWidget {
  /// Creates the human-readable matcher editor for one thinking rule.
  const _ThinkingMatchEditor({required this.rule, required this.onChanged});

  final _ThinkingRuleEditor rule;
  final ValueChanged<_ThinkingRuleEditor> onChanged;

  /// Builds grouped model and endpoint matching controls.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: colorScheme.surface.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.5),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Text(
            '匹配条件',
            style: textTheme.labelLarge?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 2),
          Text(
            '这些条件只属于当前供应商；多个值用逗号分隔。三项全空代表全部模型。',
            style: textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 8),
          Row(
            children: <Widget>[
              Expanded(
                child: _ThinkingInlineTextField(
                  label: '模型前缀',
                  value: _joinThinkingValues(rule.modelPrefixes),
                  hint: 'gemini-2.5, claude-3',
                  onChanged: (value) => onChanged(
                    rule.copyWith(modelPrefixes: _splitThinkingValues(value)),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: _ThinkingInlineTextField(
                  label: '端点后缀',
                  value: _joinThinkingValues(rule.endpointSuffixes),
                  hint: '/responses',
                  onChanged: (value) => onChanged(
                    rule.copyWith(
                      endpointSuffixes: _splitThinkingValues(value),
                    ),
                  ),
                ),
              ),
            ],
          ),
          _ThinkingInlineTextField(
            label: '模型正则',
            value: _joinThinkingValues(rule.modelRegexes),
            hint: r'(?i)(?:^|/)gpt-[5-9]',
            onChanged: (value) => onChanged(
              rule.copyWith(modelRegexes: _splitThinkingValues(value)),
            ),
          ),
        ],
      ),
    );
  }
}

class _ThinkingActionListEditor extends StatelessWidget {
  /// Creates an editor for request write actions.
  const _ThinkingActionListEditor({
    required this.title,
    required this.actions,
    required this.onChanged,
  });

  final String title;
  final List<_ThinkingActionEditor> actions;
  final ValueChanged<List<_ThinkingActionEditor>> onChanged;

  /// Builds editable action rows.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: colorScheme.surface.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.5),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  title,
                  style: textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              TextButton.icon(
                onPressed: () => onChanged(<_ThinkingActionEditor>[
                  ...actions,
                  const _ThinkingActionEditor(path: '', value: 'true'),
                ]),
                icon: const Icon(Icons.add, size: 16),
                label: const Text('添加'),
              ),
            ],
          ),
          if (actions.isEmpty)
            Text(
              '未配置',
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            )
          else
            for (var index = 0; index < actions.length; index++) ...<Widget>[
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Expanded(
                    child: _ThinkingInlineTextField(
                      label: '请求路径',
                      value: actions[index].path,
                      hint: 'reasoning.effort',
                      onChanged: (value) => onChanged(
                        _replaceAt<_ThinkingActionEditor>(
                          actions,
                          index,
                          actions[index].copyWith(path: value),
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: _ThinkingInlineTextField(
                      label: '写入值（JSON）',
                      value: actions[index].value,
                      hint: '"high", true, 1024',
                      onChanged: (value) => onChanged(
                        _replaceAt<_ThinkingActionEditor>(
                          actions,
                          index,
                          actions[index].copyWith(value: value),
                        ),
                      ),
                    ),
                  ),
                  IconButton(
                    tooltip: '删除',
                    onPressed: () => onChanged(
                      _removeAt<_ThinkingActionEditor>(actions, index),
                    ),
                    icon: Icon(Icons.delete_outline, color: colorScheme.error),
                  ),
                ],
              ),
            ],
        ],
      ),
    );
  }
}

class _ThinkingOptionListEditor extends StatelessWidget {
  /// Creates an editor for level-based thinking options.
  const _ThinkingOptionListEditor({
    required this.options,
    required this.defaultPath,
    required this.onChanged,
  });

  final List<_ThinkingOptionEditor> options;
  final String defaultPath;
  final ValueChanged<List<_ThinkingOptionEditor>> onChanged;

  /// Builds editable option rows.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: colorScheme.surface.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.5),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  '档位列表',
                  style: textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              TextButton.icon(
                onPressed: () => onChanged(<_ThinkingOptionEditor>[
                  ...options,
                  _newThinkingOption(options.length, defaultPath),
                ]),
                icon: const Icon(Icons.add, size: 16),
                label: const Text('添加档位'),
              ),
            ],
          ),
          if (options.isEmpty)
            Text(
              '未配置',
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            )
          else
            for (var index = 0; index < options.length; index++) ...<Widget>[
              _ThinkingOptionCard(
                index: index,
                option: options[index],
                defaultPath: defaultPath,
                onChanged: (option) => onChanged(
                  _replaceAt<_ThinkingOptionEditor>(options, index, option),
                ),
                onDelete: () =>
                    onChanged(_removeAt<_ThinkingOptionEditor>(options, index)),
                onMoveUp: index == 0
                    ? null
                    : () => onChanged(
                        _moveAt<_ThinkingOptionEditor>(
                          options,
                          index,
                          index - 1,
                        ),
                      ),
                onMoveDown: index == options.length - 1
                    ? null
                    : () => onChanged(
                        _moveAt<_ThinkingOptionEditor>(
                          options,
                          index,
                          index + 1,
                        ),
                      ),
              ),
              const SizedBox(height: 6),
            ],
        ],
      ),
    );
  }
}

class _ThinkingOptionCard extends StatelessWidget {
  /// Creates one collapsible thinking option card.
  const _ThinkingOptionCard({
    required this.index,
    required this.option,
    required this.defaultPath,
    required this.onChanged,
    required this.onDelete,
    required this.onMoveUp,
    required this.onMoveDown,
  });

  final int index;
  final _ThinkingOptionEditor option;
  final String defaultPath;
  final ValueChanged<_ThinkingOptionEditor> onChanged;
  final VoidCallback onDelete;
  final VoidCallback? onMoveUp;
  final VoidCallback? onMoveDown;

  /// Builds one editable option card.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
      borderRadius: BorderRadius.circular(8),
      child: ExpansionTile(
        tilePadding: const EdgeInsets.only(left: 10, right: 4),
        childrenPadding: const EdgeInsets.fromLTRB(10, 0, 10, 10),
        title: Text('档位 ${index + 1}: ${option.label}'),
        subtitle: Text(_thinkingOptionPathSummary(option, defaultPath)),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            IconButton(
              tooltip: '上移',
              onPressed: onMoveUp,
              icon: const Icon(Icons.keyboard_arrow_up),
            ),
            IconButton(
              tooltip: '下移',
              onPressed: onMoveDown,
              icon: const Icon(Icons.keyboard_arrow_down),
            ),
            IconButton(
              tooltip: '删除',
              onPressed: onDelete,
              icon: Icon(Icons.delete_outline, color: colorScheme.error),
            ),
          ],
        ),
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: _ThinkingInlineTextField(
                  label: 'id',
                  value: option.id,
                  hint: 'high',
                  onChanged: (value) => onChanged(option.copyWith(id: value)),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: _ThinkingInlineTextField(
                  label: '显示名称',
                  value: option.label,
                  hint: '高',
                  onChanged: (value) =>
                      onChanged(option.copyWith(label: value)),
                ),
              ),
            ],
          ),
          Row(
            children: <Widget>[
              Expanded(
                child: _ThinkingInlineTextField(
                  label: '写入路径',
                  value: option.path,
                  hint: defaultPath,
                  onChanged: (value) => onChanged(option.copyWith(path: value)),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: _ThinkingInlineTextField(
                  label: '写入值（JSON）',
                  value: option.value,
                  hint: '"high", true, 1024',
                  onChanged: (value) =>
                      onChanged(option.copyWith(value: value)),
                ),
              ),
            ],
          ),
          _ThinkingActionListEditor(
            title: '档位附加动作',
            actions: option.actions,
            onChanged: (actions) =>
                onChanged(option.copyWith(actions: actions)),
          ),
        ],
      ),
    );
  }
}

class _ThinkingInlineTextField extends StatelessWidget {
  /// Creates a compact text field for thinking configuration forms.
  const _ThinkingInlineTextField({
    required this.label,
    required this.value,
    required this.hint,
    required this.onChanged,
  });

  final String label;
  final String value;
  final String hint;
  final ValueChanged<String> onChanged;

  /// Builds a text form field using the current editor value.
  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            label,
            style: textTheme.labelSmall?.copyWith(
              color: colorScheme.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 4),
          TextFormField(
            initialValue: value,
            maxLines: 1,
            decoration: InputDecoration(
              hintText: hint,
              isDense: true,
              contentPadding: const EdgeInsets.symmetric(
                horizontal: 12,
                vertical: 12,
              ),
            ),
            onChanged: onChanged,
          ),
        ],
      ),
    );
  }
}

class _ThinkingRuleEditor {
  /// Creates one editable provider-scoped thinking rule.
  const _ThinkingRuleEditor({
    required this.modelPrefixes,
    required this.modelRegexes,
    required this.endpointSuffixes,
    required this.control,
    required this.requiredValue,
    required this.defaultPath,
    required this.enableActions,
    required this.disableActions,
    required this.options,
  });

  final List<String> modelPrefixes;
  final List<String> modelRegexes;
  final List<String> endpointSuffixes;
  final String control;
  final bool requiredValue;
  final String defaultPath;
  final List<_ThinkingActionEditor> enableActions;
  final List<_ThinkingActionEditor> disableActions;
  final List<_ThinkingOptionEditor> options;

  /// Returns a modified copy of this editor rule.
  _ThinkingRuleEditor copyWith({
    List<String>? modelPrefixes,
    List<String>? modelRegexes,
    List<String>? endpointSuffixes,
    String? control,
    bool? requiredValue,
    String? defaultPath,
    List<_ThinkingActionEditor>? enableActions,
    List<_ThinkingActionEditor>? disableActions,
    List<_ThinkingOptionEditor>? options,
  }) {
    return _ThinkingRuleEditor(
      modelPrefixes: modelPrefixes ?? this.modelPrefixes,
      modelRegexes: modelRegexes ?? this.modelRegexes,
      endpointSuffixes: endpointSuffixes ?? this.endpointSuffixes,
      control: control ?? this.control,
      requiredValue: requiredValue ?? this.requiredValue,
      defaultPath: defaultPath ?? this.defaultPath,
      enableActions: enableActions ?? this.enableActions,
      disableActions: disableActions ?? this.disableActions,
      options: options ?? this.options,
    );
  }
}

class _ThinkingActionEditor {
  /// Creates one editable request write action.
  const _ThinkingActionEditor({required this.path, required this.value});

  final String path;
  final String value;

  /// Returns a modified copy of this action.
  _ThinkingActionEditor copyWith({String? path, String? value}) {
    return _ThinkingActionEditor(
      path: path ?? this.path,
      value: value ?? this.value,
    );
  }
}

class _ThinkingOptionEditor {
  /// Creates one editable thinking level option.
  const _ThinkingOptionEditor({
    required this.id,
    required this.label,
    required this.path,
    required this.value,
    required this.actions,
  });

  final String id;
  final String label;
  final String path;
  final String value;
  final List<_ThinkingActionEditor> actions;

  /// Returns a modified copy of this option.
  _ThinkingOptionEditor copyWith({
    String? id,
    String? label,
    String? path,
    String? value,
    List<_ThinkingActionEditor>? actions,
  }) {
    return _ThinkingOptionEditor(
      id: id ?? this.id,
      label: label ?? this.label,
      path: path ?? this.path,
      value: value ?? this.value,
      actions: actions ?? this.actions,
    );
  }
}

/// Creates a new provider-owned thinking rule.
_ThinkingRuleEditor _newThinkingRule() {
  return _ThinkingRuleEditor(
    modelPrefixes: const <String>[],
    modelRegexes: const <String>[],
    endpointSuffixes: const <String>[],
    control: 'levels',
    requiredValue: false,
    defaultPath: 'reasoning_effort',
    enableActions: const <_ThinkingActionEditor>[],
    disableActions: const <_ThinkingActionEditor>[],
    options: <_ThinkingOptionEditor>[
      const _ThinkingOptionEditor(
        id: 'low',
        label: 'low',
        path: 'reasoning_effort',
        value: '"low"',
        actions: <_ThinkingActionEditor>[],
      ),
      const _ThinkingOptionEditor(
        id: 'high',
        label: 'high',
        path: 'reasoning_effort',
        value: '"high"',
        actions: <_ThinkingActionEditor>[],
      ),
      const _ThinkingOptionEditor(
        id: 'max',
        label: 'max',
        path: 'reasoning_effort',
        value: '"max"',
        actions: <_ThinkingActionEditor>[],
      ),
    ],
  );
}

/// Creates a new editable thinking level option.
_ThinkingOptionEditor _newThinkingOption(int index, String defaultPath) {
  final label = 'option-${index + 1}';
  return _ThinkingOptionEditor(
    id: label,
    label: label,
    path: defaultPath,
    value: 'null',
    actions: const <_ThinkingActionEditor>[],
  );
}

/// Parses current Rust thinking rules into UI editor models.
List<_ThinkingRuleEditor> _parseThinkingRuleEditors(String raw) {
  final decoded = jsonDecode(raw);
  if (decoded is! List<Object?>) {
    throw const FormatException('thinkingConfigurations 必须是 JSON 数组');
  }
  return <_ThinkingRuleEditor>[
    for (var index = 0; index < decoded.length; index++)
      _thinkingRuleFromJson(_jsonObject(decoded[index], '规则 ${index + 1}')),
  ];
}

/// Serializes UI editor models into current Rust thinking rule JSON.
String _serializeThinkingRuleEditors(List<_ThinkingRuleEditor> rules) {
  final encoded = <Map<String, Object?>>[
    for (var index = 0; index < rules.length; index++)
      _thinkingRuleToJson(rules[index], index),
  ];
  return const JsonEncoder.withIndent('  ').convert(encoded);
}

/// Converts one JSON object into an editable thinking rule.
_ThinkingRuleEditor _thinkingRuleFromJson(Map<String, Object?> json) {
  final options = _thinkingOptionsFromJson(json['options']);
  return _ThinkingRuleEditor(
    modelPrefixes: _stringListFromJson(json['model_prefix'], 'model_prefix'),
    modelRegexes: _stringListFromJson(json['model_regex'], 'model_regex'),
    endpointSuffixes: _stringListFromJson(
      json['endpoint_suffix'],
      'endpoint_suffix',
    ),
    control: _thinkingControlFromJson(json['control']),
    requiredValue: _boolFromJson(json['required'], 'required'),
    defaultPath: _defaultThinkingPath(options),
    enableActions: _thinkingActionsFromJson(json['enable'], 'enable'),
    disableActions: _thinkingActionsFromJson(json['disable'], 'disable'),
    options: options,
  );
}

/// Converts one editable thinking rule into a Rust JSON object.
Map<String, Object?> _thinkingRuleToJson(_ThinkingRuleEditor rule, int index) {
  final label = '规则 ${index + 1}';
  _validateThinkingRule(rule, label);
  return <String, Object?>{
    'model_prefix': rule.modelPrefixes,
    'model_regex': rule.modelRegexes,
    'endpoint_suffix': rule.endpointSuffixes,
    'control': rule.control,
    'required': rule.requiredValue,
    'enable': _thinkingActionsToJson(rule.enableActions, '$label.enable'),
    'disable': _thinkingActionsToJson(rule.disableActions, '$label.disable'),
    'options': rule.control == 'levels'
        ? _thinkingOptionsToJson(rule.options, rule.defaultPath, label)
        : <Map<String, Object?>>[],
  };
}

/// Validates one rule before serialization.
void _validateThinkingRule(_ThinkingRuleEditor rule, String label) {
  _thinkingControlLabel(rule.control);
  if (rule.control == 'levels' && rule.options.isEmpty) {
    throw FormatException('$label 缺少档位列表');
  }
}

/// Converts action JSON into editor rows.
List<_ThinkingActionEditor> _thinkingActionsFromJson(
  Object? value,
  String field,
) {
  if (value is! List<Object?>) {
    throw FormatException('$field 必须是 JSON 数组');
  }
  return <_ThinkingActionEditor>[
    for (var index = 0; index < value.length; index++)
      _thinkingActionFromJson(_jsonObject(value[index], '$field[$index]')),
  ];
}

/// Converts one action JSON object into an editor row.
_ThinkingActionEditor _thinkingActionFromJson(Map<String, Object?> json) {
  if (!json.containsKey('value')) {
    throw const FormatException('动作缺少 value');
  }
  return _ThinkingActionEditor(
    path: _stringFromJson(json['path'], 'path'),
    value: jsonEncode(json['value']),
  );
}

/// Converts editor action rows into JSON objects.
List<Map<String, Object?>> _thinkingActionsToJson(
  List<_ThinkingActionEditor> actions,
  String label,
) {
  return <Map<String, Object?>>[
    for (var index = 0; index < actions.length; index++)
      _thinkingActionToJson(actions[index], '$label[$index]'),
  ];
}

/// Converts one editor action into a JSON object.
Map<String, Object?> _thinkingActionToJson(
  _ThinkingActionEditor action,
  String label,
) {
  final path = action.path.trim();
  if (path.isEmpty) {
    throw FormatException('$label 缺少 path');
  }
  return <String, Object?>{
    'path': path,
    'value': _decodeThinkingValue(action.value, '$label.value'),
  };
}

/// Converts option JSON into editor rows.
List<_ThinkingOptionEditor> _thinkingOptionsFromJson(Object? value) {
  if (value is! List<Object?>) {
    throw const FormatException('options 必须是 JSON 数组');
  }
  return <_ThinkingOptionEditor>[
    for (var index = 0; index < value.length; index++)
      _thinkingOptionFromJson(_jsonObject(value[index], 'options[$index]')),
  ];
}

/// Converts one option JSON object into an editor row.
_ThinkingOptionEditor _thinkingOptionFromJson(Map<String, Object?> json) {
  if (!json.containsKey('value')) {
    throw const FormatException('档位缺少 value');
  }
  final value = json['value'];
  final id = _optionalStringFromJson(json['id']).trim();
  final visibleId = id.isEmpty ? _thinkingValueId(value) : id;
  final label = _optionalStringFromJson(json['label']).trim();
  return _ThinkingOptionEditor(
    id: visibleId,
    label: label.isEmpty ? visibleId : label,
    path: _optionalStringFromJson(json['path']),
    value: jsonEncode(value),
    actions: _optionalThinkingActionsFromJson(json['actions'], 'actions'),
  );
}

/// Converts editor options into JSON objects.
List<Map<String, Object?>> _thinkingOptionsToJson(
  List<_ThinkingOptionEditor> options,
  String defaultPath,
  String label,
) {
  final ids = <String>{};
  return <Map<String, Object?>>[
    for (var index = 0; index < options.length; index++)
      _thinkingOptionToJson(
        options[index],
        defaultPath,
        '$label.options[$index]',
        ids,
      ),
  ];
}

/// Converts one editor option into a JSON object.
Map<String, Object?> _thinkingOptionToJson(
  _ThinkingOptionEditor option,
  String defaultPath,
  String label,
  Set<String> usedIds,
) {
  final id = option.id.trim();
  final optionLabel = option.label.trim();
  final path = option.path.trim().isEmpty
      ? defaultPath.trim()
      : option.path.trim();
  if (id.isEmpty) {
    throw FormatException('$label 缺少 id');
  }
  if (optionLabel.isEmpty) {
    throw FormatException('$label 缺少 label');
  }
  if (path.isEmpty) {
    throw FormatException('$label 缺少 path');
  }
  if (!usedIds.add(id)) {
    throw FormatException('$label id 重复：$id');
  }
  return <String, Object?>{
    'id': id,
    'label': optionLabel,
    'path': path,
    'value': _decodeThinkingValue(option.value, '$label.value'),
    'actions': _thinkingActionsToJson(option.actions, '$label.actions'),
  };
}

/// Converts an optional action list into editor rows.
List<_ThinkingActionEditor> _optionalThinkingActionsFromJson(
  Object? value,
  String field,
) {
  if (value == null) {
    return const <_ThinkingActionEditor>[];
  }
  return _thinkingActionsFromJson(value, field);
}

/// Decodes one text field as a JSON value.
Object? _decodeThinkingValue(String value, String label) {
  final text = value.trim();
  if (text.isEmpty) {
    throw FormatException('$label 不能为空');
  }
  try {
    return jsonDecode(text);
  } on FormatException catch (error) {
    throw FormatException('$label JSON 无效：${error.message}');
  }
}

/// Reads a JSON object at the specified location.
Map<String, Object?> _jsonObject(Object? value, String label) {
  if (value is! Map) {
    throw FormatException('$label 必须是 JSON 对象');
  }
  return value.cast<String, Object?>();
}

/// Reads a required string list from JSON.
List<String> _stringListFromJson(Object? value, String field) {
  if (value is! List<Object?>) {
    throw FormatException('$field 必须是 JSON 数组');
  }
  return <String>[
    for (var index = 0; index < value.length; index++)
      _stringFromJson(value[index], '$field[$index]'),
  ];
}

/// Reads a required string from JSON.
String _stringFromJson(Object? value, String field) {
  if (value is! String) {
    throw FormatException('$field 必须是字符串');
  }
  return value;
}

/// Reads an optional string from JSON.
String _optionalStringFromJson(Object? value) {
  if (value == null) {
    return '';
  }
  if (value is! String) {
    throw const FormatException('字段必须是字符串');
  }
  return value;
}

/// Reads a required boolean from JSON.
bool _boolFromJson(Object? value, String field) {
  if (value is! bool) {
    throw FormatException('$field 必须是布尔值');
  }
  return value;
}

/// Reads and validates a thinking control value.
String _thinkingControlFromJson(Object? value) {
  final control = _stringFromJson(value, 'control');
  _thinkingControlLabel(control);
  return control;
}

/// Returns the display label for one thinking control value.
String _thinkingControlLabel(String value) {
  return switch (value) {
    'levels' => '多档位滑块',
    'toggle_only' => '仅开关',
    'unsupported' => '不支持思考',
    _ => throw FormatException('control 不支持：$value'),
  };
}

/// Joins a list of rule values for text editing.
String _joinThinkingValues(List<String> values) {
  return values.join(', ');
}

/// Splits a comma-separated editor value list.
List<String> _splitThinkingValues(String value) {
  return value
      .split(',')
      .map((item) => item.trim())
      .where((item) => item.isNotEmpty)
      .toList(growable: false);
}

/// Returns a stable option id from a JSON value.
String _thinkingValueId(Object? value) {
  if (value is String) {
    return value;
  }
  return jsonEncode(value);
}

/// Returns a common option path for the rule-level path editor.
String _defaultThinkingPath(List<_ThinkingOptionEditor> options) {
  final paths = <String>{
    for (final option in options)
      if (option.path.trim().isNotEmpty) option.path.trim(),
  };
  if (paths.length == 1) {
    return paths.single;
  }
  return '';
}

/// Builds a compact title for a thinking rule.
String _thinkingRulePreviewTitle(_ThinkingRuleEditor rule) {
  final parts = <String>[
    for (final value in rule.modelPrefixes) '前缀 $value',
    for (final value in rule.modelRegexes) '正则 $value',
    for (final value in rule.endpointSuffixes) '端点 $value',
  ];
  if (parts.isEmpty) {
    return '所有模型';
  }
  final visible = parts.take(3).join('、');
  return parts.length > 3 ? '$visible、…' : visible;
}

/// Builds a compact path summary for a thinking rule.
String _thinkingRulePathSummary(_ThinkingRuleEditor rule) {
  if (rule.defaultPath.trim().isNotEmpty) {
    return rule.defaultPath.trim();
  }
  final paths = <String>[
    for (final action in rule.enableActions)
      if (action.path.trim().isNotEmpty) action.path.trim(),
    for (final action in rule.disableActions)
      if (action.path.trim().isNotEmpty) action.path.trim(),
    for (final option in rule.options)
      if (option.path.trim().isNotEmpty) option.path.trim(),
  ];
  if (paths.isEmpty) {
    return '未设置请求路径';
  }
  return paths.first;
}

/// Builds a compact path summary for a thinking level option.
String _thinkingOptionPathSummary(
  _ThinkingOptionEditor option,
  String defaultPath,
) {
  final path = option.path.trim().isEmpty
      ? defaultPath.trim()
      : option.path.trim();
  if (path.isEmpty) {
    return '未设置写入路径';
  }
  return path;
}

/// Builds the summary text for all configured control types.
String _thinkingControlSummary(List<_ThinkingRuleEditor> rules) {
  final labels = <String>[];
  for (final rule in rules) {
    final label = _thinkingControlLabel(rule.control);
    if (!labels.any((item) => item == label)) {
      labels.add(label);
    }
  }
  if (labels.isEmpty) {
    return '未配置';
  }
  return labels.join(' / ');
}

/// Replaces one item in a list.
List<T> _replaceAt<T>(List<T> items, int index, T value) {
  return <T>[
    for (var i = 0; i < items.length; i++) i == index ? value : items[i],
  ];
}

/// Removes one item from a list.
List<T> _removeAt<T>(List<T> items, int index) {
  return <T>[
    for (var i = 0; i < items.length; i++)
      if (i != index) items[i],
  ];
}

/// Moves one item inside a list.
List<T> _moveAt<T>(List<T> items, int from, int to) {
  final copy = <T>[...items];
  final item = copy.removeAt(from);
  copy.insert(to, item);
  return copy;
}

sealed class _AvailableModelSelection {
  const _AvailableModelSelection();
}

class _AvailableModelsPicked extends _AvailableModelSelection {
  const _AvailableModelsPicked(this.models);

  final List<core_proxy.AvailableProviderModel> models;
}

class _AvailableModelCustom extends _AvailableModelSelection {
  const _AvailableModelCustom();
}

class _AvailableModelDialog extends StatefulWidget {
  const _AvailableModelDialog({required this.models});

  final List<core_proxy.AvailableProviderModel> models;

  static Future<_AvailableModelSelection?> show({
    required BuildContext context,
    required List<core_proxy.AvailableProviderModel> models,
  }) {
    return showDialog<_AvailableModelSelection>(
      context: context,
      builder: (context) => _AvailableModelDialog(models: models),
    );
  }

  @override
  State<_AvailableModelDialog> createState() => _AvailableModelDialogState();
}

class _AvailableModelDialogState extends State<_AvailableModelDialog> {
  final _searchController = TextEditingController();
  final Set<String> _selectedModelIds = <String>{};

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  /// Filters catalog models according to the current search query.
  List<core_proxy.AvailableProviderModel> _filteredModels(
    AppLocalizations l10n,
  ) {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) {
      return widget.models;
    }
    return widget.models
        .where((model) {
          final text =
              '${model.modelId} ${_availableModelSubtitle(l10n, model)}'
                  .toLowerCase();
          return text.contains(query);
        })
        .toList(growable: false);
  }

  /// Toggles the selected state of one catalog model.
  void _toggleModel(core_proxy.AvailableProviderModel model) {
    setState(() {
      if (_selectedModelIds.contains(model.modelId)) {
        _selectedModelIds.remove(model.modelId);
      } else {
        _selectedModelIds.add(model.modelId);
      }
    });
  }

  /// Returns the selected catalog models in their displayed catalog order.
  List<core_proxy.AvailableProviderModel> _selectedModels() {
    return widget.models
        .where((model) => _selectedModelIds.contains(model.modelId))
        .toList(growable: false);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final filteredModels = _filteredModels(l10n);
    return AlertDialog(
      title: Text(l10n.settingsModelAddModel),
      content: SizedBox(
        width: 520,
        height: 500,
        child: Column(
          children: <Widget>[
            TextField(
              controller: _searchController,
              decoration: InputDecoration(
                prefixIcon: const Icon(Icons.search),
                labelText: l10n.search,
              ),
              onChanged: (_) => setState(() {}),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: ListView(
                children: <Widget>[
                  for (final model in filteredModels)
                    Material(
                      type: MaterialType.transparency,
                      child: ListTile(
                        dense: true,
                        visualDensity: VisualDensity.compact,
                        contentPadding: EdgeInsets.zero,
                        leading: Checkbox(
                          value: _selectedModelIds.contains(model.modelId),
                          onChanged: (_) => _toggleModel(model),
                        ),
                        title: Text(model.modelId),
                        subtitle: Text(_availableModelSubtitle(l10n, model)),
                        onTap: () => _toggleModel(model),
                      ),
                    ),
                  Material(
                    type: MaterialType.transparency,
                    child: ListTile(
                      dense: true,
                      visualDensity: VisualDensity.compact,
                      contentPadding: EdgeInsets.zero,
                      leading: const Icon(Icons.add),
                      title: Text(l10n.settingsModelCustomModel),
                      subtitle: Text(l10n.settingsModelModelId),
                      onTap: () => Navigator.of(
                        context,
                      ).pop(const _AvailableModelCustom()),
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
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _selectedModelIds.isEmpty
              ? null
              : () => Navigator.of(
                  context,
                ).pop(_AvailableModelsPicked(_selectedModels())),
          child: Text(
            '${l10n.settingsModelAddModelShort} (${_selectedModelIds.length})',
          ),
        ),
      ],
    );
  }
}

class _TextInputDialog extends StatefulWidget {
  const _TextInputDialog({
    required this.title,
    required this.label,
    this.validator,
  });

  final String title;
  final String label;
  final String? Function(String? value)? validator;

  /// Shows a validated dialog for entering a single text value.
  static Future<String?> show({
    required BuildContext context,
    required String title,
    required String label,
    String? Function(String? value)? validator,
  }) {
    return showDialog<String>(
      context: context,
      builder: (context) =>
          _TextInputDialog(title: title, label: label, validator: validator),
    );
  }

  @override
  State<_TextInputDialog> createState() => _TextInputDialogState();
}

class _TextInputDialogState extends State<_TextInputDialog> {
  final _formKey = GlobalKey<FormState>();
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Validates and returns the entered value while preserving invalid input.
  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    Navigator.of(context).pop(_controller.text.trim());
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(widget.title),
      content: Form(
        key: _formKey,
        autovalidateMode: AutovalidateMode.onUserInteraction,
        child: _DialogTextField(
          controller: _controller,
          label: widget.label,
          requiredField: true,
          validator: widget.validator,
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

class _ProviderModelManager extends StatelessWidget {
  const _ProviderModelManager({
    required this.providers,
    required this.summaries,
    required this.chatBinding,
    required this.expandedProviderIds,
    required this.onToggleProviderExpanded,
    required this.onEditProvider,
    required this.onAddModel,
    required this.onSelectModel,
    required this.testingModelKey,
    required this.onEditModelSettings,
  });

  final List<core_proxy.ProviderProfile> providers;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding chatBinding;
  final Set<String> expandedProviderIds;
  final ValueChanged<String> onToggleProviderExpanded;
  final void Function(core_proxy.ProviderProfile provider) onEditProvider;
  final void Function(core_proxy.ProviderProfile provider) onAddModel;
  final void Function(String providerId, String modelId) onSelectModel;
  final String? testingModelKey;
  final void Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onEditModelSettings;

  @override
  Widget build(BuildContext context) {
    if (providers.isEmpty) {
      return const SizedBox.shrink();
    }
    return Column(
      children: <Widget>[
        for (final provider in providers)
          _ProviderModelGroup(
            provider: provider,
            summaries: summaries,
            chatBinding: chatBinding,
            expanded: expandedProviderIds.contains(provider.id),
            onToggleExpanded: () => onToggleProviderExpanded(provider.id),
            onEditProvider: () => onEditProvider(provider),
            onAddModel: () => onAddModel(provider),
            onSelectModel: onSelectModel,
            testingModelKey: testingModelKey,
            onEditModelSettings: onEditModelSettings,
          ),
      ],
    );
  }
}

class _ProviderModelGroup extends StatelessWidget {
  const _ProviderModelGroup({
    required this.provider,
    required this.summaries,
    required this.chatBinding,
    required this.expanded,
    required this.onToggleExpanded,
    required this.onEditProvider,
    required this.onAddModel,
    required this.onSelectModel,
    required this.testingModelKey,
    required this.onEditModelSettings,
  });

  final core_proxy.ProviderProfile provider;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding chatBinding;
  final bool expanded;
  final VoidCallback onToggleExpanded;
  final VoidCallback onEditProvider;
  final VoidCallback onAddModel;
  final void Function(String providerId, String modelId) onSelectModel;
  final String? testingModelKey;
  final void Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onEditModelSettings;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final chatProvider = provider.id == chatBinding.providerId;
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
                        color: chatProvider
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
                            provider.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.titleSmall
                                ?.copyWith(fontWeight: FontWeight.w700),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            '${provider.providerTypeId} · ${provider.models.length}',
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
                        onPressed: onAddModel,
                        style: SettingsControlStyles.sectionTextButton(),
                        icon: const Icon(Icons.playlist_add, size: 18),
                        label: Text(l10n.settingsModelAddModelShort),
                      ),
                      SettingsEntityIconButton(
                        tooltip: l10n.edit,
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
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: <Widget>[
                          if (provider.models.isEmpty)
                            Padding(
                              padding: const EdgeInsets.symmetric(vertical: 12),
                              child: Text(
                                l10n.settingsModelAddModel,
                                style: TextStyle(
                                  color: colorScheme.onSurfaceVariant,
                                ),
                              ),
                            )
                          else
                            _ProviderModelList(
                              provider: provider,
                              summaries: summaries,
                              chatBinding: chatBinding,
                              onSelectModel: onSelectModel,
                              testingModelKey: testingModelKey,
                              onEditModelSettings: onEditModelSettings,
                            ),
                        ],
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

class _ProviderModelList extends StatelessWidget {
  const _ProviderModelList({
    required this.provider,
    required this.summaries,
    required this.chatBinding,
    required this.onSelectModel,
    required this.testingModelKey,
    required this.onEditModelSettings,
  });

  final core_proxy.ProviderProfile provider;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding chatBinding;
  final void Function(String providerId, String modelId) onSelectModel;
  final String? testingModelKey;
  final void Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onEditModelSettings;

  @override
  Widget build(BuildContext context) {
    final tiles = <Widget>[
      for (final model in provider.models)
        if (_summaryForModelOrNull(summaries, provider.id, model.id)
            case final summary?)
          _ProviderModelTile(
            provider: provider,
            model: model,
            summary: summary,
            selected:
                provider.id == chatBinding.providerId &&
                model.id == chatBinding.modelId,
            onSelect: onSelectModel,
            testing: testingModelKey == _modelTestKey(provider.id, model.id),
            onEditSettings: () => onEditModelSettings(provider, model),
          ),
    ];
    if (tiles.isEmpty) {
      return const SizedBox.shrink();
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (var index = 0; index < tiles.length; index++) ...<Widget>[
          if (index > 0) const SizedBox(height: 4),
          tiles[index],
        ],
      ],
    );
  }
}

class _ProviderModelTile extends StatelessWidget {
  const _ProviderModelTile({
    required this.provider,
    required this.model,
    required this.summary,
    required this.selected,
    required this.onSelect,
    required this.testing,
    required this.onEditSettings,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final core_proxy.ProviderModelSummary summary;
  final bool selected;
  final void Function(String providerId, String modelId) onSelect;
  final bool testing;
  final VoidCallback onEditSettings;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: selected
          ? colorScheme.primaryContainer.withValues(alpha: 0.24)
          : Colors.transparent,
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: onEditSettings,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          child: Row(
            children: <Widget>[
              SizedBox(
                width: 16,
                height: 16,
                child: Icon(
                  Icons.memory_outlined,
                  size: 16,
                  color: selected
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
                      model.id,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 3),
                    _ModelCapabilityIcons(capabilities: summary.capabilities),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              if (selected)
                SettingsActivePill(label: l10n.settingsModelCurrentActive)
              else
                SettingsSetActiveButton(
                  label: l10n.settingsModelSetCurrentActive,
                  onPressed: () => onSelect(provider.id, model.id),
                ),
              const SizedBox(width: 6),
              Tooltip(
                message: '模型设置',
                child: Icon(
                  Icons.tune_outlined,
                  size: 18,
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              if (testing) ...<Widget>[
                const SizedBox(width: 8),
                const SizedBox.square(
                  dimension: 24,
                  child: Center(child: M3LoadingIndicator(size: 24)),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _ModelCapabilityIcons extends StatelessWidget {
  const _ModelCapabilityIcons({required this.capabilities});

  final core_proxy.ModelCapabilities capabilities;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final color = Theme.of(context).colorScheme.onSurfaceVariant;
    final icons = <Widget>[
      if (capabilities.toolCall)
        _CapabilityIcon(
          icon: Icons.build_outlined,
          tooltip: l10n.settingsModelToolCall,
          color: color,
        ),
      if (capabilities.directImage)
        _CapabilityIcon(
          icon: Icons.image_outlined,
          tooltip: l10n.settingsModelDirectImage,
          color: color,
        ),
      if (capabilities.directAudio)
        _CapabilityIcon(
          icon: Icons.graphic_eq,
          tooltip: l10n.settingsModelDirectAudio,
          color: color,
        ),
      if (capabilities.directVideo)
        _CapabilityIcon(
          icon: Icons.videocam_outlined,
          tooltip: l10n.settingsModelDirectVideo,
          color: color,
        ),
    ];
    if (icons.isEmpty) {
      return const SizedBox.shrink();
    }
    return Padding(
      padding: const EdgeInsets.only(top: 2),
      child: Wrap(spacing: 6, runSpacing: 3, children: icons),
    );
  }
}

class _CapabilityIcon extends StatelessWidget {
  const _CapabilityIcon({
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
      child: Icon(icon, size: 14, color: color),
    );
  }
}

class _FunctionMappingTile extends StatelessWidget {
  const _FunctionMappingTile({
    required this.functionType,
    required this.binding,
    required this.summary,
    required this.onTap,
  });

  final core_proxy.FunctionType functionType;
  final core_proxy.FunctionModelBinding binding;
  final core_proxy.ProviderModelSummary? summary;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final warning = summary == null
        ? l10n.settingsModelFunctionMappingsMissing(
            binding.providerId,
            binding.modelId,
          )
        : _functionMappingWarning(l10n, functionType, summary!);
    return Material(
      type: MaterialType.transparency,
      child: ListTile(
        dense: true,
        visualDensity: VisualDensity.compact,
        contentPadding: EdgeInsets.zero,
        title: Text(_functionTypeTitle(l10n, functionType)),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(_functionTypeDescription(l10n, functionType)),
            const SizedBox(height: 4),
            Text(
              summary == null
                  ? '${binding.providerId} · ${binding.modelId}'
                  : l10n.settingsModelFunctionMappingsCurrent(
                      summary!.providerName,
                      binding.modelId,
                    ),
              style: TextStyle(
                color: summary == null
                    ? colorScheme.error
                    : colorScheme.primary,
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
      ),
    );
  }
}

class _FunctionModelSelection {
  const _FunctionModelSelection({
    required this.providerId,
    required this.modelId,
  });

  final String providerId;
  final String modelId;
}

class _FunctionModelSelectorDialog extends StatefulWidget {
  const _FunctionModelSelectorDialog({
    required this.functionType,
    required this.summaries,
    required this.currentBinding,
  });

  final core_proxy.FunctionType functionType;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding currentBinding;

  static Future<_FunctionModelSelection?> show({
    required BuildContext context,
    required core_proxy.FunctionType functionType,
    required List<core_proxy.ProviderModelSummary> summaries,
    required core_proxy.FunctionModelBinding currentBinding,
  }) {
    return showDialog<_FunctionModelSelection>(
      context: context,
      builder: (context) => _FunctionModelSelectorDialog(
        functionType: functionType,
        summaries: summaries,
        currentBinding: currentBinding,
      ),
    );
  }

  @override
  State<_FunctionModelSelectorDialog> createState() =>
      _FunctionModelSelectorDialogState();
}

class _FunctionModelSelectorDialogState
    extends State<_FunctionModelSelectorDialog> {
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _selectModel(core_proxy.ProviderModelSummary summary) {
    Navigator.of(context).pop(
      _FunctionModelSelection(
        providerId: summary.providerId,
        modelId: summary.modelId,
      ),
    );
  }

  List<core_proxy.ProviderModelSummary> _filteredModels() {
    final candidates = widget.summaries
        .where(
          (summary) => _functionModelSupported(widget.functionType, summary),
        )
        .toList(growable: false);
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) {
      return candidates;
    }
    return candidates
        .where(
          (summary) =>
              '${summary.modelId} ${summary.providerName} ${summary.providerTypeId}'
                  .toLowerCase()
                  .contains(query),
        )
        .toList(growable: false);
  }

  Widget _modelList(AppLocalizations l10n) {
    final filteredModels = _filteredModels();
    return Column(
      children: <Widget>[
        TextField(
          controller: _searchController,
          decoration: InputDecoration(
            prefixIcon: const Icon(Icons.search),
            labelText: l10n.search,
          ),
          onChanged: (_) => setState(() {}),
        ),
        const SizedBox(height: 8),
        Expanded(
          child: filteredModels.isEmpty
              ? Center(child: Text(l10n.noData))
              : ListView.builder(
                  itemCount: filteredModels.length,
                  itemBuilder: (context, index) {
                    final summary = filteredModels[index];
                    return _FunctionModelOptionTile(
                      summary: summary,
                      selected:
                          summary.providerId ==
                              widget.currentBinding.providerId &&
                          summary.modelId == widget.currentBinding.modelId,
                      onTap: () => _selectModel(summary),
                    );
                  },
                ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    if (widget.summaries.isEmpty) {
      return AlertDialog(
        title: Text(
          l10n.settingsModelFunctionMappingsSelect(
            _functionTypeTitle(l10n, widget.functionType),
          ),
        ),
        content: SizedBox(width: 420, child: Text(l10n.noData)),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(l10n.cancel),
          ),
        ],
      );
    }
    return AlertDialog(
      title: Text(
        l10n.settingsModelFunctionMappingsSelect(
          _functionTypeTitle(l10n, widget.functionType),
        ),
      ),
      content: SizedBox(width: 560, height: 480, child: _modelList(l10n)),
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
    required this.selected,
    required this.onTap,
  });

  final core_proxy.ProviderModelSummary summary;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      type: MaterialType.transparency,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 9),
          child: Row(
            children: <Widget>[
              SizedBox(
                width: 24,
                child: Icon(
                  selected ? Icons.check_circle : Icons.circle_outlined,
                  size: 20,
                  color: selected
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
                      summary.modelId,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    Text(
                      summary.providerName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(color: colorScheme.onSurfaceVariant),
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

class _ModelSettingsChange {
  const _ModelSettingsChange({
    required this.capabilities,
    required this.builtinTools,
    required this.context,
    required this.summary,
  });

  final core_proxy.ModelCapabilities capabilities;
  final List<core_proxy.ModelBuiltinTool> builtinTools;
  final core_proxy.ModelContextSpec context;
  final core_proxy.ModelSummarySettings summary;
}

sealed class _ModelSettingsEditorResult {
  const _ModelSettingsEditorResult();
}

class _ModelSettingsSaved extends _ModelSettingsEditorResult {
  const _ModelSettingsSaved(this.change);

  final _ModelSettingsChange change;
}

class _ModelSettingsDeleteRequested extends _ModelSettingsEditorResult {
  const _ModelSettingsDeleteRequested();
}

class _ModelSettingsEditorDialog extends StatefulWidget {
  const _ModelSettingsEditorDialog({
    required this.providerName,
    required this.modelId,
    required this.initialCapabilities,
    required this.initialBuiltinTools,
    required this.initialContext,
    required this.initialSummary,
    required this.onTest,
  });

  final String providerName;
  final String modelId;
  final core_proxy.ModelCapabilities initialCapabilities;
  final List<core_proxy.ModelBuiltinTool> initialBuiltinTools;
  final core_proxy.ModelContextSpec initialContext;
  final core_proxy.ModelSummarySettings initialSummary;
  final Future<core_proxy.ModelConnectionTestReport?> Function() onTest;

  static Future<_ModelSettingsEditorResult?> show({
    required BuildContext context,
    required String providerName,
    required String modelId,
    required core_proxy.ModelCapabilities initialCapabilities,
    required List<core_proxy.ModelBuiltinTool> initialBuiltinTools,
    required core_proxy.ModelContextSpec initialContext,
    required core_proxy.ModelSummarySettings initialSummary,
    required Future<core_proxy.ModelConnectionTestReport?> Function() onTest,
  }) {
    return showDialog<_ModelSettingsEditorResult>(
      context: context,
      builder: (context) => _ModelSettingsEditorDialog(
        providerName: providerName,
        modelId: modelId,
        initialCapabilities: initialCapabilities,
        initialBuiltinTools: initialBuiltinTools,
        initialContext: initialContext,
        initialSummary: initialSummary,
        onTest: onTest,
      ),
    );
  }

  @override
  State<_ModelSettingsEditorDialog> createState() =>
      _ModelSettingsEditorDialogState();
}

class _ModelSettingsEditorDialogState
    extends State<_ModelSettingsEditorDialog> {
  late bool _toolCall;
  late bool _directImage;
  late bool _directAudio;
  late bool _directVideo;
  late List<core_proxy.ModelBuiltinTool> _builtinTools;
  late bool _enableMaxContextMode;
  late bool _enableSummary;
  late bool _enableSummaryByMessageCount;
  late final TextEditingController _maxContextLengthController;
  late final TextEditingController _summaryThresholdController;
  late final TextEditingController _summaryMessageCountController;
  String? _maxContextLengthError;
  bool _testingConnection = false;

  @override
  void initState() {
    super.initState();
    final caps = widget.initialCapabilities;
    _toolCall = caps.toolCall;
    _directImage = caps.directImage;
    _directAudio = caps.directAudio;
    _directVideo = caps.directVideo;
    _builtinTools = widget.initialBuiltinTools;
    _enableMaxContextMode = widget.initialContext.enableMaxContextMode;
    _enableSummary = widget.initialSummary.enableSummary;
    _enableSummaryByMessageCount =
        widget.initialSummary.enableSummaryByMessageCount;
    _maxContextLengthController = TextEditingController(
      text: widget.initialContext.maxContextLength.toStringAsFixed(0),
    );
    _summaryThresholdController = TextEditingController(
      text: widget.initialSummary.summaryTokenThreshold.toString(),
    );
    _summaryMessageCountController = TextEditingController(
      text: widget.initialSummary.summaryMessageCountThreshold.toString(),
    );
  }

  @override
  void dispose() {
    _maxContextLengthController.dispose();
    _summaryThresholdController.dispose();
    _summaryMessageCountController.dispose();
    super.dispose();
  }

  void _setBuiltinToolEnabled(int index, bool enabled) {
    final current = _builtinTools[index];
    final updated = core_proxy.ModelBuiltinTool(
      toolType: current.toolType,
      displayName: current.displayName,
      enabled: enabled,
      requestFormat: current.requestFormat,
      exclusivity: current.exclusivity,
      config: current.config,
    );
    setState(() {
      _builtinTools = <core_proxy.ModelBuiltinTool>[
        for (var i = 0; i < _builtinTools.length; i++)
          i == index ? updated : _builtinTools[i],
      ];
      if (enabled &&
          current.exclusivity ==
              core_proxy.BuiltinToolExclusivity.exclusiveWithExternalTools) {
        _toolCall = false;
      }
    });
  }

  Future<void> _runConnectionTest() async {
    setState(() {
      _testingConnection = true;
    });
    try {
      final report = await widget.onTest();
      if (mounted &&
          report != null &&
          _connectionTestSucceeded(
            report,
            core_proxy.ModelConnectionTestType.chat,
          )) {
        setState(() {
          _directImage = _connectionTestSucceeded(
            report,
            core_proxy.ModelConnectionTestType.image,
          );
          _directAudio = _connectionTestSucceeded(
            report,
            core_proxy.ModelConnectionTestType.audio,
          );
          _directVideo = _connectionTestSucceeded(
            report,
            core_proxy.ModelConnectionTestType.video,
          );
          _toolCall = _connectionTestSucceeded(
            report,
            core_proxy.ModelConnectionTestType.toolCall,
          );
        });
      }
    } finally {
      if (mounted) {
        setState(() {
          _testingConnection = false;
        });
      }
    }
  }

  void _save() {
    final maxContextLength = double.tryParse(
      _maxContextLengthController.text.trim(),
    );
    if (maxContextLength == null || maxContextLength <= 0) {
      setState(() {
        _maxContextLengthError = AppLocalizations.of(
          context,
        )!.settingsModelMaxContextLengthInvalid;
      });
      return;
    }
    Navigator.of(context).pop(
      _ModelSettingsSaved(
        _ModelSettingsChange(
          capabilities: core_proxy.ModelCapabilities(
            directImage: _directImage,
            directAudio: _directAudio,
            directVideo: _directVideo,
            toolCall: _toolCall,
          ),
          builtinTools: _builtinTools,
          context: core_proxy.ModelContextSpec(
            maxContextLength: maxContextLength,
            enableMaxContextMode: _enableMaxContextMode,
          ),
          summary: core_proxy.ModelSummarySettings(
            enableSummary: _enableSummary,
            summaryTokenThreshold:
                double.tryParse(_summaryThresholdController.text) ?? 0,
            enableSummaryByMessageCount: _enableSummaryByMessageCount,
            summaryMessageCountThreshold:
                int.tryParse(_summaryMessageCountController.text) ?? 0,
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final textStyle = Theme.of(context).textTheme.bodyMedium;
    return AlertDialog(
      title: Text(l10n.settingsModelEditModelSettings),
      contentPadding: EdgeInsets.zero,
      content: SizedBox(
        width: 520,
        child: SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(24, 12, 24, 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Text(
                '${widget.providerName} \u00b7 ${widget.modelId}',
                style: TextStyle(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 12),
              Align(
                alignment: Alignment.centerLeft,
                child: OutlinedButton.icon(
                  onPressed: _testingConnection ? null : _runConnectionTest,
                  icon: _testingConnection
                      ? const SizedBox.square(
                          dimension: 18,
                          child: Center(child: M3LoadingIndicator(size: 18)),
                        )
                      : const Icon(Icons.wifi_find_outlined, size: 18),
                  label: Text(l10n.settingsModelTestModel),
                ),
              ),
              const SizedBox(height: 12),
              Text(
                l10n.settingsModelCapabilities,
                style: const TextStyle(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 4),
              _ModelSettingsSwitch(
                title: l10n.settingsModelToolCall,
                subtitle: l10n.settingsModelToolCallDescription,
                value: _toolCall,
                onChanged: (v) => setState(() => _toolCall = v),
              ),
              _ModelSettingsSwitch(
                title: l10n.settingsModelDirectImage,
                subtitle: l10n.settingsModelDirectImageDescription,
                value: _directImage,
                onChanged: (v) => setState(() => _directImage = v),
              ),
              _ModelSettingsSwitch(
                title: l10n.settingsModelDirectAudio,
                subtitle: l10n.settingsModelDirectAudioDescription,
                value: _directAudio,
                onChanged: (v) => setState(() => _directAudio = v),
              ),
              _ModelSettingsSwitch(
                title: l10n.settingsModelDirectVideo,
                subtitle: l10n.settingsModelDirectVideoDescription,
                value: _directVideo,
                onChanged: (v) => setState(() => _directVideo = v),
              ),
              if (_builtinTools.isNotEmpty) ...<Widget>[
                const SizedBox(height: 12),
                Text(
                  l10n.settingsModelBuiltinTools,
                  style: const TextStyle(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 4),
                for (var index = 0; index < _builtinTools.length; index++)
                  _ModelSettingsSwitch(
                    title: _builtinTools[index].displayName,
                    subtitle: _builtinToolSubtitle(l10n, _builtinTools[index]),
                    value: _builtinTools[index].enabled,
                    onChanged: (value) => _setBuiltinToolEnabled(index, value),
                  ),
              ],
              const SizedBox(height: 12),
              Text(
                l10n.settingsModelContext,
                style: const TextStyle(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 10),
              TextField(
                controller: _maxContextLengthController,
                style: textStyle,
                decoration: InputDecoration(
                  labelText: l10n.settingsModelMaxContextLength,
                  errorText: _maxContextLengthError,
                ),
                keyboardType: TextInputType.number,
                onChanged: (_) {
                  setState(() => _maxContextLengthError = null);
                },
              ),
              const SizedBox(height: 8),
              _ModelSettingsSwitch(
                title: l10n.settingsModelMaxContextMode,
                subtitle:
                    '${l10n.settingsModelMaxContextLength}: ${_maxContextLengthController.text.trim()}k',
                value: _enableMaxContextMode,
                onChanged: (v) => setState(() => _enableMaxContextMode = v),
              ),
              const SizedBox(height: 12),
              Text(
                l10n.settingsModelSummary,
                style: const TextStyle(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 10),
              _ModelSettingsSwitch(
                title: l10n.enable,
                subtitle: '',
                value: _enableSummary,
                onChanged: (v) => setState(() => _enableSummary = v),
              ),
              if (_enableSummary) ...<Widget>[
                TextField(
                  controller: _summaryThresholdController,
                  style: textStyle,
                  decoration: InputDecoration(
                    labelText: l10n.settingsModelSummaryThreshold,
                  ),
                  keyboardType: TextInputType.number,
                ),
                const SizedBox(height: 8),
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                  visualDensity: VisualDensity.compact,
                  title: Text(l10n.settingsModelSummaryByMessageCount),
                  value: _enableSummaryByMessageCount,
                  onChanged: (v) =>
                      setState(() => _enableSummaryByMessageCount = v),
                ),
                if (_enableSummaryByMessageCount)
                  TextField(
                    controller: _summaryMessageCountController,
                    style: textStyle,
                    decoration: InputDecoration(
                      labelText: l10n.settingsModelSummaryMessageCount,
                    ),
                    keyboardType: TextInputType.number,
                  ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton.icon(
          style: TextButton.styleFrom(
            foregroundColor: Theme.of(context).colorScheme.error,
          ),
          onPressed: () =>
              Navigator.of(context).pop(const _ModelSettingsDeleteRequested()),
          icon: const Icon(Icons.delete_outline),
          label: Text(l10n.delete),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
    );
  }
}

class _ModelSettingsSwitch extends StatelessWidget {
  const _ModelSettingsSwitch({
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
    return SwitchListTile(
      contentPadding: EdgeInsets.zero,
      dense: true,
      visualDensity: VisualDensity.compact,
      title: Text(title),
      subtitle: subtitle.isNotEmpty ? Text(subtitle) : null,
      value: value,
      onChanged: onChanged,
    );
  }
}

class _ConnectionTestReportDialog extends StatelessWidget {
  const _ConnectionTestReportDialog({required this.report});

  final core_proxy.ModelConnectionTestReport report;

  static Future<void> show({
    required BuildContext context,
    required core_proxy.ModelConnectionTestReport report,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) => _ConnectionTestReportDialog(report: report),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return AlertDialog(
      title: Text(l10n.settingsModelConnectionTestSection),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              '${report.providerName} \u00b7 ${report.modelId}',
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            ),
            const SizedBox(height: 8),
            for (final item in report.items)
              _ConnectionTestItemTile(item: item),
          ],
        ),
      ),
      actions: <Widget>[
        FilledButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.ok),
        ),
      ],
    );
  }
}

class _ConnectionTestErrorDialog extends StatelessWidget {
  const _ConnectionTestErrorDialog({required this.message});

  final String message;

  static Future<void> show({
    required BuildContext context,
    required String message,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) => _ConnectionTestErrorDialog(message: message),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsModelConnectionTestSection),
      content: Text(message),
      actions: <Widget>[
        FilledButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.ok),
        ),
      ],
    );
  }
}

enum _AddProviderModelErrorAction { custom, dismiss }

class _AddProviderModelErrorDialog extends StatelessWidget {
  const _AddProviderModelErrorDialog({
    required this.errorDetails,
    required this.providerName,
    required this.showCustomAction,
  });

  final core_proxy.CoreProxyErrorDetails errorDetails;
  final String providerName;

  /// Shows the model import error dialog.
  static Future<_AddProviderModelErrorAction?> show({
    required BuildContext context,
    required core_proxy.CoreProxyErrorDetails errorDetails,
    required String providerName,
    bool showCustomAction = false,
  }) {
    return showDialog<_AddProviderModelErrorAction>(
      context: context,
      builder: (context) => _AddProviderModelErrorDialog(
        errorDetails: errorDetails,
        providerName: providerName,
        showCustomAction: showCustomAction,
      ),
    );
  }

  final bool showCustomAction;

  /// Adds the known provider display name to structured error details.
  core_proxy.CoreProxyErrorDetails _errorDetailsWithProviderName() {
    return core_proxy.CoreProxyErrorDetails(
      errorType: errorDetails.errorType,
      message: errorDetails.message,
      variant: errorDetails.variant,
      kind: errorDetails.kind,
      httpStatus: errorDetails.httpStatus,
      remoteMessage: errorDetails.remoteMessage,
      fields: <String, Object?>{
        ...errorDetails.fields,
        'providerName': providerName,
      },
    );
  }

  /// Builds the model import error dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsModelAddModel),
      content: SizedBox(
        width: 520,
        child: CommonNetworkErrorView(
          errorDetails: _errorDetailsWithProviderName(),
        ),
      ),
      actions: <Widget>[
        if (showCustomAction)
          TextButton(
            onPressed: () =>
                Navigator.of(context).pop(_AddProviderModelErrorAction.custom),
            child: Text(l10n.settingsModelCustomModel),
          ),
        FilledButton(
          onPressed: () =>
              Navigator.of(context).pop(_AddProviderModelErrorAction.dismiss),
          child: Text(l10n.ok),
        ),
      ],
    );
  }
}

class _DeleteModelBlockedDialog extends StatelessWidget {
  const _DeleteModelBlockedDialog({required this.functionTypes});

  final List<core_proxy.FunctionType> functionTypes;

  static Future<void> show({
    required BuildContext context,
    required List<core_proxy.FunctionType> functionTypes,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) =>
          _DeleteModelBlockedDialog(functionTypes: functionTypes),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final names = functionTypes
        .map((functionType) => _functionTypeTitle(l10n, functionType))
        .join(' · ');
    return AlertDialog(
      title: Text(l10n.delete),
      content: Text(l10n.settingsModelDeleteBlocked(names)),
      actions: <Widget>[
        FilledButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.ok),
        ),
      ],
    );
  }
}

class _DeleteProviderBlockedDialog extends StatelessWidget {
  const _DeleteProviderBlockedDialog({required this.functionTypes});

  final List<core_proxy.FunctionType> functionTypes;

  static Future<void> show({
    required BuildContext context,
    required List<core_proxy.FunctionType> functionTypes,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) =>
          _DeleteProviderBlockedDialog(functionTypes: functionTypes),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final names = functionTypes
        .map((functionType) => _functionTypeTitle(l10n, functionType))
        .join(' · ');
    return AlertDialog(
      title: Text(l10n.delete),
      content: Text(l10n.settingsModelDeleteProviderBlocked(names)),
      actions: <Widget>[
        FilledButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.ok),
        ),
      ],
    );
  }
}

class _DeleteProviderConfirmDialog extends StatelessWidget {
  const _DeleteProviderConfirmDialog({
    required this.providerName,
    required this.modelCount,
  });

  final String providerName;
  final int modelCount;

  static Future<bool?> show({
    required BuildContext context,
    required String providerName,
    required int modelCount,
  }) {
    return showDialog<bool>(
      context: context,
      builder: (context) => _DeleteProviderConfirmDialog(
        providerName: providerName,
        modelCount: modelCount,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.delete),
      content: Text(
        l10n.settingsModelDeleteProviderConfirm(providerName, modelCount),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.settingsModelDeleteProviderConfirmAction),
        ),
      ],
    );
  }
}

class _ConnectionTestItemTile extends StatelessWidget {
  const _ConnectionTestItemTile({required this.item});

  final core_proxy.CoreOperitProvidersChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem
  item;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final color = item.success ? Colors.green : colorScheme.error;
    return Material(
      type: MaterialType.transparency,
      child: ListTile(
        dense: true,
        visualDensity: VisualDensity.compact,
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
                    style: SettingsControlStyles.sectionTitleTextStyle(context),
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

class _DialogTextField extends StatelessWidget {
  const _DialogTextField({
    required this.controller,
    required this.label,
    this.requiredField = false,
    this.obscureText = false,
    this.numberOnly = false,
    this.maxLines = 1,
    this.keyboardType,
    this.inputFormatters,
    this.suffixIcon,
    this.validator,
  });

  final TextEditingController controller;
  final String label;
  final bool requiredField;
  final bool obscureText;
  final bool numberOnly;
  final int maxLines;
  final TextInputType? keyboardType;
  final List<TextInputFormatter>? inputFormatters;
  final Widget? suffixIcon;
  final String? Function(String? value)? validator;

  /// Builds a text field with the dialog's standard validation rules.
  @override
  Widget build(BuildContext context) {
    final textStyle = Theme.of(context).textTheme.bodyMedium;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: TextFormField(
        controller: controller,
        style: textStyle,
        obscureText: obscureText,
        maxLines: obscureText ? 1 : maxLines,
        keyboardType:
            keyboardType ??
            (numberOnly ? TextInputType.number : TextInputType.text),
        inputFormatters: inputFormatters,
        decoration: InputDecoration(labelText: label, suffixIcon: suffixIcon),
        validator: (value) {
          final text = value?.trim() ?? '';
          if (requiredField && text.isEmpty) {
            return label;
          }
          if (numberOnly && text.isEmpty) {
            return label;
          }
          return validator?.call(value);
        },
      ),
    );
  }
}

const List<core_proxy.FunctionType> _functionTypes = <core_proxy.FunctionType>[
  core_proxy.FunctionType.chat,
  core_proxy.FunctionType.summary,
  core_proxy.FunctionType.titleGeneration,
  core_proxy.FunctionType.memory,
  core_proxy.FunctionType.uiController,
  core_proxy.FunctionType.translation,
  core_proxy.FunctionType.grep,
  core_proxy.FunctionType.roleResponsePlanner,
  core_proxy.FunctionType.imageRecognition,
  core_proxy.FunctionType.audioRecognition,
  core_proxy.FunctionType.videoRecognition,
];

List<core_proxy.FunctionType> _boundFunctionTypesForModel(
  Map<core_proxy.FunctionType, core_proxy.FunctionModelBinding> bindings,
  String providerId,
  String modelId,
) {
  final result = <core_proxy.FunctionType>[];
  for (final entry in bindings.entries) {
    final binding = entry.value;
    if (binding.providerId == providerId && binding.modelId == modelId) {
      result.add(entry.key);
    }
  }
  return result;
}

List<core_proxy.FunctionType> _boundFunctionTypesForProvider(
  Map<core_proxy.FunctionType, core_proxy.FunctionModelBinding> bindings,
  String providerId,
) {
  final result = <core_proxy.FunctionType>[];
  for (final entry in bindings.entries) {
    final binding = entry.value;
    if (binding.providerId == providerId) {
      result.add(entry.key);
    }
  }
  return result;
}

String _functionTypeTitle(
  AppLocalizations l10n,
  core_proxy.FunctionType functionType,
) {
  return switch (functionType) {
    core_proxy.FunctionType.chat => l10n.settingsModelFunctionChat,
    core_proxy.FunctionType.summary => l10n.settingsModelFunctionSummary,
    core_proxy.FunctionType.titleGeneration =>
      l10n.settingsModelFunctionTitleGeneration,
    core_proxy.FunctionType.memory => l10n.settingsModelFunctionMemory,
    core_proxy.FunctionType.uiController =>
      l10n.settingsModelFunctionUiController,
    core_proxy.FunctionType.translation =>
      l10n.settingsModelFunctionTranslation,
    core_proxy.FunctionType.grep => l10n.settingsModelFunctionGrep,
    core_proxy.FunctionType.roleResponsePlanner =>
      l10n.settingsModelFunctionRoleResponsePlanner,
    core_proxy.FunctionType.imageRecognition =>
      l10n.settingsModelFunctionImageRecognition,
    core_proxy.FunctionType.audioRecognition =>
      l10n.settingsModelFunctionAudioRecognition,
    core_proxy.FunctionType.videoRecognition =>
      l10n.settingsModelFunctionVideoRecognition,
  };
}

String _providerCatalogLabel(
  AppLocalizations l10n,
  core_proxy.ProviderCatalogEntry entry,
) {
  return l10n.settingsModelProviderTypeOption(
    _providerTypeLocalName(l10n, entry.providerTypeId),
    entry.displayName,
  );
}

String _providerTypeLocalName(AppLocalizations l10n, String providerTypeId) {
  return switch (providerTypeId) {
    'OPENAI' => l10n.settingsModelProviderTypeOpenai,
    'XAI' => l10n.settingsModelProviderTypeXai,
    'OPENAI_RESPONSES' => l10n.settingsModelProviderTypeOpenaiResponses,
    'OPENAI_CODEX' => l10n.settingsModelProviderTypeOpenaiCodex,
    'OPENAI_RESPONSES_GENERIC' =>
      l10n.settingsModelProviderTypeOpenaiResponsesGeneric,
    'OPENAI_GENERIC' => l10n.settingsModelProviderTypeOpenaiGeneric,
    'ANTHROPIC' => l10n.settingsModelProviderTypeAnthropic,
    'ANTHROPIC_GENERIC' => l10n.settingsModelProviderTypeAnthropicGeneric,
    'GOOGLE' => l10n.settingsModelProviderTypeGoogle,
    'GEMINI_GENERIC' => l10n.settingsModelProviderTypeGeminiGeneric,
    'BAIDU' => l10n.settingsModelProviderTypeBaidu,
    'ALIYUN' => l10n.settingsModelProviderTypeAliyun,
    'XUNFEI' => l10n.settingsModelProviderTypeXunfei,
    'ZHIPU' => l10n.settingsModelProviderTypeZhipu,
    'BAICHUAN' => l10n.settingsModelProviderTypeBaichuan,
    'MOONSHOT' => l10n.settingsModelProviderTypeMoonshot,
    'MIMO' => l10n.settingsModelProviderTypeMimo,
    'DEEPSEEK' => l10n.settingsModelProviderTypeDeepseek,
    'MISTRAL' => l10n.settingsModelProviderTypeMistral,
    'SILICONFLOW' => l10n.settingsModelProviderTypeSiliconflow,
    'IFLOW' => l10n.settingsModelProviderTypeIflow,
    'OPENROUTER' => l10n.settingsModelProviderTypeOpenrouter,
    'OPENCODE' => l10n.settingsModelProviderTypeOpencode,
    'FOUR_ROUTER' => l10n.settingsModelProviderTypeFourRouter,
    'NOUS_PORTAL' => l10n.settingsModelProviderTypeNousPortal,
    'INFINIAI' => l10n.settingsModelProviderTypeInfiniai,
    'ALIPAY_BAILING' => l10n.settingsModelProviderTypeAlipayBailing,
    'DOUBAO' => l10n.settingsModelProviderTypeDoubao,
    'NVIDIA' => l10n.settingsModelProviderTypeNvidia,
    'LMSTUDIO' => l10n.settingsModelProviderTypeLmstudio,
    'OLLAMA' => l10n.settingsModelProviderTypeOllama,
    'OPENAI_LOCAL' => l10n.settingsModelProviderTypeOpenaiLocal,
    'LOCAL_MODEL' => l10n.settingsModelProviderTypeLocalModel,
    'MNN' => l10n.settingsModelProviderTypeMnn,
    'LLAMA_CPP' => l10n.settingsModelProviderTypeLlamaCpp,
    'PPINFRA' => l10n.settingsModelProviderTypePpinfra,
    'NOVITA' => l10n.settingsModelProviderTypeNovita,
    'MINIMAX' => l10n.settingsModelProviderTypeMinimax,
    'OTHER' => l10n.settingsModelProviderTypeOther,
    _ => providerTypeId,
  };
}

String _functionTypeDescription(
  AppLocalizations l10n,
  core_proxy.FunctionType functionType,
) {
  return switch (functionType) {
    core_proxy.FunctionType.chat => l10n.settingsModelFunctionChatDescription,
    core_proxy.FunctionType.summary =>
      l10n.settingsModelFunctionSummaryDescription,
    core_proxy.FunctionType.titleGeneration =>
      l10n.settingsModelFunctionTitleGenerationDescription,
    core_proxy.FunctionType.memory =>
      l10n.settingsModelFunctionMemoryDescription,
    core_proxy.FunctionType.uiController =>
      l10n.settingsModelFunctionUiControllerDescription,
    core_proxy.FunctionType.translation =>
      l10n.settingsModelFunctionTranslationDescription,
    core_proxy.FunctionType.grep => l10n.settingsModelFunctionGrepDescription,
    core_proxy.FunctionType.roleResponsePlanner =>
      l10n.settingsModelFunctionRoleResponsePlannerDescription,
    core_proxy.FunctionType.imageRecognition =>
      l10n.settingsModelFunctionImageRecognitionDescription,
    core_proxy.FunctionType.audioRecognition =>
      l10n.settingsModelFunctionAudioRecognitionDescription,
    core_proxy.FunctionType.videoRecognition =>
      l10n.settingsModelFunctionVideoRecognitionDescription,
  };
}

String? _functionMappingWarning(
  AppLocalizations l10n,
  core_proxy.FunctionType functionType,
  core_proxy.ProviderModelSummary summary,
) {
  return switch (functionType) {
    core_proxy.FunctionType.imageRecognition
        when !summary.capabilities.directImage =>
      l10n.settingsModelFunctionImageUnsupported,
    core_proxy.FunctionType.audioRecognition
        when !summary.capabilities.directAudio =>
      l10n.settingsModelFunctionAudioUnsupported,
    core_proxy.FunctionType.videoRecognition
        when !summary.capabilities.directVideo =>
      l10n.settingsModelFunctionVideoUnsupported,
    _ => null,
  };
}

bool _functionModelSupported(
  core_proxy.FunctionType functionType,
  core_proxy.ProviderModelSummary summary,
) {
  return switch (functionType) {
    core_proxy.FunctionType.imageRecognition =>
      summary.capabilities.directImage,
    core_proxy.FunctionType.audioRecognition =>
      summary.capabilities.directAudio,
    core_proxy.FunctionType.videoRecognition =>
      summary.capabilities.directVideo,
    _ => true,
  };
}

String _availableModelSubtitle(
  AppLocalizations l10n,
  core_proxy.AvailableProviderModel model,
) {
  final labels = <String>[];
  final capabilities = model.capabilities;
  if (capabilities != null) {
    if (capabilities.directImage) {
      labels.add(l10n.settingsModelDirectImage);
    }
    if (capabilities.directAudio) {
      labels.add(l10n.settingsModelDirectAudio);
    }
    if (capabilities.directVideo) {
      labels.add(l10n.settingsModelDirectVideo);
    }
    if (capabilities.toolCall) {
      labels.add(l10n.settingsModelToolCall);
    }
  }
  if (model.builtinTools.isNotEmpty) {
    labels.add(l10n.settingsModelBuiltinTools);
  }
  final context = model.context;
  if (context != null) {
    labels.add('${context.maxContextLength.toStringAsFixed(0)}k');
  }
  return labels.isEmpty ? '-' : labels.join(' · ');
}

String _builtinToolSubtitle(
  AppLocalizations l10n,
  core_proxy.ModelBuiltinTool tool,
) {
  final labels = <String>[];
  labels.add('${tool.requestFormat}');
  if (tool.exclusivity ==
      core_proxy.BuiltinToolExclusivity.exclusiveWithExternalTools) {
    labels.add(l10n.settingsModelBuiltinToolExclusive);
  }
  return labels.join(' · ');
}

core_proxy.ProviderModelSummary? _summaryForModelOrNull(
  List<core_proxy.ProviderModelSummary> summaries,
  String providerId,
  String modelId,
) {
  for (final summary in summaries) {
    if (summary.providerId == providerId && summary.modelId == modelId) {
      return summary;
    }
  }
  return null;
}

String _connectionTestTypeLabel(
  AppLocalizations l10n,
  core_proxy.ModelConnectionTestType type,
) {
  return switch (type) {
    core_proxy.ModelConnectionTestType.chat => l10n.settingsModelTestItemChat,
    core_proxy.ModelConnectionTestType.toolCall =>
      l10n.settingsModelTestItemToolCall,
    core_proxy.ModelConnectionTestType.image => l10n.settingsModelTestItemImage,
    core_proxy.ModelConnectionTestType.audio => l10n.settingsModelTestItemAudio,
    core_proxy.ModelConnectionTestType.video => l10n.settingsModelTestItemVideo,
  };
}

String _modelTestKey(String providerId, String modelId) {
  return '$providerId:$modelId';
}

bool _connectionTestSucceeded(
  core_proxy.ModelConnectionTestReport report,
  core_proxy.ModelConnectionTestType type,
) {
  return report.items.any((item) => item.type == type && item.success);
}
