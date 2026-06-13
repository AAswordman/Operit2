// ignore_for_file: file_names

import 'dart:convert';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../theme/OperitFormStyles.dart';

class MCPImportResult {
  const MCPImportResult({required this.message});

  final String message;
}

class MCPImportDialog extends StatefulWidget {
  const MCPImportDialog({super.key, required this.clients});

  final GeneratedCoreProxyClients clients;

  @override
  State<MCPImportDialog> createState() => _MCPImportDialogState();
}

enum _MCPImportMode { zip, github, config, form }

class _MCPImportDialogState extends State<MCPImportDialog> {
  final _formKey = GlobalKey<FormState>();
  final _configFormPaneKey = GlobalKey<_MCPFormConfigPaneState>();
  final _pluginIdController = TextEditingController();
  final _repoUrlController = TextEditingController();
  final _nameController = TextEditingController();
  final _mergeConfigController = TextEditingController();

  _MCPImportMode _mode = _MCPImportMode.zip;
  XFile? _zipFile;
  bool _busy = false;

  @override
  void dispose() {
    _pluginIdController.dispose();
    _repoUrlController.dispose();
    _nameController.dispose();
    _mergeConfigController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return AlertDialog(
      icon: const Icon(Icons.cloud_outlined),
      title: const Text('添加 MCP'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              SegmentedButton<_MCPImportMode>(
                segments: const <ButtonSegment<_MCPImportMode>>[
                  ButtonSegment<_MCPImportMode>(
                    value: _MCPImportMode.zip,
                    icon: Icon(Icons.archive_outlined),
                    label: Text('ZIP'),
                  ),
                  ButtonSegment<_MCPImportMode>(
                    value: _MCPImportMode.github,
                    icon: Icon(Icons.code),
                    label: Text('GitHub'),
                  ),
                  ButtonSegment<_MCPImportMode>(
                    value: _MCPImportMode.config,
                    icon: Icon(Icons.data_object),
                    label: Text('配置'),
                  ),
                  ButtonSegment<_MCPImportMode>(
                    value: _MCPImportMode.form,
                    icon: Icon(Icons.tune_outlined),
                    label: Text('表单'),
                  ),
                ],
                selected: <_MCPImportMode>{_mode},
                onSelectionChanged: _busy
                    ? null
                    : (value) {
                        setState(() {
                          _mode = value.single;
                        });
                      },
              ),
              const SizedBox(height: 16),
              AnimatedSwitcher(
                duration: const Duration(milliseconds: 160),
                child: _mode == _MCPImportMode.config
                    ? _JsonMergePane(
                        key: const ValueKey<_MCPImportMode>(
                          _MCPImportMode.config,
                        ),
                        controller: _mergeConfigController,
                        enabled: !_busy,
                      )
                    : _mode == _MCPImportMode.form
                    ? MCPFormConfigPane(
                        key: _configFormPaneKey,
                        enabled: !_busy,
                        onMergeForm: _mergeFormConfig,
                      )
                    : Form(
                        key: _formKey,
                        child: Column(
                          key: ValueKey<_MCPImportMode>(_mode),
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            if (_mode == _MCPImportMode.zip)
                              _ZipPickerRow(
                                file: _zipFile,
                                enabled: !_busy,
                                onPick: _pickZip,
                              )
                            else
                              TextFormField(
                                controller: _repoUrlController,
                                enabled: !_busy,
                                decoration: const InputDecoration(
                                  labelText: 'GitHub 仓库 URL',
                                  prefixIcon: Icon(Icons.link),
                                ),
                                validator: _required,
                              ),
                            const SizedBox(height: 12),
                            TextFormField(
                              controller: _pluginIdController,
                              enabled: !_busy,
                              decoration: const InputDecoration(
                                labelText: 'MCP ID',
                                prefixIcon: Icon(Icons.tag),
                              ),
                              validator: _required,
                            ),
                            const SizedBox(height: 12),
                            TextFormField(
                              controller: _nameController,
                              enabled: !_busy,
                              decoration: const InputDecoration(
                                labelText: '名称',
                                prefixIcon: Icon(Icons.title),
                              ),
                              validator: _required,
                            ),
                            const SizedBox(height: 12),
                            Text(
                              '简介会在启动并获取工具后生成。',
                              style: Theme.of(context).textTheme.bodySmall
                                  ?.copyWith(
                                    color: Theme.of(
                                      context,
                                    ).colorScheme.onSurfaceVariant,
                                  ),
                            ),
                          ],
                        ),
                      ),
              ),
              if (_busy) ...<Widget>[
                const SizedBox(height: 16),
                LinearProgressIndicator(
                  minHeight: 2,
                  color: colorScheme.primary,
                ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
        if (_mode == _MCPImportMode.zip || _mode == _MCPImportMode.github)
          FilledButton(
            onPressed: _busy ? null : _installPlugin,
            child: const Text('安装'),
          )
        else
          FilledButton(
            onPressed: _busy
                ? null
                : _mode == _MCPImportMode.config
                ? _mergeConfig
                : _mergeFormConfigFromPane,
            child: const Text('合并'),
          ),
      ],
    );
  }

  Future<void> _pickZip() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(label: 'Zip', extensions: <String>['zip']),
      ],
    );
    if (file == null) {
      return;
    }
    setState(() {
      _zipFile = file;
    });
  }

  Future<void> _mergeConfig() async {
    final jsonConfig = _mergeConfigController.text.trim();
    if (jsonConfig.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('请粘贴 MCP 配置'),
          behavior: SnackBarBehavior.floating,
        ),
      );
      return;
    }
    await _run(() async {
      final count = await widget.clients.mcpLocalServer.mergeConfigFromJson(
        jsonConfig: jsonConfig,
      );
      return '已导入 $count 个 MCP 服务';
    });
  }

  Future<void> _mergeFormConfig(String jsonConfig) async {
    await _run(() async {
      final count = await widget.clients.mcpLocalServer.mergeConfigFromJson(
        jsonConfig: jsonConfig,
      );
      return '已导入 $count 个 MCP 服务';
    });
  }

  Future<void> _mergeFormConfigFromPane() async {
    await _configFormPaneKey.currentState?.merge();
  }

  Future<void> _installPlugin() async {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    if (_mode == _MCPImportMode.zip && _zipFile == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('请选择 ZIP 文件'),
          behavior: SnackBarBehavior.floating,
        ),
      );
      return;
    }
    await _run(() {
      final pluginId = _pluginIdController.text.trim();
      final name = _nameController.text.trim();
      if (_mode == _MCPImportMode.zip) {
        return widget.clients.mcpRepository.installMcpServerFromZipForFlutter(
          pluginId: pluginId,
          zipPath: _zipFile!.path,
          name: name,
          description: '',
          mcpConfig: '',
        );
      }
      return widget.clients.mcpRepository.installMcpServerWithObjectForFlutter(
        pluginId: pluginId,
        repoUrl: _repoUrlController.text.trim(),
        name: name,
        description: '',
        mcpConfig: '',
      );
    }, successMessage: '已安装 MCP');
  }

  Future<void> _run(
    Future<String> Function() action, {
    String? successMessage,
  }) async {
    setState(() {
      _busy = true;
    });
    try {
      final result = await action();
      if (!mounted) {
        return;
      }
      Navigator.of(
        context,
      ).pop(MCPImportResult(message: successMessage ?? result));
    } catch (error, stackTrace) {
      debugPrint('Failed to import MCP: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _busy = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  String? _required(String? value) {
    return value == null || value.trim().isEmpty ? '必填' : null;
  }
}

class _ZipPickerRow extends StatelessWidget {
  const _ZipPickerRow({
    required this.file,
    required this.enabled,
    required this.onPick,
  });

  final XFile? file;
  final bool enabled;
  final VoidCallback onPick;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OutlinedButton.icon(
      onPressed: enabled ? onPick : null,
      icon: const Icon(Icons.folder_zip_outlined),
      label: Align(
        alignment: Alignment.centerLeft,
        child: Text(
          file?.name ?? '选择 ZIP 文件',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(
            color: file == null ? colorScheme.onSurfaceVariant : null,
          ),
        ),
      ),
    );
  }
}

class MCPFormConfigPane extends StatefulWidget {
  const MCPFormConfigPane({
    super.key,
    required this.enabled,
    required this.onMergeForm,
  });

  final bool enabled;
  final Future<void> Function(String jsonConfig) onMergeForm;

  @override
  State<MCPFormConfigPane> createState() => _MCPFormConfigPaneState();
}

class _MCPFormConfigPaneState extends State<MCPFormConfigPane> {
  final _formKey = GlobalKey<FormState>();
  final _serverIdController = TextEditingController();
  final _commandController = TextEditingController();
  final _argsController = TextEditingController();
  final _urlController = TextEditingController();
  final _headersController = TextEditingController();
  final _envController = TextEditingController();

  bool _remote = false;
  String _type = 'streamable-http';

  @override
  void dispose() {
    _serverIdController.dispose();
    _commandController.dispose();
    _argsController.dispose();
    _urlController.dispose();
    _headersController.dispose();
    _envController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Form(
          key: _formKey,
          child: _FormConfigPane(
            remote: _remote,
            type: _type,
            enabled: widget.enabled,
            serverIdController: _serverIdController,
            commandController: _commandController,
            argsController: _argsController,
            urlController: _urlController,
            headersController: _headersController,
            envController: _envController,
            onRemoteChanged: (value) {
              setState(() {
                _remote = value;
              });
            },
            onTypeChanged: (value) {
              setState(() {
                _type = value;
              });
            },
          ),
        ),
      ],
    );
  }

  Future<void> merge() async {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    final serverId = _serverIdController.text.trim();
    final config = <String, Object?>{
      'mcpServers': <String, Object?>{
        serverId: _remote
            ? <String, Object?>{
                'url': _urlController.text.trim(),
                'type': _type,
                'headers': _parseMapLines(_headersController.text),
                'disabled': false,
                'autoApprove': <String>[],
              }
            : <String, Object?>{
                'command': _commandController.text.trim(),
                'args': _lineList(_argsController.text),
                'env': _parseMapLines(_envController.text),
                'disabled': false,
                'autoApprove': <String>[],
              },
      },
    };
    await widget.onMergeForm(_jsonEncode(config));
  }

  List<String> _lineList(String value) {
    return value
        .split('\n')
        .map((line) => line.trim())
        .where((line) => line.isNotEmpty)
        .toList(growable: false);
  }

  Map<String, String> _parseMapLines(String value) {
    final map = <String, String>{};
    for (final line in value.split('\n')) {
      final trimmed = line.trim();
      if (trimmed.isEmpty) {
        continue;
      }
      final separator = trimmed.indexOf(':');
      if (separator <= 0) {
        continue;
      }
      final key = trimmed.substring(0, separator).trim();
      final itemValue = trimmed.substring(separator + 1).trim();
      if (key.isNotEmpty) {
        map[key] = itemValue;
      }
    }
    return map;
  }

  String _jsonEncode(Object? value) {
    return const JsonEncoder.withIndent('  ').convert(value);
  }
}

class _JsonMergePane extends StatelessWidget {
  const _JsonMergePane({
    super.key,
    required this.controller,
    required this.enabled,
  });

  final TextEditingController controller;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        TextField(
          controller: controller,
          enabled: enabled,
          minLines: 8,
          maxLines: 14,
          decoration: const InputDecoration(
            labelText: 'MCP 配置',
            hintText: '{\n  "mcpServers": {\n    ...\n  }\n}',
            alignLabelWithHint: true,
          ),
        ),
      ],
    );
  }
}

class _FormConfigPane extends StatelessWidget {
  const _FormConfigPane({
    required this.remote,
    required this.type,
    required this.enabled,
    required this.serverIdController,
    required this.commandController,
    required this.argsController,
    required this.urlController,
    required this.headersController,
    required this.envController,
    required this.onRemoteChanged,
    required this.onTypeChanged,
  });

  final bool remote;
  final String type;
  final bool enabled;
  final TextEditingController serverIdController;
  final TextEditingController commandController;
  final TextEditingController argsController;
  final TextEditingController urlController;
  final TextEditingController headersController;
  final TextEditingController envController;
  final ValueChanged<bool> onRemoteChanged;
  final ValueChanged<String> onTypeChanged;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        SegmentedButton<bool>(
          segments: const <ButtonSegment<bool>>[
            ButtonSegment<bool>(
              value: false,
              icon: Icon(Icons.terminal_outlined),
              label: Text('本地'),
            ),
            ButtonSegment<bool>(
              value: true,
              icon: Icon(Icons.public_outlined),
              label: Text('远程'),
            ),
          ],
          selected: <bool>{remote},
          onSelectionChanged: enabled
              ? (value) => onRemoteChanged(value.single)
              : null,
        ),
        const SizedBox(height: 12),
        TextFormField(
          controller: serverIdController,
          enabled: enabled,
          decoration: const InputDecoration(
            labelText: '服务 ID',
            prefixIcon: Icon(Icons.tag),
          ),
          validator: _required,
        ),
        const SizedBox(height: 12),
        if (remote) ...<Widget>[
          TextFormField(
            controller: urlController,
            enabled: enabled,
            decoration: const InputDecoration(
              labelText: 'URL',
              prefixIcon: Icon(Icons.link),
            ),
            validator: _required,
          ),
          const SizedBox(height: 12),
          DropdownButtonFormField<String>(
            initialValue: type,
            style: OperitFormStyles.dropdownTextStyle(context),
            decoration: const InputDecoration(labelText: '传输'),
            items: const <DropdownMenuItem<String>>[
              DropdownMenuItem<String>(
                value: 'streamable-http',
                child: Text('streamable-http'),
              ),
              DropdownMenuItem<String>(value: 'sse', child: Text('sse')),
            ],
            onChanged: enabled
                ? (value) {
                    if (value != null) {
                      onTypeChanged(value);
                    }
                  }
                : null,
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: headersController,
            enabled: enabled,
            minLines: 2,
            maxLines: 4,
            decoration: const InputDecoration(
              labelText: 'Headers',
              helperText: '每行一个，格式：Name: Value',
              alignLabelWithHint: true,
            ),
          ),
        ] else ...<Widget>[
          TextFormField(
            controller: commandController,
            enabled: enabled,
            decoration: const InputDecoration(
              labelText: '命令',
              prefixIcon: Icon(Icons.terminal_outlined),
            ),
            validator: _required,
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: argsController,
            enabled: enabled,
            minLines: 2,
            maxLines: 4,
            decoration: const InputDecoration(
              labelText: '参数',
              helperText: '每行一个参数',
              alignLabelWithHint: true,
            ),
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: envController,
            enabled: enabled,
            minLines: 2,
            maxLines: 4,
            decoration: const InputDecoration(
              labelText: '环境变量',
              helperText: '每行一个，格式：Name: Value',
              alignLabelWithHint: true,
            ),
          ),
        ],
      ],
    );
  }

  String? _required(String? value) {
    return value == null || value.trim().isEmpty ? '必填' : null;
  }
}
