// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitFormStyles.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

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
  String? _testingModelKey;
  final Set<String> _expandedProviderIds = <String>{};
  bool _providerExpansionInitialized = false;

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
    final chatBinding = await functionManager.getModelBindingForFunction(
      functionType: core_proxy.FunctionType.chat,
    );
    final data = _ModelSettingsData(
      providers: await modelManager.getProviderProfiles(),
      summaries: await modelManager.getAllModelSummaries(),
      chatBinding: chatBinding,
      currentConfig: await modelManager.getResolvedModelConfig(
        providerId: chatBinding.providerId,
        modelId: chatBinding.modelId,
      ),
      functionBindings: await functionManager
          .functionModelBindingFlowSnapshot(),
      maxImageHistoryUserTurns: await apiPreferences
          .maxImageHistoryUserTurnsFlowSnapshot(),
      maxMediaHistoryUserTurns: await apiPreferences
          .maxMediaHistoryUserTurnsFlowSnapshot(),
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

  void _reload() {
    setState(() {
      _future = _load();
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
    _ModelSettingsData data,
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
        models: provider.models,
      ),
    );
    _reload();
  }

  Future<void> _deleteProvider(core_proxy.ProviderProfile provider) async {
    final bindings = await widget.clients.preferencesFunctionalConfigManager
        .functionModelBindingFlowSnapshot();
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

  Future<void> _addProviderModel(core_proxy.ProviderProfile provider) async {
    final messenger = ScaffoldMessenger.of(context);
    try {
      final models = await widget.clients.preferencesModelConfigManager
          .getAvailableProviderModels(providerId: provider.id);
      if (!mounted) {
        return;
      }
      final selection = await _AvailableModelDialog.show(
        context: context,
        models: models,
      );
      if (selection == null) {
        return;
      }
      switch (selection) {
        case _AvailableModelPicked(:final model):
          await widget.clients.preferencesModelConfigManager
              .addProviderModelFromAvailable(
                providerId: provider.id,
                modelId: model.modelId,
              );
          _reload();
        case _AvailableModelCustom():
          if (!mounted) {
            return;
          }
          await _createCustomProviderModel(provider);
      }
    } catch (error) {
      messenger.showSnackBar(SnackBar(content: Text('$error')));
    }
  }

  Future<void> _createCustomProviderModel(
    core_proxy.ProviderProfile provider,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final modelId = await _TextInputDialog.show(
      context: context,
      title: l10n.settingsModelCustomModel,
      label: l10n.settingsModelModelId,
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
        .functionModelBindingFlowSnapshot();
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
      final report = await widget.clients.preferencesModelConfigManager
          .testModelConnection(providerId: provider.id, modelId: model.id);
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
    return FutureBuilder<_ModelSettingsData>(
      future: _future,
      builder: (context, snapshot) {
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

class _ModelSettingsData {
  const _ModelSettingsData({
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
    } else {
      // Default to DEEPSEEK if present
      _selectedProviderTypeId = _catalogDeepseek()?.providerTypeId;
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
    return widget.catalogEntries.isNotEmpty
        ? widget.catalogEntries.first
        : null;
  }

  core_proxy.ProviderCatalogEntry? _selectedCatalog() {
    if (_selectedProviderTypeId == null) {
      return null;
    }
    for (final entry in widget.catalogEntries) {
      if (entry.providerTypeId == _selectedProviderTypeId) {
        return entry;
      }
    }
    return null;
  }

  bool get _needsCustomEndpoint {
    final catalog = _selectedCatalog();
    return catalog == null || catalog.defaultEndpoint.trim().isEmpty;
  }

  void _onProviderTypeChanged(String? providerTypeId) {
    setState(() {
      _selectedProviderTypeId = providerTypeId;
    });
    if (!_needsCustomEndpoint && _endpointController.text.isEmpty) {
      final catalog = _selectedCatalog();
      if (catalog != null) {
        _endpointController.text = catalog.defaultEndpoint;
      }
    }
  }

  void _save() {
    if (!_formKey.currentState!.validate() || _selectedProviderTypeId == null) {
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
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final editing = widget.provider != null;
    return AlertDialog(
      title: Text(
        editing
            ? l10n.settingsModelEditProvider
            : l10n.settingsModelCreateProvider,
      ),
      content: SizedBox(
        width: 560,
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
                  child: DropdownButtonFormField<String>(
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
                if (_needsCustomEndpoint)
                  _DialogTextField(
                    controller: _endpointController,
                    label: l10n.settingsModelApiEndpoint,
                    requiredField: true,
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

class _AvailableModelPicked extends _AvailableModelSelection {
  const _AvailableModelPicked(this.model);

  final core_proxy.AvailableProviderModel model;
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

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

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

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final filteredModels = _filteredModels(l10n);
    return AlertDialog(
      title: Text(l10n.settingsModelAddModel),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
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
            ListView(
              shrinkWrap: true,
              children: <Widget>[
                for (final model in filteredModels)
                  Material(
                    type: MaterialType.transparency,
                    child: ListTile(
                      dense: true,
                      visualDensity: VisualDensity.compact,
                      contentPadding: EdgeInsets.zero,
                      title: Text(model.modelId),
                      subtitle: Text(_availableModelSubtitle(l10n, model)),
                      onTap: () => Navigator.of(
                        context,
                      ).pop(_AvailableModelPicked(model)),
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

class _TextInputDialog extends StatefulWidget {
  const _TextInputDialog({required this.title, required this.label});

  final String title;
  final String label;

  static Future<String?> show({
    required BuildContext context,
    required String title,
    required String label,
  }) {
    return showDialog<String>(
      context: context,
      builder: (context) => _TextInputDialog(title: title, label: label),
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
        child: _DialogTextField(
          controller: _controller,
          label: widget.label,
          requiredField: true,
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

  final core_proxy.CoreApiChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem
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
  });

  final TextEditingController controller;
  final String label;
  final bool requiredField;
  final bool obscureText;
  final bool numberOnly;
  final int maxLines;

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
        keyboardType: numberOnly ? TextInputType.number : TextInputType.text,
        decoration: InputDecoration(labelText: label),
        validator: (value) {
          final text = value?.trim() ?? '';
          if (requiredField && text.isEmpty) {
            return label;
          }
          if (numberOnly && text.isEmpty) {
            return label;
          }
          return null;
        },
      ),
    );
  }
}

const List<core_proxy.FunctionType> _functionTypes = <core_proxy.FunctionType>[
  core_proxy.FunctionType.chat,
  core_proxy.FunctionType.summary,
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
    'OPENAI_RESPONSES' => l10n.settingsModelProviderTypeOpenaiResponses,
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
    'FOUR_ROUTER' => l10n.settingsModelProviderTypeFourRouter,
    'NOUS_PORTAL' => l10n.settingsModelProviderTypeNousPortal,
    'INFINIAI' => l10n.settingsModelProviderTypeInfiniai,
    'ALIPAY_BAILING' => l10n.settingsModelProviderTypeAlipayBailing,
    'DOUBAO' => l10n.settingsModelProviderTypeDoubao,
    'NVIDIA' => l10n.settingsModelProviderTypeNvidia,
    'LMSTUDIO' => l10n.settingsModelProviderTypeLmstudio,
    'OLLAMA' => l10n.settingsModelProviderTypeOllama,
    'OPENAI_LOCAL' => l10n.settingsModelProviderTypeOpenaiLocal,
    'MNN' => l10n.settingsModelProviderTypeMnn,
    'LLAMA_CPP' => l10n.settingsModelProviderTypeLlamaCpp,
    'PPINFRA' => l10n.settingsModelProviderTypePpinfra,
    'NOVITA' => l10n.settingsModelProviderTypeNovita,
    'OTHER' => l10n.settingsModelProviderTypeOther,
    _ => throw UnsupportedError('missing provider type i18n: $providerTypeId'),
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
