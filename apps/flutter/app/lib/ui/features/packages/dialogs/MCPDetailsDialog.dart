// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../theme/OperitFormStyles.dart';
import '../utils/MCPCommandRunner.dart';
import 'MCPToolRunDialog.dart';

class MCPDetailsDialog extends StatefulWidget {
  const MCPDetailsDialog({
    super.key,
    required this.serverId,
    required this.metadata,
    required this.server,
    required this.status,
    required this.clients,
    required this.onConfigSaved,
  });

  final String serverId;
  final core_proxy.PluginMetadata? metadata;
  final core_proxy.ServerConfig? server;
  final core_proxy.ServerStatus? status;
  final GeneratedCoreProxyClients clients;
  final Future<void> Function() onConfigSaved;

  @override
  State<MCPDetailsDialog> createState() => _MCPDetailsDialogState();
}

class _MCPDetailsDialogState extends State<MCPDetailsDialog> {
  bool _busy = false;
  bool _regeneratingDescription = false;
  String? _error;
  core_proxy.PluginMetadata? _metadataOverride;

  core_proxy.PluginMetadata get _metadata {
    final override = _metadataOverride;
    if (override != null) {
      return override;
    }
    final current = widget.metadata;
    if (current != null) {
      return current;
    }
    return core_proxy.PluginMetadata(
      name: widget.serverId,
      description: '',
      author: 'Unknown',
      version: '1.0.0',
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final metadata = _metadata;
    final name = metadata.name.trim().isNotEmpty
        ? metadata.name
        : widget.serverId;
    return AlertDialog(
      icon: const Icon(Icons.extension_outlined),
      title: Text(name),
      contentPadding: const EdgeInsets.fromLTRB(24, 12, 24, 0),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 680, maxHeight: 640),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: <Widget>[
                  _Badge(text: widget.serverId),
                  if (metadata.version.trim().isNotEmpty)
                    _Badge(text: metadata.version),
                  _Badge(
                    text: widget.server?.disabled == true ? '已停用' : '已启用',
                    color: widget.server?.disabled == true
                        ? colorScheme.errorContainer
                        : colorScheme.primaryContainer,
                  ),
                ],
              ),
              const SizedBox(height: 14),
              _Description(
                metadata: metadata,
                onEdit: _busy ? null : () => _showMetadataEditDialog(context),
                onRegenerate: _busy ? null : _regenerateDescription,
                regenerating: _regeneratingDescription,
              ),
              const SizedBox(height: 16),
              Row(
                children: <Widget>[
                  const Expanded(child: _SectionTitle(text: '配置')),
                  TextButton.icon(
                    onPressed: widget.server == null || _busy
                        ? null
                        : () => _showConfigEditDialog(context),
                    icon: const Icon(Icons.edit_outlined),
                    label: const Text('编辑'),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              _ConfigSummary(server: widget.server),
              if (widget.status?.errorMessage?.trim().isNotEmpty ==
                  true) ...<Widget>[
                const SizedBox(height: 12),
                _ErrorCard(message: widget.status!.errorMessage!),
              ],
              if (_error?.trim().isNotEmpty == true) ...<Widget>[
                const SizedBox(height: 12),
                _ErrorCard(message: _error!),
              ],
              const SizedBox(height: 16),
              Row(
                children: <Widget>[
                  Expanded(child: _SectionTitle(text: '工具')),
                  Text(
                    '${widget.status?.cachedTools?.length ?? 0}',
                    style: theme.textTheme.labelMedium?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              if (widget.status?.cachedTools?.isNotEmpty == true)
                for (final tool in widget.status!.cachedTools!)
                  _ToolTile(
                    serverId: widget.serverId,
                    tool: tool,
                    onRun: () => _showRunDialog(context, tool),
                  )
              else
                _EmptyCard(message: '当前没有缓存的工具。'),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton.icon(
          onPressed:
              widget.server == null || widget.server?.disabled == true || _busy
              ? null
              : _startServer,
          icon: const Icon(Icons.play_arrow),
          label: const Text('启动'),
        ),
        TextButton.icon(
          onPressed: widget.server == null || _busy ? null : _killServer,
          icon: const Icon(Icons.stop_circle_outlined),
          label: const Text('Kill'),
        ),
        TextButton.icon(
          onPressed: _busy ? null : _confirmDelete,
          icon: const Icon(Icons.delete_outline),
          label: const Text('删除'),
          style: TextButton.styleFrom(foregroundColor: colorScheme.error),
        ),
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
      ],
    );
  }

  Future<void> _startServer() async {
    await _runLifecycleAction(() {
      return startMcpServer(clients: widget.clients, serverId: widget.serverId);
    });
  }

  Future<void> _killServer() async {
    await _runLifecycleAction(() {
      return killMcpServer(clients: widget.clients, serverId: widget.serverId);
    });
  }

  Future<void> _runLifecycleAction(Future<String> Function() action) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await action();
      await widget.onConfigSaved();
      if (!mounted) {
        return;
      }
      setState(() {
        _busy = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to update MCP lifecycle: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _busy = false;
        _error = error.toString();
      });
    }
  }

  void _showRunDialog(BuildContext context, core_proxy.CachedToolInfo tool) {
    showDialog<void>(
      context: context,
      builder: (context) {
        return MCPToolRunDialog(
          serverId: widget.serverId,
          tool: tool,
          clients: widget.clients,
        );
      },
    );
  }

  void _showConfigEditDialog(BuildContext context) {
    final current = widget.server;
    if (current == null) {
      return;
    }
    showDialog<void>(
      context: context,
      builder: (context) {
        return _MCPConfigEditDialog(
          serverId: widget.serverId,
          server: current,
          clients: widget.clients,
          onSaved: widget.onConfigSaved,
        );
      },
    );
  }

  void _showMetadataEditDialog(BuildContext context) {
    showDialog<void>(
      context: context,
      builder: (context) {
        return _MCPMetadataEditDialog(
          serverId: widget.serverId,
          metadata: _metadata,
          clients: widget.clients,
          onSaved: widget.onConfigSaved,
        );
      },
    );
  }

  Future<void> _regenerateDescription() async {
    setState(() {
      _busy = true;
      _regeneratingDescription = true;
      _error = null;
    });
    try {
      final metadata = _metadata;
      final generated = await widget.clients.application
          .mcpRepository()
          .generatePluginDescription(
            pluginId: widget.serverId,
            pluginName: metadata.name,
          );
      final updatedMetadata = core_proxy.PluginMetadata(
        name: metadata.name,
        description: generated,
        author: metadata.author,
        version: metadata.version,
      );
      await widget.clients.permissionsMcpRuntimeMcpLocalServer
          .addOrUpdatePluginMetadata(
            pluginId: widget.serverId,
            metadata: updatedMetadata,
          );
      await widget.onConfigSaved();
      if (!mounted) {
        return;
      }
      setState(() {
        _metadataOverride = updatedMetadata;
        _busy = false;
        _regeneratingDescription = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to regenerate MCP description: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _busy = false;
        _regeneratingDescription = false;
        _error = error.toString();
      });
    }
  }

  Future<void> _confirmDelete() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) {
        return AlertDialog(
          icon: const Icon(Icons.delete_outline),
          title: Text('删除 ${widget.serverId}'),
          content: const Text('删除后会移除此 MCP 的配置、简介、状态和本地插件文件。'),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('取消'),
            ),
            FilledButton.tonalIcon(
              onPressed: () => Navigator.of(context).pop(true),
              icon: const Icon(Icons.delete_outline),
              label: const Text('删除'),
            ),
          ],
        );
      },
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await killMcpServer(clients: widget.clients, serverId: widget.serverId);
      await widget.clients.permissionsMcpRuntimeMcpLocalServer.removeMcpServer(
        serverId: widget.serverId,
      );
      await widget.onConfigSaved();
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop();
    } catch (error, stackTrace) {
      debugPrint('Failed to delete MCP server: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _busy = false;
        _error = error.toString();
      });
    }
  }
}

class _Description extends StatelessWidget {
  const _Description({
    required this.metadata,
    required this.onEdit,
    required this.onRegenerate,
    required this.regenerating,
  });

  final core_proxy.PluginMetadata metadata;
  final VoidCallback? onEdit;
  final VoidCallback? onRegenerate;
  final bool regenerating;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final description = metadata.description;
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                const Expanded(child: _SectionTitle(text: '简介')),
                IconButton(
                  tooltip: '编辑简介',
                  onPressed: onEdit,
                  icon: const Icon(Icons.edit_note_outlined),
                ),
                IconButton(
                  tooltip: '重新生成简介',
                  onPressed: onRegenerate,
                  icon: regenerating
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.auto_fix_high_outlined),
                ),
              ],
            ),
            const SizedBox(height: 6),
            Text(
              description.trim().isNotEmpty ? description : '暂无简介。',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 12,
              runSpacing: 6,
              children: <Widget>[
                _MetaText(label: '作者', value: metadata.author),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _MCPMetadataEditDialog extends StatefulWidget {
  const _MCPMetadataEditDialog({
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
  State<_MCPMetadataEditDialog> createState() => _MCPMetadataEditDialogState();
}

class _MCPMetadataEditDialogState extends State<_MCPMetadataEditDialog> {
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
    final colorScheme = theme.colorScheme;
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
                      color: colorScheme.error,
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

class _MCPConfigEditDialog extends StatefulWidget {
  const _MCPConfigEditDialog({
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
  State<_MCPConfigEditDialog> createState() => _MCPConfigEditDialogState();
}

class _MCPConfigEditDialogState extends State<_MCPConfigEditDialog> {
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
    final colorScheme = theme.colorScheme;
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
                  DropdownButtonFormField<String>(
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
                      color: colorScheme.error,
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

class _ConfigSummary extends StatelessWidget {
  const _ConfigSummary({required this.server});

  final core_proxy.ServerConfig? server;

  @override
  Widget build(BuildContext context) {
    final rows = <_ConfigRow>[
      if (server?.url?.trim().isNotEmpty == true)
        _ConfigRow('URL', server!.url!),
      if (server?.type?.trim().isNotEmpty == true)
        _ConfigRow('传输', server!.type!),
      if (server != null && server!.command.trim().isNotEmpty)
        _ConfigRow('命令', _commandLine(server!)),
      if (server?.args.isNotEmpty == true)
        _ConfigRow('参数', server!.args.join(' ')),
      if (server?.autoApprove.isNotEmpty == true)
        _ConfigRow('自动批准', server!.autoApprove.join(', ')),
      if (server?.env.isNotEmpty == true)
        _ConfigRow('环境变量', server!.env.keys.join(', ')),
      if (server?.headers.isNotEmpty == true)
        _ConfigRow('Headers', server!.headers.keys.join(', ')),
    ];
    return Card(
      elevation: 0,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          children: <Widget>[
            for (var index = 0; index < rows.length; index++) ...<Widget>[
              _ConfigLine(row: rows[index]),
              if (index != rows.length - 1) const Divider(height: 16),
            ],
            if (rows.isEmpty) const _EmptyLine(text: '没有可显示的配置摘要。'),
          ],
        ),
      ),
    );
  }

  String _commandLine(core_proxy.ServerConfig server) {
    final args = server.args.join(' ');
    if (args.trim().isEmpty) {
      return server.command;
    }
    return '${server.command} $args';
  }
}

class _ToolTile extends StatelessWidget {
  const _ToolTile({
    required this.serverId,
    required this.tool,
    required this.onRun,
  });

  final String serverId;
  final core_proxy.CachedToolInfo tool;
  final VoidCallback onRun;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final parameterNames = _parameterNames(tool.inputSchema);
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.24),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 10, 10, 10),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            const Padding(
              padding: EdgeInsets.only(top: 2),
              child: Icon(Icons.build_outlined, size: 20),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    tool.name,
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 3),
                  SelectableText(
                    'ID: $serverId:${tool.name}',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                  if (tool.description.trim().isNotEmpty) ...<Widget>[
                    const SizedBox(height: 6),
                    Text(
                      tool.description,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                  if (parameterNames.isNotEmpty) ...<Widget>[
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 6,
                      runSpacing: 6,
                      children: <Widget>[
                        for (final name in parameterNames)
                          _Badge(text: name, compact: true),
                      ],
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(width: 8),
            FilledButton.tonalIcon(
              onPressed: onRun,
              icon: const Icon(Icons.play_arrow),
              label: const Text('运行'),
            ),
          ],
        ),
      ),
    );
  }
}

class _ErrorCard extends StatelessWidget {
  const _ErrorCard({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      elevation: 0,
      color: colorScheme.errorContainer.withValues(alpha: 0.45),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(Icons.error_outline, color: colorScheme.error, size: 20),
            const SizedBox(width: 10),
            Expanded(child: SelectableText(message)),
          ],
        ),
      ),
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: Theme.of(
        context,
      ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
    );
  }
}

class _Badge extends StatelessWidget {
  const _Badge({required this.text, this.color, this.compact = false});

  final String text;
  final Color? color;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: EdgeInsets.symmetric(
        horizontal: compact ? 8 : 10,
        vertical: compact ? 4 : 5,
      ),
      decoration: BoxDecoration(
        color: color ?? colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        text,
        style: Theme.of(
          context,
        ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
      ),
    );
  }
}

class _MetaText extends StatelessWidget {
  const _MetaText({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Text(
      '$label: $value',
      style: Theme.of(
        context,
      ).textTheme.bodySmall?.copyWith(color: colorScheme.onSurfaceVariant),
    );
  }
}

class _ConfigLine extends StatelessWidget {
  const _ConfigLine({required this.row});

  final _ConfigRow row;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        SizedBox(
          width: 86,
          child: Text(
            row.label,
            style: theme.textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        Expanded(child: SelectableText(row.value)),
      ],
    );
  }
}

class _EmptyCard extends StatelessWidget {
  const _EmptyCard({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.24),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Center(
          child: Text(
            message,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ),
    );
  }
}

class _EmptyLine extends StatelessWidget {
  const _EmptyLine({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Align(
      alignment: Alignment.centerLeft,
      child: Text(
        text,
        style: Theme.of(
          context,
        ).textTheme.bodySmall?.copyWith(color: colorScheme.onSurfaceVariant),
      ),
    );
  }
}

class _ConfigRow {
  const _ConfigRow(this.label, this.value);

  final String label;
  final String value;
}

List<String> _parameterNames(String inputSchema) {
  final schema = jsonDecode(inputSchema) as Map<String, Object?>;
  final properties =
      (schema['properties'] as Map<Object?, Object?>?) ?? <Object?, Object?>{};
  return properties.keys.map((key) => key as String).toList()..sort();
}
