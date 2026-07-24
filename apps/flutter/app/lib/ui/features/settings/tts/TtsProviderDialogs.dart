// ignore_for_file: file_names

part of 'TtsSettingsPanel.dart';

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

sealed class _TtsProviderEditResult {
  const _TtsProviderEditResult();
}

class _TtsProviderEditSaved extends _TtsProviderEditResult {
  const _TtsProviderEditSaved(this.values);

  final _TtsProviderEditValues values;
}

class _TtsProviderEditDeleted extends _TtsProviderEditResult {
  const _TtsProviderEditDeleted();
}

class _TtsProviderDialog extends StatefulWidget {
  const _TtsProviderDialog({
    required this.config,
    required this.providerCatalogEntries,
    required this.deleteBlockedReason,
  });

  final core_proxy.TtsConfig config;
  final List<core_proxy.TtsProviderCatalogEntry> providerCatalogEntries;
  final String? deleteBlockedReason;

  /// Opens the TTS provider editor and returns the selected provider action.
  static Future<_TtsProviderEditResult?> show({
    required BuildContext context,
    required core_proxy.TtsConfig config,
    required List<core_proxy.TtsProviderCatalogEntry> providerCatalogEntries,
    required String? deleteBlockedReason,
  }) {
    return showDialog<_TtsProviderEditResult>(
      context: context,
      builder: (context) => _TtsProviderDialog(
        config: config,
        providerCatalogEntries: providerCatalogEntries,
        deleteBlockedReason: deleteBlockedReason,
      ),
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
    final l10n = AppLocalizations.of(context)!;
    final isHttpTts = _isHttpProviderType(_providerType);
    final isSystemTts = _isSystemProviderType(_providerType);
    final isLocalModelTts = _isLocalModelProviderType(_providerType);
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
                  items: _providerCatalogItems(widget.providerCatalogEntries),
                  onChanged: (value) {
                    if (value == null) {
                      return;
                    }
                    final catalog = _requiredTtsProviderCatalog(
                      widget.providerCatalogEntries,
                      value,
                    );
                    setState(() {
                      _providerType = catalog.providerTypeId;
                      _nameController.text = catalog.displayName;
                      _endpointController.text = catalog.defaultEndpoint;
                      _httpMethodController.text = catalog.defaultHttpMethod;
                      _contentTypeController.text = catalog.defaultContentType;
                      _requestBodyController.text = catalog.defaultRequestBody;
                      _headersController.text = _encodeJsonList(
                        catalog.defaultHeaders,
                      );
                      _responsePipelineController.text = _encodeJsonList(
                        catalog.defaultResponsePipeline,
                      );
                    });
                  },
                ),
                const SizedBox(height: 10),
                _field(_nameController, '供应商名称', requiredField: true),
                if (!isSystemTts && !isLocalModelTts) ...[
                  _field(_endpointController, 'Endpoint', requiredField: true),
                  _field(_apiKeyController, 'API Key', obscureText: true),
                ],
                if (isHttpTts) ...[
                  _field(_httpMethodController, 'HTTP 方法', requiredField: true),
                  _field(_contentTypeController, 'Content-Type'),
                  _field(
                    _requestBodyController,
                    '请求体模板',
                    requiredField:
                        isHttpTts &&
                        _httpMethodController.text.trim().toUpperCase() ==
                            'POST',
                    minLines: 4,
                  ),
                  _field(_headersController, 'Headers JSON', minLines: 3),
                  _field(_responsePipelineController, '响应管道 JSON', minLines: 4),
                ],
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
      ),
      actions: <Widget>[
        Tooltip(
          message: widget.deleteBlockedReason ?? l10n.settingsTtsDeleteProvider,
          child: TextButton.icon(
            onPressed: widget.deleteBlockedReason == null ? _delete : null,
            icon: const Icon(Icons.delete_outline),
            label: Text(l10n.delete),
          ),
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

  /// Returns the explicit request to delete this TTS provider.
  void _delete() {
    Navigator.of(context).pop(const _TtsProviderEditDeleted());
  }

  /// Validates provider fields and returns the saved provider values.
  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    late final List<core_proxy.TtsHttpHeader> headers;
    late final List<core_proxy.TtsHttpResponsePipelineStep> responsePipeline;
    try {
      headers = _isHttpProviderType(_providerType)
          ? _decodeHeaders(_headersController.text.trim(), 'Headers JSON')
          : const <core_proxy.TtsHttpHeader>[];
      responsePipeline = _isHttpProviderType(_providerType)
          ? _decodePipeline(
              _responsePipelineController.text.trim(),
              '响应管道 JSON',
            )
          : const <core_proxy.TtsHttpResponsePipelineStep>[];
    } catch (error) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('$error')));
      return;
    }
    Navigator.of(context).pop(
      _TtsProviderEditSaved(
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
    Navigator.of(
      context,
    ).pop(_CustomTtsVoiceValues(model: model, voice: voice));
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
    required this.onTest,
  });

  final core_proxy.TtsConfig config;
  final String? deleteBlockedReason;
  final Future<void> Function() onTest;

  static Future<_TtsVoiceEditResult?> show({
    required BuildContext context,
    required core_proxy.TtsConfig config,
    required String? deleteBlockedReason,
    required Future<void> Function() onTest,
  }) {
    return showDialog<_TtsVoiceEditResult>(
      context: context,
      builder: (context) => _TtsVoiceConfigDialog(
        config: config,
        deleteBlockedReason: deleteBlockedReason,
        onTest: onTest,
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
  bool _testingPlayback = false;

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

  bool get _requiresModel => _ttsConfigUsesPlaceholder(widget.config, 'model');

  bool get _requiresVoice => _ttsConfigUsesPlaceholder(widget.config, 'voice');

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

  Future<void> _runTest() async {
    setState(() {
      _testingPlayback = true;
    });
    try {
      await widget.onTest();
    } finally {
      if (mounted) {
        setState(() {
          _testingPlayback = false;
        });
      }
    }
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
              Align(
                alignment: Alignment.centerLeft,
                child: OutlinedButton.icon(
                  onPressed: _testingPlayback ? null : _runTest,
                  icon: _testingPlayback
                      ? const SizedBox.square(
                          dimension: 18,
                          child: Center(child: M3LoadingIndicator(size: 18)),
                        )
                      : const Icon(Icons.volume_up_outlined, size: 18),
                  label: const Text('测试音色'),
                ),
              ),
              const SizedBox(height: 12),
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
  const _TtsConfigDialog({required this.providerCatalogEntries});

  final List<core_proxy.TtsProviderCatalogEntry> providerCatalogEntries;

  static Future<core_proxy.TtsConfig?> show({
    required BuildContext context,
    required List<core_proxy.TtsProviderCatalogEntry> providerCatalogEntries,
  }) {
    return showDialog<core_proxy.TtsConfig>(
      context: context,
      builder: (context) =>
          _TtsConfigDialog(providerCatalogEntries: providerCatalogEntries),
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
    final catalog = _requiredTtsProviderCatalog(
      widget.providerCatalogEntries,
      _TtsProviderTypes.system,
    );
    _providerType = catalog.providerTypeId;
    _nameController = TextEditingController(text: catalog.displayName);
    _endpointController = TextEditingController(text: catalog.defaultEndpoint);
    _apiKeyController = TextEditingController();
    _modelController = TextEditingController(text: catalog.defaultModel);
    _voiceController = TextEditingController();
    _formatController = TextEditingController(
      text: catalog.defaultResponseFormat,
    );
    _speedController = TextEditingController(text: '1.0');
    _httpMethodController = TextEditingController(
      text: catalog.defaultHttpMethod,
    );
    _contentTypeController = TextEditingController(
      text: catalog.defaultContentType,
    );
    _requestBodyController = TextEditingController(
      text: catalog.defaultRequestBody,
    );
    _headersController = TextEditingController(
      text: _encodeJsonList(catalog.defaultHeaders),
    );
    _responsePipelineController = TextEditingController(
      text: _encodeJsonList(catalog.defaultResponsePipeline),
    );
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
    final catalog = _requiredTtsProviderCatalog(
      widget.providerCatalogEntries,
      _providerType,
    );
    final isSystemTts = _isSystemProviderType(_providerType);
    final isHttpTts = _isHttpProviderType(_providerType);
    final isLocalModelTts = _isLocalModelProviderType(_providerType);
    final requireModel =
        isLocalModelTts ||
        (!isSystemTts && _ttsProviderCatalogUsesPlaceholder(catalog, 'model'));
    final requireVoice =
        isLocalModelTts ||
        (!isSystemTts && _ttsProviderCatalogUsesPlaceholder(catalog, 'voice'));
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
                  items: _providerCatalogItems(widget.providerCatalogEntries),
                  onChanged: (value) {
                    if (value == null) {
                      return;
                    }
                    final catalog = _requiredTtsProviderCatalog(
                      widget.providerCatalogEntries,
                      value,
                    );
                    setState(() {
                      _providerType = catalog.providerTypeId;
                      _nameController.text = catalog.displayName;
                      _endpointController.text = catalog.defaultEndpoint;
                      _modelController.text = catalog.defaultModel;
                      _voiceController.clear();
                      _formatController.text = catalog.defaultResponseFormat;
                      _httpMethodController.text = catalog.defaultHttpMethod;
                      _contentTypeController.text = catalog.defaultContentType;
                      _requestBodyController.text = catalog.defaultRequestBody;
                      _headersController.text = _encodeJsonList(
                        catalog.defaultHeaders,
                      );
                      _responsePipelineController.text = _encodeJsonList(
                        catalog.defaultResponsePipeline,
                      );
                    });
                  },
                ),
                const SizedBox(height: 10),
                _field(_nameController, '供应商名称', requiredField: true),
                if (isLocalModelTts) ...[
                  _field(_modelController, '模型 ID@版本', requiredField: true),
                  _field(_voiceController, 'Speaker ID', requiredField: true),
                ] else if (!isSystemTts) ...[
                  _field(_endpointController, 'Endpoint', requiredField: true),
                  _field(_apiKeyController, 'API Key', obscureText: true),
                  _field(
                    _modelController,
                    '模型 / Locale',
                    requiredField: requireModel,
                  ),
                  _field(_voiceController, '音色', requiredField: requireVoice),
                ] else ...[
                  _field(_modelController, '语言标签（可选，例如 zh-CN）'),
                  _field(_voiceController, '系统声音名称（可选）'),
                ],
                if (isHttpTts) ...[
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
                    requiredField:
                        isHttpTts &&
                        _httpMethodController.text.trim().toUpperCase() ==
                            'POST',
                    minLines: 4,
                  ),
                  _field(_headersController, 'Headers JSON', minLines: 3),
                  _field(_responsePipelineController, '响应管道 JSON', minLines: 4),
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
        FilledButton(onPressed: _submit, child: const Text('保存')),
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
    final isHttpTts = _isHttpProviderType(_providerType);
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
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('$error')));
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
