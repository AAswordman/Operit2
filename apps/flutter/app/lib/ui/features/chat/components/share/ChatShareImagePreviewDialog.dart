// ignore_for_file: file_names

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../../common/components/OperitDialog.dart';
import '../../../../../core/path/OperitClientPaths.dart';

class ChatShareImagePreviewDialog extends StatelessWidget {
  const ChatShareImagePreviewDialog({
    super.key,
    required this.imageFile,
    required this.onDismiss,
  });

  final File imageFile;
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
          onPressed: () {
            Clipboard.setData(ClipboardData(text: imageFile.path));
          },
          icon: const Icon(Icons.content_copy),
          tooltip: '复制路径',
        ),
        IconButton(
          onPressed: () => _saveImage(context),
          icon: const Icon(Icons.save_alt),
          tooltip: '保存',
        ),
        FilledButton.icon(
          onPressed: () {
            launchUrl(Uri.file(imageFile.path));
          },
          icon: const Icon(Icons.open_in_new, size: 18),
          label: const Text('打开'),
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
                  child: Center(child: Image.file(imageFile)),
                ),
              ),
            ),
          ),
          const SizedBox(height: 12),
          Text(
            imageFile.path,
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
    final outputDirectory = await OperitClientPaths.shareImageExportsDir();
    final outputFile = File(
      '${outputDirectory.path}${Platform.pathSeparator}operit_share_${DateTime.now().millisecondsSinceEpoch}.png',
    );
    await imageFile.copy(outputFile.path);
    if (!context.mounted) {
      return;
    }
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text('已保存：${outputFile.path}')));
  }
}
