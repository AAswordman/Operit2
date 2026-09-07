// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/proxy/generated/CoreProxyModels.g.dart';

/// Shows the actual migration result, including every nonempty migration note.
Future<void> showOperit1ImportResultDialog(
  BuildContext context,
  Operit1SnapshotImportResult result,
) async {
  if (!context.mounted) return;
  final notes = result.modelConfig.skippedFields
      .map((note) => note.trim())
      .where((note) => note.isNotEmpty)
      .toSet()
      .toList();
  final summary = <String>[
    '聊天：${result.importedChats} 个，消息：${result.importedMessages} 条',
    '模型：${result.modelConfig.importedModelCount} 个',
    '记忆：${result.importedMemories} 条，关联：${result.importedMemoryLinks} 条',
    '工作区：${result.importedWorkspaces} 个',
    '资源文件：${result.importedFiles + result.importedExternalFiles + result.importedWorkspaceFiles} 个',
    if (notes.isNotEmpty) '\n迁移说明：\n${notes.join('\n\n')}',
  ].join('\n');
  await showDialog<void>(
    context: context,
    barrierDismissible: false,
    builder: (dialogContext) => AlertDialog(
      title: const Text('导入结果'),
      content: SingleChildScrollView(child: SelectableText(summary)),
      actions: <Widget>[
        FilledButton(
          onPressed: () => Navigator.of(dialogContext).pop(),
          child: const Text('完成'),
        ),
      ],
    ),
  );
}
