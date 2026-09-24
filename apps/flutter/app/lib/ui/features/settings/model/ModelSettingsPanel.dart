// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';

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
import 'CodexLoginDialog.dart';
import 'ModelConnectionTestCapabilities.dart';
import 'ProviderLogo.dart';
import '../tts/TtsSettingsPanel.dart';

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
    return data;
  }

  /// Returns the load operation started when the panel was initialized.
  Future<ModelSettingsData> get loadFuture => _future!;

  void _reload() {
    if (!mounted) {
      return;
    }
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
      currentBinding: _resolveFunctionBinding(data, functionType),
      chatBinding: data.chatBinding,
      followsChat: data.functionBindings[functionType]!.followsChat,
    );
    if (selected == null) {
      return;
    }
    if (selected is _FunctionModelFollowChat) {
      await widget.clients.preferencesFunctionalConfigManager
          .setFunctionFollowChat(functionType: functionType);
      _reload();
      return;
    }
    final modelSelection = selected as _FunctionModelSelection;
    if (functionType == core_proxy.FunctionType.chat &&
        modelSelection.modelId.toLowerCase().contains('autoglm')) {
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
      providerId: modelSelection.providerId,
      modelId: modelSelection.modelId,
    );
    _reload();
  }

  /// Assigns every non-chat function to follow the current chat model binding.
  Future<void> _setAllFunctionsFollowChat() async {
    await widget.clients.preferencesFunctionalConfigManager
        .setAllFunctionsFollowChat();
    _reload();
  }

  /// Creates a provider after rejecting names already used by other profiles.
  Future<void> _createProvider() async {
    final data = await _future!;
    final providers = data.providers;
    final catalogEntries = await widget.clients.preferencesModelConfigManager
        .getProviderCatalogEntries();
    if (!mounted) {
      return;
    }
    final result = await _ProviderEditorDialog.show(
      context: context,
      catalogEntries: catalogEntries,
      occupiedProviderNames: _occupiedProviderNames(providers, null),
      clients: widget.clients,
    );
    if (result == null || result is! _ProviderEditSaveResult) {
      return;
    }
    try {
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
      _reload();
    } on CoreLinkError catch (error) {
      if (!mounted) {
        return;
      }
      await _showProviderConfigError(
        title: AppLocalizations.of(context)!.settingsModelCreateProvider,
        error: error,
      );
    }
  }

  /// Updates a provider after rejecting names already used by other profiles.
  Future<void> _editProvider(
    core_proxy.ProviderProfile provider,
    List<core_proxy.ProviderProfile> providers,
  ) async {
    final catalogEntries = await widget.clients.preferencesModelConfigManager
        .getProviderCatalogEntries();
    if (!mounted) {
      return;
    }
    final result = await _ProviderEditorDialog.show(
      context: context,
      catalogEntries: catalogEntries,
      occupiedProviderNames: _occupiedProviderNames(providers, provider.id),
      clients: widget.clients,
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
    try {
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
    } on CoreLinkError catch (error) {
      if (!mounted) {
        return;
      }
      await _showProviderConfigError(
        title: AppLocalizations.of(context)!.settingsModelEditProvider,
        error: error,
      );
    }
  }

  /// Collects trimmed provider names already taken by other profiles.
  Set<String> _occupiedProviderNames(
    List<core_proxy.ProviderProfile> providers,
    String? currentProviderId,
  ) {
    return <String>{
      for (final provider in providers)
        if (provider.id != currentProviderId) provider.name.trim(),
    };
  }

  /// Shows the runtime error returned while creating or updating a provider.
  Future<void> _showProviderConfigError({
    required String title,
    required CoreLinkError error,
  }) async {
    if (!mounted) {
      return;
    }
    await _ProviderConfigErrorDialog.show(
      context: context,
      title: title,
      message: error.message,
    );
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
    _reload();
  }

  /// Opens one provider detail page using the latest loaded snapshot.
  Future<void> _openProviderDetail(core_proxy.ProviderProfile provider) async {
    final data = await _future!;
    if (!mounted) {
      return;
    }
    await _ProviderDetailScreen.open(
      context: context,
      providerId: provider.id,
      initialData: data,
      reload: load,
      onSelectModel: _selectChatModel,
      onAddModel: _addProviderModel,
      onEditProvider: _editOpenedProvider,
      clients: widget.clients,
      onDeleteModel: _deleteModel,
      onTestModelConnection: _testModelConnection,
    );
  }

  /// Edits a provider from the detail page using the latest provider list.
  Future<void> _editOpenedProvider(core_proxy.ProviderProfile provider) async {
    final data = await _future!;
    await _editProvider(provider, data.providers);
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
      onTest: (capabilities) =>
          _testModelConnection(provider, model, capabilities),
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
    core_proxy.ModelCapabilities capabilities,
  ) async {
    if (!mounted) {
      return null;
    }
    final l10n = AppLocalizations.of(context)!;
    final testKey = _modelTestKey(provider.id, model.id);
    setState(() {
      _testingModelKey = testKey;
    });
    try {
      await widget.clients.preferencesModelConfigManager
          .updateCapabilitiesForModel(
            providerId: provider.id,
            modelId: model.id,
            capabilities: capabilities,
          );
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
        current: capabilities,
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
    required core_proxy.ModelCapabilities current,
  }) async {
    final chatPassed = connectionTestSucceeded(
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
          capabilities: capabilitiesFromConnectionTest(report, current),
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
              icon: Icons.dns_outlined,
              initiallyExpanded: true,
              action: SettingsSectionAddButton(
                tooltip: '添加模型供应商',
                onPressed: _createProvider,
              ),
              children: <Widget>[
                _ProviderCardList(
                  providers: data.providers,
                  summaries: data.summaries,
                  chatBinding: data.chatBinding,
                  onOpenProvider: _openProviderDetail,
                ),
              ],
            ),
            TtsProviderSection(
              clients: widget.clients,
              initiallyExpanded: false,
            ),
            SttProviderSection(
              clients: widget.clients,
              initiallyExpanded: false,
            ),
            _SectionCard(
              title: l10n.settingsModelFunctionMappingsSection,
              icon: Icons.tune_outlined,
              initiallyExpanded: false,
              children: <Widget>[
                _FunctionMappingGroups(
                  data: data,
                  onSelectFunction: _selectFunctionModel,
                  onFollowAll: _setAllFunctionsFollowChat,
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
  const _ProviderEditorDialog({
    required this.catalogEntries,
    required this.occupiedProviderNames,
    required this.clients,
    this.provider,
  });

  final List<core_proxy.ProviderCatalogEntry> catalogEntries;
  final Set<String> occupiedProviderNames;
  final GeneratedCoreProxyClients clients;
  final core_proxy.ProviderProfile? provider;

  static Future<_ProviderEditResult?> show({
    required BuildContext context,
    required List<core_proxy.ProviderCatalogEntry> catalogEntries,
    required Set<String> occupiedProviderNames,
    required GeneratedCoreProxyClients clients,
    core_proxy.ProviderProfile? provider,
  }) {
    return showDialog<_ProviderEditResult>(
      context: context,
      builder: (context) => _ProviderEditorDialog(
        catalogEntries: catalogEntries,
        occupiedProviderNames: occupiedProviderNames,
        clients: clients,
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
  core_proxy.CodexSessionStatus? _codexStatus;

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
      if (provider.providerTypeId == 'OPENAI_CODEX') {
        _endpointController.text =
            'https://chatgpt.com/backend-api/codex/responses';
        unawaited(_refreshCodexStatus());
      }
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
    if (providerTypeId == 'OPENAI_CODEX') {
      unawaited(_refreshCodexStatus());
    }
  }

  bool get _isCodexProvider => _selectedProviderTypeId == 'OPENAI_CODEX';

  Future<void> _refreshCodexStatus() async {
    try {
      final status = await widget.clients.servicesCodexOAuthService
          .sessionStatus();
      if (!mounted) {
        return;
      }
      setState(() {
        _codexStatus = status;
      });
    } catch (_) {
      if (!mounted) {
        return;
      }
      setState(() {
        _codexStatus = null;
      });
    }
  }

  Future<void> _openCodexLogin() async {
    await showCodexLoginDialog(context: context, clients: widget.clients);
    if (!mounted) {
      return;
    }
    await _refreshCodexStatus();
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
        endpoint: _isCodexProvider
            ? 'https://chatgpt.com/backend-api/codex/responses'
            : _endpointController.text.trim(),
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
                  validator: (value) {
                    final isDuplicate = widget.occupiedProviderNames.contains(
                      value!.trim(),
                    );
                    return isDuplicate
                        ? l10n.settingsModelDuplicateProviderName
                        : null;
                  },
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
                  readOnly: _isCodexProvider,
                  keyboardType: TextInputType.url,
                  inputFormatters: <TextInputFormatter>[
                    FilteringTextInputFormatter.deny(RegExp(r'\s')),
                  ],
                  suffixIcon: _isCodexProvider
                      ? const Icon(Icons.lock_outline, size: 20)
                      : (endpointOptions.isEmpty
                          ? null
                          : IconButton(
                              tooltip: l10n.settingsModelApiEndpoint,
                              icon: const Icon(Icons.arrow_drop_down_rounded),
                              onPressed: _showEndpointOptionsDialog,
                            )),
                ),
                if (_isCodexProvider)
                  _CodexLoginField(
                    status: _codexStatus,
                    onLogin: _openCodexLogin,
                  )
                else
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

enum _AvailableModelListScope { fetched, all }

// Lucide SVG Icons (MIT License)
const String _kSvgSearch =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>';
const String _kSvgClose =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>';
const String _kSvgLayers =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="m22 12.5-8.58 3.91a2 2 0 0 1-1.66 0L2 12.5"/><path d="m22 17.5-8.58 3.91a2 2 0 0 1-1.66 0L2 17.5"/></svg>';
const String _kSvgSparkles =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/><path d="M20 3v4"/><path d="M22 5h-4"/><path d="M4 17v2"/><path d="M5 18H3"/></svg>';
const String _kSvgHistory =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/></svg>';
const String _kSvgCheck =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>';
const String _kSvgCpu =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="16" height="16" x="4" y="4" rx="2"/><rect width="6" height="6" x="9" y="9" rx="1"/><path d="M15 2v2"/><path d="M15 20v2"/><path d="M2 15h2"/><path d="M2 9h2"/><path d="M20 15h2"/><path d="M20 9h2"/><path d="M9 2v2"/><path d="M9 20v2"/></svg>';
const String _kSvgChevronRight =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>';
const String _kSvgChevronDown =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>';
const String _kSvgCheckSquare =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="m9 12 2 2 4-4"/></svg>';
const String _kSvgCloudOff =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m2 2 20 20"/><path d="M5.782 5.782A7 7 0 0 0 9 19h8.5a4.5 4.5 0 0 0 1.307-.193"/><path d="M21.532 16.5A4.5 4.5 0 0 0 17.5 10h-1.79A7.008 7.008 0 0 0 10 5.07"/></svg>';
const String _kSvgSearchX =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m13.5 8.5-5 5"/><path d="m8.5 8.5 5 5"/><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>';

class _LucideIcon extends StatelessWidget {
  const _LucideIcon(this.svg, {this.size = 18, this.color});

  final String svg;
  final double size;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final effectiveColor = color ?? Theme.of(context).colorScheme.onSurface;
    return SvgPicture.string(
      svg,
      width: size,
      height: size,
      colorFilter: ColorFilter.mode(effectiveColor, BlendMode.srcIn),
    );
  }
}

class _ModernCheckbox extends StatelessWidget {
  const _ModernCheckbox({required this.checked, required this.onTap});

  final bool checked;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(6),
      child: Container(
        width: 18,
        height: 18,
        decoration: BoxDecoration(
          color: checked ? colorScheme.primary : Colors.transparent,
          borderRadius: BorderRadius.circular(5),
          border: Border.all(
            color: checked
                ? colorScheme.primary
                : colorScheme.outlineVariant.withValues(alpha: 0.5),
            width: 1.4,
          ),
        ),
        child: checked
            ? Center(
                child: _LucideIcon(
                  _kSvgCheck,
                  size: 13,
                  color: colorScheme.onPrimary,
                ),
              )
            : null,
      ),
    );
  }
}

class _TransformerScopeSlider extends StatelessWidget {
  const _TransformerScopeSlider({
    required this.scope,
    required this.fetchedLabel,
    required this.allLabel,
    required this.onChanged,
  });

  final _AvailableModelListScope scope;
  final String fetchedLabel;
  final String allLabel;
  final ValueChanged<_AvailableModelListScope> onChanged;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isAll = scope == _AvailableModelListScope.all;

    return Container(
      height: 38,
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.35),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final itemWidth = (constraints.maxWidth - 2) / 2;
          return Stack(
            children: <Widget>[
              // Fluid Transformer sliding thumb
              AnimatedAlign(
                duration: const Duration(milliseconds: 260),
                curve: Curves.fastOutSlowIn,
                alignment: isAll ? Alignment.centerRight : Alignment.centerLeft,
                child: Container(
                  width: itemWidth,
                  height: double.infinity,
                  decoration: BoxDecoration(
                    color: colorScheme.primaryContainer.withValues(alpha: 0.65),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: colorScheme.primary.withValues(alpha: 0.35),
                      width: 1,
                    ),
                    boxShadow: <BoxShadow>[
                      BoxShadow(
                        color: colorScheme.shadow.withValues(alpha: 0.12),
                        blurRadius: 4,
                        offset: const Offset(0, 1),
                      ),
                    ],
                  ),
                ),
              ),
              // Option labels with smooth morph animation
              Row(
                children: <Widget>[
                  Expanded(
                    child: _SliderOptionItem(
                      icon: _kSvgSparkles,
                      label: fetchedLabel,
                      selected: !isAll,
                      onTap: () => onChanged(_AvailableModelListScope.fetched),
                    ),
                  ),
                  Expanded(
                    child: _SliderOptionItem(
                      icon: _kSvgHistory,
                      label: allLabel,
                      selected: isAll,
                      onTap: () => onChanged(_AvailableModelListScope.all),
                    ),
                  ),
                ],
              ),
            ],
          );
        },
      ),
    );
  }
}

class _SliderOptionItem extends StatelessWidget {
  const _SliderOptionItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Center(
          child: AnimatedDefaultTextStyle(
            duration: const Duration(milliseconds: 220),
            curve: Curves.easeInOut,
            style: textTheme.labelMedium!.copyWith(
              fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
              color: selected
                  ? colorScheme.onPrimaryContainer
                  : colorScheme.onSurfaceVariant,
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              mainAxisAlignment: MainAxisAlignment.center,
              children: <Widget>[
                AnimatedScale(
                  scale: selected ? 1.06 : 0.94,
                  duration: const Duration(milliseconds: 220),
                  curve: Curves.easeOutBack,
                  child: _LucideIcon(
                    icon,
                    size: 14,
                    color: selected
                        ? colorScheme.primary
                        : colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(width: 6),
                Flexible(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _AvailableModelDialogState extends State<_AvailableModelDialog> {
  final _searchController = TextEditingController();
  final Set<String> _selectedModelIds = <String>{};
  late _AvailableModelListScope _scope;

  @override
  void initState() {
    super.initState();
    final hasFetched = widget.models.any(_availableProviderModelIsFetched);
    _scope = hasFetched
        ? _AvailableModelListScope.fetched
        : _AvailableModelListScope.all;
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  /// Returns models in the current fetched-or-history scope.
  List<core_proxy.AvailableProviderModel> _scopedModels() {
    if (_scope == _AvailableModelListScope.all) {
      return widget.models;
    }
    return widget.models
        .where(_availableProviderModelIsFetched)
        .toList(growable: false);
  }

  /// Filters scoped models according to the current search query.
  List<core_proxy.AvailableProviderModel> _filteredModels(
    AppLocalizations l10n,
  ) {
    final query = _searchController.text.trim().toLowerCase();
    final scopedModels = _scopedModels();
    if (query.isEmpty) {
      return scopedModels;
    }
    return scopedModels
        .where((model) {
          final text =
              '${model.modelId} ${_availableModelSubtitle(l10n, model, includeSource: _scope == _AvailableModelListScope.all)}'
                  .toLowerCase();
          return text.contains(query);
        })
        .toList(growable: false);
  }

  /// Switches between fetched-only and catalog-history listings.
  void _setScope(_AvailableModelListScope scope) {
    setState(() {
      _scope = scope;
      if (scope == _AvailableModelListScope.fetched) {
        _selectedModelIds.removeWhere((modelId) {
          return widget.models.any(
            (model) =>
                model.modelId == modelId &&
                !_availableProviderModelIsFetched(model),
          );
        });
      }
    });
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

  /// Returns whether all currently filtered models are selected.
  bool _areAllFilteredSelected(
    List<core_proxy.AvailableProviderModel> filtered,
  ) {
    if (filtered.isEmpty) {
      return false;
    }
    return filtered.every((model) => _selectedModelIds.contains(model.modelId));
  }

  /// Selects or clears all currently filtered models.
  void _toggleSelectAll(List<core_proxy.AvailableProviderModel> filtered) {
    setState(() {
      if (_areAllFilteredSelected(filtered)) {
        for (final model in filtered) {
          _selectedModelIds.remove(model.modelId);
        }
      } else {
        for (final model in filtered) {
          _selectedModelIds.add(model.modelId);
        }
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textTheme = theme.textTheme;
    final l10n = AppLocalizations.of(context)!;
    final query = _searchController.text.trim();
    final filteredModels = _filteredModels(l10n);
    final includeSource = _scope == _AvailableModelListScope.all;
    final fetchedCount = widget.models
        .where(_availableProviderModelIsFetched)
        .length;
    final allCount = widget.models.length;
    final allFilteredSelected = _areAllFilteredSelected(filteredModels);

    final viewport = MediaQuery.sizeOf(context);
    final dialogWidth = (viewport.width - 32).clamp(320.0, 560.0).toDouble();
    final dialogHeight = (viewport.height - 48).clamp(320.0, 600.0).toDouble();

    return Dialog(
      insetPadding: const EdgeInsets.all(16),
      clipBehavior: Clip.antiAlias,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: SizedBox(
        width: dialogWidth,
        height: dialogHeight,
        child: Column(
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(18, 12, 12, 12),
              child: Row(
                children: <Widget>[
                  Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      color: colorScheme.primary.withValues(alpha: 0.12),
                      borderRadius: BorderRadius.circular(9),
                    ),
                    child: Center(
                      child: _LucideIcon(
                        _kSvgLayers,
                        size: 18,
                        color: colorScheme.primary,
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      l10n.settingsModelAddModel,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  IconButton(
                    tooltip: MaterialLocalizations.of(
                      context,
                    ).closeButtonTooltip,
                    icon: _LucideIcon(
                      _kSvgClose,
                      size: 16,
                      color: colorScheme.onSurfaceVariant,
                    ),
                    visualDensity: VisualDensity.compact,
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
              child: Column(
                children: <Widget>[
                  Container(
                    height: 38,
                    decoration: BoxDecoration(
                      color: colorScheme.surfaceContainerHighest.withValues(
                        alpha: 0.35,
                      ),
                      borderRadius: BorderRadius.circular(10),
                      border: Border.all(
                        color: colorScheme.outlineVariant.withValues(
                          alpha: 0.25,
                        ),
                      ),
                    ),
                    child: TextField(
                      controller: _searchController,
                      style: textTheme.bodyMedium,
                      textAlignVertical: TextAlignVertical.center,
                      decoration: InputDecoration(
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 12,
                        ),
                        border: InputBorder.none,
                        prefixIcon: Padding(
                          padding: const EdgeInsets.all(10),
                          child: _LucideIcon(
                            _kSvgSearch,
                            size: 16,
                            color: colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.7,
                            ),
                          ),
                        ),
                        prefixIconConstraints: const BoxConstraints(
                          minWidth: 36,
                          minHeight: 36,
                        ),
                        hintText: l10n.search,
                        hintStyle: textTheme.bodyMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant.withValues(
                            alpha: 0.6,
                          ),
                        ),
                        suffixIcon: _searchController.text.isNotEmpty
                            ? IconButton(
                                icon: _LucideIcon(
                                  _kSvgClose,
                                  size: 14,
                                  color: colorScheme.onSurfaceVariant,
                                ),
                                visualDensity: VisualDensity.compact,
                                padding: EdgeInsets.zero,
                                onPressed: () {
                                  _searchController.clear();
                                  setState(() {});
                                },
                              )
                            : null,
                        suffixIconConstraints: const BoxConstraints(
                          minWidth: 32,
                          minHeight: 32,
                        ),
                      ),
                      onChanged: (_) => setState(() {}),
                    ),
                  ),
                  const SizedBox(height: 10),
                  Row(
                    children: <Widget>[
                      Expanded(
                        child: _TransformerScopeSlider(
                          scope: _scope,
                          fetchedLabel:
                              '${l10n.settingsModelAvailableFetchedOnly} ($fetchedCount)',
                          allLabel:
                              '${l10n.settingsModelAvailableIncludeHistory} ($allCount)',
                          onChanged: _setScope,
                        ),
                      ),
                      if (filteredModels.isNotEmpty) ...<Widget>[
                        const SizedBox(width: 10),
                        Material(
                          color: colorScheme.surfaceContainerHighest.withValues(
                            alpha: 0.35,
                          ),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(10),
                            side: BorderSide(
                              color: colorScheme.outlineVariant.withValues(
                                alpha: 0.15,
                              ),
                            ),
                          ),
                          child: InkWell(
                            onTap: () => _toggleSelectAll(filteredModels),
                            borderRadius: BorderRadius.circular(10),
                            child: Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 10,
                                vertical: 7,
                              ),
                              child: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: <Widget>[
                                  _LucideIcon(
                                    allFilteredSelected
                                        ? _kSvgClose
                                        : _kSvgCheckSquare,
                                    size: 14,
                                    color: allFilteredSelected
                                        ? colorScheme.error
                                        : colorScheme.primary,
                                  ),
                                  const SizedBox(width: 5),
                                  Text(
                                    allFilteredSelected
                                        ? l10n.clear
                                        : l10n.settingsWorkspaceSelectAllCurrentList,
                                    style: textTheme.labelMedium?.copyWith(
                                      color: allFilteredSelected
                                          ? colorScheme.error
                                          : colorScheme.primary,
                                      fontWeight: FontWeight.w600,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                ],
              ),
            ),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.fromLTRB(16, 4, 16, 12),
                children: <Widget>[
                  if (filteredModels.isEmpty &&
                      query.isEmpty &&
                      _scope == _AvailableModelListScope.fetched)
                    Padding(
                      padding: const EdgeInsets.symmetric(
                        vertical: 36,
                        horizontal: 16,
                      ),
                      child: Center(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            _LucideIcon(
                              _kSvgCloudOff,
                              size: 36,
                              color: colorScheme.onSurfaceVariant.withValues(
                                alpha: 0.5,
                              ),
                            ),
                            const SizedBox(height: 10),
                            Text(
                              l10n.settingsModelAvailableEmptyFetched,
                              textAlign: TextAlign.center,
                              style: textTheme.bodySmall?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ],
                        ),
                      ),
                    )
                  else if (filteredModels.isEmpty && query.isNotEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 36),
                      child: Center(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            _LucideIcon(
                              _kSvgSearchX,
                              size: 36,
                              color: colorScheme.onSurfaceVariant.withValues(
                                alpha: 0.5,
                              ),
                            ),
                            const SizedBox(height: 10),
                            Text(
                              l10n.noData,
                              style: textTheme.bodyMedium?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  for (final model in filteredModels)
                    _AvailableModelItemCard(
                      model: model,
                      selected: _selectedModelIds.contains(model.modelId),
                      includeSource: includeSource,
                      onTap: () => _toggleModel(model),
                    ),
                  _CustomModelActionCard(
                    onTap: () => Navigator.of(
                      context,
                    ).pop(const _AvailableModelCustom()),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Padding(
              padding: const EdgeInsets.fromLTRB(18, 12, 16, 12),
              child: Row(
                children: <Widget>[
                  Text(
                    _selectedModelIds.isEmpty
                        ? ''
                        : l10n.settingsWorkspaceSelectedCount(
                            _selectedModelIds.length,
                            filteredModels.length,
                          ),
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const Spacer(),
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    style: TextButton.styleFrom(
                      visualDensity: VisualDensity.compact,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 8,
                      ),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                    child: Text(l10n.cancel),
                  ),
                  const SizedBox(width: 10),
                  FilledButton(
                    onPressed: _selectedModelIds.isEmpty
                        ? null
                        : () => Navigator.of(
                            context,
                          ).pop(_AvailableModelsPicked(_selectedModels())),
                    style: FilledButton.styleFrom(
                      visualDensity: VisualDensity.compact,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 18,
                        vertical: 8,
                      ),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                    child: Text(
                      '${l10n.settingsModelAddModelShort} (${_selectedModelIds.length})',
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _AvailableModelItemCard extends StatelessWidget {
  const _AvailableModelItemCard({
    required this.model,
    required this.selected,
    required this.includeSource,
    required this.onTap,
  });

  final core_proxy.AvailableProviderModel model;
  final bool selected;
  final bool includeSource;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textTheme = theme.textTheme;
    final l10n = AppLocalizations.of(context)!;
    final contextLength = model.context?.maxContextLength;

    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Material(
        color: selected
            ? colorScheme.primaryContainer.withValues(alpha: 0.22)
            : colorScheme.surfaceContainerHighest.withValues(alpha: 0.2),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: BorderSide(
            color: selected
                ? colorScheme.primary.withValues(alpha: 0.45)
                : colorScheme.outlineVariant.withValues(alpha: 0.18),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 9),
            child: Row(
              children: <Widget>[
                _ModernCheckbox(checked: selected, onTap: onTap),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Text(
                        model.modelId,
                        style: textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                          color: selected
                              ? colorScheme.primary
                              : colorScheme.onSurface,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      const SizedBox(height: 4),
                      Wrap(
                        spacing: 6,
                        runSpacing: 4,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: <Widget>[
                          if (contextLength != null)
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 6,
                                vertical: 1.5,
                              ),
                              decoration: BoxDecoration(
                                color: colorScheme.primary.withValues(
                                  alpha: 0.12,
                                ),
                                borderRadius: BorderRadius.circular(6),
                              ),
                              child: Text(
                                _formatContextLength(contextLength) ??
                                    '${contextLength.round()}K',
                                style: textTheme.labelSmall?.copyWith(
                                  color: colorScheme.primary,
                                  fontWeight: FontWeight.w600,
                                  fontSize: 11,
                                ),
                              ),
                            ),
                          if (includeSource)
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 6,
                                vertical: 1.5,
                              ),
                              decoration: BoxDecoration(
                                color: colorScheme.surfaceContainerHighest,
                                borderRadius: BorderRadius.circular(6),
                              ),
                              child: Text(
                                _availableProviderModelIsFetched(model)
                                    ? l10n.settingsModelAvailableSourceFetched
                                    : l10n.settingsModelAvailableSourceHistory,
                                style: textTheme.labelSmall?.copyWith(
                                  color: colorScheme.onSurfaceVariant,
                                  fontSize: 11,
                                ),
                              ),
                            ),
                          if (model.capabilities != null)
                            _ModelCapabilityCapsules(
                              capabilities: model.capabilities!,
                            ),
                          if (model.builtinTools.isNotEmpty)
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 5,
                                vertical: 1.5,
                              ),
                              decoration: BoxDecoration(
                                color: colorScheme.tertiaryContainer.withValues(
                                  alpha: 0.35,
                                ),
                                borderRadius: BorderRadius.circular(6),
                              ),
                              child: Text(
                                l10n.settingsModelBuiltinTools,
                                style: textTheme.labelSmall?.copyWith(
                                  color: colorScheme.tertiary,
                                  fontSize: 10,
                                ),
                              ),
                            ),
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _CustomModelActionCard extends StatelessWidget {
  const _CustomModelActionCard({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final l10n = AppLocalizations.of(context)!;

    return Padding(
      padding: const EdgeInsets.only(top: 2, bottom: 4),
      child: Material(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.18),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: BorderSide(
            color: colorScheme.outlineVariant.withValues(alpha: 0.22),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            child: Row(
              children: <Widget>[
                Container(
                  width: 28,
                  height: 28,
                  decoration: BoxDecoration(
                    color: colorScheme.primary.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(7),
                  ),
                  child: Center(
                    child: _LucideIcon(
                      _kSvgCpu,
                      size: 16,
                      color: colorScheme.primary,
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Text(
                        l10n.settingsModelCustomModel,
                        style: textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                          color: colorScheme.onSurface,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        l10n.settingsModelModelId,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                          fontSize: 11,
                        ),
                      ),
                    ],
                  ),
                ),
                _LucideIcon(
                  _kSvgChevronRight,
                  size: 16,
                  color: colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
                ),
              ],
            ),
          ),
        ),
      ),
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

class _ProviderCardList extends StatelessWidget {
  const _ProviderCardList({
    required this.providers,
    required this.summaries,
    required this.chatBinding,
    required this.onOpenProvider,
  });

  final List<core_proxy.ProviderProfile> providers;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding chatBinding;
  final void Function(core_proxy.ProviderProfile provider) onOpenProvider;

  @override
  Widget build(BuildContext context) {
    if (providers.isEmpty) {
      return const SizedBox.shrink();
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (var index = 0; index < providers.length; index++) ...<Widget>[
          if (index > 0) const SizedBox(height: 8),
          _ProviderCard(
            provider: providers[index],
            summaries: summaries,
            chatBinding: chatBinding,
            onOpen: () => onOpenProvider(providers[index]),
          ),
        ],
      ],
    );
  }
}

class _ProviderCard extends StatelessWidget {
  const _ProviderCard({
    required this.provider,
    required this.summaries,
    required this.chatBinding,
    required this.onOpen,
  });

  final core_proxy.ProviderProfile provider;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding chatBinding;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final chatProvider = provider.id == chatBinding.providerId;
    final displayedModel = _displayedModelForProvider(provider, chatBinding);
    final displayedSummary = displayedModel == null
        ? null
        : _summaryForModelOrNull(summaries, provider.id, displayedModel.id);
    final contextLabel = _formatContextLength(
      displayedModel?.contextOverride?.maxContextLength,
    );
    final multimodal = _isMultimodalCapabilities(
      displayedSummary?.capabilities,
    );
    final radius = BorderRadius.circular(12);
    return Material(
      color: chatProvider
          ? colorScheme.primaryContainer.withValues(alpha: 0.16)
          : colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
      shape: RoundedRectangleBorder(
        borderRadius: radius,
        side: BorderSide(
          color: chatProvider
              ? colorScheme.primary.withValues(alpha: 0.45)
              : colorScheme.outlineVariant.withValues(alpha: 0.28),
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        borderRadius: radius,
        onTap: onOpen,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: Row(
            children: <Widget>[
              ProviderLogo(
                providerTypeId: provider.providerTypeId,
                fallbackName: provider.name,
                size: 30,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Text(
                      provider.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Row(
                      children: <Widget>[
                        Flexible(
                          child: Text(
                            displayedModel?.id ?? l10n.settingsModelNoModels,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.bodySmall!
                                .copyWith(color: colorScheme.onSurfaceVariant),
                          ),
                        ),
                        if (multimodal) ...<Widget>[
                          const SizedBox(width: 6),
                          _ProviderInfoBadge(
                            label: l10n.settingsModelMultimodalBadge,
                          ),
                        ],
                        if (contextLabel != null) ...<Widget>[
                          const SizedBox(width: 6),
                          _ProviderInfoBadge(label: contextLabel),
                        ],
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              if (chatProvider)
                const SettingsActivePill(label: '当前主模型')
              else
                Icon(
                  Icons.chevron_right,
                  size: 20,
                  color: colorScheme.onSurfaceVariant,
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ProviderInfoBadge extends StatelessWidget {
  const _ProviderInfoBadge({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
      decoration: ShapeDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.6),
        shape: const StadiumBorder(),
      ),
      child: Text(
        label,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(
          color: colorScheme.onSurfaceVariant,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _ProviderHeaderCapsule extends StatelessWidget {
  const _ProviderHeaderCapsule({
    required this.provider,
    required this.modelCountLabel,
  });

  final core_proxy.ProviderProfile provider;
  final String modelCountLabel;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final label = '${provider.name} · $modelCountLabel';

    return Container(
      padding: const EdgeInsets.fromLTRB(4, 4, 11, 4),
      decoration: ShapeDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.45),
        shape: StadiumBorder(
          side: BorderSide(
            color: colorScheme.outlineVariant.withValues(alpha: 0.35),
          ),
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          ProviderLogo(
            providerTypeId: provider.providerTypeId,
            fallbackName: provider.name,
            size: 22,
            contentScale: 0.66,
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: colorScheme.onSurface,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _CapsuleActionButton extends StatelessWidget {
  const _CapsuleActionButton({
    required this.label,
    required this.onTap,
    this.tooltip,
    this.primary = false,
  });

  final String label;
  final VoidCallback? onTap;
  final String? tooltip;
  final bool primary;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isEnabled = onTap != null;
    final backgroundColor = primary
        ? (isEnabled
              ? colorScheme.primaryContainer.withValues(alpha: 0.7)
              : colorScheme.surfaceContainerHighest.withValues(alpha: 0.2))
        : (isEnabled
              ? colorScheme.surfaceContainerHighest.withValues(alpha: 0.45)
              : colorScheme.surfaceContainerHighest.withValues(alpha: 0.2));
    final foregroundColor = primary
        ? (isEnabled
              ? colorScheme.onPrimaryContainer
              : colorScheme.onSurface.withValues(alpha: 0.38))
        : (isEnabled
              ? colorScheme.onSurface
              : colorScheme.onSurface.withValues(alpha: 0.38));
    final borderColor = primary
        ? (isEnabled
              ? colorScheme.primary.withValues(alpha: 0.25)
              : colorScheme.outlineVariant.withValues(alpha: 0.15))
        : (isEnabled
              ? colorScheme.outlineVariant.withValues(alpha: 0.35)
              : colorScheme.outlineVariant.withValues(alpha: 0.15));

    Widget button = Material(
      color: backgroundColor,
      shape: StadiumBorder(side: BorderSide(color: borderColor)),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        customBorder: const StadiumBorder(),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          child: Text(
            label,
            style: Theme.of(context).textTheme.labelMedium?.copyWith(
              color: foregroundColor,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ),
    );

    if (tooltip != null) {
      button = Tooltip(message: tooltip!, child: button);
    }
    return button;
  }
}

class _ProviderDetailScreen extends StatefulWidget {
  const _ProviderDetailScreen({
    required this.providerId,
    required this.initialData,
    required this.reload,
    required this.onSelectModel,
    required this.onAddModel,
    required this.onEditProvider,
    required this.clients,
    required this.onDeleteModel,
    required this.onTestModelConnection,
  });

  final String providerId;
  final ModelSettingsData initialData;
  final Future<ModelSettingsData> Function() reload;
  final Future<void> Function(String providerId, String modelId) onSelectModel;
  final Future<void> Function(core_proxy.ProviderProfile provider) onAddModel;
  final Future<void> Function(core_proxy.ProviderProfile provider)
  onEditProvider;
  final GeneratedCoreProxyClients clients;
  final Future<void> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onDeleteModel;
  final Future<core_proxy.ModelConnectionTestReport?> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
    core_proxy.ModelCapabilities capabilities,
  )
  onTestModelConnection;

  static Future<void> open({
    required BuildContext context,
    required String providerId,
    required ModelSettingsData initialData,
    required Future<ModelSettingsData> Function() reload,
    required Future<void> Function(String providerId, String modelId)
    onSelectModel,
    required Future<void> Function(core_proxy.ProviderProfile provider)
    onAddModel,
    required Future<void> Function(core_proxy.ProviderProfile provider)
    onEditProvider,
    required GeneratedCoreProxyClients clients,
    required Future<void> Function(
      core_proxy.ProviderProfile provider,
      core_proxy.ModelProfile model,
    )
    onDeleteModel,
    required Future<core_proxy.ModelConnectionTestReport?> Function(
      core_proxy.ProviderProfile provider,
      core_proxy.ModelProfile model,
      core_proxy.ModelCapabilities capabilities,
    )
    onTestModelConnection,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) => _ProviderDetailScreen(
        providerId: providerId,
        initialData: initialData,
        reload: reload,
        onSelectModel: onSelectModel,
        onAddModel: onAddModel,
        onEditProvider: onEditProvider,
        clients: clients,
        onDeleteModel: onDeleteModel,
        onTestModelConnection: onTestModelConnection,
      ),
    );
  }

  @override
  State<_ProviderDetailScreen> createState() => _ProviderDetailScreenState();
}

class _ProviderDetailScreenState extends State<_ProviderDetailScreen> {
  late ModelSettingsData _data = widget.initialData;
  String? _expandedModelId;

  void _toggleModelSettings(String modelId) {
    setState(() {
      _expandedModelId = _expandedModelId == modelId ? null : modelId;
    });
  }

  core_proxy.ProviderProfile? get _provider {
    for (final provider in _data.providers) {
      if (provider.id == widget.providerId) {
        return provider;
      }
    }
    return null;
  }

  Future<void> _run(Future<void> Function() action) async {
    await action();
    await _refresh();
  }

  Future<void> _refresh() async {
    try {
      final data = await widget.reload();
      if (!mounted) {
        return;
      }
      final providerExists = data.providers.any(
        (provider) => provider.id == widget.providerId,
      );
      if (!providerExists) {
        Navigator.of(context).pop();
        return;
      }
      setState(() {
        _data = data;
      });
    } catch (_) {
      // Keep showing the last snapshot if a background refresh fails.
    }
  }

  Future<core_proxy.ModelConnectionTestReport?> _testModelConnection(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
    core_proxy.ModelCapabilities capabilities,
  ) async {
    if (!mounted) {
      return null;
    }
    final l10n = AppLocalizations.of(context)!;
    try {
      await widget.clients.preferencesModelConfigManager
          .updateCapabilitiesForModel(
            providerId: provider.id,
            modelId: model.id,
            capabilities: capabilities,
          );
      final report = await widget.clients.application.testModelConnection(
        providerId: provider.id,
        modelId: model.id,
      );
      if (!mounted || report == null) {
        return report;
      }
      final chatPassed = connectionTestSucceeded(
        report,
        core_proxy.ModelConnectionTestType.chat,
      );
      if (chatPassed) {
        await widget.clients.preferencesModelConfigManager
            .updateCapabilitiesForModel(
              providerId: provider.id,
              modelId: model.id,
              capabilities: capabilitiesFromConnectionTest(
                report,
                capabilities,
              ),
            );
        if (mounted) {
          await _refresh();
        }
      }
      return report;
    } catch (error) {
      if (mounted) {
        await _ConnectionTestErrorDialog.show(
          context: context,
          message: l10n.settingsModelConnectionTestError('$error'),
        );
      }
      return null;
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final provider = _provider;
    if (provider == null) {
      return const Dialog(
        child: SizedBox(
          width: 420,
          height: 220,
          child: Center(child: M3LoadingIndicator(size: 32)),
        ),
      );
    }
    final viewport = MediaQuery.sizeOf(context);
    final dialogWidth = (viewport.width - 32).clamp(320.0, 760.0).toDouble();
    final dialogHeight = (viewport.height - 48).clamp(320.0, 700.0).toDouble();
    return Dialog(
      insetPadding: const EdgeInsets.all(16),
      child: SizedBox(
        width: dialogWidth,
        height: dialogHeight,
        child: Column(
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 10, 8, 10),
              child: Row(
                children: <Widget>[
                  Expanded(
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: _ProviderHeaderCapsule(
                        provider: provider,
                        modelCountLabel: l10n.settingsModelProviderModelCount(
                          provider.models.length,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  _CapsuleActionButton(
                    label: l10n.settingsModelAddModel,
                    tooltip: l10n.settingsModelAddModel,
                    primary: true,
                    onTap: () => _run(() => widget.onAddModel(provider)),
                  ),
                  const SizedBox(width: 8),
                  _CapsuleActionButton(
                    label: l10n.edit,
                    tooltip: l10n.settingsModelEditProvider,
                    onTap: () => _run(() => widget.onEditProvider(provider)),
                  ),
                  const SizedBox(width: 4),
                  IconButton(
                    tooltip: MaterialLocalizations.of(
                      context,
                    ).closeButtonTooltip,
                    style: IconButton.styleFrom(
                      hoverColor: colorScheme.surfaceContainerHighest
                          .withValues(
                            alpha:
                                Theme.of(context).brightness == Brightness.dark
                                ? 0.35
                                : 0.5,
                          ),
                      shape: const CircleBorder(),
                    ),
                    icon: _LucideIcon(
                      _kSvgClose,
                      size: 16,
                      color: colorScheme.onSurfaceVariant,
                    ),
                    visualDensity: VisualDensity.compact,
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
                children: <Widget>[
                  if (provider.models.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 28),
                      child: Column(
                        children: <Widget>[
                          Text(
                            l10n.settingsModelNoModels,
                            style: TextStyle(
                              color: colorScheme.onSurfaceVariant,
                            ),
                          ),
                          const SizedBox(height: 12),
                          FilledButton.icon(
                            onPressed: () =>
                                _run(() => widget.onAddModel(provider)),
                            style: SettingsControlStyles.sectionFilledButton(),
                            icon: const Icon(Icons.playlist_add, size: 18),
                            label: Text(l10n.settingsModelAddModel),
                          ),
                        ],
                      ),
                    )
                  else
                    _ProviderModelList(
                      provider: provider,
                      summaries: _data.summaries,
                      chatBinding: _data.chatBinding,
                      onSelectModel: (providerId, modelId) =>
                          _run(() => widget.onSelectModel(providerId, modelId)),
                      testingModelKey: null,
                      expandedModelId: _expandedModelId,
                      onToggleSettings: _toggleModelSettings,
                      clients: widget.clients,
                      onDeleteModel: (p, m) async {
                        setState(() {
                          _expandedModelId = null;
                        });
                        await _run(() => widget.onDeleteModel(p, m));
                      },
                      onTestModelConnection: _testModelConnection,
                      onModelSettingsSaved: _refresh,
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

core_proxy.ModelProfile? _displayedModelForProvider(
  core_proxy.ProviderProfile provider,
  core_proxy.FunctionModelBinding chatBinding,
) {
  if (provider.id == chatBinding.providerId) {
    for (final model in provider.models) {
      if (model.id == chatBinding.modelId) {
        return model;
      }
    }
  }
  return provider.models.isEmpty ? null : provider.models.first;
}

bool _isMultimodalCapabilities(core_proxy.ModelCapabilities? capabilities) {
  if (capabilities == null) {
    return false;
  }
  return capabilities.directImage ||
      capabilities.directAudio ||
      capabilities.directVideo;
}

String? _formatContextLength(double? maxContextLength) {
  if (maxContextLength == null || maxContextLength <= 0) {
    return null;
  }
  final k = maxContextLength >= 10000
      ? maxContextLength / 1000.0
      : maxContextLength;
  if (k >= 950) {
    final millions = k / 1000.0;
    final millionsRounded = millions.round();
    if ((millions - millionsRounded).abs() < 0.08) {
      return '${millionsRounded}M';
    }
    return '${millions.toStringAsFixed(1)}M';
  }
  return '${k.round()}K';
}

String _providerTypeDisplayName(AppLocalizations l10n, String providerTypeId) {
  try {
    return _providerTypeLocalName(l10n, providerTypeId);
  } on UnsupportedError {
    return providerTypeId;
  }
}

class _ProviderModelList extends StatelessWidget {
  const _ProviderModelList({
    required this.provider,
    required this.summaries,
    required this.chatBinding,
    required this.onSelectModel,
    required this.testingModelKey,
    required this.expandedModelId,
    required this.onToggleSettings,
    required this.clients,
    required this.onDeleteModel,
    required this.onTestModelConnection,
    required this.onModelSettingsSaved,
  });

  final core_proxy.ProviderProfile provider;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding chatBinding;
  final void Function(String providerId, String modelId) onSelectModel;
  final String? testingModelKey;
  final String? expandedModelId;
  final void Function(String modelId) onToggleSettings;
  final GeneratedCoreProxyClients clients;
  final Future<void> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onDeleteModel;
  final Future<core_proxy.ModelConnectionTestReport?> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
    core_proxy.ModelCapabilities capabilities,
  )
  onTestModelConnection;
  final VoidCallback onModelSettingsSaved;

  @override
  Widget build(BuildContext context) {
    final items = <Widget>[
      for (final model in provider.models)
        if (_summaryForModelOrNull(summaries, provider.id, model.id)
            case final summary?)
          _ProviderModelItem(
            key: ValueKey('${provider.id}_${model.id}'),
            provider: provider,
            model: model,
            summary: summary,
            selected:
                provider.id == chatBinding.providerId &&
                model.id == chatBinding.modelId,
            onSelect: onSelectModel,
            testing: testingModelKey == _modelTestKey(provider.id, model.id),
            isExpanded: expandedModelId == model.id,
            onToggleSettings: () => onToggleSettings(model.id),
            clients: clients,
            onDeleteModel: onDeleteModel,
            onTestModelConnection: onTestModelConnection,
            onModelSettingsSaved: onModelSettingsSaved,
          ),
    ];
    if (items.isEmpty) {
      return const SizedBox.shrink();
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (var index = 0; index < items.length; index++) ...<Widget>[
          if (index > 0) const SizedBox(height: 8),
          items[index],
        ],
      ],
    );
  }
}

class _ProviderModelItem extends StatelessWidget {
  const _ProviderModelItem({
    super.key,
    required this.provider,
    required this.model,
    required this.summary,
    required this.selected,
    required this.onSelect,
    required this.testing,
    required this.isExpanded,
    required this.onToggleSettings,
    required this.clients,
    required this.onDeleteModel,
    required this.onTestModelConnection,
    required this.onModelSettingsSaved,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final core_proxy.ProviderModelSummary summary;
  final bool selected;
  final void Function(String providerId, String modelId) onSelect;
  final bool testing;
  final bool isExpanded;
  final VoidCallback onToggleSettings;
  final GeneratedCoreProxyClients clients;
  final Future<void> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onDeleteModel;
  final Future<core_proxy.ModelConnectionTestReport?> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
    core_proxy.ModelCapabilities capabilities,
  )
  onTestModelConnection;
  final VoidCallback onModelSettingsSaved;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;

    final cardBgColor = selected
        ? (isDark
              ? Color.alphaBlend(
                  colorScheme.primary.withValues(alpha: 0.10),
                  colorScheme.surfaceContainerLow,
                )
              : Color.alphaBlend(
                  colorScheme.primary.withValues(alpha: 0.06),
                  colorScheme.surfaceContainerLow,
                ))
        : colorScheme.surfaceContainerLow;

    final borderColor = selected
        ? colorScheme.primary.withValues(alpha: isDark ? 0.58 : 0.48)
        : colorScheme.outlineVariant.withValues(alpha: isDark ? 0.35 : 0.55);

    return AnimatedContainer(
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeOutCubic,
      decoration: BoxDecoration(
        color: cardBgColor,
        borderRadius: BorderRadius.circular(10),
      ),
      foregroundDecoration: BoxDecoration(
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: borderColor, width: selected ? 1.0 : 0.8),
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _ProviderModelTile(
            provider: provider,
            model: model,
            summary: summary,
            selected: selected,
            onSelect: onSelect,
            testing: testing,
            isExpanded: isExpanded,
            onToggleSettings: onToggleSettings,
          ),
          AnimatedSize(
            duration: const Duration(milliseconds: 240),
            curve: Curves.easeInOutCubic,
            alignment: Alignment.topCenter,
            child: isExpanded
                ? Column(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Divider(
                        height: 1,
                        thickness: 0.8,
                        color: colorScheme.outlineVariant.withValues(
                          alpha: isDark ? 0.18 : 0.25,
                        ),
                      ),
                      _ModelInlineSettingsView(
                        provider: provider,
                        model: model,
                        clients: clients,
                        onTest: (caps) =>
                            onTestModelConnection(provider, model, caps),
                        onDeleted: () => onDeleteModel(provider, model),
                        onSaved: onModelSettingsSaved,
                      ),
                    ],
                  )
                : const SizedBox.shrink(),
          ),
        ],
      ),
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
    required this.isExpanded,
    required this.onToggleSettings,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final core_proxy.ProviderModelSummary summary;
  final bool selected;
  final void Function(String providerId, String modelId) onSelect;
  final bool testing;
  final bool isExpanded;
  final VoidCallback onToggleSettings;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final contextLabel = _formatContextLength(
      model.contextOverride?.maxContextLength,
    );

    return Material(
      color: selected
          ? colorScheme.primary.withValues(alpha: isDark ? 0.08 : 0.05)
          : Colors.transparent,
      child: InkWell(
        onTap: onToggleSettings,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 9),
          child: Row(
            children: <Widget>[
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      model.id,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                        color: colorScheme.onSurface,
                      ),
                    ),
                    if (_hasModelCapabilities(
                      summary.capabilities,
                      contextLabel,
                    )) ...<Widget>[
                      const SizedBox(height: 4),
                      _ModelCapabilityCapsules(
                        capabilities: summary.capabilities,
                        contextLengthLabel: contextLabel,
                      ),
                    ],
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
              IconButton(
                tooltip: isExpanded ? l10n.commonCollapse : l10n.commonExpand,
                style: IconButton.styleFrom(
                  hoverColor: colorScheme.surfaceContainerHighest.withValues(
                    alpha: isDark ? 0.35 : 0.5,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(6),
                  ),
                ),
                icon: AnimatedRotation(
                  turns: isExpanded ? 0.5 : 0.0,
                  duration: const Duration(milliseconds: 200),
                  curve: Curves.easeInOut,
                  child: _LucideIcon(
                    _kSvgChevronDown,
                    size: 16,
                    color: isExpanded
                        ? colorScheme.primary
                        : colorScheme.onSurfaceVariant,
                  ),
                ),
                visualDensity: VisualDensity.compact,
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
                onPressed: onToggleSettings,
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

class _ModelInlineSettingsView extends StatefulWidget {
  const _ModelInlineSettingsView({
    super.key,
    required this.provider,
    required this.model,
    required this.clients,
    required this.onTest,
    required this.onDeleted,
    required this.onSaved,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final GeneratedCoreProxyClients clients;
  final Future<core_proxy.ModelConnectionTestReport?> Function(
    core_proxy.ModelCapabilities capabilities,
  )
  onTest;
  final VoidCallback onDeleted;
  final VoidCallback onSaved;

  @override
  State<_ModelInlineSettingsView> createState() =>
      _ModelInlineSettingsViewState();
}

class _ModelInlineSettingsViewState extends State<_ModelInlineSettingsView> {
  Future<core_proxy.ResolvedModelConfig>? _configFuture;

  @override
  void initState() {
    super.initState();
    _load();
  }

  void _load() {
    _configFuture = widget.clients.preferencesModelConfigManager
        .getResolvedModelConfig(
          providerId: widget.provider.id,
          modelId: widget.model.id,
        );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final l10n = AppLocalizations.of(context)!;
    return Container(
      padding: const EdgeInsets.fromLTRB(12, 10, 12, 12),
      decoration: BoxDecoration(
        color: isDark
            ? Colors.black.withValues(alpha: 0.15)
            : colorScheme.surfaceContainerLow,
      ),
      child: FutureBuilder<core_proxy.ResolvedModelConfig>(
        future: _configFuture,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const Center(
              child: Padding(
                padding: EdgeInsets.symmetric(vertical: 16),
                child: M3LoadingIndicator(size: 20),
              ),
            );
          }
          if (snapshot.hasError) {
            return Center(
              child: Text(
                '${l10n.failed}: ${snapshot.error}',
                style: TextStyle(color: colorScheme.error),
              ),
            );
          }
          final config = snapshot.data!;
          return _ModelInlineSettingsForm(
            provider: widget.provider,
            model: widget.model,
            clients: widget.clients,
            initialConfig: config,
            onTest: widget.onTest,
            onDeleted: widget.onDeleted,
            onSaved: widget.onSaved,
          );
        },
      ),
    );
  }
}

class _ModelInlineSettingsForm extends StatefulWidget {
  const _ModelInlineSettingsForm({
    required this.provider,
    required this.model,
    required this.clients,
    required this.initialConfig,
    required this.onTest,
    required this.onDeleted,
    required this.onSaved,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final GeneratedCoreProxyClients clients;
  final core_proxy.ResolvedModelConfig initialConfig;
  final Future<core_proxy.ModelConnectionTestReport?> Function(
    core_proxy.ModelCapabilities capabilities,
  )
  onTest;
  final VoidCallback onDeleted;
  final VoidCallback onSaved;

  @override
  State<_ModelInlineSettingsForm> createState() =>
      _ModelInlineSettingsFormState();
}

class _ModelInlineSettingsFormState extends State<_ModelInlineSettingsForm> {
  late bool _toolCall;
  late bool _directImage;
  late bool _directAudio;
  late bool _directVideo;
  late List<core_proxy.ModelBuiltinTool> _builtinTools;
  late bool _enableSummary;
  late final TextEditingController _maxContextLengthController;
  late final TextEditingController _summaryThresholdController;
  String? _maxContextLengthError;
  bool _testingConnection = false;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    final caps = widget.initialConfig.capabilities;
    _toolCall = caps.toolCall;
    _directImage = caps.directImage;
    _directAudio = caps.directAudio;
    _directVideo = caps.directVideo;
    _builtinTools = widget.initialConfig.builtinTools;
    final summary = widget.initialConfig.summary;
    _enableSummary = summary.enableSummary;
    final rawContext = widget.initialConfig.context.maxContextLength;
    final normalizedContext = rawContext >= 10000
        ? (rawContext / 1024).roundToDouble()
        : rawContext;
    _maxContextLengthController = TextEditingController(
      text: normalizedContext > 0
          ? normalizedContext.toStringAsFixed(0)
          : '200',
    );
    final rawThresh = summary.summaryTokenThreshold;
    final threshPercent = rawThresh <= 0
        ? '70'
        : (rawThresh <= 1.0
              ? (rawThresh * 100).round().toString()
              : rawThresh.clamp(1, 100).round().toString());
    _summaryThresholdController = TextEditingController(text: threshPercent);
  }

  @override
  void dispose() {
    _maxContextLengthController.dispose();
    _summaryThresholdController.dispose();
    super.dispose();
  }

  core_proxy.ModelCapabilities _currentCapabilities() {
    return core_proxy.ModelCapabilities(
      directImage: _directImage,
      directAudio: _directAudio,
      directVideo: _directVideo,
      toolCall: _toolCall,
    );
  }

  Future<void> _runConnectionTest() async {
    setState(() => _testingConnection = true);
    try {
      final report = await widget.onTest(_currentCapabilities());
      if (!mounted || report == null) {
        return;
      }
      await _ConnectionTestReportDialog.show(context: context, report: report);
    } finally {
      if (mounted) {
        setState(() => _testingConnection = false);
      }
    }
  }

  void _setBuiltinToolEnabled(int index, bool enabled) {
    setState(() {
      final item = _builtinTools[index];
      _builtinTools = [
        for (var i = 0; i < _builtinTools.length; i++)
          if (i == index)
            core_proxy.ModelBuiltinTool(
              toolType: item.toolType,
              displayName: item.displayName,
              enabled: enabled,
              requestFormat: item.requestFormat,
              exclusivity: item.exclusivity,
              config: item.config,
            )
          else
            _builtinTools[i],
      ];
    });
  }

  Future<void> _save() async {
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
    setState(() => _saving = true);
    try {
      final newCaps = _currentCapabilities();
      await widget.clients.preferencesModelConfigManager
          .updateCapabilitiesForModel(
            providerId: widget.provider.id,
            modelId: widget.model.id,
            capabilities: newCaps,
          );
      await widget.clients.preferencesModelConfigManager.updateContextForModel(
        providerId: widget.provider.id,
        modelId: widget.model.id,
        context: core_proxy.ModelContextSpec(
          maxContextLength: maxContextLength,
        ),
      );
      await widget.clients.preferencesModelConfigManager.updateSummaryForModel(
        providerId: widget.provider.id,
        modelId: widget.model.id,
        summary: () {
          final rawThresh =
              double.tryParse(_summaryThresholdController.text.trim()) ?? 70.0;
          final ratioThreshold = rawThresh > 1.0
              ? (rawThresh / 100.0).clamp(0.01, 1.0)
              : rawThresh.clamp(0.01, 1.0);
          return core_proxy.ModelSummarySettings(
            enableSummary: _enableSummary,
            summaryTokenThreshold: ratioThreshold,
            enableSummaryByMessageCount: false,
            summaryMessageCountThreshold: 0,
          );
        }(),
      );
      await widget.clients.preferencesModelConfigManager
          .updateBuiltinToolsForModel(
            providerId: widget.provider.id,
            modelId: widget.model.id,
            builtinTools: _builtinTools,
          );
      widget.onSaved();
    } finally {
      if (mounted) {
        setState(() => _saving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final isZh =
        Localizations.localeOf(context).languageCode.toLowerCase() == 'zh';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        // Top Header Row with Test Model Button
        Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: Row(
            children: <Widget>[
              Text(
                isZh ? '模型配置' : 'Model Configuration',
                style: theme.textTheme.titleSmall?.copyWith(
                  fontWeight: FontWeight.w700,
                  color: colorScheme.onSurface,
                  fontSize: 13,
                ),
              ),
              const Spacer(),
              _TestModelPillButton(
                testing: _testingConnection,
                onPressed: _testingConnection ? null : _runConnectionTest,
                label: l10n.settingsModelTestModel,
              ),
            ],
          ),
        ),

        // Group 1: 核心能力 (Checkable Chips in Wrap)
        _SettingsGroupCard(
          title: l10n.settingsModelCapabilities,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.all(8),
              child: Wrap(
                spacing: 6,
                runSpacing: 6,
                children: <Widget>[
                  _CheckableCapabilityChip(
                    title: l10n.settingsModelToolCall,
                    tooltip: l10n.settingsModelToolCallDescription,
                    checked: _toolCall,
                    icon: Icons.build_outlined,
                    iconTint: _CapsuleTint.primary,
                    onChanged: (v) => setState(() => _toolCall = v),
                  ),
                  _CheckableCapabilityChip(
                    title: l10n.settingsModelDirectImage,
                    tooltip: l10n.settingsModelDirectImageDescription,
                    checked: _directImage,
                    icon: Icons.image_outlined,
                    iconTint: _CapsuleTint.primary,
                    onChanged: (v) => setState(() => _directImage = v),
                  ),
                  _CheckableCapabilityChip(
                    title: l10n.settingsModelDirectAudio,
                    tooltip: l10n.settingsModelDirectAudioDescription,
                    checked: _directAudio,
                    icon: Icons.graphic_eq,
                    iconTint: _CapsuleTint.primary,
                    onChanged: (v) => setState(() => _directAudio = v),
                  ),
                  _CheckableCapabilityChip(
                    title: l10n.settingsModelDirectVideo,
                    tooltip: l10n.settingsModelDirectVideoDescription,
                    checked: _directVideo,
                    icon: Icons.videocam_outlined,
                    iconTint: _CapsuleTint.primary,
                    onChanged: (v) => setState(() => _directVideo = v),
                  ),
                ],
              ),
            ),
            if (_builtinTools.isNotEmpty) ...<Widget>[
              const _GroupDivider(),
              Padding(
                padding: const EdgeInsets.fromLTRB(10, 8, 10, 10),
                child: Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: <Widget>[
                    for (var index = 0; index < _builtinTools.length; index++)
                      _CheckableCapabilityChip(
                        title: _builtinTools[index].displayName,
                        tooltip: _builtinToolSubtitle(
                          l10n,
                          _builtinTools[index],
                        ),
                        checked: _builtinTools[index].enabled,
                        icon: Icons.extension_outlined,
                        iconTint: _CapsuleTint.neutral,
                        onChanged: (value) =>
                            _setBuiltinToolEnabled(index, value),
                      ),
                  ],
                ),
              ),
            ],
          ],
        ),

        const SizedBox(height: 8),

        // Group 2: 上下文与总结 (Single Symmetrical Row)
        _SettingsGroupCard(
          title: isZh ? '上下文与总结' : 'Context & Summary',
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
              child: Row(
                children: <Widget>[
                  // Item 1: 上下文
                  Expanded(
                    child: Row(
                      children: <Widget>[
                        Text(
                          isZh ? '上下文:' : 'Context:',
                          style: theme.textTheme.labelMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                            color: colorScheme.onSurface,
                            fontSize: 12,
                          ),
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: _ModernNumberField(
                            controller: _maxContextLengthController,
                            suffix: 'K',
                            hintText: '200',
                            errorText: _maxContextLengthError,
                            onChanged: (_) {
                              if (_maxContextLengthError != null) {
                                setState(() => _maxContextLengthError = null);
                              }
                            },
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 14),
                  // Item 2: 总结阈值
                  Expanded(
                    child: Tooltip(
                      message: isZh
                          ? '当上下文占用达到设定百分比时触发总结（0% 为关闭，默认 70%）'
                          : 'Trigger summary when context exceeds ratio (0% to disable, default 70%)',
                      child: Row(
                        children: <Widget>[
                          Text(
                            isZh ? '总结阈值:' : 'Summary:',
                            style: theme.textTheme.labelMedium?.copyWith(
                              fontWeight: FontWeight.w600,
                              color: colorScheme.onSurface,
                              fontSize: 12,
                            ),
                          ),
                          const SizedBox(width: 6),
                          Expanded(
                            child: _ModernNumberField(
                              controller: _summaryThresholdController,
                              suffix: '%',
                              hintText: '70',
                              onChanged: (val) {
                                final numVal = double.tryParse(val.trim()) ?? 0;
                                setState(() {
                                  _enableSummary = numVal > 0;
                                });
                              },
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),

        // Bottom Action Bar
        Padding(
          padding: const EdgeInsets.only(top: 10, bottom: 2),
          child: Row(
            children: <Widget>[
              OutlinedButton.icon(
                style: OutlinedButton.styleFrom(
                  foregroundColor: colorScheme.error,
                  side: BorderSide(
                    color: colorScheme.error.withValues(
                      alpha: isDark ? 0.35 : 0.5,
                    ),
                    width: 0.8,
                  ),
                  shape: const StadiumBorder(),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 6,
                  ),
                  visualDensity: VisualDensity.compact,
                ),
                onPressed: widget.onDeleted,
                icon: const Icon(Icons.delete_outline_rounded, size: 15),
                label: Text(
                  l10n.delete,
                  style: const TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 11.5,
                  ),
                ),
              ),
              const Spacer(),
              FilledButton.icon(
                style: FilledButton.styleFrom(
                  shape: const StadiumBorder(),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 6,
                  ),
                  visualDensity: VisualDensity.compact,
                ),
                onPressed: _saving ? null : _save,
                icon: _saving
                    ? const SizedBox.square(
                        dimension: 13,
                        child: Center(child: M3LoadingIndicator(size: 13)),
                      )
                    : _LucideIcon(
                        _kSvgCheck,
                        size: 14,
                        color: colorScheme.onPrimary,
                      ),
                label: Text(
                  l10n.save,
                  style: const TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 12.5,
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _SettingsGroupCard extends StatelessWidget {
  const _SettingsGroupCard({
    required this.title,
    required this.children,
    this.trailing,
  });

  final String title;
  final List<Widget> children;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.only(left: 4, right: 4, bottom: 4),
          child: Row(
            children: <Widget>[
              Text(
                title,
                style: theme.textTheme.labelMedium?.copyWith(
                  fontWeight: FontWeight.w700,
                  color: isDark
                      ? colorScheme.onSurface.withValues(alpha: 0.8)
                      : colorScheme.onSurfaceVariant,
                  fontSize: 11.5,
                  letterSpacing: 0.2,
                ),
              ),
              if (trailing != null) ...<Widget>[const Spacer(), trailing!],
            ],
          ),
        ),
        Container(
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerLow,
            borderRadius: BorderRadius.circular(9),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(
                alpha: isDark ? 0.45 : 0.70,
              ),
              width: 1.0,
            ),
            boxShadow: isDark
                ? null
                : <BoxShadow>[
                    BoxShadow(
                      color: Colors.black.withValues(alpha: 0.03),
                      blurRadius: 3,
                      offset: const Offset(0, 1),
                    ),
                  ],
          ),
          clipBehavior: Clip.antiAlias,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: children,
          ),
        ),
      ],
    );
  }
}

class _CheckableCapabilityChip extends StatelessWidget {
  const _CheckableCapabilityChip({
    required this.title,
    required this.tooltip,
    required this.checked,
    required this.onChanged,
    required this.icon,
    required this.iconTint,
  });

  final String title;
  final String tooltip;
  final bool checked;
  final ValueChanged<bool> onChanged;
  final IconData icon;
  final _CapsuleTint iconTint;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;

    final (
      Color activeBg,
      Color activeText,
      Color activeBorder,
    ) = switch (iconTint) {
      _CapsuleTint.primary => (
        Color.alphaBlend(
          colorScheme.primary.withValues(alpha: isDark ? 0.10 : 0.04),
          colorScheme.surface,
        ),
        colorScheme.onSurface,
        colorScheme.primary.withValues(alpha: isDark ? 0.34 : 0.24),
      ),
      _CapsuleTint.neutral => (
        isDark
            ? colorScheme.surfaceContainerHigh.withValues(alpha: 0.7)
            : colorScheme.surfaceContainerHighest,
        colorScheme.onSurface,
        colorScheme.outlineVariant.withValues(alpha: isDark ? 0.6 : 0.8),
      ),
    };

    final bgColor = checked
        ? activeBg
        : (isDark
              ? colorScheme.surfaceContainer.withValues(alpha: 0.25)
              : colorScheme.surfaceContainerLow);

    final borderColor = checked
        ? activeBorder
        : colorScheme.outlineVariant.withValues(alpha: isDark ? 0.35 : 0.5);

    final textColor = checked
        ? activeText
        : colorScheme.onSurfaceVariant;

    return Tooltip(
      message: tooltip,
      child: Material(
        color: bgColor,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(6),
          side: BorderSide(color: borderColor, width: checked ? 0.9 : 0.6),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: () => onChanged(!checked),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 4.5),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                _CustomCheckbox(
                  checked: checked,
                  activeColor: colorScheme.primary.withValues(
                    alpha: isDark ? 0.78 : 0.68,
                  ),
                  onTap: () => onChanged(!checked),
                ),
                const SizedBox(width: 5),
                _TileIconBox(
                  icon: icon,
                  tint: checked ? iconTint : _CapsuleTint.neutral,
                  size: 18,
                  iconSize: 10.5,
                ),
                const SizedBox(width: 5),
                Text(
                  title,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    fontWeight: checked ? FontWeight.w600 : FontWeight.w500,
                    color: textColor,
                    fontSize: 11.0,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _CustomCheckbox extends StatelessWidget {
  const _CustomCheckbox({required this.checked, this.activeColor, this.onTap});

  final bool checked;
  final Color? activeColor;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;

    final effectiveActiveColor = activeColor ?? colorScheme.primary;

    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(3.5),
      child: Container(
        width: 13,
        height: 13,
        decoration: BoxDecoration(
          color: checked ? effectiveActiveColor : Colors.transparent,
          borderRadius: BorderRadius.circular(3.5),
          border: Border.all(
            color: checked
                ? effectiveActiveColor
                : colorScheme.outlineVariant.withValues(
                    alpha: isDark ? 0.55 : 0.65,
                  ),
            width: 1.0,
          ),
        ),
        child: checked
            ? Center(
                child: _LucideIcon(
                  _kSvgCheck,
                  size: 8.5,
                  color: colorScheme.onPrimary,
                ),
              )
            : null,
      ),
    );
  }
}

class _TileIconBox extends StatelessWidget {
  const _TileIconBox({
    required this.icon,
    required this.tint,
    this.size = 28,
    this.iconSize = 15,
  });

  final IconData icon;
  final _CapsuleTint tint;
  final double size;
  final double iconSize;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;

    final (Color bgColor, Color iconColor) = switch (tint) {
      _CapsuleTint.primary => (
        Color.alphaBlend(
          colorScheme.primary.withValues(alpha: isDark ? 0.10 : 0.04),
          colorScheme.surface,
        ),
        colorScheme.primary.withValues(alpha: isDark ? 0.76 : 0.64),
      ),
      _CapsuleTint.neutral => (
        colorScheme.surfaceContainerHigh.withValues(alpha: isDark ? 0.7 : 0.85),
        colorScheme.onSurfaceVariant,
      ),
    };

    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: bgColor,
        borderRadius: BorderRadius.circular(6),
      ),
      alignment: Alignment.center,
      child: Icon(icon, size: iconSize, color: iconColor),
    );
  }
}

class _ModernNumberField extends StatelessWidget {
  const _ModernNumberField({
    required this.controller,
    required this.suffix,
    this.hintText,
    this.errorText,
    this.onChanged,
    this.enabled = true,
  });

  final TextEditingController controller;
  final String suffix;
  final String? hintText;
  final String? errorText;
  final ValueChanged<String>? onChanged;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;

    return TextField(
      controller: controller,
      enabled: enabled,
      style: theme.textTheme.bodyMedium?.copyWith(
        fontWeight: FontWeight.w600,
        fontSize: 12.5,
        color: enabled
            ? colorScheme.onSurface
            : colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
      ),
      keyboardType: TextInputType.number,
      onChanged: onChanged,
      decoration: InputDecoration(
        isDense: true,
        hintText: hintText,
        errorText: errorText,
        filled: true,
        fillColor: enabled
            ? (isDark
                  ? colorScheme.surfaceContainer
                  : colorScheme.surfaceContainerLow)
            : (isDark
                  ? colorScheme.surfaceContainerLowest
                  : colorScheme.surfaceContainerHighest.withValues(
                      alpha: 0.25,
                    )),
        contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
        suffixText: suffix,
        suffixStyle: theme.textTheme.labelSmall?.copyWith(
          color: enabled
              ? colorScheme.onSurfaceVariant
              : colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
          fontWeight: FontWeight.w600,
          fontSize: 10.5,
        ),
        disabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(
            color: colorScheme.outlineVariant.withValues(
              alpha: isDark ? 0.2 : 0.35,
            ),
            width: 0.8,
          ),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(
            color: colorScheme.outline.withValues(alpha: isDark ? 0.4 : 0.55),
            width: 1.0,
          ),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: colorScheme.primary, width: 1.3),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: colorScheme.error, width: 1.0),
        ),
        focusedErrorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: colorScheme.error, width: 1.3),
        ),
      ),
    );
  }
}

class _GroupDivider extends StatelessWidget {
  const _GroupDivider();

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Divider(
      height: 1.0,
      thickness: 1.0,
      color: Theme.of(
        context,
      ).colorScheme.outlineVariant.withValues(alpha: isDark ? 0.35 : 0.55),
    );
  }
}

class _TestModelPillButton extends StatelessWidget {
  const _TestModelPillButton({
    required this.testing,
    required this.onPressed,
    required this.label,
  });

  final bool testing;
  final VoidCallback? onPressed;
  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Material(
      color: colorScheme.primary.withValues(alpha: isDark ? 0.12 : 0.06),
      shape: StadiumBorder(
        side: BorderSide(
          color: colorScheme.primary.withValues(alpha: isDark ? 0.28 : 0.32),
          width: 0.8,
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onPressed,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 4.5),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (testing)
                const SizedBox.square(
                  dimension: 12,
                  child: Center(child: M3LoadingIndicator(size: 12)),
                )
              else
                Icon(
                  Icons.speed_rounded,
                  size: 13.5,
                  color: colorScheme.primary,
                ),
              const SizedBox(width: 4.5),
              Text(
                label,
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: colorScheme.primary,
                  fontSize: 11,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

bool _hasModelCapabilities(
  core_proxy.ModelCapabilities? capabilities, [
  String? contextLengthLabel,
]) =>
    contextLengthLabel != null ||
    (capabilities != null &&
        (capabilities.toolCall ||
            capabilities.directImage ||
            capabilities.directAudio ||
            capabilities.directVideo));

enum _CapsuleTint { neutral, primary }

class _ModelCapabilityCapsules extends StatelessWidget {
  const _ModelCapabilityCapsules({
    super.key,
    required this.capabilities,
    this.contextLengthLabel,
  });

  final core_proxy.ModelCapabilities capabilities;
  final String? contextLengthLabel;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final isZh =
        Localizations.localeOf(context).languageCode.toLowerCase() == 'zh';

    final capsules = <Widget>[
      if (contextLengthLabel != null)
        _CapabilityCapsule(
          label: contextLengthLabel!,
          tooltip: isZh
              ? '上下文长度: $contextLengthLabel'
              : 'Context length: $contextLengthLabel',
          tint: _CapsuleTint.neutral,
        ),
      if (capabilities.toolCall)
        _CapabilityCapsule(
          label: isZh ? '工具调用' : 'Tools',
          tooltip: l10n.settingsModelToolCall,
          tint: _CapsuleTint.primary,
        ),
      if (capabilities.directImage)
        _CapabilityCapsule(
          label: isZh ? '图片' : 'Image',
          tooltip: l10n.settingsModelDirectImage,
          tint: _CapsuleTint.primary,
        ),
      if (capabilities.directAudio)
        _CapabilityCapsule(
          label: isZh ? '音频' : 'Audio',
          tooltip: l10n.settingsModelDirectAudio,
          tint: _CapsuleTint.primary,
        ),
      if (capabilities.directVideo)
        _CapabilityCapsule(
          label: isZh ? '视频' : 'Video',
          tooltip: l10n.settingsModelDirectVideo,
          tint: _CapsuleTint.primary,
        ),
    ];

    if (capsules.isEmpty) {
      return const SizedBox.shrink();
    }

    return Padding(
      padding: const EdgeInsets.only(top: 2),
      child: Wrap(spacing: 5, runSpacing: 3, children: capsules),
    );
  }
}

class _CapabilityCapsule extends StatelessWidget {
  const _CapabilityCapsule({
    required this.label,
    required this.tooltip,
    this.tint = _CapsuleTint.neutral,
  });

  final String label;
  final String tooltip;
  final _CapsuleTint tint;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;

    final (Color bgColor, Color textColor, Color borderColor) = switch (tint) {
      _CapsuleTint.primary => (
        Color.alphaBlend(
          colorScheme.primary.withValues(alpha: isDark ? 0.20 : 0.10),
          colorScheme.surfaceContainerHigh,
        ),
        colorScheme.onSurfaceVariant,
        colorScheme.primary.withValues(alpha: isDark ? 0.48 : 0.38),
      ),
      _CapsuleTint.neutral => (
        isDark
            ? colorScheme.surfaceContainerHigh
            : colorScheme.surfaceContainerHighest,
        colorScheme.onSurfaceVariant,
        colorScheme.outlineVariant.withValues(alpha: isDark ? 0.6 : 0.8),
      ),
    };

    return Tooltip(
      message: tooltip,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
        decoration: ShapeDecoration(
          color: bgColor,
          shape: StadiumBorder(
            side: BorderSide(color: borderColor, width: 0.6),
          ),
        ),
        child: Text(
          label,
          style: theme.textTheme.labelSmall?.copyWith(
            fontSize: 10,
            fontWeight: FontWeight.w600,
            color: textColor,
            height: 1.2,
          ),
        ),
      ),
    );
  }
}

typedef _ModelCapabilityIcons = _ModelCapabilityCapsules;

class _FunctionMappingGroups extends StatefulWidget {
  const _FunctionMappingGroups({
    required this.data,
    required this.onSelectFunction,
    required this.onFollowAll,
  });

  final ModelSettingsData data;
  final Future<void> Function(
    core_proxy.FunctionType functionType,
    ModelSettingsData data,
  )
  onSelectFunction;
  final Future<void> Function() onFollowAll;

  @override
  State<_FunctionMappingGroups> createState() => _FunctionMappingGroupsState();
}

class _FunctionMappingGroupsState extends State<_FunctionMappingGroups> {
  bool _showAdvanced = false;

  static const List<core_proxy.FunctionType> _coreTypes =
      <core_proxy.FunctionType>[
        core_proxy.FunctionType.chat,
        core_proxy.FunctionType.titleGeneration,
        core_proxy.FunctionType.summary,
        core_proxy.FunctionType.memory,
        core_proxy.FunctionType.roleResponsePlanner,
      ];

  static const List<core_proxy.FunctionType> _advancedTypes =
      <core_proxy.FunctionType>[
        core_proxy.FunctionType.uiController,
        core_proxy.FunctionType.translation,
        core_proxy.FunctionType.grep,
        core_proxy.FunctionType.imageRecognition,
        core_proxy.FunctionType.audioRecognition,
        core_proxy.FunctionType.videoRecognition,
      ];

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final followingCount = widget.data.functionBindings.entries
        .where(
          (entry) =>
              entry.key != core_proxy.FunctionType.chat &&
              entry.value.followsChat,
        )
        .length;
    final totalNonChat = _coreTypes.length - 1 + _advancedTypes.length;
    final customAdvancedCount = _advancedTypes
        .where((t) => !(widget.data.functionBindings[t]?.followsChat ?? true))
        .length;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.only(bottom: 4),
          child: Row(
            children: <Widget>[
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                decoration: ShapeDecoration(
                  color: colorScheme.surfaceContainerHighest.withValues(
                    alpha: 0.5,
                  ),
                  shape: const StadiumBorder(),
                ),
                child: Text(
                  '$followingCount/$totalNonChat 跟随主模型',
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const Spacer(),
              TextButton.icon(
                onPressed: widget.onFollowAll,
                style: SettingsControlStyles.sectionTextButton(),
                icon: const Icon(Icons.link, size: 16),
                label: Text(l10n.settingsModelFunctionFollowChatAll),
              ),
            ],
          ),
        ),
        for (final functionType in _coreTypes)
          _FunctionMappingRow(
            functionType: functionType,
            displayBinding: _resolveFunctionBinding(widget.data, functionType),
            followsChat:
                functionType != core_proxy.FunctionType.chat &&
                (widget.data.functionBindings[functionType]?.followsChat ??
                    false),
            summary: widget.data.summaryForBinding(
              _resolveFunctionBinding(widget.data, functionType),
            ),
            onSelect: () => widget.onSelectFunction(functionType, widget.data),
          ),
        const SizedBox(height: 4),
        Material(
          color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.22),
          borderRadius: BorderRadius.circular(8),
          child: InkWell(
            borderRadius: BorderRadius.circular(8),
            onTap: () => setState(() => _showAdvanced = !_showAdvanced),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
              child: Row(
                children: <Widget>[
                  Icon(
                    _showAdvanced
                        ? Icons.keyboard_arrow_up
                        : Icons.keyboard_arrow_down,
                    size: 18,
                    color: colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 6),
                  Text(
                    '更多高级与多模态配置 (${_advancedTypes.length})',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  if (customAdvancedCount > 0) ...<Widget>[
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 6,
                        vertical: 1,
                      ),
                      decoration: ShapeDecoration(
                        color: colorScheme.primaryContainer.withValues(
                          alpha: 0.6,
                        ),
                        shape: const StadiumBorder(),
                      ),
                      child: Text(
                        '$customAdvancedCount 个独立配置',
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                          color: colorScheme.primary,
                          fontSize: 10,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                  const Spacer(),
                ],
              ),
            ),
          ),
        ),
        if (_showAdvanced) ...<Widget>[
          const SizedBox(height: 2),
          for (final functionType in _advancedTypes)
            _FunctionMappingRow(
              functionType: functionType,
              displayBinding: _resolveFunctionBinding(
                widget.data,
                functionType,
              ),
              followsChat:
                  widget.data.functionBindings[functionType]?.followsChat ??
                  false,
              summary: widget.data.summaryForBinding(
                _resolveFunctionBinding(widget.data, functionType),
              ),
              onSelect: () =>
                  widget.onSelectFunction(functionType, widget.data),
            ),
        ],
      ],
    );
  }
}

class _FunctionMappingRow extends StatelessWidget {
  const _FunctionMappingRow({
    required this.functionType,
    required this.displayBinding,
    required this.followsChat,
    required this.summary,
    required this.onSelect,
  });

  final core_proxy.FunctionType functionType;
  final core_proxy.FunctionModelBinding displayBinding;
  final bool followsChat;
  final core_proxy.ProviderModelSummary? summary;
  final VoidCallback onSelect;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final warning = summary == null
        ? l10n.settingsModelFunctionMappingsMissing(
            displayBinding.providerId,
            displayBinding.modelId,
          )
        : _functionMappingWarning(l10n, functionType, summary!);
    final bindingText = l10n.settingsModelFunctionMappingsCurrent(
      summary?.providerName ?? displayBinding.providerId,
      displayBinding.modelId,
    );
    final description = _functionTypeDescription(l10n, functionType);

    return Tooltip(
      message: description,
      waitDuration: const Duration(milliseconds: 500),
      child: Material(
        type: MaterialType.transparency,
        child: InkWell(
          onTap: onSelect,
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            child: Row(
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Row(
                        children: <Widget>[
                          Flexible(
                            child: Text(
                              _functionTypeTitle(l10n, functionType),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: Theme.of(context).textTheme.bodyMedium
                                  ?.copyWith(fontWeight: FontWeight.w600),
                            ),
                          ),
                          if (followsChat) ...<Widget>[
                            const SizedBox(width: 6),
                            const _FollowChatBadge(),
                          ],
                        ],
                      ),
                      const SizedBox(height: 2),
                      Row(
                        children: <Widget>[
                          ProviderLogo(
                            providerTypeId: summary?.providerTypeId ?? '',
                            fallbackName:
                                summary?.providerName ??
                                displayBinding.providerId,
                            size: 16,
                            contentScale: 0.72,
                          ),
                          const SizedBox(width: 6),
                          Flexible(
                            child: Text(
                              bindingText,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: Theme.of(context).textTheme.bodySmall
                                  ?.copyWith(
                                    color: summary == null
                                        ? colorScheme.error
                                        : colorScheme.onSurfaceVariant,
                                  ),
                            ),
                          ),
                          if (warning != null) ...<Widget>[
                            const SizedBox(width: 6),
                            Icon(
                              Icons.warning_amber_outlined,
                              size: 13,
                              color: colorScheme.error,
                            ),
                            const SizedBox(width: 3),
                            Flexible(
                              child: Text(
                                warning,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: Theme.of(context).textTheme.labelSmall
                                    ?.copyWith(color: colorScheme.error),
                              ),
                            ),
                          ],
                        ],
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 6),
                Icon(
                  Icons.chevron_right,
                  size: 18,
                  color: colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _FollowChatBadge extends StatelessWidget {
  const _FollowChatBadge();

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: ShapeDecoration(
        color: colorScheme.primaryContainer.withValues(alpha: 0.45),
        shape: const StadiumBorder(),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(Icons.link, size: 10, color: colorScheme.primary),
          const SizedBox(width: 3),
          Text(
            l10n.settingsModelFunctionFollowChat,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: colorScheme.primary,
              fontSize: 10,
              fontWeight: FontWeight.w700,
            ),
          ),
        ],
      ),
    );
  }
}

core_proxy.FunctionModelBinding _resolveFunctionBinding(
  ModelSettingsData data,
  core_proxy.FunctionType functionType,
) {
  final binding = data.functionBindings[functionType]!;
  if (binding.followsChat && functionType != core_proxy.FunctionType.chat) {
    return data.chatBinding;
  }
  return binding;
}

sealed class _FunctionModelSelectionResult {
  const _FunctionModelSelectionResult();
}

class _FunctionModelSelection extends _FunctionModelSelectionResult {
  const _FunctionModelSelection({
    required this.providerId,
    required this.modelId,
  });

  final String providerId;
  final String modelId;
}

class _FunctionModelFollowChat extends _FunctionModelSelectionResult {
  const _FunctionModelFollowChat();
}

class _ProviderModelGroupEntry {
  const _ProviderModelGroupEntry({
    required this.providerId,
    required this.providerName,
    required this.providerTypeId,
    required this.models,
  });

  final String providerId;
  final String providerName;
  final String providerTypeId;
  final List<core_proxy.ProviderModelSummary> models;
}

class _FunctionModelSelectorDialog extends StatefulWidget {
  const _FunctionModelSelectorDialog({
    required this.functionType,
    required this.summaries,
    required this.currentBinding,
    required this.chatBinding,
    required this.followsChat,
  });

  final core_proxy.FunctionType functionType;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding currentBinding;
  final core_proxy.FunctionModelBinding chatBinding;
  final bool followsChat;

  static Future<_FunctionModelSelectionResult?> show({
    required BuildContext context,
    required core_proxy.FunctionType functionType,
    required List<core_proxy.ProviderModelSummary> summaries,
    required core_proxy.FunctionModelBinding currentBinding,
    required core_proxy.FunctionModelBinding chatBinding,
    required bool followsChat,
  }) {
    return showDialog<_FunctionModelSelectionResult>(
      context: context,
      builder: (context) => _FunctionModelSelectorDialog(
        functionType: functionType,
        summaries: summaries,
        currentBinding: currentBinding,
        chatBinding: chatBinding,
        followsChat: followsChat,
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

  List<_ProviderModelGroupEntry> _filteredGroups() {
    final query = _searchController.text.trim().toLowerCase();
    final groups = <String, _ProviderModelGroupEntry>{};
    for (final summary in widget.summaries) {
      if (query.isNotEmpty &&
          !'${summary.modelId} ${summary.providerName} ${summary.providerTypeId}'
              .toLowerCase()
              .contains(query)) {
        continue;
      }
      final group = groups.putIfAbsent(
        summary.providerId,
        () => _ProviderModelGroupEntry(
          providerId: summary.providerId,
          providerName: summary.providerName,
          providerTypeId: summary.providerTypeId,
          models: <core_proxy.ProviderModelSummary>[],
        ),
      );
      group.models.add(summary);
    }
    return groups.values.toList(growable: false);
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
    final groups = _filteredGroups();
    return AlertDialog(
      title: Text(
        l10n.settingsModelFunctionMappingsSelect(
          _functionTypeTitle(l10n, widget.functionType),
        ),
      ),
      content: SizedBox(
        width: 560,
        height: 520,
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
              child: groups.isEmpty
                  ? Center(child: Text(l10n.noData))
                  : ListView(
                      children: <Widget>[
                        if (widget.functionType != core_proxy.FunctionType.chat)
                          _FollowChatQuickOption(
                            selected: widget.followsChat,
                            onTap: () => Navigator.of(
                              context,
                            ).pop(const _FunctionModelFollowChat()),
                          ),
                        for (final group in groups) ...<Widget>[
                          _FunctionModelGroupHeader(
                            providerName: group.providerName,
                            providerTypeId: group.providerTypeId,
                            count: group.models.length,
                          ),
                          for (final summary in group.models)
                            _FunctionModelOptionTile(
                              summary: summary,
                              enabled: _functionModelSupported(
                                widget.functionType,
                                summary,
                              ),
                              unsupportedReason: _functionMappingWarning(
                                l10n,
                                widget.functionType,
                                summary,
                              ),
                              selected:
                                  summary.providerId ==
                                      widget.currentBinding.providerId &&
                                  summary.modelId ==
                                      widget.currentBinding.modelId,
                              onTap: () => _selectModel(summary),
                            ),
                        ],
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
      ],
    );
  }
}

class _FollowChatQuickOption extends StatelessWidget {
  const _FollowChatQuickOption({required this.selected, required this.onTap});

  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: selected
          ? colorScheme.primaryContainer.withValues(alpha: 0.24)
          : colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
      borderRadius: BorderRadius.circular(10),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(10),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
          child: Row(
            children: <Widget>[
              Icon(Icons.link, size: 20, color: colorScheme.primary),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      l10n.settingsModelFunctionFollowChat,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      l10n.settingsModelFunctionFollowChatHint,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              if (selected)
                Icon(Icons.check_circle, size: 20, color: colorScheme.primary),
            ],
          ),
        ),
      ),
    );
  }
}

class _FunctionModelGroupHeader extends StatelessWidget {
  const _FunctionModelGroupHeader({
    required this.providerName,
    required this.providerTypeId,
    required this.count,
  });

  final String providerName;
  final String providerTypeId;
  final int count;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(6, 10, 6, 4),
      child: Row(
        children: <Widget>[
          ProviderLogo(
            providerTypeId: providerTypeId,
            fallbackName: providerName,
            size: 20,
            contentScale: 0.72,
          ),
          const SizedBox(width: 7),
          Expanded(
            child: Text(
              providerName,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(
                context,
              ).textTheme.labelMedium?.copyWith(fontWeight: FontWeight.w700),
            ),
          ),
          Text(
            '$count',
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}

class _FunctionModelOptionTile extends StatelessWidget {
  const _FunctionModelOptionTile({
    required this.summary,
    required this.enabled,
    required this.unsupportedReason,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.ProviderModelSummary summary;
  final bool enabled;
  final String? unsupportedReason;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Opacity(
      opacity: enabled ? 1 : 0.45,
      child: Material(
        type: MaterialType.transparency,
        child: InkWell(
          onTap: enabled ? onTap : null,
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
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
                      const SizedBox(height: 2),
                      Row(
                        children: <Widget>[
                          _ModelCapabilityCapsules(
                            capabilities: summary.capabilities,
                          ),
                          if (!enabled &&
                              unsupportedReason != null) ...<Widget>[
                            const SizedBox(width: 6),
                            Icon(
                              Icons.warning_amber_outlined,
                              size: 13,
                              color: colorScheme.error,
                            ),
                            const SizedBox(width: 4),
                            Flexible(
                              child: Text(
                                unsupportedReason!,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: Theme.of(context).textTheme.labelSmall
                                    ?.copyWith(color: colorScheme.error),
                              ),
                            ),
                          ],
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            ),
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
  final Future<core_proxy.ModelConnectionTestReport?> Function(
    core_proxy.ModelCapabilities capabilities,
  )
  onTest;

  static Future<_ModelSettingsEditorResult?> show({
    required BuildContext context,
    required String providerName,
    required String modelId,
    required core_proxy.ModelCapabilities initialCapabilities,
    required List<core_proxy.ModelBuiltinTool> initialBuiltinTools,
    required core_proxy.ModelContextSpec initialContext,
    required core_proxy.ModelSummarySettings initialSummary,
    required Future<core_proxy.ModelConnectionTestReport?> Function(
      core_proxy.ModelCapabilities capabilities,
    )
    onTest,
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
    _enableSummary = widget.initialSummary.enableSummary;
    _enableSummaryByMessageCount =
        widget.initialSummary.enableSummaryByMessageCount;
    final rawContext = widget.initialContext.maxContextLength;
    final normalizedContext = rawContext >= 10000
        ? (rawContext / 1024).roundToDouble()
        : rawContext;
    _maxContextLengthController = TextEditingController(
      text: normalizedContext > 0
          ? normalizedContext.toStringAsFixed(0)
          : '200',
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

  core_proxy.ModelCapabilities _editorCapabilities() {
    return core_proxy.ModelCapabilities(
      directImage: _directImage,
      directAudio: _directAudio,
      directVideo: _directVideo,
      toolCall: _toolCall,
    );
  }

  Future<void> _runConnectionTest() async {
    setState(() {
      _testingConnection = true;
    });
    try {
      final current = _editorCapabilities();
      final report = await widget.onTest(current);
      if (mounted &&
          report != null &&
          connectionTestSucceeded(
            report,
            core_proxy.ModelConnectionTestType.chat,
          )) {
        final next = capabilitiesFromConnectionTest(report, current);
        setState(() {
          _directImage = next.directImage;
          _directAudio = next.directAudio;
          _directVideo = next.directVideo;
          _toolCall = next.toolCall;
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
                  suffixText: 'K',
                  errorText: _maxContextLengthError,
                ),
                keyboardType: TextInputType.number,
                onChanged: (_) {
                  setState(() => _maxContextLengthError = null);
                },
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

class _ProviderConfigErrorDialog extends StatelessWidget {
  const _ProviderConfigErrorDialog({
    required this.title,
    required this.message,
  });

  final String title;
  final String message;

  /// Shows the provider create/update runtime error dialog.
  static Future<void> show({
    required BuildContext context,
    required String title,
    required String message,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) =>
          _ProviderConfigErrorDialog(title: title, message: message),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(title),
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
    this.icon,
    this.action,
    this.initiallyExpanded = true,
  });

  final String title;
  final List<Widget> children;
  final IconData? icon;
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
                if (icon != null) ...<Widget>[
                  Icon(icon, size: 18, color: colorScheme.onSurfaceVariant),
                  const SizedBox(width: 8),
                ],
                Expanded(
                  child: Text(
                    title,
                    style: SettingsControlStyles.sectionTitleTextStyle(context),
                  ),
                ),
                if (action != null) ...<Widget>[
                  action!,
                  const SizedBox(width: 4),
                ],
              ],
            ),
            children: children,
          ),
        ),
      ),
    );
  }
}

class _CodexLoginField extends StatelessWidget {
  const _CodexLoginField({required this.status, required this.onLogin});

  final core_proxy.CodexSessionStatus? status;
  final VoidCallback onLogin;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final signedIn = status?.signedIn == true;
    final account = status == null
        ? ''
        : (status!.email.trim().isEmpty ? status!.accountId : status!.email);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Text(l10n.settingsModelCodexLoginDescription),
          const SizedBox(height: 8),
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  signedIn
                      ? l10n.settingsModelCodexSignedIn(account)
                      : l10n.settingsModelCodexSignedOut,
                ),
              ),
              FilledButton.tonal(
                onPressed: onLogin,
                child: Text(l10n.settingsModelCodexLogin),
              ),
            ],
          ),
        ],
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
    this.readOnly = false,
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
  final bool readOnly;
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
        readOnly: readOnly,
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

/// Returns whether this option came from a live provider listing.
bool _availableProviderModelIsFetched(core_proxy.AvailableProviderModel model) {
  return model.source != core_proxy.AvailableProviderModelSource.catalog;
}

/// Builds the add-model row subtitle from source, capabilities, and context.
String _availableModelSubtitle(
  AppLocalizations l10n,
  core_proxy.AvailableProviderModel model, {
  bool includeSource = false,
}) {
  final labels = <String>[];
  if (includeSource) {
    labels.add(
      _availableProviderModelIsFetched(model)
          ? l10n.settingsModelAvailableSourceFetched
          : l10n.settingsModelAvailableSourceHistory,
    );
  }
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
    final formatted = _formatContextLength(context.maxContextLength);
    if (formatted != null) {
      labels.add(formatted);
    }
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
