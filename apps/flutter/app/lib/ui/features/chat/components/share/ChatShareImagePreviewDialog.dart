// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../common/components/OperitDialog.dart';
import '../../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import 'ChatShareImageGenerator.dart';

class ChatShareImagePreviewDialog extends StatelessWidget {
  const ChatShareImagePreviewDialog({
    super.key,
    required this.image,
    required this.onDismiss,
  });

  final ChatShareImage image;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return OperitDialogScaffold(
      title: '长图预览',
      maxWidth: 560,
      maxHeight: 760,
      showCloseButton: true,
      onClose: onDismiss,
      actions: <Widget>[
        IconButton(
          onPressed: () => _saveImage(context),
          icon: const Icon(Icons.save_alt),
          tooltip: '保存',
        ),
      ],
      child: Column(
        children: <Widget>[
          Expanded(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.45,
                ),
                borderRadius: BorderRadius.circular(12),
              ),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: InteractiveViewer(
                  minScale: 0.4,
                  maxScale: 5,
                  child: Center(child: Image.memory(image.bytes)),
                ),
              ),
            ),
          ),
          const SizedBox(height: 12),
          Text(
            image.storagePath,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _saveImage(BuildContext context) async {
    const clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
    final directory = await clients.repositoryRuntimeStorageRepository
        .shareImageExportsDirPath();
    final storagePath =
        '$directory/operit_share_${DateTime.now().millisecondsSinceEpoch}.png';
    await clients.repositoryRuntimeStorageRepository.writeBase64(
      path: storagePath,
      base64Content: base64Encode(image.bytes),
    );
    if (!context.mounted) {
      return;
    }
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text('已保存：$storagePath')));
  }
}
