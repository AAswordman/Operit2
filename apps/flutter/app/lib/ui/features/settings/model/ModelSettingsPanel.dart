// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/CommonNetworkErrorView.dart';
import '../../../common/ThinkingLevelText.dart';
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
    final providers = await modelManager.getProviderProfiles();
    final summaries = await modelManager.getAllModelSummaries();
    core_proxy.ResolvedModelConfig? currentConfig;
    if (summaries.any(
      (model) =>
          model.providerId == chatBinding.providerId &&
          model.modelId == chatBinding.modelId,
    )) {
      try {
        currentConfig = await modelManager.getResolvedModelConfig(
          providerId: chatBinding.providerId,
          modelId: chatBinding.modelId,
        );
      } on CoreLinkError catch (error) {
        // A sync batch may change provider profiles after the summaries call.
        // Keep the configuration page accessible so a dangling binding can
        // be repaired. Unrelated runtime errors must still be reported.
        if (error.code != 'COMMAND_ERROR' ||
            !(error.message.startsWith('model not found: ') ||
                error.message.startsWith('provider not found: '))) {
          rethrow;
        }
      }
    }
    final data = ModelSettingsData(
      providers: providers,
      summaries: summaries,
      chatBinding: chatBinding,
      currentConfig: currentConfig,
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
  /// Saves provider edits while preserving its identity and configured models.
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
          providerTypeId: saveResult.providerTypeId,
          providerType: core_proxy.ApiProviderType.fromJson(
            saveResult.providerTypeId,
          ),
          endpoint: saveResult.endpoint,
          apiKey: saveResult.apiKey,
          useMultipleApiKeys: provider.useMultipleApiKeys,
          apiKeyPool: provider.apiKeyPool,
          currentKeyIndex: provider.currentKeyIndex,
          keyRotationMode: provider.keyRotationMode,
          customHeaders: saveResult.customHeaders,
          requestLimitPerMinute: saveResult.requestLimitPerMinute,
          maxConcurrentRequests: saveResult.maxConcurrentRequests,
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
      reload: () {
        // Keep function mappings in sync with edits saved inside the detail dialog.
        _reload();
        return loadFuture;
      },
      onSelectModel: _selectChatModel,
      onAddModel: _addProviderModel,
      onEditProvider: _editOpenedProvider,
      onDeleteProvider: _deleteProvider,
      clients: widget.clients,
      onDeleteModel: _deleteModel,
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
    final core_proxy.ModelCatalogStatus catalogStatus;
    try {
      availableModels = await widget.clients.preferencesModelConfigManager
          .getAvailableProviderModels(providerId: provider.id);
      catalogStatus = await widget.clients.preferencesModelConfigManager
          .getModelCatalogStatus();
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
    List<core_proxy.AvailableProviderModel> selectable(
      List<core_proxy.AvailableProviderModel> models,
    ) {
      return models
          .where((model) => !existingModelIds.contains(model.modelId))
          .toList(growable: false);
    }

    if (!mounted) {
      return;
    }
    final selection = await _AvailableModelDialog.show(
      context: context,
      models: selectable(availableModels),
      catalogStatus: catalogStatus,
      onRetryCatalog: () async {
        final manager = widget.clients.preferencesModelConfigManager;
        final status = await manager.refreshModelCatalog();
        final models = await manager.getAvailableProviderModels(
          providerId: provider.id,
        );
        return _AvailableModelsReload(
          models: selectable(models),
          catalogStatus: status,
        );
      },
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
                  model: model,
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
            if (data.currentConfig == null) ...<Widget>[
              CommonNetworkErrorView(
                errorText: l10n.settingsModelFunctionMappingsMissing(
                  data.chatBinding.providerId,
                  data.chatBinding.modelId,
                ),
              ),
              const SizedBox(height: 12),
            ],
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
  final core_proxy.ResolvedModelConfig? currentConfig;
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
  });

  final String name;
  final String providerTypeId;
  final String endpoint;
  final String apiKey;
  final String customHeaders;
  final int requestLimitPerMinute;
  final int maxConcurrentRequests;
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
    } else {
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

  bool get _isLocalProvider => _selectedProviderTypeId == 'LOCAL_MODEL';

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

  /// Saves provider settings from the dialog form.
  void _save() {
    if (!_formKey.currentState!.validate() || _selectedProviderTypeId == null) {
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
      ),
    );
  }

  /// Builds editable provider settings for both new and existing profiles.
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
            padding: const EdgeInsets.only(top: 12, bottom: 8),
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
                    onChanged: _onProviderTypeChanged,
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
                  requiredField: !_isLocalProvider,
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

/// Selectable models and catalog freshness after the model catalog was downloaded again.
class _AvailableModelsReload {
  const _AvailableModelsReload({
    required this.models,
    required this.catalogStatus,
  });

  final List<core_proxy.AvailableProviderModel> models;
  final core_proxy.ModelCatalogStatus catalogStatus;
}

class _AvailableModelDialog extends StatefulWidget {
  const _AvailableModelDialog({
    required this.models,
    required this.catalogStatus,
    required this.onRetryCatalog,
  });

  final List<core_proxy.AvailableProviderModel> models;
  final core_proxy.ModelCatalogStatus catalogStatus;
  final Future<_AvailableModelsReload> Function() onRetryCatalog;

  static Future<_AvailableModelSelection?> show({
    required BuildContext context,
    required List<core_proxy.AvailableProviderModel> models,
    required core_proxy.ModelCatalogStatus catalogStatus,
    required Future<_AvailableModelsReload> Function() onRetryCatalog,
  }) {
    return showDialog<_AvailableModelSelection>(
      context: context,
      builder: (context) => _AvailableModelDialog(
        models: models,
        catalogStatus: catalogStatus,
        onRetryCatalog: onRetryCatalog,
      ),
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
  const _ModernCheckbox({
    required this.checked,
    required this.onTap,
    this.partial = false,
  });

  final bool checked;

  /// Whether only some of what the box stands for is checked; the box shows a dash.
  final bool partial;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final filled = checked || partial;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(6),
      child: Container(
        width: 18,
        height: 18,
        decoration: BoxDecoration(
          color: filled ? colorScheme.primary : Colors.transparent,
          borderRadius: BorderRadius.circular(5),
          border: Border.all(
            color: filled
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
            : (partial
                  ? Center(
                      child: Icon(
                        Icons.remove_rounded,
                        size: 14,
                        color: colorScheme.onPrimary,
                      ),
                    )
                  : null),
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
  late List<core_proxy.AvailableProviderModel> _models;
  late core_proxy.ModelCatalogStatus _catalogStatus;
  bool _retryingCatalog = false;
  String? _retryError;

  @override
  void initState() {
    super.initState();
    _models = widget.models;
    _catalogStatus = widget.catalogStatus;
    final hasFetched = _models.any(_availableProviderModelIsFetched);
    _scope = hasFetched
        ? _AvailableModelListScope.fetched
        : _AvailableModelListScope.all;
  }

  /// Downloads the model catalog again and describes the provider's models with it.
  Future<void> _retryCatalog() async {
    setState(() {
      _retryingCatalog = true;
      _retryError = null;
    });
    try {
      final reload = await widget.onRetryCatalog();
      if (!mounted) {
        return;
      }
      setState(() {
        _models = reload.models;
        _catalogStatus = reload.catalogStatus;
        final modelIds = _models.map((model) => model.modelId).toSet();
        _selectedModelIds.retainWhere(modelIds.contains);
      });
    } on CoreLinkError catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _retryError = core_proxy.CoreProxyErrorDetails.fromCoreLinkError(
          error,
        ).message;
      });
    } finally {
      if (mounted) {
        setState(() => _retryingCatalog = false);
      }
    }
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  /// Returns models in the current fetched-or-history scope.
  List<core_proxy.AvailableProviderModel> _scopedModels() {
    if (_scope == _AvailableModelListScope.all) {
      return _models;
    }
    return _models
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
          return _models.any(
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
    return _models
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
    final fetchedCount = _models.where(_availableProviderModelIsFetched).length;
    final allCount = _models.length;
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
                  const SizedBox(height: 8),
                  _CatalogStatusLine(
                    status: _catalogStatus,
                    retryError: _retryError,
                    retrying: _retryingCatalog,
                    onRetry: _retryCatalog,
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
                          if (model.catalogModelId == null &&
                              model.capabilities == null)
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 6,
                                vertical: 1.5,
                              ),
                              decoration: BoxDecoration(
                                color: colorScheme.errorContainer.withValues(
                                  alpha: 0.45,
                                ),
                                borderRadius: BorderRadius.circular(6),
                              ),
                              child: Text(
                                l10n.settingsModelUnrecognized,
                                style: textTheme.labelSmall?.copyWith(
                                  color: colorScheme.onErrorContainer,
                                  fontSize: 11,
                                ),
                              ),
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

/// Says where model details come from and whether the last catalog download failed.
class _CatalogStatusLine extends StatelessWidget {
  const _CatalogStatusLine({
    required this.status,
    required this.retryError,
    required this.retrying,
    required this.onRetry,
  });

  final core_proxy.ModelCatalogStatus status;
  final String? retryError;
  final bool retrying;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final error = retryError ?? status.lastError;
    final updatedAt = status.updatedAt;
    final String message;
    if (error != null && updatedAt != null) {
      message = l10n.settingsModelCatalogUpdateFailedUsing(
        _formatCatalogTime(updatedAt),
        error,
      );
    } else if (error != null) {
      message = l10n.settingsModelCatalogUpdateFailed(error);
    } else if (updatedAt == null) {
      message = l10n.settingsModelCatalogNotDownloaded;
    } else {
      message = l10n.settingsModelCatalogUpdatedAt(
        _formatCatalogTime(updatedAt),
      );
    }
    final failed = error != null || updatedAt == null;
    final color = failed ? colorScheme.error : colorScheme.onSurfaceVariant;
    return Row(
      children: <Widget>[
        Icon(
          failed ? Icons.error_outline : Icons.info_outline,
          size: 14,
          color: color,
        ),
        const SizedBox(width: 6),
        Expanded(
          child: Text(
            message,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.labelSmall?.copyWith(
              color: color,
              fontSize: 11,
            ),
          ),
        ),
        if (failed && retrying)
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 8),
            child: M3LoadingIndicator(size: 14),
          )
        else if (failed)
          TextButton(
            onPressed: onRetry,
            style: TextButton.styleFrom(
              visualDensity: VisualDensity.compact,
              padding: const EdgeInsets.symmetric(horizontal: 8),
            ),
            child: Text(l10n.settingsModelCatalogRetry),
          ),
      ],
    );
  }
}

/// Formats a catalog download time as local month, day and time.
String _formatCatalogTime(int updatedAt) {
  final time = DateTime.fromMillisecondsSinceEpoch(updatedAt).toLocal();
  String twoDigits(int value) => value.toString().padLeft(2, '0');
  return '${twoDigits(time.month)}-${twoDigits(time.day)} '
      '${twoDigits(time.hour)}:${twoDigits(time.minute)}';
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
      displayedSummary?.context.maxContextLength,
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
    this.icon,
    this.primary = false,
    this.destructive = false,
  });

  final String label;
  final VoidCallback? onTap;
  final String? tooltip;
  final IconData? icon;
  final bool primary;

  /// Marks an action that removes something, in the error color.
  final bool destructive;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final Color backgroundColor;
    final Color foregroundColor;
    final Color borderColor;
    if (onTap == null) {
      backgroundColor = colorScheme.surfaceContainerHighest.withValues(
        alpha: 0.2,
      );
      foregroundColor = colorScheme.onSurface.withValues(alpha: 0.38);
      borderColor = colorScheme.outlineVariant.withValues(alpha: 0.15);
    } else if (primary) {
      backgroundColor = colorScheme.primaryContainer.withValues(alpha: 0.7);
      foregroundColor = colorScheme.onPrimaryContainer;
      borderColor = colorScheme.primary.withValues(alpha: 0.25);
    } else if (destructive) {
      backgroundColor = colorScheme.error.withValues(alpha: 0.08);
      foregroundColor = colorScheme.error;
      borderColor = colorScheme.error.withValues(alpha: 0.35);
    } else {
      backgroundColor = colorScheme.surfaceContainerHighest.withValues(
        alpha: 0.45,
      );
      foregroundColor = colorScheme.onSurface;
      borderColor = colorScheme.outlineVariant.withValues(alpha: 0.35);
    }

    Widget button = Material(
      color: backgroundColor,
      shape: StadiumBorder(side: BorderSide(color: borderColor)),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        customBorder: const StadiumBorder(),
        child: Padding(
          padding: EdgeInsetsDirectional.fromSTEB(
            icon == null ? 10 : 8,
            5,
            10,
            5,
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (icon != null) ...<Widget>[
                Icon(icon, size: 14, color: foregroundColor),
                const SizedBox(width: 4),
              ],
              Text(
                label,
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: foregroundColor,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
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
    required this.onDeleteProvider,
    required this.clients,
    required this.onDeleteModel,
  });

  final String providerId;
  final ModelSettingsData initialData;
  final Future<ModelSettingsData> Function() reload;
  final Future<void> Function(String providerId, String modelId) onSelectModel;
  final Future<void> Function(core_proxy.ProviderProfile provider) onAddModel;
  final Future<void> Function(core_proxy.ProviderProfile provider)
  onEditProvider;
  final Future<void> Function(core_proxy.ProviderProfile provider)
  onDeleteProvider;
  final GeneratedCoreProxyClients clients;
  final Future<void> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onDeleteModel;

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
    required Future<void> Function(core_proxy.ProviderProfile provider)
    onDeleteProvider,
    required GeneratedCoreProxyClients clients,
    required Future<void> Function(
      core_proxy.ProviderProfile provider,
      core_proxy.ModelProfile model,
    )
    onDeleteModel,
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
        onDeleteProvider: onDeleteProvider,
        clients: clients,
        onDeleteModel: onDeleteModel,
      ),
    );
  }

  @override
  State<_ProviderDetailScreen> createState() => _ProviderDetailScreenState();
}

/// How far one model's connection test has got.
enum _ModelTestPhase { queued, testing, passed, failed, restricted }

/// The latest connection test of one model in the list.
class _ModelTestResult {
  const _ModelTestResult._(this.phase, {this.report, this.error, this.note});

  /// A test a batch has not reached yet.
  const _ModelTestResult.queued() : this._(_ModelTestPhase.queued);

  const _ModelTestResult.testing() : this._(_ModelTestPhase.testing);

  /// A test that ran; it passed when every check did.
  factory _ModelTestResult.finished(
    core_proxy.ModelConnectionTestReport report, {
    String? note,
  }) {
    final restricted =
        connectionTestHasRestrictions(report) &&
        !connectionTestHasFailures(report);
    return _ModelTestResult._(
      report.success
          ? _ModelTestPhase.passed
          : restricted
          ? _ModelTestPhase.restricted
          : _ModelTestPhase.failed,
      report: report,
      note: note,
    );
  }

  /// A test that could not run at all.
  factory _ModelTestResult.crashed(String error) {
    return _ModelTestResult._(
      connectionTestRestriction(error) == null
          ? _ModelTestPhase.failed
          : _ModelTestPhase.restricted,
      error: error,
    );
  }

  factory _ModelTestResult.saved(core_proxy.ModelConnectionTestResult result) {
    final report = result.report;
    if (report != null) {
      return _ModelTestResult.finished(report);
    }
    return _ModelTestResult.crashed(result.error!);
  }

  final _ModelTestPhase phase;
  final core_proxy.ModelConnectionTestReport? report;

  /// Whether the test has an outcome to show, rather than still being waited on or run.
  bool get isFinished =>
      phase == _ModelTestPhase.passed ||
      phase == _ModelTestPhase.failed ||
      phase == _ModelTestPhase.restricted;

  /// Why the test could not run, when it could not.
  final String? error;

  /// What the test changed on the model, such as the capabilities it switched.
  final String? note;
}

class _ProviderDetailScreenState extends State<_ProviderDetailScreen> {
  /// Maximum number of models tested concurrently in one batch.
  static const int _batchConcurrency = 32;

  late ModelSettingsData _data = widget.initialData;
  String? _expandedModelId;

  /// The latest test of each model, keyed by [_modelTestKey].
  final Map<String, _ModelTestResult> _tests = <String, _ModelTestResult>{};

  /// Bumped when a model's result should show itself, keyed like [_tests].
  final Map<String, int> _reveals = <String, int>{};
  final Set<String> _checkedModelIds = <String>{};
  bool _selecting = false;
  bool _batchRunning = false;
  bool _batchStopping = false;
  int _batchDone = 0;
  int _batchTotal = 0;
  final GlobalKey<_DialogToastState> _toastKey = GlobalKey<_DialogToastState>();
  late final Future<void> _testsLoaded;

  @override
  void initState() {
    super.initState();
    _testsLoaded = _restoreTests();
  }

  Future<Map<String, _ModelTestResult>> _loadSavedTests() async {
    final results = await widget.clients.preferencesModelConfigManager
        .getModelConnectionTestResults(providerId: widget.providerId);
    return <String, _ModelTestResult>{
      for (final result in results)
        _modelTestKey(result.providerId, result.modelId):
            _ModelTestResult.saved(result),
    };
  }

  Future<void> _restoreTests() async {
    try {
      final tests = await _loadSavedTests();
      if (!mounted) {
        return;
      }
      setState(() {
        _tests.addAll(tests);
      });
    } catch (error) {
      if (mounted) {
        _notify(_errorMessage(error), error: true);
      }
    }
  }

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
      await _testsLoaded;
      final data = await widget.reload();
      if (!mounted) {
        return;
      }
      core_proxy.ProviderProfile? provider;
      for (final candidate in data.providers) {
        if (candidate.id == widget.providerId) {
          provider = candidate;
        }
      }
      if (provider == null) {
        Navigator.of(context).pop();
        return;
      }
      final modelIds = provider.models.map((model) => model.id).toSet();
      final testKeys = modelIds
          .map((modelId) => _modelTestKey(widget.providerId, modelId))
          .toSet();
      final savedTests = await _loadSavedTests();
      if (!mounted) {
        return;
      }
      setState(() {
        _data = data;
        // Checks and results of models deleted meanwhile go with them.
        _checkedModelIds.retainWhere(modelIds.contains);
        _tests.removeWhere(
          (key, result) =>
              !testKeys.contains(key) ||
              (result.isFinished && !savedTests.containsKey(key)),
        );
        for (final entry in savedTests.entries) {
          _tests.putIfAbsent(entry.key, () => entry.value);
        }
      });
    } catch (error) {
      if (mounted) {
        _notify(_errorMessage(error), error: true);
      }
    }
  }

  /// Shows a short message over the bottom of the list.
  void _notify(String message, {bool error = false}) {
    _toastKey.currentState?.show(message, error: error);
  }

  void _notifySaveFailure(Object error) {
    if (!mounted) {
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    _notify(l10n.settingsModelSaveFailed(_errorMessage(error)), error: true);
  }

  void _startSelecting([String? modelId]) {
    setState(() {
      _selecting = true;
      _expandedModelId = null;
      _checkedModelIds.clear();
      if (modelId != null) {
        _checkedModelIds.add(modelId);
      }
    });
  }

  void _stopSelecting() {
    setState(() {
      _selecting = false;
      _checkedModelIds.clear();
    });
  }

  void _toggleChecked(String modelId) {
    setState(() {
      if (!_checkedModelIds.remove(modelId)) {
        _checkedModelIds.add(modelId);
      }
    });
  }

  void _toggleCheckAll(core_proxy.ProviderProfile provider) {
    setState(() {
      if (_checkedModelIds.length == provider.models.length) {
        _checkedModelIds.clear();
      } else {
        _checkedModelIds.addAll(provider.models.map((model) => model.id));
      }
    });
  }

  List<core_proxy.ModelProfile> _checkedModels(
    core_proxy.ProviderProfile provider,
  ) {
    return provider.models
        .where((model) => _checkedModelIds.contains(model.id))
        .toList(growable: false);
  }

  /// Tests one model with the capabilities its settings show. When chat works, the
  /// capabilities switch to what the checks found. Returns the capabilities the model is
  /// left with, `null` when the test did not get that far.
  Future<core_proxy.ModelCapabilities?> _testFromSettings(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
    core_proxy.ModelCapabilities capabilities,
  ) async {
    await _testsLoaded;
    if (!mounted) {
      return null;
    }
    final key = _modelTestKey(provider.id, model.id);
    final current = _tests[key];
    if (current != null && !current.isFinished) {
      // A test of this model is already running, or a batch has it queued.
      return null;
    }
    final appliedNote = AppLocalizations.of(
      context,
    )!.settingsModelCapabilitiesApplied;
    final manager = widget.clients.preferencesModelConfigManager;
    setState(() {
      _tests[key] = const _ModelTestResult.testing();
    });
    core_proxy.ModelCapabilities? found;
    _ModelTestResult result;
    try {
      await manager.updateCapabilitiesForModel(
        providerId: provider.id,
        modelId: model.id,
        capabilities: capabilities,
      );
      final report = await widget.clients.application.testModelConnection(
        providerId: provider.id,
        modelId: model.id,
      );
      if (connectionTestSucceeded(
        report,
        core_proxy.ModelConnectionTestType.chat,
      )) {
        final tested = capabilitiesFromConnectionTest(report, capabilities);
        await manager.updateCapabilitiesForModel(
          providerId: provider.id,
          modelId: model.id,
          capabilities: tested,
        );
        found = tested;
      }
      result = _ModelTestResult.finished(
        report,
        note: found == null || _sameCapabilities(found, capabilities)
            ? null
            : appliedNote,
      );
    } catch (error) {
      result = _ModelTestResult.crashed(_errorMessage(error));
      if (mounted) {
        _notify(_errorMessage(error), error: true);
      }
    }
    if (!mounted) {
      return found;
    }
    setState(() {
      _tests[key] = result;
      _reveals[key] = (_reveals[key] ?? 0) + 1;
    });
    await _refresh();
    return found;
  }

  /// Runs one model's connection test as its settings stand.
  Future<_ModelTestResult> _testOnce(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    try {
      final report = await widget.clients.application.testModelConnection(
        providerId: provider.id,
        modelId: model.id,
      );
      return _ModelTestResult.finished(report);
    } catch (error) {
      if (mounted) {
        _notify(_errorMessage(error), error: true);
      }
      return _ModelTestResult.crashed(_errorMessage(error));
    }
  }

  /// Tests [models] a few at a time and shows each result on its model as it comes in.
  /// The models' settings are left as they are.
  Future<void> _testModels(
    core_proxy.ProviderProfile provider,
    List<core_proxy.ModelProfile> models,
  ) async {
    await _testsLoaded;
    if (!mounted || _batchRunning || models.isEmpty) {
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    setState(() {
      _batchRunning = true;
      _batchStopping = false;
      _batchDone = 0;
      _batchTotal = models.length;
      for (final model in models) {
        _tests[_modelTestKey(provider.id, model.id)] =
            const _ModelTestResult.queued();
      }
    });
    var next = 0;
    var passed = 0;
    var failed = 0;
    var restricted = 0;
    Future<void> worker() async {
      while (next < models.length && !_batchStopping && mounted) {
        final model = models[next];
        next += 1;
        // Only the models being run spin; those still waiting show a quiet ring.
        setState(() {
          _tests[_modelTestKey(provider.id, model.id)] =
              const _ModelTestResult.testing();
        });
        final result = await _testOnce(provider, model);
        if (!mounted) {
          return;
        }
        if (result.phase == _ModelTestPhase.passed) {
          passed += 1;
        } else if (result.phase == _ModelTestPhase.restricted) {
          restricted += 1;
        } else {
          failed += 1;
        }
        setState(() {
          _tests[_modelTestKey(provider.id, model.id)] = result;
          _batchDone += 1;
        });
      }
    }

    final workers = models.length < _batchConcurrency
        ? models.length
        : _batchConcurrency;
    await Future.wait(<Future<void>>[
      for (var index = 0; index < workers; index++) worker(),
    ]);
    if (!mounted) {
      return;
    }
    setState(() {
      // Discard queued markers; refresh restores their last completed checks.
      for (final model in models.skip(next)) {
        _tests.remove(_modelTestKey(provider.id, model.id));
      }
      _batchRunning = false;
      _batchStopping = false;
    });
    await _refresh();
    if (!mounted) {
      return;
    }
    if (passed + failed + restricted == 0) {
      return;
    }
    _notify(
      restricted > 0
          ? l10n.settingsModelTestSummaryWithRestricted(
              passed,
              failed,
              restricted,
            )
          : failed == 0
          ? l10n.settingsModelTestAllPassed(passed)
          : l10n.settingsModelTestSummary(passed, failed),
      error: failed > 0,
    );
  }

  void _stopTests() {
    setState(() {
      _batchStopping = true;
    });
  }

  void _testChecked(core_proxy.ProviderProfile provider) {
    final models = _checkedModels(provider);
    _stopSelecting();
    unawaited(_testModels(provider, models));
  }

  /// Deletes the checked models once the user confirms; models a function uses are kept.
  Future<void> _deleteChecked(core_proxy.ProviderProfile provider) async {
    final l10n = AppLocalizations.of(context)!;
    final chosen = _checkedModels(provider);
    if (chosen.isEmpty) {
      return;
    }
    final bindings = await widget.clients.preferencesFunctionalConfigManager
        .functionModelBindingFlow()
        .first;
    if (!mounted) {
      return;
    }
    final kept = <core_proxy.ModelProfile>[];
    final keptFunctions = <core_proxy.FunctionType>{};
    final deletable = <core_proxy.ModelProfile>[];
    for (final model in chosen) {
      final functions = _boundFunctionTypesForModel(
        bindings,
        provider.id,
        model.id,
      );
      if (functions.isEmpty) {
        deletable.add(model);
      } else {
        kept.add(model);
        keptFunctions.addAll(functions);
      }
    }
    if (deletable.isEmpty) {
      await _DeleteModelBlockedDialog.show(
        context: context,
        functionTypes: keptFunctions.toList(growable: false),
        several: true,
      );
      return;
    }
    final confirmed = await _DeleteModelsConfirmDialog.show(
      context: context,
      count: deletable.length,
      keptModelIds: kept.map((model) => model.id).toList(growable: false),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    var deleted = 0;
    Object? failure;
    for (final model in deletable) {
      try {
        await widget.clients.preferencesModelConfigManager.deleteModel(
          providerId: provider.id,
          modelId: model.id,
        );
        deleted += 1;
      } catch (error) {
        failure = error;
        break;
      }
    }
    if (!mounted) {
      return;
    }
    setState(() {
      _selecting = false;
      _checkedModelIds.clear();
    });
    await _refresh();
    if (failure != null) {
      _notify('${l10n.failed}: ${_errorMessage(failure)}', error: true);
    } else {
      _notify(l10n.settingsModelDeletedCount(deleted));
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
      constraints: BoxConstraints(maxWidth: dialogWidth),
      insetPadding: const EdgeInsets.all(16),
      child: SizedBox(
        width: dialogWidth,
        height: dialogHeight,
        child: Stack(
          children: <Widget>[
            Column(
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
                            modelCountLabel: l10n
                                .settingsModelProviderModelCount(
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
                        onTap: () =>
                            _run(() => widget.onEditProvider(provider)),
                      ),
                      const SizedBox(width: 8),
                      _CapsuleActionButton(
                        label: l10n.delete,
                        tooltip: l10n.settingsModelDeleteProviderConfirmAction,
                        onTap: () =>
                            _run(() => widget.onDeleteProvider(provider)),
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
                                    Theme.of(context).brightness ==
                                        Brightness.dark
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
                if (provider.models.isNotEmpty)
                  _ModelListToolbar(
                    selecting: _selecting,
                    checkedCount: _checkedModelIds.length,
                    modelCount: provider.models.length,
                    batchRunning: _batchRunning,
                    batchStopping: _batchStopping,
                    batchDone: _batchDone,
                    batchTotal: _batchTotal,
                    onTestAll: () =>
                        unawaited(_testModels(provider, provider.models)),
                    onStopTests: _stopTests,
                    onStartSelecting: _startSelecting,
                    onToggleCheckAll: () => _toggleCheckAll(provider),
                    onTestChecked: () => _testChecked(provider),
                    onDeleteChecked: () => unawaited(_deleteChecked(provider)),
                    onStopSelecting: _stopSelecting,
                  ),
                Expanded(
                  child: ListView(
                    padding: const EdgeInsets.fromLTRB(16, 4, 16, 20),
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
                                style:
                                    SettingsControlStyles.sectionFilledButton(),
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
                          onSelectModel: (providerId, modelId) => _run(
                            () => widget.onSelectModel(providerId, modelId),
                          ),
                          expandedModelId: _expandedModelId,
                          onToggleSettings: _toggleModelSettings,
                          clients: widget.clients,
                          tests: _tests,
                          reveals: _reveals,
                          selecting: _selecting,
                          checkedModelIds: _checkedModelIds,
                          onToggleChecked: _toggleChecked,
                          onStartSelecting: _startSelecting,
                          onDeleteModel: (p, m) async {
                            setState(() {
                              _expandedModelId = null;
                            });
                            await _run(() => widget.onDeleteModel(p, m));
                          },
                          onTestModel: _testFromSettings,
                          onModelSettingsSaved: _refresh,
                          onModelSettingsFailed: _notifySaveFailure,
                        ),
                    ],
                  ),
                ),
              ],
            ),
            Positioned(
              left: 16,
              right: 16,
              bottom: 16,
              child: _DialogToast(key: _toastKey),
            ),
          ],
        ),
      ),
    );
  }
}

/// Whether two capability sets switch the same things on.
bool _sameCapabilities(
  core_proxy.ModelCapabilities left,
  core_proxy.ModelCapabilities right,
) {
  return left.toolCall == right.toolCall &&
      left.directImage == right.directImage &&
      left.directAudio == right.directAudio &&
      left.directVideo == right.directVideo;
}

/// The message of a Core error, or the error as written.
String _errorMessage(Object error) {
  return error is CoreLinkError ? error.message : '$error';
}

/// The green that marks a passed check, legible on the panel's surfaces.
Color _testPassedColor(bool isDark) {
  return isDark ? const Color(0xFF5CC98A) : const Color(0xFF1E8F52);
}

/// Marks a check blocked by credit or upstream limits, separately from failure.
Color _testRestrictedColor(bool isDark) {
  return isDark ? const Color(0xFFE9BB62) : const Color(0xFF986600);
}

/// The actions over a provider's models: test them all, or check some to test or delete.
class _ModelListToolbar extends StatelessWidget {
  const _ModelListToolbar({
    required this.selecting,
    required this.checkedCount,
    required this.modelCount,
    required this.batchRunning,
    required this.batchStopping,
    required this.batchDone,
    required this.batchTotal,
    required this.onTestAll,
    required this.onStopTests,
    required this.onStartSelecting,
    required this.onToggleCheckAll,
    required this.onTestChecked,
    required this.onDeleteChecked,
    required this.onStopSelecting,
  });

  final bool selecting;
  final int checkedCount;
  final int modelCount;
  final bool batchRunning;
  final bool batchStopping;
  final int batchDone;
  final int batchTotal;
  final VoidCallback onTestAll;
  final VoidCallback onStopTests;
  final VoidCallback onStartSelecting;
  final VoidCallback onToggleCheckAll;
  final VoidCallback onTestChecked;
  final VoidCallback onDeleteChecked;
  final VoidCallback onStopSelecting;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final labelStyle = theme.textTheme.labelLarge?.copyWith(
      fontWeight: FontWeight.w600,
      fontSize: 12.5,
      color: colorScheme.onSurfaceVariant,
    );
    final Widget content;
    if (selecting) {
      final anyChecked = checkedCount > 0;
      content = Row(
        key: const ValueKey<String>('selecting'),
        children: <Widget>[
          _ModernCheckbox(
            checked: checkedCount == modelCount,
            partial: anyChecked && checkedCount < modelCount,
            onTap: onToggleCheckAll,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              anyChecked
                  ? l10n.settingsModelCheckedCount(checkedCount)
                  : l10n.settingsModelCheckAll,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: labelStyle?.copyWith(color: colorScheme.onSurface),
            ),
          ),
          _CapsuleActionButton(
            label: l10n.settingsModelTestModel,
            icon: Icons.speed_rounded,
            onTap: anyChecked && !batchRunning ? onTestChecked : null,
          ),
          const SizedBox(width: 6),
          _CapsuleActionButton(
            label: l10n.delete,
            icon: Icons.delete_outline_rounded,
            destructive: true,
            onTap: anyChecked ? onDeleteChecked : null,
          ),
          const SizedBox(width: 6),
          _CapsuleActionButton(label: l10n.cancel, onTap: onStopSelecting),
        ],
      );
    } else {
      content = Row(
        key: const ValueKey<String>('browsing'),
        children: <Widget>[
          if (batchRunning) ...<Widget>[
            const SizedBox.square(
              dimension: 14,
              child: Center(child: M3LoadingIndicator(size: 14)),
            ),
            const SizedBox(width: 8),
            Flexible(
              child: Text(
                l10n.settingsModelTestProgress(batchDone, batchTotal),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: labelStyle,
              ),
            ),
          ],
          const Spacer(),
          if (batchRunning)
            _CapsuleActionButton(
              label: l10n.stop,
              icon: Icons.stop_rounded,
              onTap: batchStopping ? null : onStopTests,
            )
          else
            _CapsuleActionButton(
              label: l10n.settingsModelTestAll,
              icon: Icons.speed_rounded,
              onTap: onTestAll,
            ),
          const SizedBox(width: 6),
          _CapsuleActionButton(
            label: l10n.settingsModelSelect,
            icon: Icons.checklist_rounded,
            onTap: onStartSelecting,
          ),
        ],
      );
    }
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 6),
      child: SizedBox(
        height: 32,
        child: AnimatedSwitcher(
          duration: const Duration(milliseconds: 220),
          switchInCurve: Curves.easeOutCubic,
          switchOutCurve: Curves.easeInCubic,
          transitionBuilder: (child, animation) => FadeTransition(
            opacity: animation,
            child: SlideTransition(
              position: Tween<Offset>(
                begin: const Offset(0, 0.25),
                end: Offset.zero,
              ).animate(animation),
              child: child,
            ),
          ),
          child: content,
        ),
      ),
    );
  }
}

/// A short message that rises over the bottom of the list and leaves by itself.
class _DialogToast extends StatefulWidget {
  const _DialogToast({super.key});

  @override
  State<_DialogToast> createState() => _DialogToastState();
}

class _DialogToastState extends State<_DialogToast> {
  String? _message;
  bool _error = false;
  bool _visible = false;
  Timer? _timer;

  /// Shows [message] for a moment, replacing any message still showing.
  void show(String message, {bool error = false}) {
    _timer?.cancel();
    setState(() {
      _message = message;
      _error = error;
      _visible = true;
    });
    _timer = Timer(Duration(milliseconds: error ? 4200 : 2600), _dismiss);
  }

  void _dismiss() {
    _timer?.cancel();
    if (mounted) {
      setState(() {
        _visible = false;
      });
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final message = _message;
    if (message == null) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    // The toast sits on the inverse surface, so its accents come from the other theme.
    final accent = _error
        ? (isDark ? const Color(0xFFB3261E) : const Color(0xFFFFB4AB))
        : colorScheme.inversePrimary;
    return IgnorePointer(
      ignoring: !_visible,
      child: AnimatedOpacity(
        opacity: _visible ? 1 : 0,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOutCubic,
        child: AnimatedSlide(
          offset: _visible ? Offset.zero : const Offset(0, 0.4),
          duration: const Duration(milliseconds: 280),
          curve: Curves.easeOutCubic,
          child: Center(
            child: DecoratedBox(
              decoration: ShapeDecoration(
                color: colorScheme.inverseSurface,
                shape: const StadiumBorder(),
                shadows: <BoxShadow>[
                  BoxShadow(
                    color: Colors.black.withValues(alpha: isDark ? 0.45 : 0.22),
                    blurRadius: 18,
                    offset: const Offset(0, 6),
                  ),
                ],
              ),
              child: Material(
                type: MaterialType.transparency,
                child: InkWell(
                  customBorder: const StadiumBorder(),
                  onTap: _dismiss,
                  child: Padding(
                    padding: const EdgeInsets.fromLTRB(14, 9, 16, 9),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Icon(
                          _error
                              ? Icons.error_outline_rounded
                              : Icons.check_circle_outline_rounded,
                          size: 16,
                          color: accent,
                        ),
                        const SizedBox(width: 8),
                        Flexible(
                          child: Text(
                            message,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.bodyMedium?.copyWith(
                              color: colorScheme.onInverseSurface,
                              fontSize: 12.5,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Asks before deleting several models, naming the ones a function keeps.
class _DeleteModelsConfirmDialog extends StatelessWidget {
  const _DeleteModelsConfirmDialog({
    required this.count,
    required this.keptModelIds,
  });

  final int count;
  final List<String> keptModelIds;

  static Future<bool?> show({
    required BuildContext context,
    required int count,
    required List<String> keptModelIds,
  }) {
    return showDialog<bool>(
      context: context,
      builder: (context) =>
          _DeleteModelsConfirmDialog(count: count, keptModelIds: keptModelIds),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    return AlertDialog(
      title: Text(l10n.settingsModelDeleteModelsTitle(count)),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(l10n.settingsModelDeleteModelsMessage),
          if (keptModelIds.isNotEmpty) ...<Widget>[
            const SizedBox(height: 10),
            Text(
              l10n.settingsModelDeleteModelsKept(
                keptModelIds.length,
                keptModelIds.join(' · '),
              ),
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.delete),
        ),
      ],
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

class _ProviderModelList extends StatelessWidget {
  const _ProviderModelList({
    required this.provider,
    required this.summaries,
    required this.chatBinding,
    required this.onSelectModel,
    required this.expandedModelId,
    required this.onToggleSettings,
    required this.clients,
    required this.tests,
    required this.reveals,
    required this.selecting,
    required this.checkedModelIds,
    required this.onToggleChecked,
    required this.onStartSelecting,
    required this.onDeleteModel,
    required this.onTestModel,
    required this.onModelSettingsSaved,
    required this.onModelSettingsFailed,
  });

  final core_proxy.ProviderProfile provider;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding chatBinding;
  final void Function(String providerId, String modelId) onSelectModel;
  final String? expandedModelId;
  final void Function(String modelId) onToggleSettings;
  final GeneratedCoreProxyClients clients;
  final Map<String, _ModelTestResult> tests;
  final Map<String, int> reveals;
  final bool selecting;
  final Set<String> checkedModelIds;
  final void Function(String modelId) onToggleChecked;
  final void Function(String modelId) onStartSelecting;
  final Future<void> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onDeleteModel;
  final Future<core_proxy.ModelCapabilities?> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
    core_proxy.ModelCapabilities capabilities,
  )
  onTestModel;
  final VoidCallback onModelSettingsSaved;
  final ValueChanged<Object> onModelSettingsFailed;

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
            active:
                provider.id == chatBinding.providerId &&
                model.id == chatBinding.modelId,
            onSelect: onSelectModel,
            isExpanded: !selecting && expandedModelId == model.id,
            onToggleSettings: () => onToggleSettings(model.id),
            clients: clients,
            test: tests[_modelTestKey(provider.id, model.id)],
            revealToken: reveals[_modelTestKey(provider.id, model.id)] ?? 0,
            showTestStatus: tests.isNotEmpty,
            selecting: selecting,
            checked: checkedModelIds.contains(model.id),
            onToggleChecked: () => onToggleChecked(model.id),
            onStartSelecting: () => onStartSelecting(model.id),
            onDeleteModel: onDeleteModel,
            onTestModel: onTestModel,
            onModelSettingsSaved: onModelSettingsSaved,
            onModelSettingsFailed: onModelSettingsFailed,
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
    required this.active,
    required this.onSelect,
    required this.isExpanded,
    required this.onToggleSettings,
    required this.clients,
    required this.test,
    required this.revealToken,
    required this.showTestStatus,
    required this.selecting,
    required this.checked,
    required this.onToggleChecked,
    required this.onStartSelecting,
    required this.onDeleteModel,
    required this.onTestModel,
    required this.onModelSettingsSaved,
    required this.onModelSettingsFailed,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final core_proxy.ProviderModelSummary summary;

  /// Whether chats use this model.
  final bool active;
  final void Function(String providerId, String modelId) onSelect;
  final bool isExpanded;
  final VoidCallback onToggleSettings;
  final GeneratedCoreProxyClients clients;
  final _ModelTestResult? test;
  final int revealToken;
  final bool showTestStatus;
  final bool selecting;
  final bool checked;
  final VoidCallback onToggleChecked;
  final VoidCallback onStartSelecting;
  final Future<void> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onDeleteModel;
  final Future<core_proxy.ModelCapabilities?> Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
    core_proxy.ModelCapabilities capabilities,
  )
  onTestModel;
  final VoidCallback onModelSettingsSaved;
  final ValueChanged<Object> onModelSettingsFailed;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    // While picking models, the checked ones stand out instead of the active one.
    final highlighted = selecting ? checked : active;

    final cardBgColor = highlighted
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

    final borderColor = highlighted
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
        border: Border.all(color: borderColor, width: highlighted ? 1.0 : 0.8),
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
            active: active,
            highlighted: highlighted,
            onSelect: onSelect,
            isExpanded: isExpanded,
            onToggleSettings: onToggleSettings,
            test: test,
            revealToken: revealToken,
            showTestStatus: showTestStatus,
            selecting: selecting,
            checked: checked,
            onToggleChecked: onToggleChecked,
            onStartSelecting: onStartSelecting,
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
                        testing: !(test?.isFinished ?? true),
                        onTest: (capabilities) =>
                            onTestModel(provider, model, capabilities),
                        onDeleted: () => onDeleteModel(provider, model),
                        onSaved: onModelSettingsSaved,
                        onFailed: onModelSettingsFailed,
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
    required this.active,
    required this.highlighted,
    required this.onSelect,
    required this.isExpanded,
    required this.onToggleSettings,
    required this.test,
    required this.revealToken,
    required this.showTestStatus,
    required this.selecting,
    required this.checked,
    required this.onToggleChecked,
    required this.onStartSelecting,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final core_proxy.ProviderModelSummary summary;
  final bool active;
  final bool highlighted;
  final void Function(String providerId, String modelId) onSelect;
  final bool isExpanded;
  final VoidCallback onToggleSettings;
  final _ModelTestResult? test;
  final int revealToken;

  /// Whether the row keeps room for a test dot, so names stay aligned while any model has one.
  final bool showTestStatus;
  final bool selecting;
  final bool checked;
  final VoidCallback onToggleChecked;
  final VoidCallback onStartSelecting;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final contextLabel = _formatContextLength(summary.context.maxContextLength);

    return Material(
      color: highlighted
          ? colorScheme.primary.withValues(alpha: isDark ? 0.08 : 0.05)
          : Colors.transparent,
      child: InkWell(
        onTap: selecting ? onToggleChecked : onToggleSettings,
        onLongPress: selecting ? null : onStartSelecting,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 9),
          child: Row(
            children: <Widget>[
              AnimatedSize(
                duration: const Duration(milliseconds: 200),
                curve: Curves.easeOutCubic,
                alignment: AlignmentDirectional.centerStart,
                child: selecting
                    ? Padding(
                        padding: const EdgeInsetsDirectional.only(end: 12),
                        child: _ModernCheckbox(
                          checked: checked,
                          onTap: onToggleChecked,
                        ),
                      )
                    : const SizedBox.shrink(),
              ),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Row(
                      children: <Widget>[
                        if (showTestStatus) ...<Widget>[
                          _TestStatusDot(
                            result: test,
                            revealToken: revealToken,
                          ),
                          const SizedBox(width: 6),
                        ],
                        Flexible(
                          child: Text(
                            model.id,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.bodyMedium?.copyWith(
                              fontWeight: FontWeight.w600,
                              color: colorScheme.onSurface,
                            ),
                          ),
                        ),
                      ],
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
              if (active)
                SettingsActivePill(label: l10n.settingsModelCurrentActive)
              else if (!selecting)
                SettingsSetActiveButton(
                  label: l10n.settingsModelSetCurrentActive,
                  onPressed: () => onSelect(provider.id, model.id),
                ),
              if (!selecting) ...<Widget>[
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
                  constraints: const BoxConstraints(
                    minWidth: 28,
                    minHeight: 28,
                  ),
                  onPressed: onToggleSettings,
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// A model's latest test as a small dot: a spinner while it runs, green when every check
/// passed, red when one failed. Hovering or tapping a finished dot shows what the test found.
class _TestStatusDot extends StatefulWidget {
  const _TestStatusDot({required this.result, required this.revealToken});

  final _ModelTestResult? result;

  /// Changes when the result should show what it found without being asked.
  final int revealToken;

  @override
  State<_TestStatusDot> createState() => _TestStatusDotState();
}

class _TestStatusDotState extends State<_TestStatusDot>
    with SingleTickerProviderStateMixin {
  final OverlayPortalController _portal = OverlayPortalController();
  final Object _tapGroup = Object();
  late final AnimationController _motion;
  late final CurvedAnimation _curve;
  Timer? _hideTimer;
  bool _pinned = false;
  bool _hovered = false;

  bool get _finished => widget.result?.isFinished ?? false;

  @override
  void initState() {
    super.initState();
    // Built here rather than on first use: a dot that never opened its bubble would
    // otherwise build the controller inside dispose(), where its ticker cannot look up
    // the ticker mode of an element that is already deactivated.
    _motion = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 200),
      reverseDuration: const Duration(milliseconds: 130),
    );
    _curve = CurvedAnimation(
      parent: _motion,
      curve: Curves.easeOutCubic,
      reverseCurve: Curves.easeInCubic,
    );
  }

  @override
  void didUpdateWidget(covariant _TestStatusDot oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_finished) {
      // A new test replaces the result whose bubble is open, so the bubble goes with it.
      if (_portal.isShowing || _pinned) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted && !_finished) {
            _hideNow();
          }
        });
      }
    } else if (widget.revealToken != oldWidget.revealToken) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted && _finished) {
          _show(linger: true);
        }
      });
    }
  }

  @override
  void dispose() {
    _hideTimer?.cancel();
    _curve.dispose();
    _motion.dispose();
    super.dispose();
  }

  void _show({bool linger = false}) {
    _hideTimer?.cancel();
    if (!_portal.isShowing) {
      _portal.show();
    }
    _motion.forward();
    if (linger) {
      // A result that shows itself leaves on its own; a failure stays longer to be read.
      _hideTimer = Timer(
        widget.result?.phase == _ModelTestPhase.passed
            ? const Duration(milliseconds: 2400)
            : const Duration(milliseconds: 5200),
        _hideUnlessHeld,
      );
    }
  }

  void _hideUnlessHeld() {
    if (!_pinned && !_hovered) {
      _hide();
    }
  }

  void _hideSoon() {
    _hideTimer?.cancel();
    _hideTimer = Timer(const Duration(milliseconds: 180), _hideUnlessHeld);
  }

  void _hide() {
    _hideTimer?.cancel();
    _pinned = false;
    _motion.reverse().whenComplete(() {
      if (mounted && _motion.isDismissed && _portal.isShowing) {
        _portal.hide();
      }
    });
  }

  void _hideNow() {
    _hideTimer?.cancel();
    _pinned = false;
    _motion.value = 0;
    if (_portal.isShowing) {
      _portal.hide();
    }
  }

  void _toggle() {
    if (_pinned) {
      _hide();
    } else {
      _pinned = true;
      _show();
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final result = widget.result;
    final Widget mark;
    final String? label;
    switch (result?.phase) {
      case null:
        mark = const SizedBox.shrink();
        label = null;
      case _ModelTestPhase.queued:
        // Waiting for its turn in a batch: a quiet ring, not a spinner.
        mark = Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            border: Border.all(
              color: colorScheme.outline.withValues(alpha: 0.7),
              width: 1.2,
            ),
          ),
        );
        label = null;
      case _ModelTestPhase.testing:
        mark = SizedBox.square(
          dimension: 10,
          child: CircularProgressIndicator(
            strokeWidth: 1.6,
            color: colorScheme.primary,
          ),
        );
        label = l10n.settingsModelTestingConnection;
      case _ModelTestPhase.passed:
        mark = _dot(_testPassedColor(theme.brightness == Brightness.dark));
        label = l10n.settingsModelConnectionTestPassed;
      case _ModelTestPhase.failed:
        mark = _dot(colorScheme.error);
        label = l10n.settingsModelConnectionTestFailed;
      case _ModelTestPhase.restricted:
        mark = _dot(_testRestrictedColor(theme.brightness == Brightness.dark));
        label = l10n.settingsModelConnectionTestRestricted;
    }
    return Semantics(
      label: label,
      button: _finished,
      // The copy button's Tooltip also lays out an overlay. Its ancestors must
      // establish their transforms during layout, before either overlay paints.
      child: OverlayPortal.overlayChildLayoutBuilder(
        controller: _portal,
        overlayChildBuilder: _buildBubble,
        child: TapRegion(
          groupId: _tapGroup,
          onTapOutside: (_) {
            if (_portal.isShowing) {
              _hide();
            }
          },
          child: CallbackShortcuts(
            bindings: <ShortcutActivator, VoidCallback>{
              const SingleActivator(LogicalKeyboardKey.escape): () {
                if (_portal.isShowing) {
                  _hide();
                }
              },
            },
            child: MouseRegion(
              cursor: _finished ? SystemMouseCursors.click : MouseCursor.defer,
              onEnter: (_) {
                _hovered = true;
                if (_finished) {
                  _show();
                }
              },
              onExit: (_) {
                _hovered = false;
                _hideSoon();
              },
              child: InkResponse(
                onTap: _finished ? _toggle : null,
                radius: 14,
                child: SizedBox.square(
                  dimension: 18,
                  child: Center(
                    child: AnimatedSwitcher(
                      duration: const Duration(milliseconds: 220),
                      switchInCurve: Curves.easeOutBack,
                      switchOutCurve: Curves.easeInCubic,
                      transitionBuilder: (child, animation) => ScaleTransition(
                        scale: animation,
                        child: FadeTransition(opacity: animation, child: child),
                      ),
                      child: KeyedSubtree(
                        key: ValueKey<_ModelTestPhase?>(result?.phase),
                        child: mark,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _dot(Color color) {
    return Container(
      width: 8,
      height: 8,
      decoration: BoxDecoration(color: color, shape: BoxShape.circle),
    );
  }

  Widget _buildBubble(BuildContext context, OverlayChildLayoutInfo info) {
    final result = widget.result;
    if (result == null || !result.isFinished) {
      return const SizedBox.shrink();
    }
    final target = MatrixUtils.transformRect(
      info.childPaintTransform,
      Offset.zero & info.childSize,
    );
    final direction = Directionality.of(context);
    final below = target.top + 240 < info.overlaySize.height;
    return Positioned(
      left: direction == TextDirection.ltr ? target.left - 6 : null,
      right: direction == TextDirection.rtl
          ? info.overlaySize.width - target.right + 6
          : null,
      top: below ? target.bottom + 6 : null,
      bottom: below ? null : info.overlaySize.height - target.top + 6,
      child: TapRegion(
        groupId: _tapGroup,
        child: MouseRegion(
          onEnter: (_) {
            _hovered = true;
            _hideTimer?.cancel();
          },
          onExit: (_) {
            _hovered = false;
            _hideSoon();
          },
          child: FadeTransition(
            opacity: _curve,
            child: ScaleTransition(
              scale: Tween<double>(begin: 0.9, end: 1).animate(_curve),
              alignment: below
                  ? AlignmentDirectional.topStart.resolve(
                      Directionality.of(context),
                    )
                  : AlignmentDirectional.bottomStart.resolve(
                      Directionality.of(context),
                    ),
              child: _TestResultBubble(result: result),
            ),
          ),
        ),
      ),
    );
  }
}

/// What a model's test found: each check, why one failed, and what the test changed.
class _TestResultBubble extends StatefulWidget {
  const _TestResultBubble({required this.result});

  final _ModelTestResult result;

  @override
  State<_TestResultBubble> createState() => _TestResultBubbleState();
}

class _TestResultBubbleState extends State<_TestResultBubble> {
  bool _copied = false;
  Timer? _copiedTimer;

  @override
  void dispose() {
    _copiedTimer?.cancel();
    super.dispose();
  }

  Future<void> _copy(String text) async {
    await Clipboard.setData(ClipboardData(text: text));
    if (!mounted) {
      return;
    }
    setState(() {
      _copied = true;
    });
    _copiedTimer?.cancel();
    _copiedTimer = Timer(const Duration(milliseconds: 1400), () {
      if (mounted) {
        setState(() {
          _copied = false;
        });
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final result = widget.result;
    final report = result.report;
    final passed = result.phase == _ModelTestPhase.passed;
    final restricted = result.phase == _ModelTestPhase.restricted;
    final statusColor = passed
        ? _testPassedColor(isDark)
        : restricted
        ? _testRestrictedColor(isDark)
        : colorScheme.error;
    final failures = <String>[
      if (result.error != null) result.error!,
      if (report != null)
        for (final item in report.items)
          if (!item.success && item.error != null)
            '${_connectionTestTypeLabel(l10n, item.type)}: ${item.error}',
    ];
    final title = passed
        ? l10n.settingsModelConnectionTestPassed
        : restricted
        ? l10n.settingsModelConnectionTestRestricted
        : (report == null
              ? l10n.settingsModelTestCouldNotRun
              : l10n.settingsModelConnectionTestFailed);
    final detailStyle = theme.textTheme.bodySmall?.copyWith(
      fontSize: 11,
      height: 1.35,
      color: colorScheme.onSurfaceVariant,
    );
    return Material(
      type: MaterialType.transparency,
      child: Container(
        constraints: const BoxConstraints(minWidth: 180, maxWidth: 300),
        padding: const EdgeInsetsDirectional.fromSTEB(12, 9, 6, 10),
        decoration: BoxDecoration(
          color: isDark
              ? colorScheme.surfaceContainerHigh
              : colorScheme.surface,
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
            color: colorScheme.outlineVariant.withValues(
              alpha: isDark ? 0.5 : 0.7,
            ),
            width: 0.8,
          ),
          boxShadow: <BoxShadow>[
            BoxShadow(
              color: Colors.black.withValues(alpha: isDark ? 0.42 : 0.14),
              blurRadius: 18,
              offset: const Offset(0, 6),
            ),
          ],
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(
                  passed
                      ? Icons.check_circle_rounded
                      : restricted
                      ? Icons.pause_circle_outline_rounded
                      : Icons.error_rounded,
                  size: 15,
                  color: statusColor,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    title,
                    style: theme.textTheme.labelLarge?.copyWith(
                      fontSize: 12.5,
                      fontWeight: FontWeight.w700,
                      color: colorScheme.onSurface,
                    ),
                  ),
                ),
                if (failures.isNotEmpty)
                  IconButton(
                    tooltip: l10n.settingsModelCopyError,
                    onPressed: () => unawaited(_copy(failures.join('\n'))),
                    icon: AnimatedSwitcher(
                      duration: const Duration(milliseconds: 160),
                      child: Icon(
                        _copied ? Icons.check_rounded : Icons.copy_rounded,
                        key: ValueKey<bool>(_copied),
                        size: 14,
                        color: _copied
                            ? _testPassedColor(isDark)
                            : colorScheme.onSurfaceVariant,
                      ),
                    ),
                    visualDensity: VisualDensity.compact,
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(
                      minWidth: 26,
                      minHeight: 26,
                    ),
                  )
                else
                  const SizedBox(width: 6),
              ],
            ),
            if (report != null)
              for (final item in report.items)
                Padding(
                  padding: const EdgeInsetsDirectional.only(top: 6, end: 6),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Padding(
                        padding: const EdgeInsets.only(top: 1),
                        child: Icon(
                          item.success
                              ? Icons.check_rounded
                              : connectionTestRestriction(item.error) != null
                              ? Icons.pause_rounded
                              : Icons.close_rounded,
                          size: 13,
                          color: item.success
                              ? _testPassedColor(isDark)
                              : connectionTestRestriction(item.error) != null
                              ? _testRestrictedColor(isDark)
                              : colorScheme.error,
                        ),
                      ),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            Text(
                              _connectionTestItemLabel(
                                l10n,
                                item.type,
                                item.success ? null : item.error,
                              ),
                              style: theme.textTheme.bodySmall?.copyWith(
                                fontSize: 11.5,
                                color: colorScheme.onSurface,
                              ),
                            ),
                            if (!item.success && item.error != null)
                              Text(
                                item.error!,
                                maxLines: 4,
                                overflow: TextOverflow.ellipsis,
                                style: detailStyle,
                              ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
            if (result.error != null)
              Padding(
                padding: const EdgeInsetsDirectional.only(top: 6, end: 6),
                child: Text(
                  result.error!,
                  maxLines: 5,
                  overflow: TextOverflow.ellipsis,
                  style: detailStyle,
                ),
              ),
            if (result.note != null)
              Padding(
                padding: const EdgeInsetsDirectional.only(top: 8, end: 6),
                child: Text(result.note!, style: detailStyle),
              ),
          ],
        ),
      ),
    );
  }
}

class _ModelInlineSettingsView extends StatefulWidget {
  const _ModelInlineSettingsView({
    required this.provider,
    required this.model,
    required this.clients,
    required this.testing,
    required this.onTest,
    required this.onDeleted,
    required this.onSaved,
    required this.onFailed,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final GeneratedCoreProxyClients clients;

  /// Whether the model's connection test is running.
  final bool testing;
  final Future<core_proxy.ModelCapabilities?> Function(
    core_proxy.ModelCapabilities capabilities,
  )
  onTest;
  final VoidCallback onDeleted;
  final VoidCallback onSaved;
  final ValueChanged<Object> onFailed;

  @override
  State<_ModelInlineSettingsView> createState() =>
      _ModelInlineSettingsViewState();
}

/// A configured model's resolved settings and its thinking levels.
class _ModelInlineSettingsData {
  const _ModelInlineSettingsData({
    required this.config,
    required this.thinking,
  });

  final core_proxy.ResolvedModelConfig config;
  final core_proxy.ModelThinkingSettings thinking;
}

class _ModelInlineSettingsViewState extends State<_ModelInlineSettingsView> {
  Future<_ModelInlineSettingsData>? _dataFuture;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  void _load() {
    _loadGeneration += 1;
    _dataFuture = _fetch();
  }

  Future<_ModelInlineSettingsData> _fetch() async {
    final manager = widget.clients.preferencesModelConfigManager;
    final config = await manager.getResolvedModelConfig(
      providerId: widget.provider.id,
      modelId: widget.model.id,
    );
    final thinking = await manager.getThinkingSettingsForModel(
      providerId: widget.provider.id,
      modelId: widget.model.id,
    );
    return _ModelInlineSettingsData(config: config, thinking: thinking);
  }

  /// Gives back the catalog's values for everything the user set on this model.
  Future<void> _restoreDefaults() async {
    try {
      await widget.clients.preferencesModelConfigManager
          .clearModelSpecOverrides(
            providerId: widget.provider.id,
            modelId: widget.model.id,
          );
    } catch (error) {
      widget.onFailed(error);
      return;
    }
    if (!mounted) {
      return;
    }
    // The catalog's values arrive in a fresh form.
    setState(_load);
    widget.onSaved();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final l10n = AppLocalizations.of(context)!;
    return Container(
      padding: const EdgeInsets.fromLTRB(12, 12, 12, 12),
      decoration: BoxDecoration(
        color: isDark
            ? Colors.black.withValues(alpha: 0.15)
            : colorScheme.surfaceContainerLow,
      ),
      child: FutureBuilder<_ModelInlineSettingsData>(
        future: _dataFuture,
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
          final data = snapshot.data!;
          return _ModelInlineSettingsForm(
            key: ValueKey<int>(_loadGeneration),
            provider: widget.provider,
            model: widget.model,
            clients: widget.clients,
            initialConfig: data.config,
            initialThinking: data.thinking,
            testing: widget.testing,
            onTest: widget.onTest,
            onRestoreDefaults: () => unawaited(_restoreDefaults()),
            onDeleted: widget.onDeleted,
            onSaved: widget.onSaved,
            onFailed: widget.onFailed,
          );
        },
      ),
    );
  }
}

/// What the more-actions menu of a model's settings can do.
enum _ModelCardAction {
  describeThinking,
  useCatalogThinking,
  restoreDefaults,
  delete,
}

/// A model's settings, each saved as soon as it changes.
class _ModelInlineSettingsForm extends StatefulWidget {
  const _ModelInlineSettingsForm({
    super.key,
    required this.provider,
    required this.model,
    required this.clients,
    required this.initialConfig,
    required this.initialThinking,
    required this.testing,
    required this.onTest,
    required this.onRestoreDefaults,
    required this.onDeleted,
    required this.onSaved,
    required this.onFailed,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final GeneratedCoreProxyClients clients;
  final core_proxy.ResolvedModelConfig initialConfig;
  final core_proxy.ModelThinkingSettings initialThinking;
  final bool testing;
  final Future<core_proxy.ModelCapabilities?> Function(
    core_proxy.ModelCapabilities capabilities,
  )
  onTest;
  final VoidCallback onRestoreDefaults;
  final VoidCallback onDeleted;
  final VoidCallback onSaved;
  final ValueChanged<Object> onFailed;

  @override
  State<_ModelInlineSettingsForm> createState() =>
      _ModelInlineSettingsFormState();
}

class _ModelInlineSettingsFormState extends State<_ModelInlineSettingsForm> {
  /// Below this width the labels sit above their controls.
  static const double _wideLayoutWidth = 520;

  late bool _toolCall;
  late bool _directImage;
  late bool _directAudio;
  late bool _directVideo;
  late bool _claude1hPromptCache;
  late List<core_proxy.ModelBuiltinTool> _builtinTools;
  late core_proxy.ModelThinkingSettings _thinking;

  /// The level just picked, shown while it saves.
  core_proxy.ThinkingLevel? _pendingLevel;
  int _thinkingChanges = 0;
  late final TextEditingController _contextController;
  late final TextEditingController _summaryController;
  final FocusNode _contextFocus = FocusNode();
  final FocusNode _summaryFocus = FocusNode();
  late String _savedContext;
  late String _savedSummary;
  late double _summaryRatio;
  bool _contextInvalid = false;
  bool _summaryInvalid = false;

  /// Saves run one after another, so a later change never lands before an earlier one.
  Future<void> _writes = Future<void>.value();

  @override
  void initState() {
    super.initState();
    final config = widget.initialConfig;
    _applyCapabilities(config.capabilities);
    _claude1hPromptCache = config.request.enableClaude1HPromptCache;
    _builtinTools = config.builtinTools;
    _thinking = widget.initialThinking;
    final rawContext = config.context.maxContextLength;
    final normalizedContext = rawContext >= 10000
        ? (rawContext / 1000).roundToDouble()
        : rawContext;
    _savedContext = normalizedContext > 0
        ? normalizedContext.toStringAsFixed(0)
        : '200';
    _contextController = TextEditingController(text: _savedContext);
    final summary = config.summary;
    final rawRatio = summary.summaryTokenThreshold;
    _summaryRatio = rawRatio <= 0
        ? 0.7
        : (rawRatio <= 1.0 ? rawRatio : (rawRatio / 100).clamp(0.01, 1.0));
    // A summary that is off reads 0, which is how it is turned off.
    _savedSummary = summary.enableSummary
        ? (_summaryRatio * 100).round().toString()
        : '0';
    _summaryController = TextEditingController(text: _savedSummary);
    _contextFocus.addListener(_commitContextOnBlur);
    _summaryFocus.addListener(_commitSummaryOnBlur);
  }

  /// Saves the context window when the user leaves its field.
  void _commitContextOnBlur() {
    if (!_contextFocus.hasFocus) {
      _commitContext();
    }
  }

  /// Saves the summary threshold when the user leaves its field.
  void _commitSummaryOnBlur() {
    if (!_summaryFocus.hasFocus) {
      _commitSummary();
    }
  }

  @override
  void dispose() {
    // Releasing a focused field must not call back into a state that is going away.
    _contextFocus.removeListener(_commitContextOnBlur);
    _summaryFocus.removeListener(_commitSummaryOnBlur);
    // Typing without leaving the field still counts when the settings close.
    _commitContext(disposing: true);
    _commitSummary(disposing: true);
    _contextFocus.dispose();
    _summaryFocus.dispose();
    _contextController.dispose();
    _summaryController.dispose();
    super.dispose();
  }

  bool get _supportsClaudePromptCache => <String>{
    'ANTHROPIC',
    'ANTHROPIC_GENERIC',
  }.contains(widget.provider.providerTypeId.trim().toUpperCase());

  core_proxy.ModelCapabilities _currentCapabilities() {
    return core_proxy.ModelCapabilities(
      directImage: _directImage,
      directAudio: _directAudio,
      directVideo: _directVideo,
      toolCall: _toolCall,
    );
  }

  void _applyCapabilities(core_proxy.ModelCapabilities capabilities) {
    _toolCall = capabilities.toolCall;
    _directImage = capabilities.directImage;
    _directAudio = capabilities.directAudio;
    _directVideo = capabilities.directVideo;
  }

  /// Saves one change after those before it; when saving fails, [undo] puts it back.
  void _persist(Future<Object?> Function() write, {VoidCallback? undo}) {
    final onSaved = widget.onSaved;
    final onFailed = widget.onFailed;
    _writes = _writes.then((_) async {
      try {
        await write();
        onSaved();
      } catch (error) {
        if (mounted && undo != null) {
          setState(undo);
        }
        onFailed(error);
      }
    });
  }

  void _changeCapabilities(VoidCallback change) {
    final before = _currentCapabilities();
    setState(change);
    final after = _currentCapabilities();
    _persist(
      () => widget.clients.preferencesModelConfigManager
          .updateCapabilitiesForModel(
            providerId: widget.provider.id,
            modelId: widget.model.id,
            capabilities: after,
          ),
      undo: () => _applyCapabilities(before),
    );
  }

  void _setClaudeCache(bool enabled) {
    setState(() {
      _claude1hPromptCache = enabled;
    });
    _persist(
      () => widget.clients.preferencesModelConfigManager.updateRequestForModel(
        providerId: widget.provider.id,
        modelId: widget.model.id,
        request: core_proxy.ModelRequestSpec(
          supportsStructuredTools:
              widget.initialConfig.request.supportsStructuredTools,
          enableClaude1HPromptCache: enabled,
        ),
      ),
      undo: () {
        _claude1hPromptCache = !enabled;
      },
    );
  }

  void _setBuiltinToolEnabled(int index, bool enabled) {
    final before = _builtinTools;
    setState(() {
      _builtinTools = <core_proxy.ModelBuiltinTool>[
        for (var i = 0; i < before.length; i++)
          if (i == index)
            core_proxy.ModelBuiltinTool(
              toolType: before[i].toolType,
              displayName: before[i].displayName,
              enabled: enabled,
              requestFormat: before[i].requestFormat,
              exclusivity: before[i].exclusivity,
              config: before[i].config,
            )
          else
            before[i],
      ];
    });
    final after = _builtinTools;
    _persist(
      () => widget.clients.preferencesModelConfigManager
          .updateBuiltinToolsForModel(
            providerId: widget.provider.id,
            modelId: widget.model.id,
            builtinTools: after,
          ),
      undo: () {
        _builtinTools = before;
      },
    );
  }

  /// Saves the context window once the user leaves the field. An entry that is not a
  /// positive number is marked and left unsaved.
  void _commitContext({bool disposing = false}) {
    final text = _contextController.text.trim();
    final value = double.tryParse(text);
    if (value == null || value <= 0) {
      if (!disposing && !_contextInvalid) {
        setState(() {
          _contextInvalid = true;
        });
      }
      return;
    }
    if (!disposing && _contextInvalid) {
      setState(() {
        _contextInvalid = false;
      });
    }
    if (text == _savedContext) {
      return;
    }
    final previous = _savedContext;
    _savedContext = text;
    _persist(
      () => widget.clients.preferencesModelConfigManager.updateContextForModel(
        providerId: widget.provider.id,
        modelId: widget.model.id,
        context: core_proxy.ModelContextSpec(maxContextLength: value),
      ),
      undo: () {
        _savedContext = previous;
        _contextController.text = previous;
      },
    );
  }

  /// Saves the share of the context that starts a summary once the user leaves the field;
  /// 0 turns summaries off. An entry outside 0 to 100 is marked and left unsaved.
  void _commitSummary({bool disposing = false}) {
    final text = _summaryController.text.trim();
    final value = double.tryParse(text);
    if (value == null || value < 0 || value > 100) {
      if (!disposing && !_summaryInvalid) {
        setState(() {
          _summaryInvalid = true;
        });
      }
      return;
    }
    if (!disposing && _summaryInvalid) {
      setState(() {
        _summaryInvalid = false;
      });
    }
    if (text == _savedSummary) {
      return;
    }
    final previousText = _savedSummary;
    final previousRatio = _summaryRatio;
    final enabled = value > 0;
    if (enabled) {
      // Values up to 1 are read as a share, larger ones as a percentage.
      _summaryRatio = (value > 1 ? value / 100 : value).clamp(0.01, 1.0);
    }
    _savedSummary = text;
    final ratio = _summaryRatio;
    _persist(
      () => widget.clients.preferencesModelConfigManager.updateSummaryForModel(
        providerId: widget.provider.id,
        modelId: widget.model.id,
        summary: core_proxy.ModelSummarySettings(
          enableSummary: enabled,
          summaryTokenThreshold: ratio,
        ),
      ),
      undo: () {
        _savedSummary = previousText;
        _summaryRatio = previousRatio;
        _summaryController.text = previousText;
      },
    );
  }

  /// Applies a thinking change; the newest one decides what the control shows.
  void _changeThinking(
    Future<core_proxy.ModelThinkingSettings> Function() change,
  ) {
    final ticket = ++_thinkingChanges;
    final onSaved = widget.onSaved;
    final onFailed = widget.onFailed;
    _writes = _writes.then((_) async {
      try {
        final settings = await change();
        if (mounted) {
          setState(() {
            _thinking = settings;
            if (ticket == _thinkingChanges) {
              _pendingLevel = null;
            }
          });
        }
        onSaved();
      } catch (error) {
        if (mounted && ticket == _thinkingChanges) {
          setState(() {
            _pendingLevel = null;
          });
        }
        onFailed(error);
      }
    });
  }

  void _setThinkingLevel(core_proxy.ThinkingLevel level) {
    setState(() {
      _pendingLevel = level;
    });
    _changeThinking(
      () =>
          widget.clients.preferencesModelConfigManager.setThinkingLevelForModel(
            providerId: widget.provider.id,
            modelId: widget.model.id,
            level: level,
          ),
    );
  }

  Future<void> _describeThinking() async {
    final spec = await _ThinkingDescribeDialog.show(
      context: context,
      initial: _thinking.spec,
    );
    if (spec == null || !mounted) {
      return;
    }
    _changeThinking(
      () => widget.clients.preferencesModelConfigManager
          .setThinkingOverrideForModel(
            providerId: widget.provider.id,
            modelId: widget.model.id,
            spec: spec,
          ),
    );
  }

  void _useCatalogThinking() {
    _changeThinking(
      () => widget.clients.preferencesModelConfigManager
          .setThinkingOverrideForModel(
            providerId: widget.provider.id,
            modelId: widget.model.id,
            spec: null,
          ),
    );
  }

  /// Tests the model as saved and shows the capabilities the test leaves it with.
  Future<void> _test() async {
    await _writes;
    if (!mounted) {
      return;
    }
    final found = await widget.onTest(_currentCapabilities());
    if (found == null || !mounted) {
      return;
    }
    setState(() {
      _applyCapabilities(found);
    });
  }

  void _onAction(_ModelCardAction action) {
    switch (action) {
      case _ModelCardAction.describeThinking:
        unawaited(_describeThinking());
      case _ModelCardAction.useCatalogThinking:
        _useCatalogThinking();
      case _ModelCardAction.restoreDefaults:
        // Changes still saving would otherwise land on top of the restored values.
        unawaited(
          _writes.then((_) {
            if (mounted) {
              widget.onRestoreDefaults();
            }
          }),
        );
      case _ModelCardAction.delete:
        widget.onDeleted();
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= _wideLayoutWidth;
        final rows = <(String, Widget)>[
          (l10n.settingsModelCapabilities, _capabilityChips(l10n)),
          if (_builtinTools.isNotEmpty)
            (l10n.settingsModelBuiltinTools, _builtinToolChips(l10n)),
          (
            l10n.settingsModelThinking,
            _ModelThinkingControl(
              settings: _thinking,
              selected: _pendingLevel ?? _thinking.current,
              expand: !wide,
              onSelect: _setThinkingLevel,
              onDescribe: () => unawaited(_describeThinking()),
            ),
          ),
        ];
        final actions = _actions(l10n);
        return wide
            ? _wideLayout(context, l10n, rows, actions)
            : _narrowLayout(l10n, rows, actions);
      },
    );
  }

  /// Labels in a column beside their controls, the actions closing the last row.
  Widget _wideLayout(
    BuildContext context,
    AppLocalizations l10n,
    List<(String, Widget)> rows,
    Widget actions,
  ) {
    final labelTop = (_settingsControlHeight(context) - 16) / 2;
    TableRow row(String label, Widget control, {bool last = false}) {
      final bottom = last ? 0.0 : 10.0;
      return TableRow(
        children: <Widget>[
          Padding(
            padding: EdgeInsetsDirectional.only(
              top: labelTop,
              end: 16,
              bottom: bottom,
            ),
            child: _SettingsRowLabel(label),
          ),
          Padding(
            padding: EdgeInsets.only(bottom: bottom),
            child: control,
          ),
        ],
      );
    }

    return Table(
      columnWidths: const <int, TableColumnWidth>{
        0: IntrinsicColumnWidth(),
        1: FlexColumnWidth(),
      },
      children: <TableRow>[
        for (final (label, control) in rows) row(label, control),
        row(
          l10n.settingsModelContext,
          Row(
            children: <Widget>[
              ..._contextFields(l10n, labelContext: false, expand: false),
              const Spacer(),
              actions,
            ],
          ),
          last: true,
        ),
      ],
    );
  }

  /// Each label above its control, the actions on a line of their own.
  Widget _narrowLayout(
    AppLocalizations l10n,
    List<(String, Widget)> rows,
    Widget actions,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        for (final (label, control) in rows) ...<Widget>[
          _SettingsRowLabel(label),
          const SizedBox(height: 6),
          control,
          const SizedBox(height: 12),
        ],
        Row(children: _contextFields(l10n, labelContext: true, expand: true)),
        const SizedBox(height: 10),
        Align(alignment: AlignmentDirectional.centerEnd, child: actions),
      ],
    );
  }

  List<Widget> _contextFields(
    AppLocalizations l10n, {
    required bool labelContext,
    required bool expand,
  }) {
    Widget field({
      required TextEditingController controller,
      required FocusNode focusNode,
      required String suffix,
      required bool invalid,
      required VoidCallback commit,
      required double width,
    }) {
      final input = _ModernNumberField(
        controller: controller,
        focusNode: focusNode,
        suffix: suffix,
        invalid: invalid,
        onSubmitted: (_) => commit(),
      );
      return expand
          ? Expanded(child: input)
          : SizedBox(width: width, child: input);
    }

    return <Widget>[
      if (labelContext) ...<Widget>[
        _SettingsRowLabel(l10n.settingsModelContext),
        const SizedBox(width: 8),
      ],
      field(
        controller: _contextController,
        focusNode: _contextFocus,
        suffix: 'K',
        invalid: _contextInvalid,
        commit: _commitContext,
        width: 96,
      ),
      const SizedBox(width: 16),
      Tooltip(
        message: l10n.settingsModelSummaryThresholdDescription,
        child: _SettingsRowLabel(l10n.settingsModelSummaryThreshold),
      ),
      const SizedBox(width: 8),
      field(
        controller: _summaryController,
        focusNode: _summaryFocus,
        suffix: '%',
        invalid: _summaryInvalid,
        commit: _commitSummary,
        width: 76,
      ),
    ];
  }

  Widget _capabilityChips(AppLocalizations l10n) {
    return Wrap(
      spacing: 6,
      runSpacing: 6,
      children: <Widget>[
        _CapabilityToggleChip(
          title: l10n.settingsModelCapabilityToolCall,
          tooltip: l10n.settingsModelToolCallDescription,
          icon: Icons.build_outlined,
          checked: _toolCall,
          onChanged: (value) => _changeCapabilities(() {
            _toolCall = value;
          }),
        ),
        _CapabilityToggleChip(
          title: l10n.settingsModelCapabilityImage,
          tooltip: l10n.settingsModelDirectImageDescription,
          icon: Icons.image_outlined,
          checked: _directImage,
          onChanged: (value) => _changeCapabilities(() {
            _directImage = value;
          }),
        ),
        _CapabilityToggleChip(
          title: l10n.settingsModelCapabilityAudio,
          tooltip: l10n.settingsModelDirectAudioDescription,
          icon: Icons.graphic_eq,
          checked: _directAudio,
          onChanged: (value) => _changeCapabilities(() {
            _directAudio = value;
          }),
        ),
        _CapabilityToggleChip(
          title: l10n.settingsModelCapabilityVideo,
          tooltip: l10n.settingsModelDirectVideoDescription,
          icon: Icons.videocam_outlined,
          checked: _directVideo,
          onChanged: (value) => _changeCapabilities(() {
            _directVideo = value;
          }),
        ),
        if (_supportsClaudePromptCache)
          _CapabilityToggleChip(
            title: l10n.settingsModelClaudeCache,
            tooltip: l10n.settingsModelClaudeCacheDescription,
            icon: Icons.cached_outlined,
            checked: _claude1hPromptCache,
            onChanged: _setClaudeCache,
          ),
      ],
    );
  }

  Widget _builtinToolChips(AppLocalizations l10n) {
    return Wrap(
      spacing: 6,
      runSpacing: 6,
      children: <Widget>[
        for (var index = 0; index < _builtinTools.length; index++)
          _CapabilityToggleChip(
            title: _builtinTools[index].displayName,
            tooltip: _builtinToolSubtitle(l10n, _builtinTools[index]),
            icon: Icons.extension_outlined,
            checked: _builtinTools[index].enabled,
            onChanged: (value) => _setBuiltinToolEnabled(index, value),
          ),
      ],
    );
  }

  Widget _actions(AppLocalizations l10n) {
    final describedThinking =
        _thinking.source == core_proxy.ThinkingSpecSource.declared;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        _TestModelPillButton(
          testing: widget.testing,
          onPressed: widget.testing ? null : () => unawaited(_test()),
          label: l10n.settingsModelTestModel,
        ),
        const SizedBox(width: 2),
        _ModelCardMenu(
          canUseCatalogThinking: describedThinking,
          canRestoreDefaults:
              describedThinking || _modelHasSpecOverrides(widget.model),
          onSelected: _onAction,
        ),
      ],
    );
  }
}

/// Height of the compact controls in a model's settings, roomier where people tap.
double _settingsControlHeight(BuildContext context) {
  return 36 + Theme.of(context).visualDensity.vertical * 4;
}

/// The name of one row of a model's settings.
class _SettingsRowLabel extends StatelessWidget {
  const _SettingsRowLabel(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Text(
      text,
      maxLines: 1,
      style: theme.textTheme.labelMedium?.copyWith(
        fontSize: 12,
        height: 16 / 12,
        fontWeight: FontWeight.w600,
        color: theme.colorScheme.onSurfaceVariant,
      ),
    );
  }
}

/// The more-actions button of a model's settings.
class _ModelCardMenu extends StatelessWidget {
  const _ModelCardMenu({
    required this.canUseCatalogThinking,
    required this.canRestoreDefaults,
    required this.onSelected,
  });

  final bool canUseCatalogThinking;
  final bool canRestoreDefaults;
  final ValueChanged<_ModelCardAction> onSelected;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    PopupMenuItem<_ModelCardAction> item(
      _ModelCardAction action,
      IconData icon,
      String label, {
      bool destructive = false,
    }) {
      final color = destructive ? colorScheme.error : colorScheme.onSurface;
      return PopupMenuItem<_ModelCardAction>(
        value: action,
        height: 40,
        child: Row(
          children: <Widget>[
            Icon(
              icon,
              size: 17,
              color: destructive ? color : colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 10),
            Text(
              label,
              style: theme.textTheme.bodyMedium?.copyWith(
                fontSize: 13,
                color: color,
              ),
            ),
          ],
        ),
      );
    }

    return PopupMenuButton<_ModelCardAction>(
      tooltip: l10n.settingsModelMoreActions,
      position: PopupMenuPosition.under,
      padding: EdgeInsets.zero,
      // As small as the test button beside it, so the row keeps its height.
      style: IconButton.styleFrom(
        minimumSize: const Size(30, 30),
        padding: EdgeInsets.zero,
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        visualDensity: VisualDensity.compact,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      ),
      icon: Icon(
        Icons.more_horiz_rounded,
        size: 18,
        color: colorScheme.onSurfaceVariant,
      ),
      onSelected: onSelected,
      itemBuilder: (context) => <PopupMenuEntry<_ModelCardAction>>[
        item(
          _ModelCardAction.describeThinking,
          Icons.psychology_outlined,
          l10n.settingsModelDescribeThinking,
        ),
        if (canUseCatalogThinking)
          item(
            _ModelCardAction.useCatalogThinking,
            Icons.undo_rounded,
            l10n.settingsModelThinkingUseCatalog,
          ),
        if (canRestoreDefaults)
          item(
            _ModelCardAction.restoreDefaults,
            Icons.restart_alt_rounded,
            l10n.settingsModelRestoreDefaults,
          ),
        const PopupMenuDivider(),
        item(
          _ModelCardAction.delete,
          Icons.delete_outline_rounded,
          l10n.settingsModelDeleteModel,
          destructive: true,
        ),
      ],
    );
  }
}

/// Returns whether the user set any pricing, context, capability, tool-format or thinking
/// value.
bool _modelHasSpecOverrides(core_proxy.ModelProfile model) {
  final capabilities = model.capabilitiesOverride;
  return model.pricingOverride != null ||
      model.contextOverride != null ||
      capabilities.directImage != null ||
      capabilities.directAudio != null ||
      capabilities.directVideo != null ||
      capabilities.toolCall != null ||
      model.requestOverride.supportsStructuredTools != null ||
      model.thinkingOverride != null;
}

/// A model's thinking: the levels it offers as one segmented control, or why it has none.
class _ModelThinkingControl extends StatelessWidget {
  const _ModelThinkingControl({
    required this.settings,
    required this.selected,
    required this.expand,
    required this.onSelect,
    required this.onDescribe,
  });

  final core_proxy.ModelThinkingSettings settings;

  /// The level shown as chosen, which runs ahead of [settings] while a change saves.
  final core_proxy.ThinkingLevel? selected;

  /// Whether the levels share the whole width rather than keep a compact size.
  final bool expand;
  final ValueChanged<core_proxy.ThinkingLevel> onSelect;
  final VoidCallback onDescribe;

  /// Explains the levels where that helps: who described them, or that the level carries
  /// over from the one set last.
  String? _hint(AppLocalizations l10n) {
    final lines = <String>[
      if (settings.source == core_proxy.ThinkingSpecSource.declared)
        l10n.settingsModelThinkingSourceDeclared,
      if (settings.source == core_proxy.ThinkingSpecSource.localRuntime)
        l10n.settingsModelThinkingSourceLocalRuntime,
      if (!settings.chosen) l10n.settingsModelThinkingCarried,
    ];
    return lines.isEmpty ? null : lines.join('\n');
  }

  /// Says why a model offers no levels.
  String _statusText(AppLocalizations l10n) {
    if (!settings.spec.supported) {
      return settings.source == core_proxy.ThinkingSpecSource.unknown
          ? l10n.settingsModelThinkingUnknown
          : l10n.settingsModelThinkingNotSupported;
    }
    if (thinkingAlwaysOn(settings.spec)) {
      return l10n.thinkingAlwaysOn;
    }
    return l10n.settingsModelThinkingNotAdjustable;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    if (settings.levels.isEmpty) {
      return ConstrainedBox(
        constraints: BoxConstraints(minHeight: _settingsControlHeight(context)),
        child: Row(
          children: <Widget>[
            Flexible(
              child: Text(
                _statusText(l10n),
                style: theme.textTheme.bodySmall?.copyWith(
                  fontSize: 11.5,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            if (settings.source ==
                core_proxy.ThinkingSpecSource.unknown) ...<Widget>[
              const SizedBox(width: 4),
              TextButton(
                onPressed: onDescribe,
                style: TextButton.styleFrom(
                  visualDensity: VisualDensity.compact,
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  minimumSize: const Size(0, 28),
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  textStyle: theme.textTheme.labelSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                child: Text(l10n.settingsModelThinkingDescribe),
              ),
            ],
          ],
        ),
      );
    }
    final segments = _ThinkingSegments(
      levels: settings.levels,
      selected: selected,
      expand: expand,
      onSelect: onSelect,
    );
    final hint = _hint(l10n);
    return Align(
      alignment: AlignmentDirectional.centerStart,
      child: hint == null ? segments : Tooltip(message: hint, child: segments),
    );
  }
}

/// A model's thinking levels as one segmented control, the chosen one under a sliding thumb.
class _ThinkingSegments extends StatelessWidget {
  const _ThinkingSegments({
    required this.levels,
    required this.selected,
    required this.expand,
    required this.onSelect,
  });

  /// Widest a segment grows when the levels keep a compact size.
  static const double _compactSegmentWidth = 62;

  final List<core_proxy.ThinkingLevelOption> levels;
  final core_proxy.ThinkingLevel? selected;
  final bool expand;
  final ValueChanged<core_proxy.ThinkingLevel> onSelect;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final hasBudget = levels.any((option) => option.budgetTokens != null);
    final height = _settingsControlHeight(context) + (hasBudget ? 10 : 0);
    final index = levels.indexWhere((option) => option.level == selected);
    return LayoutBuilder(
      builder: (context, constraints) {
        final fitted = constraints.maxWidth / levels.length;
        final segmentWidth = expand || fitted < _compactSegmentWidth
            ? fitted
            : _compactSegmentWidth;
        return Container(
          width: segmentWidth * levels.length,
          height: height,
          decoration: BoxDecoration(
            color: isDark
                ? colorScheme.surfaceContainer.withValues(alpha: 0.6)
                : colorScheme.surfaceContainerHigh.withValues(alpha: 0.7),
            borderRadius: BorderRadius.circular(8),
          ),
          foregroundDecoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(
                alpha: isDark ? 0.35 : 0.55,
              ),
              width: 0.8,
            ),
          ),
          child: Stack(
            children: <Widget>[
              if (index >= 0)
                AnimatedPositioned(
                  duration: const Duration(milliseconds: 260),
                  curve: Curves.easeOutCubic,
                  left: index * segmentWidth + 2,
                  top: 2,
                  bottom: 2,
                  width: segmentWidth - 4,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: Color.alphaBlend(
                        colorScheme.primary.withValues(
                          alpha: isDark ? 0.24 : 0.14,
                        ),
                        colorScheme.surface,
                      ),
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(
                        color: colorScheme.primary.withValues(
                          alpha: isDark ? 0.55 : 0.4,
                        ),
                        width: 0.8,
                      ),
                    ),
                  ),
                ),
              Row(
                children: <Widget>[
                  for (var i = 0; i < levels.length; i++)
                    _ThinkingSegment(
                      option: levels[i],
                      width: segmentWidth,
                      selected: i == index,
                      onTap: i == index
                          ? null
                          : () => onSelect(levels[i].level),
                    ),
                ],
              ),
            ],
          ),
        );
      },
    );
  }
}

/// One level of the segmented thinking control.
class _ThinkingSegment extends StatelessWidget {
  const _ThinkingSegment({
    required this.option,
    required this.width,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.ThinkingLevelOption option;
  final double width;
  final bool selected;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final color = selected ? colorScheme.primary : colorScheme.onSurfaceVariant;
    final budget = option.budgetTokens;
    Widget segment = Semantics(
      button: true,
      selected: selected,
      child: SizedBox(
        width: width,
        child: Material(
          type: MaterialType.transparency,
          child: InkWell(
            onTap: onTap,
            borderRadius: BorderRadius.circular(6),
            child: Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    thinkingLevelName(option.level),
                    maxLines: 1,
                    softWrap: false,
                    overflow: TextOverflow.fade,
                    style: theme.textTheme.labelMedium?.copyWith(
                      fontSize: 11.5,
                      fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
                      color: color,
                    ),
                  ),
                  if (budget != null)
                    Text(
                      thinkingBudgetText(budget),
                      maxLines: 1,
                      style: theme.textTheme.labelSmall?.copyWith(
                        fontSize: 9.5,
                        color: color.withValues(alpha: 0.75),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    if (budget != null) {
      segment = Tooltip(message: '$budget tokens', child: segment);
    }
    return segment;
  }
}

/// One thinking level to pick, styled like the capability chips beside it.
class _ThinkingLevelPill extends StatelessWidget {
  const _ThinkingLevelPill({
    required this.label,
    required this.detail,
    required this.tooltip,
    required this.selected,
    required this.onTap,
  });

  final String label;

  /// Secondary text after the label, such as a budget's token count.
  final String? detail;
  final String? tooltip;
  final bool selected;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final background = selected
        ? Color.alphaBlend(
            colorScheme.primary.withValues(alpha: isDark ? 0.22 : 0.12),
            colorScheme.surface,
          )
        : (isDark
              ? colorScheme.surfaceContainer.withValues(alpha: 0.25)
              : colorScheme.surfaceContainerLow);
    final border = selected
        ? colorScheme.primary.withValues(alpha: isDark ? 0.6 : 0.45)
        : colorScheme.outlineVariant.withValues(alpha: isDark ? 0.35 : 0.5);
    final textColor = selected
        ? colorScheme.primary
        : colorScheme.onSurfaceVariant;
    final pill = Material(
      color: background,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(6),
        side: BorderSide(color: border, width: selected ? 1.0 : 0.6),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 180),
          curve: Curves.easeOutCubic,
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Text(
                label,
                style: theme.textTheme.bodyMedium?.copyWith(
                  fontSize: 11.5,
                  fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
                  color: textColor,
                ),
              ),
              if (detail != null) ...<Widget>[
                const SizedBox(width: 4),
                Text(
                  detail!,
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontSize: 10,
                    color: textColor.withValues(alpha: 0.7),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
    return tooltip == null ? pill : Tooltip(message: tooltip!, child: pill);
  }
}

/// The kinds of thinking control a user can describe a model with.
enum _ThinkingKind { none, always, toggle, levels, budget }

/// Lets the user describe how a model thinks; returns `null` when cancelled.
class _ThinkingDescribeDialog extends StatefulWidget {
  const _ThinkingDescribeDialog({required this.initial});

  final core_proxy.ModelThinkingSpec initial;

  static Future<core_proxy.ModelThinkingSpec?> show({
    required BuildContext context,
    required core_proxy.ModelThinkingSpec initial,
  }) {
    return showDialog<core_proxy.ModelThinkingSpec>(
      context: context,
      builder: (context) => _ThinkingDescribeDialog(initial: initial),
    );
  }

  @override
  State<_ThinkingDescribeDialog> createState() =>
      _ThinkingDescribeDialogState();
}

class _ThinkingDescribeDialogState extends State<_ThinkingDescribeDialog> {
  /// The graded levels a user can pick, weakest first.
  static const List<core_proxy.ThinkingLevel> _gradedLevels =
      <core_proxy.ThinkingLevel>[
        core_proxy.ThinkingLevel.minimal,
        core_proxy.ThinkingLevel.low,
        core_proxy.ThinkingLevel.medium,
        core_proxy.ThinkingLevel.high,
        core_proxy.ThinkingLevel.xhigh,
        core_proxy.ThinkingLevel.max,
      ];

  late _ThinkingKind _kind;
  late Set<core_proxy.ThinkingLevel> _levels;
  late bool _canTurnOff;
  late final TextEditingController _budgetMin;
  late final TextEditingController _budgetMax;
  bool _showLevelsRequired = false;

  @override
  void initState() {
    super.initState();
    final spec = widget.initial;
    final graded = spec.efforts
        .where((level) => level != core_proxy.ThinkingLevel.off)
        .toSet();
    _kind = !spec.supported
        ? _ThinkingKind.none
        : graded.isNotEmpty
        ? _ThinkingKind.levels
        : spec.budget != null
        ? _ThinkingKind.budget
        : spec.toggle
        ? _ThinkingKind.toggle
        : _ThinkingKind.always;
    _levels = graded;
    _canTurnOff =
        spec.toggle || spec.efforts.contains(core_proxy.ThinkingLevel.off);
    _budgetMin = TextEditingController(
      text: spec.budget?.min?.toString() ?? '',
    );
    _budgetMax = TextEditingController(
      text: spec.budget?.max?.toString() ?? '',
    );
  }

  @override
  void dispose() {
    _budgetMin.dispose();
    _budgetMax.dispose();
    super.dispose();
  }

  void _save() {
    final spec = switch (_kind) {
      _ThinkingKind.none => const core_proxy.ModelThinkingSpec(
        supported: false,
        toggle: false,
        efforts: <core_proxy.ThinkingLevel>[],
        budget: null,
      ),
      _ThinkingKind.always => const core_proxy.ModelThinkingSpec(
        supported: true,
        toggle: false,
        efforts: <core_proxy.ThinkingLevel>[],
        budget: null,
      ),
      _ThinkingKind.toggle => const core_proxy.ModelThinkingSpec(
        supported: true,
        toggle: true,
        efforts: <core_proxy.ThinkingLevel>[],
        budget: null,
      ),
      // "Can be turned off" is both a switch and effort `none`, so every API can
      // spell it in the field it reads.
      _ThinkingKind.levels => core_proxy.ModelThinkingSpec(
        supported: true,
        toggle: _canTurnOff,
        efforts: <core_proxy.ThinkingLevel>[
          if (_canTurnOff) core_proxy.ThinkingLevel.off,
          ..._gradedLevels.where(_levels.contains),
        ],
        budget: null,
      ),
      _ThinkingKind.budget => core_proxy.ModelThinkingSpec(
        supported: true,
        toggle: _canTurnOff,
        efforts: const <core_proxy.ThinkingLevel>[],
        budget: core_proxy.ThinkingBudgetRange(
          min: int.tryParse(_budgetMin.text.trim()),
          max: int.tryParse(_budgetMax.text.trim()),
        ),
      ),
    };
    if (_kind == _ThinkingKind.levels && _levels.isEmpty) {
      setState(() => _showLevelsRequired = true);
      return;
    }
    Navigator.of(context).pop(spec);
  }

  String _kindLabel(AppLocalizations l10n, _ThinkingKind kind) {
    return switch (kind) {
      _ThinkingKind.none => l10n.settingsModelThinkingKindNone,
      _ThinkingKind.always => l10n.settingsModelThinkingKindAlways,
      _ThinkingKind.toggle => l10n.settingsModelThinkingKindSwitch,
      _ThinkingKind.levels => l10n.settingsModelThinkingKindLevels,
      _ThinkingKind.budget => l10n.settingsModelThinkingKindBudget,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return AlertDialog(
      title: Text(l10n.settingsModelThinkingDescribeTitle),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              for (final kind in _ThinkingKind.values)
                _ThinkingKindOption(
                  label: _kindLabel(l10n, kind),
                  selected: _kind == kind,
                  onTap: () => setState(() {
                    _kind = kind;
                    _showLevelsRequired = false;
                  }),
                ),
              AnimatedSize(
                duration: const Duration(milliseconds: 200),
                curve: Curves.easeOutCubic,
                alignment: Alignment.topCenter,
                child: switch (_kind) {
                  _ThinkingKind.levels => Padding(
                    padding: const EdgeInsets.fromLTRB(32, 6, 0, 0),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Wrap(
                          spacing: 6,
                          runSpacing: 6,
                          children: <Widget>[
                            for (final level in _gradedLevels)
                              _ThinkingLevelPill(
                                label: thinkingLevelName(level),
                                detail: null,
                                tooltip: null,
                                selected: _levels.contains(level),
                                onTap: () => setState(() {
                                  _showLevelsRequired = false;
                                  if (!_levels.remove(level)) {
                                    _levels.add(level);
                                  }
                                }),
                              ),
                          ],
                        ),
                        if (_showLevelsRequired) ...<Widget>[
                          const SizedBox(height: 6),
                          Text(
                            l10n.settingsModelThinkingLevelsRequired,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: colorScheme.error,
                            ),
                          ),
                        ],
                        _ThinkingTurnOffSwitch(
                          label: l10n.settingsModelThinkingCanTurnOff,
                          value: _canTurnOff,
                          onChanged: (value) =>
                              setState(() => _canTurnOff = value),
                        ),
                      ],
                    ),
                  ),
                  _ThinkingKind.budget => Padding(
                    padding: const EdgeInsets.fromLTRB(32, 6, 0, 0),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Row(
                          children: <Widget>[
                            Expanded(
                              child: _ThinkingBudgetField(
                                controller: _budgetMin,
                                label: l10n.settingsModelThinkingBudgetMin,
                              ),
                            ),
                            const SizedBox(width: 10),
                            Expanded(
                              child: _ThinkingBudgetField(
                                controller: _budgetMax,
                                label: l10n.settingsModelThinkingBudgetMax,
                              ),
                            ),
                          ],
                        ),
                        _ThinkingTurnOffSwitch(
                          label: l10n.settingsModelThinkingCanTurnOff,
                          value: _canTurnOff,
                          onChanged: (value) =>
                              setState(() => _canTurnOff = value),
                        ),
                      ],
                    ),
                  ),
                  _ => const SizedBox(width: double.infinity),
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
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
    );
  }
}

/// One kind of thinking control in the describe dialog, picked like a radio button.
class _ThinkingKindOption extends StatelessWidget {
  const _ThinkingKindOption({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return InkWell(
      borderRadius: BorderRadius.circular(8),
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 8),
        child: Row(
          children: <Widget>[
            AnimatedSwitcher(
              duration: const Duration(milliseconds: 160),
              child: Icon(
                selected
                    ? Icons.radio_button_checked
                    : Icons.radio_button_unchecked,
                key: ValueKey<bool>(selected),
                size: 20,
                color: selected
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                label,
                style: theme.textTheme.bodyMedium?.copyWith(
                  fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// The "can be turned off" switch under a graded or budget description.
class _ThinkingTurnOffSwitch extends StatelessWidget {
  const _ThinkingTurnOffSwitch({
    required this.label,
    required this.value,
    required this.onChanged,
  });

  final String label;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 6),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Text(label, style: Theme.of(context).textTheme.bodySmall),
          ),
          Switch(value: value, onChanged: onChanged),
        ],
      ),
    );
  }
}

/// A token count field of a budget description; empty leaves the bound unknown.
class _ThinkingBudgetField extends StatelessWidget {
  const _ThinkingBudgetField({required this.controller, required this.label});

  final TextEditingController controller;
  final String label;

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: controller,
      keyboardType: TextInputType.number,
      inputFormatters: <TextInputFormatter>[
        FilteringTextInputFormatter.digitsOnly,
      ],
      decoration: InputDecoration(
        labelText: label,
        isDense: true,
        border: const OutlineInputBorder(),
      ),
    );
  }
}

/// A switch shaped as a chip: filled and tinted when on, quiet when off.
class _CapabilityToggleChip extends StatelessWidget {
  const _CapabilityToggleChip({
    required this.title,
    required this.tooltip,
    required this.checked,
    required this.onChanged,
    required this.icon,
  });

  final String title;
  final String tooltip;
  final bool checked;
  final ValueChanged<bool> onChanged;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final background = checked
        ? Color.alphaBlend(
            colorScheme.primary.withValues(alpha: isDark ? 0.18 : 0.10),
            colorScheme.surface,
          )
        : (isDark
              ? colorScheme.surfaceContainer.withValues(alpha: 0.25)
              : colorScheme.surfaceContainerLow);
    final border = checked
        ? colorScheme.primary.withValues(alpha: isDark ? 0.5 : 0.38)
        : colorScheme.outlineVariant.withValues(alpha: isDark ? 0.35 : 0.5);
    final foreground = checked
        ? colorScheme.primary
        : colorScheme.onSurfaceVariant;
    return Tooltip(
      message: tooltip,
      child: Semantics(
        toggled: checked,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 180),
          curve: Curves.easeOutCubic,
          height: _settingsControlHeight(context),
          decoration: ShapeDecoration(
            color: background,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(7),
              side: BorderSide(color: border, width: checked ? 0.9 : 0.6),
            ),
          ),
          child: Material(
            type: MaterialType.transparency,
            child: InkWell(
              onTap: () => onChanged(!checked),
              borderRadius: BorderRadius.circular(7),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 10),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Icon(icon, size: 14, color: foreground),
                    const SizedBox(width: 6),
                    Text(
                      title,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        fontSize: 12,
                        fontWeight: checked ? FontWeight.w600 : FontWeight.w500,
                        color: checked
                            ? colorScheme.onSurface
                            : colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ModernNumberField extends StatelessWidget {
  const _ModernNumberField({
    required this.controller,
    required this.suffix,
    this.focusNode,
    this.invalid = false,
    this.onSubmitted,
  });

  final TextEditingController controller;
  final String suffix;
  final FocusNode? focusNode;

  /// Whether the entry is not a value the field takes; the border turns red.
  final bool invalid;
  final ValueChanged<String>? onSubmitted;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    OutlineInputBorder border(Color color, double width) {
      return OutlineInputBorder(
        borderRadius: BorderRadius.circular(6),
        borderSide: BorderSide(color: color, width: width),
      );
    }

    final idle = border(
      invalid
          ? colorScheme.error
          : colorScheme.outline.withValues(alpha: isDark ? 0.4 : 0.55),
      1.0,
    );
    final focused = border(
      invalid ? colorScheme.error : colorScheme.primary,
      1.3,
    );
    // Even padding centers the digits in the same height as the chips beside them.
    final verticalPadding = (_settingsControlHeight(context) - 18) / 2;
    return TextField(
      controller: controller,
      focusNode: focusNode,
      style: theme.textTheme.bodyMedium?.copyWith(
        fontWeight: FontWeight.w600,
        fontSize: 12.5,
        height: 18 / 12.5,
        color: colorScheme.onSurface,
      ),
      keyboardType: TextInputType.number,
      textInputAction: TextInputAction.done,
      onSubmitted: onSubmitted,
      decoration: InputDecoration(
        isDense: true,
        filled: true,
        fillColor: isDark
            ? colorScheme.surfaceContainer
            : colorScheme.surfaceContainerLow,
        contentPadding: EdgeInsets.symmetric(
          horizontal: 10,
          vertical: verticalPadding,
        ),
        suffixText: suffix,
        suffixStyle: theme.textTheme.labelSmall?.copyWith(
          color: colorScheme.onSurfaceVariant,
          fontWeight: FontWeight.w600,
          fontSize: 10.5,
        ),
        enabledBorder: idle,
        focusedBorder: focused,
        disabledBorder: border(
          colorScheme.outlineVariant.withValues(alpha: isDark ? 0.2 : 0.35),
          0.8,
        ),
      ),
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
          label: l10n.settingsModelCapabilityToolCall,
          tooltip: l10n.settingsModelToolCall,
          tint: _CapsuleTint.primary,
        ),
      if (capabilities.directImage)
        _CapabilityCapsule(
          label: l10n.settingsModelCapabilityImage,
          tooltip: l10n.settingsModelDirectImage,
          tint: _CapsuleTint.primary,
        ),
      if (capabilities.directAudio)
        _CapabilityCapsule(
          label: l10n.settingsModelCapabilityAudio,
          tooltip: l10n.settingsModelDirectAudio,
          tint: _CapsuleTint.primary,
        ),
      if (capabilities.directVideo)
        _CapabilityCapsule(
          label: l10n.settingsModelCapabilityVideo,
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
  const _DeleteModelBlockedDialog({
    required this.functionTypes,
    required this.several,
  });

  final List<core_proxy.FunctionType> functionTypes;

  /// Whether the user tried to delete several models rather than one.
  final bool several;

  static Future<void> show({
    required BuildContext context,
    required List<core_proxy.FunctionType> functionTypes,
    bool several = false,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) => _DeleteModelBlockedDialog(
        functionTypes: functionTypes,
        several: several,
      ),
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
      content: Text(
        several
            ? l10n.settingsModelDeleteModelsBlocked(names)
            : l10n.settingsModelDeleteBlocked(names),
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

String _connectionTestItemLabel(
  AppLocalizations l10n,
  core_proxy.ModelConnectionTestType type,
  String? error,
) {
  final label = _connectionTestTypeLabel(l10n, type);
  return switch (connectionTestRestriction(error)) {
    ModelConnectionTestRestriction.credits =>
      '$label · ${l10n.settingsModelTestCreditRestricted}',
    ModelConnectionTestRestriction.rateLimit =>
      '$label · ${l10n.settingsModelTestRateRestricted}',
    null => label,
  };
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
