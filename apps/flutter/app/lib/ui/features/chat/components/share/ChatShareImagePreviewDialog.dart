// ignore_for_file: file_names

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/material.dart';

import '../../../../../core/host/FileSaveService.dart';
import '../../../../../core/host/FileShareService.dart';
import '../../../../common/components/OperitDialog.dart';
import 'ChatShareImageGenerator.dart';

class ChatShareImagePreviewDialog extends StatefulWidget {
  const ChatShareImagePreviewDialog({
    super.key,
    required this.image,
    required this.onDismiss,
  });

  final ChatShareImage image;
  final VoidCallback onDismiss;

  @override
  State<ChatShareImagePreviewDialog> createState() =>
      _ChatShareImagePreviewDialogState();
}

class _ChatShareImagePreviewDialogState
    extends State<ChatShareImagePreviewDialog> {
  bool _busy = false;
  late final String _fileName =
      'operit_share_${DateTime.now().millisecondsSinceEpoch}.png';

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitDialogScaffold(
      title: '长图预览',
      maxWidth: 560,
      maxHeight: 760,
      showCloseButton: true,
      onClose: widget.onDismiss,
      actions: <Widget>[
        OutlinedButton.icon(
          onPressed: _busy ? null : _saveImage,
          icon: const Icon(Icons.save_alt),
          label: const Text('保存到文件'),
        ),
        Builder(
          builder: (buttonContext) => FilledButton.icon(
            onPressed: _busy ? null : () => _shareImage(buttonContext),
            icon: const Icon(Icons.share_outlined),
            label: const Text('分享图片'),
          ),
        ),
      ],
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.45),
          borderRadius: BorderRadius.circular(12),
        ),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(12),
          child: InteractiveViewer(
            minScale: 0.4,
            maxScale: 5,
            child: Center(child: Image.memory(widget.image.bytes)),
          ),
        ),
      ),
    );
  }

  Future<void> _saveImage() => _runAction('保存失败', () async {
    final path = await FileSaveService.saveBytes(
      bytes: widget.image.bytes,
      name: _fileName,
      mimeType: 'image/png',
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(
          label: 'PNG 图片',
          extensions: <String>['png'],
          mimeTypes: <String>['image/png'],
          uniformTypeIdentifiers: <String>['public.png'],
        ),
      ],
    );
    if (path != null && mounted) {
      _showMessage('图片已保存');
    }
  });

  Future<void> _shareImage(BuildContext buttonContext) async {
    final box = buttonContext.findRenderObject()! as RenderBox;
    final origin = box.localToGlobal(Offset.zero) & box.size;
    await _runAction(
      '分享失败',
      () => FileShareService.shareBytes(
        bytes: widget.image.bytes,
        name: _fileName,
        mimeType: 'image/png',
        sharePositionOrigin: origin,
      ),
    );
  }

  Future<void> _runAction(
    String errorLabel,
    Future<void> Function() action,
  ) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await action();
    } catch (error) {
      if (mounted) _showMessage('$errorLabel：$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _showMessage(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }
}
