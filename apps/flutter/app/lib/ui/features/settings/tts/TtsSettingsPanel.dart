// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../common/components/M3LoadingIndicator.dart';

class TtsSettingsPanel extends StatefulWidget {
  const TtsSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<TtsSettingsPanel> createState() => _TtsSettingsPanelState();
}

class TtsConfigManagementController {
  VoidCallback? _create;

  void create() => _create?.call();
}

class TtsConfigManagementSection extends StatelessWidget {
  const TtsConfigManagementSection({
    super.key,
    required this.clients,
    this.embedded = true,
    this.controller,
  });

  final GeneratedCoreProxyClients clients;
  final bool embedded;
  final TtsConfigManagementController? controller;

  @override
  Widget build(BuildContext context) {
    return _TtsConfigManagementView(
      clients: clients,
      embedded: embedded,
      controller: controller,
    );
  }
}

class _TtsSettingsPanelState extends State<TtsSettingsPanel> {
  @override
  Widget build(BuildContext context) {
    return TtsConfigManagementSection(clients: widget.clients, embedded: false);
  }
}

class _TtsConfigManagementView extends StatefulWidget {
  const _TtsConfigManagementView({
    required this.clients,
    required this.embedded,
    required this.controller,
  });

  final GeneratedCoreProxyClients clients;
  final bool embedded;
  final TtsConfigManagementController? controller;

  @override
  State<_TtsConfigManagementView> createState() =>
      _TtsConfigManagementViewState();
}

class _TtsConfigManagementViewState extends State<_TtsConfigManagementView> {
  Future<List<_TtsConfig>>? _future;

  @override
  void initState() {
    super.initState();
    _registerController();
    _reload();
  }

  @override
  void didUpdateWidget(covariant _TtsConfigManagementView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller?._create = null;
      _registerController();
    }
  }

  @override
  void dispose() {
    widget.controller?._create = null;
    super.dispose();
  }

  void _registerController() {
    widget.controller?._create = () {
      _openEditor();
    };
  }

  void _reload() {
    setState(() {
      _future = _loadConfigs();
    });
  }

  Future<List<_TtsConfig>> _loadConfigs() async {
    final value = await widget.clients.bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('preferences.ttsConfigManager'),
        methodName: 'getAllTtsConfigs',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>)
        .map((item) => _TtsConfig.fromJson(item as Map<String, Object?>))
        .toList(growable: false);
  }

  Future<void> _save(_TtsConfig config) async {
    final methodName = config.id.isEmpty ? 'createTtsConfig' : 'updateTtsConfig';
    await widget.clients.bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('preferences.ttsConfigManager'),
        methodName: methodName,
        args: <String, Object?>{'config': config.toJson()},
      ),
    );
    _reload();
  }

  Future<void> _delete(_TtsConfig config) async {
    await widget.clients.bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('preferences.ttsConfigManager'),
        methodName: 'deleteTtsConfig',
        args: <String, Object?>{'id': config.id},
      ),
    );
    _reload();
  }

  Future<void> _openEditor([_TtsConfig? config]) async {
    final edited = await _TtsConfigDialog.show(context: context, config: config);
    if (edited == null) {
      return;
    }
    await _save(edited);
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<List<_TtsConfig>>(
      future: _future,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(child: M3LoadingIndicator());
        }
        if (snapshot.hasError) {
          return Center(child: Text('TTS 配置加载失败：${snapshot.error}'));
        }
        final configs = snapshot.data ?? const <_TtsConfig>[];
        final children = <Widget>[
          if (!widget.embedded) ...[
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    '共享 TTS 配置',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                ),
                FilledButton.icon(
                  onPressed: () => _openEditor(),
                  icon: const Icon(Icons.add),
                  label: const Text('新建'),
                ),
              ],
            ),
            const SizedBox(height: 12),
          ],
          if (configs.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Text(
                '暂无 TTS 配置。角色卡需要显式绑定这里的配置。',
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            )
          else
            for (final config in configs)
              Card.filled(
                margin: const EdgeInsets.only(bottom: 8),
                child: ListTile(
                  contentPadding: const EdgeInsets.only(left: 16, right: 8),
                  title: Text(config.name),
                  subtitle: Text(
                    '${config.providerType} · ${config.model} · ${config.voice} · ${config.enabled ? '启用' : '禁用'}',
                  ),
                  trailing: Wrap(
                    spacing: 4,
                    children: <Widget>[
                      IconButton(
                        tooltip: '编辑',
                        icon: const Icon(Icons.edit_outlined),
                        onPressed: () => _openEditor(config),
                      ),
                      IconButton(
                        tooltip: '删除',
                        icon: const Icon(Icons.delete_outline),
                        onPressed: () => _delete(config),
                      ),
                    ],
                  ),
                ),
              ),
        ];
        if (widget.embedded) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: children,
          );
        }
        return ListView(
          padding: const EdgeInsets.all(16),
          children: <Widget>[
            ...children,
          ],
        );
      },
    );
  }
}

class _TtsConfigDialog extends StatefulWidget {
  const _TtsConfigDialog({this.config});

  final _TtsConfig? config;

  static Future<_TtsConfig?> show({
    required BuildContext context,
    _TtsConfig? config,
  }) {
    return showDialog<_TtsConfig>(
      context: context,
      builder: (context) => _TtsConfigDialog(config: config),
    );
  }

  @override
  State<_TtsConfigDialog> createState() => _TtsConfigDialogState();
}

class _TtsConfigDialogState extends State<_TtsConfigDialog> {
  final _formKey = GlobalKey<FormState>();
  String? _speedError;
  late String _providerType;
  late final TextEditingController _nameController;
  late final TextEditingController _endpointController;
  late final TextEditingController _apiKeyController;
  late final TextEditingController _modelController;
  late final TextEditingController _voiceController;
  late final TextEditingController _formatController;
  late final TextEditingController _speedController;
  late bool _enabled;

  @override
  void initState() {
    super.initState();
    final config = widget.config;
    _providerType = config?.providerType ?? 'SYSTEM_TTS';
    _nameController = TextEditingController(text: config?.name ?? '系统 TTS');
    _endpointController = TextEditingController(
      text: config?.endpoint ?? 'https://api.openai.com/v1/audio/speech',
    );
    _apiKeyController = TextEditingController(text: config?.apiKey ?? '');
    _modelController = TextEditingController(text: config?.model ?? '');
    _voiceController = TextEditingController(text: config?.voice ?? '');
    _formatController = TextEditingController(text: config?.responseFormat ?? 'wav');
    _speedController = TextEditingController(text: '${config?.speed ?? 1.0}');
    _enabled = config?.enabled ?? true;
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
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.config == null ? '新建 TTS 配置' : '编辑 TTS 配置'),
      content: SingleChildScrollView(
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              DropdownButtonFormField<String>(
                value: _providerType,
                decoration: const InputDecoration(labelText: '类型'),
                items: const <DropdownMenuItem<String>>[
                  DropdownMenuItem(value: 'SYSTEM_TTS', child: Text('系统 TTS')),
                  DropdownMenuItem(
                    value: 'OPENAI_COMPATIBLE',
                    child: Text('OpenAI 兼容'),
                  ),
                ],
                onChanged: (value) {
                  if (value == null) {
                    return;
                  }
                  setState(() {
                    _providerType = value;
                    if (value == 'SYSTEM_TTS') {
                      _formatController.text = 'wav';
                    }
                  });
                },
              ),
              const SizedBox(height: 10),
              _field(_nameController, '名称', requiredField: true),
              if (_providerType == 'OPENAI_COMPATIBLE') ...[
                _field(_endpointController, 'Endpoint', requiredField: true),
                _field(_apiKeyController, 'API Key', obscureText: true),
                _field(_modelController, '模型', requiredField: true),
                _field(_voiceController, '声音', requiredField: true),
              ] else ...[
                _field(_modelController, '语言标签（可选，例如 zh-CN）'),
                _field(_voiceController, '系统声音名称（可选）'),
              ],
              _field(_formatController, '音频格式', requiredField: true),
              _field(_speedController, '速度', requiredField: true, numberField: true),
              SwitchListTile(
                title: const Text('启用'),
                value: _enabled,
                onChanged: (value) => setState(() => _enabled = value),
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
  }) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: TextFormField(
        controller: controller,
        decoration: InputDecoration(labelText: label, errorText: numberField ? _speedError : null),
        obscureText: obscureText,
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
    setState(() => _speedError = null);
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    final speed = double.parse(_speedController.text.trim());
    final now = DateTime.now().millisecondsSinceEpoch;
    final current = widget.config;
    Navigator.of(context).pop(
      _TtsConfig(
        id: current?.id ?? '',
        name: _nameController.text.trim(),
        providerType: _providerType,
        endpoint: _endpointController.text.trim(),
        apiKey: _apiKeyController.text.trim(),
        model: _modelController.text.trim(),
        voice: _voiceController.text.trim(),
        responseFormat: _formatController.text.trim(),
        speed: speed,
        enabled: _enabled,
        createdAt: current?.createdAt ?? now,
        updatedAt: now,
      ),
    );
  }
}

class _TtsConfig {
  const _TtsConfig({
    required this.id,
    required this.name,
    required this.providerType,
    required this.endpoint,
    required this.apiKey,
    required this.model,
    required this.voice,
    required this.responseFormat,
    required this.speed,
    required this.enabled,
    required this.createdAt,
    required this.updatedAt,
  });

  factory _TtsConfig.fromJson(Map<String, Object?> json) {
    return _TtsConfig(
      id: json['id'] as String,
      name: json['name'] as String,
      providerType: json['providerType'] as String,
      endpoint: json['endpoint'] as String,
      apiKey: json['apiKey'] as String,
      model: json['model'] as String,
      voice: json['voice'] as String,
      responseFormat: json['responseFormat'] as String,
      speed: (json['speed'] as num).toDouble(),
      enabled: json['enabled'] as bool,
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
    );
  }

  final String id;
  final String name;
  final String providerType;
  final String endpoint;
  final String apiKey;
  final String model;
  final String voice;
  final String responseFormat;
  final double speed;
  final bool enabled;
  final int createdAt;
  final int updatedAt;

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'providerType': providerType,
      'endpoint': endpoint,
      'apiKey': apiKey,
      'model': model,
      'voice': voice,
      'responseFormat': responseFormat,
      'speed': speed,
      'enabled': enabled,
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }
}

String _requestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';
