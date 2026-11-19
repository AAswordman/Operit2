// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

/// Shows a non-modal status while a manual terminal launch is pending.
Future<void> launchWorkspaceTerminal({
  required BuildContext context,
  required String terminal,
  required String terminalType,
  required Future<void> Function() launch,
}) async {
  OverlayEntry? progress;
  final timer = Timer(const Duration(milliseconds: 400), () {
    if (!context.mounted) {
      return;
    }
    final overlay = Overlay.of(context);
    final entry = OverlayEntry(
      builder: (context) => Positioned(
        left: 16,
        right: 16,
        bottom: 16,
        child: SafeArea(
          child: IgnorePointer(
            child: Center(
              child: Material(
                elevation: 4,
                borderRadius: BorderRadius.circular(8),
                child: const Padding(
                  padding: EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                      SizedBox(width: 12),
                      Flexible(child: Text('正在启动终端，首次使用可能需要下载组件…')),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    overlay.insert(entry);
    progress = entry;
  });
  try {
    try {
      await launch();
    } finally {
      timer.cancel();
      progress?.remove();
      progress?.dispose();
    }
  } catch (error, stackTrace) {
    debugPrint(
      'Failed to launch terminal $terminal/$terminalType: $error\n$stackTrace',
    );
    if (!context.mounted) {
      return;
    }
    await showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('终端启动失败'),
        scrollable: true,
        content: SelectableText('$terminal / $terminalType\n\n$error'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('关闭'),
          ),
        ],
      ),
    );
  }
}
