// ignore_for_file: file_names

import 'package:flutter/material.dart';

/// Presents a manual terminal launch failure at its own entry point without failing chat.
Future<void> launchWorkspaceTerminal({
  required BuildContext context,
  required String terminal,
  required String terminalType,
  required Future<void> Function() launch,
}) async {
  try {
    await launch();
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
