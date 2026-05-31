// ignore_for_file: file_names

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:url_launcher/url_launcher.dart';

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
    return Dialog(
      insetPadding: const EdgeInsets.symmetric(horizontal: 18, vertical: 24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560, maxHeight: 760),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 14),
          child: Column(
            children: <Widget>[
              Row(
                children: <Widget>[
                  Text(
                    '长图预览',
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const Spacer(),
                  IconButton(
                    onPressed: onDismiss,
                    icon: const Icon(Icons.close),
                    tooltip: '关闭',
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
              const SizedBox(height: 8),
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
              Row(
                children: <Widget>[
                  Expanded(
                    child: Text(
                      imageFile.path,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    onPressed: () {
                      Clipboard.setData(ClipboardData(text: imageFile.path));
                    },
                    icon: const Icon(Icons.content_copy),
                    tooltip: '复制路径',
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton(
                    onPressed: () => _saveImage(context),
                    icon: const Icon(Icons.save_alt),
                    tooltip: '保存',
                    visualDensity: VisualDensity.compact,
                  ),
                  FilledButton.icon(
                    onPressed: () {
                      launchUrl(Uri.file(imageFile.path));
                    },
                    icon: const Icon(Icons.open_in_new, size: 18),
                    label: const Text('打开'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _saveImage(BuildContext context) async {
    final directory = await getApplicationDocumentsDirectory();
    final outputDirectory = Directory(
      '${directory.path}${Platform.pathSeparator}Operit',
    );
    await outputDirectory.create(recursive: true);
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
