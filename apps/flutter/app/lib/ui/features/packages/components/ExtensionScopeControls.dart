// ignore_for_file: file_names
import 'package:flutter/material.dart';

/// Asks where a new extension should live and explains shared configuration.
Future<String?> chooseExtensionScope(BuildContext context) =>
    showDialog<String>(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('选择存放位置'),
        children: [
          SimpleDialogOption(
            onPressed: () => Navigator.pop(context, 'device'),
            child: const ListTile(
              leading: Icon(Icons.devices),
              title: Text('仅本设备'),
              subtitle: Text('内容和配置仅保存在本设备，不同步到其他设备。'),
            ),
          ),
          SimpleDialogOption(
            onPressed: () => Navigator.pop(context, 'space'),
            child: const ListTile(
              leading: Icon(Icons.cloud_outlined),
              title: Text('设备空间'),
              subtitle: Text('内容、配置和启用状态同步到空间中的其他设备。配置中的密钥也会共享。'),
            ),
          ),
        ],
      ),
    );
