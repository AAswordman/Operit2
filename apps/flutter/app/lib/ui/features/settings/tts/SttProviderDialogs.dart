// ignore_for_file: file_names

part of 'TtsSettingsPanel.dart';

class _SttConfigDialog extends StatefulWidget {
  const _SttConfigDialog({
    required this.config,
    required this.providerCatalogEntries,
    required this.loadModels,
  });

  final core_proxy.SttConfig? config;
  final List<core_proxy.SttProviderCatalogEntry> providerCatalogEntries;
  final Future<List<core_proxy.AvailableSttModel>> Function(
    String providerTypeId,
  )
  loadModels;

  /// Opens the STT provider editor and returns one validated configuration.
  static Future<core_proxy.SttConfig?> show({
    required BuildContext context,
    required core_proxy.SttConfig? config,
    required List<core_proxy.SttProviderCatalogEntry> providerCatalogEntries,
    required Future<List<core_proxy.AvailableSttModel>> Function(
      String providerTypeId,
    )
    loadModels,
  }) {
    return showDialog<core_proxy.SttConfig>(
      context: context,
      builder: (context) => _SttConfigDialog(
        config: config,
        providerCatalogEntries: providerCatalogEntries,
        loadModels: loadModels,
      ),
    );
  }

  /// Creates mutable STT provider dialog state.
  @override
  State<_SttConfigDialog> createState() => _SttConfigDialogState();
}

class _SttConfigDialogState extends State<_SttConfigDialog> {
  final _formKey = GlobalKey<FormState>();
  late String _providerType;
  late final TextEditingController _nameController;
  late final TextEditingController _endpointController;
  late final TextEditingController _apiKeyController;
  late final TextEditingController _modelController;
  late final TextEditingController _fileFieldController;
  late final TextEditingController _modelFieldController;
  late final TextEditingController _languageFieldController;
  late final TextEditingController _responsePathController;
  late final TextEditingController _headersController;
  List<core_proxy.AvailableSttModel> _availableModels = const [];
  bool _loadingModels = true;
  String? _modelLoadError;

  /// Initializes controllers from one config or the first catalog entry.
  @override
  void initState() {
    super.initState();
    final config = widget.config;
    final catalog = config == null
        ? widget.providerCatalogEntries.first
        : _requiredSttProviderCatalog(
            widget.providerCatalogEntries,
            config.providerType,
          );
    _providerType = catalog.providerTypeId;
    _nameController = TextEditingController(
      text: config?.name ?? catalog.displayName,
    );
    _endpointController = TextEditingController(
      text: config?.endpoint ?? catalog.defaultEndpoint,
    );
    _apiKeyController = TextEditingController(text: config?.apiKey ?? '');
    _modelController = TextEditingController(
      text: config?.model ?? catalog.defaultModel,
    );
    _fileFieldController = TextEditingController(
      text: config?.fileFieldName ?? catalog.defaultFileFieldName,
    );
    _modelFieldController = TextEditingController(
      text: config?.modelFieldName ?? catalog.defaultModelFieldName,
    );
    _languageFieldController = TextEditingController(
      text: config?.languageFieldName ?? catalog.defaultLanguageFieldName,
    );
    _responsePathController = TextEditingController(
      text: config?.responseTextJsonPath ?? catalog.defaultResponseTextJsonPath,
    );
    _headersController = TextEditingController(
      text: _encodeJsonList(config?.headers ?? catalog.defaultHeaders),
    );
    _loadAvailableModels();
  }

  /// Releases every STT provider text controller.
  @override
  void dispose() {
    _nameController.dispose();
    _endpointController.dispose();
    _apiKeyController.dispose();
    _modelController.dispose();
    _fileFieldController.dispose();
    _modelFieldController.dispose();
    _languageFieldController.dispose();
    _responsePathController.dispose();
    _headersController.dispose();
    super.dispose();
  }

  /// Loads catalog or installed models for the selected STT provider.
  Future<void> _loadAvailableModels() async {
    final requestedProviderType = _providerType;
    setState(() {
      _loadingModels = true;
      _modelLoadError = null;
    });
    try {
      final models = await widget.loadModels(requestedProviderType);
      if (!mounted || _providerType != requestedProviderType) {
        return;
      }
      setState(() {
        _availableModels = models;
        _loadingModels = false;
      });
    } catch (error) {
      if (!mounted || _providerType != requestedProviderType) {
        return;
      }
      setState(() {
        _availableModels = const [];
        _loadingModels = false;
        _modelLoadError = '$error';
      });
    }
  }

  /// Applies one provider catalog entry and refreshes its model list.
  void _selectProviderType(String providerType) {
    final catalog = _requiredSttProviderCatalog(
      widget.providerCatalogEntries,
      providerType,
    );
    setState(() {
      _providerType = catalog.providerTypeId;
      _nameController.text = catalog.displayName;
      _endpointController.text = catalog.defaultEndpoint;
      _apiKeyController.clear();
      _modelController.text = catalog.defaultModel;
      _fileFieldController.text = catalog.defaultFileFieldName;
      _modelFieldController.text = catalog.defaultModelFieldName;
      _languageFieldController.text = catalog.defaultLanguageFieldName;
      _responsePathController.text = catalog.defaultResponseTextJsonPath;
      _headersController.text = _encodeJsonList(catalog.defaultHeaders);
    });
    _loadAvailableModels();
  }

  /// Builds the STT provider configuration form.
  @override
  Widget build(BuildContext context) {
    final localModel = _isLocalSttProviderType(_providerType);
    return AlertDialog(
      title: Text(widget.config == null ? '新建 STT 供应商' : '编辑 STT 供应商'),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                DropdownButtonFormField<String>(
                  key: ValueKey<String>(_providerType),
                  initialValue: _providerType,
                  decoration: const InputDecoration(labelText: '供应商类型'),
                  items: _sttProviderCatalogItems(
                    widget.providerCatalogEntries,
                  ),
                  onChanged: (value) {
                    if (value != null) {
                      _selectProviderType(value);
                    }
                  },
                ),
                const SizedBox(height: 10),
                _field(_nameController, '供应商名称', requiredField: true),
                if (!localModel) ...<Widget>[
                  _field(_endpointController, 'Endpoint', requiredField: true),
                  _field(_apiKeyController, 'API Key', obscureText: true),
                ],
                _buildModelField(localModel),
                if (!localModel)
                  ExpansionTile(
                    tilePadding: EdgeInsets.zero,
                    childrenPadding: EdgeInsets.zero,
                    title: Text(
                      'Multipart 请求配置',
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                    children: <Widget>[
                      _field(
                        _fileFieldController,
                        '音频字段名',
                        requiredField: true,
                      ),
                      _field(
                        _modelFieldController,
                        '模型字段名',
                        requiredField: true,
                      ),
                      _field(_languageFieldController, '语言字段名'),
                      _field(
                        _responsePathController,
                        '识别文本 JSONPath',
                        requiredField: true,
                      ),
                      _field(_headersController, 'Headers JSON', minLines: 3),
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
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _loadingModels || _modelLoadError != null ? null : _submit,
          child: const Text('保存'),
        ),
      ],
    );
  }

  /// Builds a catalog-backed or custom STT model selector.
  Widget _buildModelField(bool localModel) {
    if (_loadingModels) {
      return const Padding(
        padding: EdgeInsets.only(bottom: 10),
        child: SizedBox(
          height: 48,
          child: Center(child: M3LoadingIndicator(size: 24)),
        ),
      );
    }
    final loadError = _modelLoadError;
    if (loadError != null) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 10),
        child: Row(
          children: <Widget>[
            Expanded(
              child: Text(
                '模型列表加载失败：$loadError',
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
            IconButton(
              onPressed: _loadAvailableModels,
              icon: const Icon(Icons.refresh),
              tooltip: '重新加载模型',
            ),
          ],
        ),
      );
    }
    if (_availableModels.isEmpty) {
      if (localModel) {
        return Padding(
          padding: const EdgeInsets.only(bottom: 10),
          child: InputDecorator(
            decoration: const InputDecoration(labelText: '模型'),
            child: const Text('没有已安装的 STT 本地模型'),
          ),
        );
      }
      return _field(_modelController, '模型', requiredField: true);
    }
    String? selectedModel;
    for (final model in _availableModels) {
      if (model.model == _modelController.text.trim()) {
        selectedModel = model.model;
      }
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: DropdownButtonFormField<String>(
        key: ValueKey<String?>(selectedModel),
        initialValue: selectedModel,
        isExpanded: true,
        decoration: const InputDecoration(labelText: '模型'),
        items: _availableModels
            .map(
              (model) => DropdownMenuItem<String>(
                value: model.model,
                child: Text(
                  model.displayName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            )
            .toList(growable: false),
        onChanged: (value) {
          if (value != null) {
            _modelController.text = value;
          }
        },
        validator: (value) => value == null ? '请选择模型' : null,
      ),
    );
  }

  /// Builds one reusable STT text form field.
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
          if (requiredField && (value?.trim().isEmpty ?? true)) {
            return '$label不能为空';
          }
          return null;
        },
      ),
    );
  }

  /// Validates and returns one STT provider configuration.
  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    if (_modelController.text.trim().isEmpty) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('请选择 STT 模型')));
      return;
    }
    final localModel = _isLocalSttProviderType(_providerType);
    late final List<core_proxy.SttHttpHeader> headers;
    try {
      headers = localModel
          ? const <core_proxy.SttHttpHeader>[]
          : _decodeSttHeaders(_headersController.text.trim());
    } catch (error) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('$error')));
      return;
    }
    final now = DateTime.now().millisecondsSinceEpoch;
    final existing = widget.config;
    Navigator.of(context).pop(
      core_proxy.SttConfig(
        id: existing?.id ?? '',
        name: _nameController.text.trim(),
        providerType: _providerType,
        endpoint: localModel ? '' : _endpointController.text.trim(),
        apiKey: localModel ? '' : _apiKeyController.text.trim(),
        model: _modelController.text.trim(),
        fileFieldName: localModel ? '' : _fileFieldController.text.trim(),
        modelFieldName: localModel ? '' : _modelFieldController.text.trim(),
        languageFieldName: localModel
            ? ''
            : _languageFieldController.text.trim(),
        responseTextJsonPath: localModel
            ? r'$.text'
            : _responsePathController.text.trim(),
        headers: headers,
        createdAt: existing?.createdAt ?? now,
        updatedAt: now,
      ),
    );
  }
}

/// Returns whether one STT provider type uses installed local models.
bool _isLocalSttProviderType(String providerType) {
  return providerType.trim().toUpperCase() == 'LOCAL_MODEL';
}

/// Builds STT provider catalog dropdown entries.
List<DropdownMenuItem<String>> _sttProviderCatalogItems(
  List<core_proxy.SttProviderCatalogEntry> entries,
) {
  return entries
      .map(
        (entry) => DropdownMenuItem<String>(
          value: entry.providerTypeId,
          child: Text(entry.displayName),
        ),
      )
      .toList(growable: false);
}

/// Returns one exact STT provider catalog entry.
core_proxy.SttProviderCatalogEntry _requiredSttProviderCatalog(
  List<core_proxy.SttProviderCatalogEntry> entries,
  String providerType,
) {
  final normalized = providerType.trim().toUpperCase();
  for (final entry in entries) {
    if (entry.providerTypeId.trim().toUpperCase() == normalized) {
      return entry;
    }
  }
  throw StateError('STT provider catalog not found: $providerType');
}

/// Decodes STT request headers from a structured JSON list.
List<core_proxy.SttHttpHeader> _decodeSttHeaders(String raw) {
  return _decodeJsonList(
    raw,
    'Headers JSON',
  ).map(core_proxy.SttHttpHeader.fromJson).toList(growable: false);
}
