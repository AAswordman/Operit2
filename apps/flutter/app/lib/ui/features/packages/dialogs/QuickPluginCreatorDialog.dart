// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../screens/QuickPluginCreatorSetupSupport.dart';

class QuickPluginCreatorDialog extends StatefulWidget {
  /// Creates the requirement dialog for an AI-assisted plugin development draft.
  const QuickPluginCreatorDialog({super.key, required this.clients});

  final GeneratedCoreProxyClients clients;

  /// Opens the requirement dialog and returns the confirmed development request.
  static Future<String?> show({
    required BuildContext context,
    required GeneratedCoreProxyClients clients,
  }) {
    return showDialog<String>(
      context: context,
      builder: (context) => QuickPluginCreatorDialog(clients: clients),
    );
  }

  /// Creates the state that prepares the bundled authoring resources.
  @override
  State<QuickPluginCreatorDialog> createState() =>
      _QuickPluginCreatorDialogState();
}

class _QuickPluginCreatorDialogState extends State<QuickPluginCreatorDialog> {
  final TextEditingController _requirementController = TextEditingController();
  bool _confirmRunning = false;
  QuickPluginCreatorSetupResult? _setupResult;
  String? _requirementError;

  /// Releases the requirement input controller.
  @override
  void dispose() {
    _requirementController.dispose();
    super.dispose();
  }

  /// Validates the requirement and prepares verified resources before returning it.
  Future<void> _confirm() async {
    if (_confirmRunning) {
      return;
    }
    final requirement = _requirementController.text.trim();
    if (requirement.isEmpty) {
      setState(() {
        _requirementError = '请先输入插件需求';
      });
      return;
    }
    setState(() {
      _confirmRunning = true;
      _setupResult = null;
    });
    final setupResult = await runQuickPluginCreatorSetup(widget.clients);
    if (!mounted) {
      return;
    }
    if (!setupResult.success) {
      setState(() {
        _confirmRunning = false;
        _setupResult = setupResult;
      });
      return;
    }
    Navigator.of(context).pop(requirement);
  }

  /// Builds the requirement form and reports the actual preparation outcome.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final setupResult = _setupResult;
    return AlertDialog(
      title: const Text('快速创作你的插件'),
      content: SizedBox(
        width: 520,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              const _DialogSectionTitle('插件需求'),
              const SizedBox(height: 8),
              Text(
                '确认后会准备并开放 Operit 插件开发套件 Skill、启用 operit_editor 平台编辑工具包，然后跳转聊天并填入需求草稿。发送草稿后才开始开发；本操作不会自动生成、安装或发布插件。',
                style: TextStyle(color: colorScheme.onSurfaceVariant),
              ),
              const SizedBox(height: 10),
              TextField(
                controller: _requirementController,
                minLines: 4,
                maxLines: 8,
                decoration: InputDecoration(
                  border: const OutlineInputBorder(),
                  hintText: '例如：做一个可以批量整理下载目录图片并生成索引的工具',
                  errorText: _requirementError,
                ),
                onChanged: (_) {
                  if (_requirementError != null) {
                    setState(() {
                      _requirementError = null;
                    });
                  }
                },
              ),
              if (setupResult != null && !setupResult.success) ...<Widget>[
                const SizedBox(height: 8),
                Text(
                  setupResult.error!,
                  style: TextStyle(color: colorScheme.error),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _confirmRunning ? null : () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _confirmRunning ? null : _confirm,
          child: _confirmRunning
              ? const M3LoadingIndicator(size: 16)
              : const Text('准备并前往聊天'),
        ),
      ],
    );
  }
}

class _DialogSectionTitle extends StatelessWidget {
  /// Creates a titled section within the requirement form.
  const _DialogSectionTitle(this.text);

  final String text;

  /// Renders the title using the current dialog typography.
  @override
  Widget build(BuildContext context) {
    return Text(text, style: const TextStyle(fontWeight: FontWeight.w700));
  }
}
