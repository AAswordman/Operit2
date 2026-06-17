// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../utils/PackageDisplayUtils.dart';

class PackageToolRunDialog extends StatefulWidget {
  const PackageToolRunDialog({
    super.key,
    required this.packageName,
    required this.tool,
    required this.clients,
  });

  final String packageName;
  final core_proxy.PackageTool tool;
  final GeneratedCoreProxyClients clients;

  @override
  State<PackageToolRunDialog> createState() => _PackageToolRunDialogState();
}

class _PackageToolRunDialogState extends State<PackageToolRunDialog> {
  late final Map<String, TextEditingController> _controllers;
  bool _executing = false;
  core_proxy.CoreApiChatEnhanceConversationMarkupManagerToolResult? _result;

  @override
  void initState() {
    super.initState();
    _controllers = <String, TextEditingController>{
      for (final parameter in widget.tool.parameters)
        parameter.name: TextEditingController(),
    };
  }

  @override
  void dispose() {
    for (final controller in _controllers.values) {
      controller.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final toolId = '${widget.packageName}:${widget.tool.name}';
    return AlertDialog(
      icon: const Icon(Icons.play_circle_outline),
      title: Text(widget.tool.name),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560, maxHeight: 620),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                'ID: $toolId',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                localizedText(widget.tool.description),
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              if (widget.tool.parameters.isNotEmpty) ...<Widget>[
                const SizedBox(height: 16),
                Text(
                  '参数',
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 8),
                for (final parameter in widget.tool.parameters) ...<Widget>[
                  TextField(
                    controller: _controllers[parameter.name],
                    decoration: InputDecoration(
                      labelText:
                          '${parameter.name}${parameter.requiredValue ? " *" : ""}',
                      helperText: localizedText(parameter.description),
                      border: const OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 10),
                ],
              ],
              if (_result != null) ...<Widget>[
                const SizedBox(height: 12),
                _ExecutionResultCard(result: _result!),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _executing ? null : () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
        FilledButton.icon(
          onPressed: _executing ? null : _execute,
          icon: _executing
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.play_arrow),
          label: Text(_executing ? '运行中' : '运行'),
        ),
      ],
    );
  }

  Future<void> _execute() async {
    final missing = widget.tool.parameters
        .where((parameter) => parameter.requiredValue)
        .where((parameter) => _controllers[parameter.name]!.text.trim().isEmpty)
        .map((parameter) => parameter.name)
        .toList(growable: false);
    if (missing.isNotEmpty) {
      setState(() {
        _result =
            core_proxy.CoreApiChatEnhanceConversationMarkupManagerToolResult(
              toolName: '${widget.packageName}:${widget.tool.name}',
              success: false,
              result: '',
              error: '缺少必填参数：${missing.join(", ")}',
            );
      });
      return;
    }

    setState(() {
      _executing = true;
      _result = null;
    });
    try {
      final result = await widget.clients.permissionsAiToolHandler.executeTool(
        tool: core_proxy.CoreApiChatEnhanceToolExecutionManagerAiTool(
          name: '${widget.packageName}:${widget.tool.name}',
          parameters: widget.tool.parameters
              .map(
                (parameter) =>
                    core_proxy.CoreApiChatEnhanceToolExecutionManagerToolParameter(
                      name: parameter.name,
                      value: _controllers[parameter.name]!.text,
                    ),
              )
              .toList(growable: false),
        ),
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _result = result;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _result =
            core_proxy.CoreApiChatEnhanceConversationMarkupManagerToolResult(
              toolName: '${widget.packageName}:${widget.tool.name}',
              success: false,
              result: '',
              error: error.toString(),
            );
      });
    } finally {
      if (mounted) {
        setState(() {
          _executing = false;
        });
      }
    }
  }
}

class _ExecutionResultCard extends StatelessWidget {
  const _ExecutionResultCard({required this.result});

  final core_proxy.CoreApiChatEnhanceConversationMarkupManagerToolResult result;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      elevation: 0,
      color: result.success
          ? colorScheme.primaryContainer.withValues(alpha: 0.35)
          : colorScheme.errorContainer.withValues(alpha: 0.45),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(
              result.success ? Icons.check_circle_outline : Icons.error_outline,
              color: result.success ? colorScheme.primary : colorScheme.error,
              size: 20,
            ),
            const SizedBox(width: 10),
            Expanded(child: SelectableText(_toolResultText(result))),
          ],
        ),
      ),
    );
  }
}

String _toolResultText(
  core_proxy.CoreApiChatEnhanceConversationMarkupManagerToolResult result,
) {
  if (!result.success) {
    return result.error ?? '';
  }
  final value = result.result;
  if (value == null) {
    return '';
  }
  if (value is String) {
    return value;
  }
  return const JsonEncoder.withIndent('  ').convert(value);
}
