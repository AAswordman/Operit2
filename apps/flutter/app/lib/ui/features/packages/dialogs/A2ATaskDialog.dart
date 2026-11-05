// ignore_for_file: file_names

import 'dart:convert';
import 'package:flutter/material.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';

/// Manual invocation uses the same send/get/cancel operations as AI package calls.
class A2ATaskDialog extends StatefulWidget {
  const A2ATaskDialog({super.key, required this.clients, required this.id});
  final GeneratedCoreProxyClients clients;
  final String id;
  @override
  State<A2ATaskDialog> createState() => _A2ATaskDialogState();
}

class _A2ATaskDialogState extends State<A2ATaskDialog> {
  final _message = TextEditingController();
  final _task = TextEditingController();
  final _context = TextEditingController();
  bool _busy = false;
  String _result = '';

  @override
  void dispose() {
    _message.dispose();
    _task.dispose();
    _context.dispose();
    super.dispose();
  }

  Future<void> _invoke(String operation) async {
    setState(() => _busy = true);
    try {
      final text = await widget.clients.application.invokeA2A(
        id: widget.id,
        operation: operation,
        argumentsJson: jsonEncode({
          'message': _message.text,
          'task_id': _task.text.trim(),
          'context_id': _context.text.trim(),
        }),
      );
      final result = jsonDecode(text) as Map<String, dynamic>;
      final payload =
          (result['task'] ?? result['message'] ?? result)
              as Map<String, dynamic>;
      if (!mounted) return;
      setState(() {
        _result = const JsonEncoder.withIndent('  ').convert(result);
        _task.text =
            '${payload['taskId'] ?? (payload['status'] != null ? payload['id'] : null) ?? _task.text}';
        _context.text = '${payload['contextId'] ?? _context.text}';
      });
    } catch (error) {
      if (mounted) setState(() => _result = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('A2A 任务'),
    content: SizedBox(
      width: 600,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _message,
              enabled: !_busy,
              minLines: 3,
              maxLines: 6,
              decoration: const InputDecoration(labelText: '任务或补充消息'),
            ),
            TextField(
              controller: _task,
              enabled: !_busy,
              decoration: const InputDecoration(labelText: '任务 ID（新任务可留空）'),
            ),
            TextField(
              controller: _context,
              enabled: !_busy,
              decoration: const InputDecoration(labelText: '上下文 ID（可选）'),
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                FilledButton(
                  onPressed: _busy ? null : () => _invoke('send_message'),
                  child: const Text('发送'),
                ),
                OutlinedButton(
                  onPressed: _busy ? null : () => _invoke('get_task'),
                  child: const Text('查询任务'),
                ),
                OutlinedButton(
                  onPressed: _busy ? null : () => _invoke('cancel_task'),
                  child: const Text('请求取消'),
                ),
              ],
            ),
            if (_busy)
              const Padding(
                padding: EdgeInsets.only(top: 12),
                child: LinearProgressIndicator(),
              ),
            if (_result.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: SelectableText(_result),
              ),
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('关闭'),
      ),
    ],
  );
}
