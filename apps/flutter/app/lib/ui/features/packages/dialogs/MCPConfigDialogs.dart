// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../theme/OperitFormStyles.dart';
import '../utils/MCPCommandRunner.dart';

/// Edits the display metadata the MCP list shows for one server.
class MCPMetadataEditDialog extends StatefulWidget {
  const MCPMetadataEditDialog({
    super.key,
    required this.serverId,
    required this.metadata,
    required this.clients,
    required this.onSaved,
  });

  final String serverId;
  final core_proxy.PluginMetadata metadata;
  final GeneratedCoreProxyClients clients;
  final Future<void> Function() onSaved;

  @override
  State<MCPMetadataEditDialog> createState() => _MCPMetadataEditDialogState();
}

class _MCPMetadataEditDialogState extends State<MCPMetadataEditDialog> {
  late final TextEditingController _nameController;
  late final TextEditingController _descriptionController;
  late final TextEditingController _authorController;
  late final TextEditingController _versionController;
  bool _saving = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController(text: widget.metadata.name);
    _descriptionController = TextEditingController(
      text: widget.metadata.description,
    );
    _authorController = TextEditingController(text: widget.metadata.author);
    _versionController = TextEditingController(text: widget.metadata.version);
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descriptionController.dispose();
    _authorController.dispose();
    _versionController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AlertDialog(
      icon: const Icon(Icons.edit_note_outlined),
      title: Text('编辑 ${widget.serverId}'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              TextField(
                controller: _nameController,
                decoration: const InputDecoration(
                  labelText: '名称',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 10),
              TextField(
                controller: _descriptionController,
                minLines: 3,
                maxLines: 6,
                decoration: const InputDecoration(
                  labelText: '简介',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 10),
              Row(
                children: <Widget>[
                  Expanded(
                    child: TextField(
                      controller: _authorController,
                      decoration: const InputDecoration(
                        labelText: '作者',
                        border: OutlineInputBorder(),
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  SizedBox(
                    width: 150,
                    child: TextField(
                      controller: _versionController,
                      decoration: const InputDecoration(
                        labelText: '版本',
                        border: OutlineInputBorder(),
                      ),
                    ),
                  ),
                ],
              ),
              if (_error != null) ...<Widget>[
                const SizedBox(height: 12),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    _error!,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.error,
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _saving ? null : () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton.icon(
          onPressed: _saving ? null : _save,
          icon: _saving
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.save_outlined),
          label: Text(_saving ? '保存中' : '保存'),
        ),
      ],
    );
  }

  Future<void> _save() async {
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      await widget.clients.permissionsMcpRuntimeMcpLocalServer
          .addOrUpdatePluginMetadata(
            pluginId: widget.serverId,
            metadata: core_proxy.PluginMetadata(
              name: _nameController.text.trim(),
              description: _descriptionController.text.trim(),
              author: _authorController.text.trim(),
              version: _versionController.text.trim(),
            ),
          );
      await widget.onSaved();
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop();
    } catch (error, stackTrace) {
      debugPrint('Failed to save MCP metadata: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _saving = false;
        _error = error.toString();
      });
    }
  }
}

/// Edits how one MCP server is launched or reached and restarts it on save.
class MCPConfigEditDialog extends StatefulWidget {
  const MCPConfigEditDialog({
    super.key,
    required this.serverId,
    required this.server,
    required this.clients,
    required this.onSaved,
  });

  final String serverId;
  final core_proxy.ServerConfig server;
  final GeneratedCoreProxyClients clients;
  final Future<void> Function() onSaved;

  @override
  State<MCPConfigEditDialog> createState() => _MCPConfigEditDialogState();
}

class _MCPConfigEditDialogState extends State<MCPConfigEditDialog> {
  late bool _remote;
  late final TextEditingController _commandController;
  late final TextEditingController _argsController;
  late final TextEditingController _urlController;
  late String _type;
  late final TextEditingController _headersController;
  late final TextEditingController _envController;
  late final TextEditingController _autoApproveController;
  bool _saving = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _remote = widget.server.url?.trim().isNotEmpty == true;
    _commandController = TextEditingController(text: widget.server.command);
    _argsController = TextEditingController(
      text: widget.server.args.join('\n'),
    );
    _urlController = TextEditingController(text: widget.server.url ?? '');
    _type = widget.server.type?.trim().isNotEmpty == true
        ? widget.server.type!
        : 'streamable-http';
    _headersController = TextEditingController(
      text: _mapToLines(widget.server.headers),
    );
    _envController = TextEditingController(
      text: _mapToLines(widget.server.env),
    );
    _autoApproveController = TextEditingController(
      text: widget.server.autoApprove.join('\n'),
    );
  }

  @override
  void dispose() {
    _commandController.dispose();
    _argsController.dispose();
    _urlController.dispose();
    _headersController.dispose();
    _envController.dispose();
    _autoApproveController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AlertDialog(
      icon: const Icon(Icons.tune_outlined),
      title: Text('编辑 ${widget.serverId}'),
      content: SizedBox(
        width: 560,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 620),
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Align(
                  alignment: Alignment.centerLeft,
                  child: SegmentedButton<bool>(
                    segments: const <ButtonSegment<bool>>[
                      ButtonSegment<bool>(
                        value: false,
                        label: Text('本地'),
                        icon: Icon(Icons.terminal_outlined),
                      ),
                      ButtonSegment<bool>(
                        value: true,
                        label: Text('远程'),
                        icon: Icon(Icons.public_outlined),
                      ),
                    ],
                    selected: <bool>{_remote},
                    onSelectionChanged: (value) {
                      setState(() {
                        _remote = value.first;
                      });
                    },
                  ),
                ),
                const SizedBox(height: 16),
                if (_remote) ...<Widget>[
                  TextField(
                    controller: _urlController,
                    decoration: const InputDecoration(
                      labelText: 'URL',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 10),
                  OperitFormStyles.dropdownButtonFormField<String>(
                    context,
                    initialValue: _type,
                    style: OperitFormStyles.dropdownTextStyle(context),
                    decoration: const InputDecoration(
                      labelText: '传输',
                      border: OutlineInputBorder(),
                    ),
                    items: const <DropdownMenuItem<String>>[
                      DropdownMenuItem<String>(
                        value: 'streamable-http',
                        child: Text('streamable-http'),
                      ),
                      DropdownMenuItem<String>(
                        value: 'sse',
                        child: Text('sse'),
                      ),
                    ],
                    onChanged: (value) {
                      if (value == null) {
                        return;
                      }
                      setState(() {
                        _type = value;
                      });
                    },
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: _headersController,
                    minLines: 3,
                    maxLines: 6,
                    decoration: const InputDecoration(
                      labelText: 'Headers',
                      helperText: '每行一个，格式：Name: Value',
                      border: OutlineInputBorder(),
                    ),
                  ),
                ] else ...<Widget>[
                  TextField(
                    controller: _commandController,
                    decoration: const InputDecoration(
                      labelText: '命令',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: _argsController,
                    minLines: 3,
                    maxLines: 6,
                    decoration: const InputDecoration(
                      labelText: '参数',
                      helperText: '每行一个参数',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: _envController,
                    minLines: 3,
                    maxLines: 6,
                    decoration: const InputDecoration(
                      labelText: '环境变量',
                      helperText: '每行一个，格式：Name: Value',
                      border: OutlineInputBorder(),
                    ),
                  ),
                ],
                const SizedBox(height: 10),
                TextField(
                  controller: _autoApproveController,
                  minLines: 2,
                  maxLines: 5,
                  decoration: const InputDecoration(
                    labelText: '自动批准工具',
                    helperText: '每行一个工具名',
                    border: OutlineInputBorder(),
                  ),
                ),
                if (_error != null) ...<Widget>[
                  const SizedBox(height: 12),
                  Text(
                    _error!,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.error,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _saving ? null : () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton.icon(
          onPressed: _saving ? null : _save,
          icon: _saving
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.save_outlined),
          label: Text(_saving ? '保存中' : '保存'),
        ),
      ],
    );
  }

  Future<void> _save() async {
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final headers = _parseMapLines(_headersController.text, 'Headers');
      final env = _parseMapLines(_envController.text, '环境变量');
      final server = core_proxy.ServerConfig(
        command: _remote ? '' : _commandController.text.trim(),
        args: _remote ? <String>[] : _lineList(_argsController.text),
        url: _remote ? _urlController.text.trim() : null,
        type: _remote ? _type : null,
        headers: _remote ? headers : <String, String>{},
        disabled: widget.server.disabled,
        autoApprove: _lineList(_autoApproveController.text),
        env: _remote ? <String, String>{} : env,
      );
      final saved = await widget.clients.permissionsMcpRuntimeMcpLocalServer
          .savePluginConfig(
            pluginId: widget.serverId,
            configJson: jsonEncode(<String, Object?>{
              'mcpServers': <String, Object?>{widget.serverId: server.toJson()},
            }),
          );
      if (!mounted) {
        return;
      }
      if (!saved) {
        setState(() {
          _saving = false;
          _error = '配置未保存，请检查必填字段。';
        });
        return;
      }
      if (server.disabled) {
        await killMcpServer(clients: widget.clients, serverId: widget.serverId);
      } else {
        await restartMcpServer(
          clients: widget.clients,
          serverId: widget.serverId,
        );
      }
      await widget.onSaved();
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _saving = false;
        _error = error.toString();
      });
    }
  }

  List<String> _lineList(String text) {
    return text
        .split('\n')
        .map((line) => line.trim())
        .where((line) => line.isNotEmpty)
        .toList(growable: false);
  }

  Map<String, String> _parseMapLines(String text, String label) {
    final map = <String, String>{};
    for (final rawLine in text.split('\n')) {
      final line = rawLine.trim();
      if (line.isEmpty) {
        continue;
      }
      final separatorIndex = line.indexOf(':');
      if (separatorIndex <= 0) {
        throw '$label 格式错误：$line';
      }
      final key = line.substring(0, separatorIndex).trim();
      final value = line.substring(separatorIndex + 1).trim();
      if (key.isEmpty) {
        throw '$label 名称不能为空：$line';
      }
      map[key] = value;
    }
    return map;
  }

  String _mapToLines(Map<String, String> map) {
    return map.entries
        .map((entry) => '${entry.key}: ${entry.value}')
        .join('\n');
  }
}

/// Lists parameter names declared by an MCP tool's JSON schema.
List<String> mcpToolParameterNames(String inputSchema) {
  final schema = jsonDecode(inputSchema) as Map<String, Object?>;
  final properties =
      (schema['properties'] as Map<Object?, Object?>?) ?? <Object?, Object?>{};
  return properties.keys.map((key) => key as String).toList()..sort();
}
