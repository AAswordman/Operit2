// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:file_selector/file_selector.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';

/// Adds and edits the same portable a2aAgents manifest that the marketplace installs.
class A2AImportDialog extends StatefulWidget {
  const A2AImportDialog({
    super.key,
    required this.clients,
    this.id,
    this.configJson,
  });
  final GeneratedCoreProxyClients clients;
  final String? id;
  final String? configJson;

  @override
  State<A2AImportDialog> createState() => _A2AImportDialogState();
}

class _A2AImportDialogState extends State<A2AImportDialog> {
  final _id = TextEditingController();
  final _url = TextEditingController();
  final _headers = TextEditingController(text: '{}');
  final _manifest = TextEditingController();
  Map<String, dynamic>? _card;
  Map<String, dynamic> _savedConfig = <String, dynamic>{};
  bool _busy = false;
  bool _advanced = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _id.text = widget.id ?? '';
    if (widget.configJson != null) {
      _savedConfig = jsonDecode(widget.configJson!) as Map<String, dynamic>;
      _url.text = _savedConfig['url'] as String? ?? '';
      _headers.text = const JsonEncoder.withIndent(
        '  ',
      ).convert(_savedConfig['headers'] ?? <String, String>{});
      _card = _savedConfig['card'] as Map<String, dynamic>?;
    }
  }

  @override
  void dispose() {
    for (final controller in [_id, _url, _headers, _manifest]) {
      controller.dispose();
    }
    super.dispose();
  }

  Map<String, dynamic> _config() {
    final headers = jsonDecode(_headers.text);
    if (headers is! Map<String, dynamic> ||
        headers.values.any((v) => v is! String)) {
      throw const FormatException('请求头必须是 JSON 字符串映射');
    }
    return <String, dynamic>{
      ..._savedConfig,
      'url': _url.text.trim(),
      'headers': headers,
      'disabled': _savedConfig['disabled'] ?? false,
      'card': _card,
    };
  }

  Future<void> _run(Future<void> Function() action) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await action();
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _discover() => _run(() async {
    final text = await widget.clients.application.discoverA2AAgent(
      configJson: jsonEncode(_config()),
    );
    final config = jsonDecode(text) as Map<String, dynamic>;
    if (mounted) setState(() => _card = config['card'] as Map<String, dynamic>);
  });

  Future<void> _save() => _run(() async {
    final text = _advanced
        ? _manifest.text
        : jsonEncode(<String, dynamic>{
            'a2aAgents': <String, dynamic>{_id.text.trim(): _config()},
          });
    await widget.clients.application.importA2AConfig(configJson: text);
    if (mounted) Navigator.of(context).pop(true);
  });

  Future<void> _importFile() => _run(() async {
    final file = await openFile(
      acceptedTypeGroups: const [
        XTypeGroup(label: 'A2A', extensions: ['json', 'a2a']),
      ],
    );
    if (file == null) return;
    final text = await file.readAsString();
    if (mounted) {
      setState(() {
        _advanced = true;
        _manifest.text = text;
      });
    }
  });

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.id == null ? '添加 A2A' : '编辑 A2A'),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              const Text('连接其他 Agent，协作完成任务。连接会作为扩展包统一管理。'),
              const SizedBox(height: 16),
              if (widget.id == null) ...[
                SegmentedButton<bool>(
                  segments: const [
                    ButtonSegment(value: false, label: Text('服务地址')),
                    ButtonSegment(value: true, label: Text('JSON 配置')),
                  ],
                  selected: {_advanced},
                  onSelectionChanged: _busy
                      ? null
                      : (value) => setState(() => _advanced = value.first),
                ),
                const SizedBox(height: 16),
              ],
              if (_advanced)
                TextField(
                  controller: _manifest,
                  enabled: !_busy,
                  minLines: 8,
                  maxLines: 16,
                  decoration: const InputDecoration(
                    labelText: 'A2A 配置',
                    hintText:
                        '{"a2aAgents":{"research":{"url":"https://example.com","headers":{}}}}',
                    border: OutlineInputBorder(),
                  ),
                )
              else ...[
                TextField(
                  controller: _id,
                  enabled: !_busy && widget.id == null,
                  decoration: const InputDecoration(
                    labelText: 'ID',
                    helperText: '字母、数字、点、下划线或短横线；包名为 a2a.ID',
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: _url,
                  enabled: !_busy,
                  onChanged: (_) => setState(() => _card = null),
                  decoration: const InputDecoration(
                    labelText: '服务地址或 Agent Card 地址',
                    hintText: 'https://example.com',
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: _headers,
                  enabled: !_busy,
                  minLines: 2,
                  maxLines: 5,
                  autocorrect: false,
                  enableSuggestions: false,
                  decoration: const InputDecoration(
                    labelText: '请求头（JSON）',
                    helperText: '可填写 Authorization 等认证请求头',
                  ),
                ),
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  onPressed: _busy ? null : _discover,
                  icon: const Icon(Icons.refresh_rounded),
                  label: const Text('读取 Agent Card'),
                ),
                if (_card != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    '${_card!['name'] ?? ''}',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  Text('${_card!['description'] ?? ''}'),
                  for (final skill
                      in (_card!['skills'] as List<dynamic>? ?? []))
                    Padding(
                      padding: const EdgeInsets.only(top: 6),
                      child: Text(
                        '${skill['name'] ?? ''}：${skill['description'] ?? ''}',
                      ),
                    ),
                ],
              ],
              if (widget.id == null)
                TextButton.icon(
                  onPressed: _busy ? null : _importFile,
                  icon: const Icon(Icons.file_open_outlined),
                  label: const Text('从配置文件导入'),
                ),
              if (_error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: SelectableText(
                    _error!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _busy ? null : _save,
          child: Text(_busy ? '处理中…' : '保存'),
        ),
      ],
    );
  }
}
