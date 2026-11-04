// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../model/ExtensionCatalogController.dart';
import 'ExtensionNavigation.dart';

enum _QuickAction { open, toggle, update, copyId }

/// Quick actions for one extension: a menu at the pointer for right-clicks,
/// a bottom sheet for long-presses.
///
/// Actions whose consequences need explaining, such as sharing or removing,
/// stay on the detail and device pages where they confirm in place.
Future<void> showExtensionQuickActions(
  BuildContext context, {
  required ExtensionCatalogController controller,
  required core_proxy.ExtensionCatalogEntry entry,
  Offset? pointer,
}) async {
  final key = ExtensionCatalogController.entryKey(entry);
  final enabled =
      controller.pendingEnabled(key) ??
      ExtensionCatalogController.isEnabled(entry);
  final skill = entry.kind == 'skill';
  final items = <(_QuickAction, IconData, String)>[
    (_QuickAction.open, Icons.open_in_new_rounded, '打开详情'),
    (
      _QuickAction.toggle,
      enabled ? Icons.toggle_off_outlined : Icons.toggle_on_outlined,
      skill
          ? (enabled ? '对 AI 隐藏' : '让 AI 可见')
          : (enabled ? '停用' : '启用'),
    ),
    if (controller.hasUpdate(entry))
      (_QuickAction.update, Icons.arrow_circle_up_rounded, '更新'),
    (_QuickAction.copyId, Icons.copy_rounded, '复制 ID'),
  ];
  final _QuickAction? action;
  if (pointer != null) {
    final overlay =
        Overlay.of(context).context.findRenderObject()! as RenderBox;
    action = await showMenu<_QuickAction>(
      context: context,
      position: RelativeRect.fromRect(
        pointer & const Size(1, 1),
        Offset.zero & overlay.size,
      ),
      items: <PopupMenuEntry<_QuickAction>>[
        for (final (value, icon, label) in items)
          PopupMenuItem<_QuickAction>(
            value: value,
            child: Row(
              children: <Widget>[
                Icon(icon, size: 18),
                const SizedBox(width: 12),
                Text(label),
              ],
            ),
          ),
      ],
    );
  } else {
    unawaited(HapticFeedback.selectionClick());
    action = await showModalBottomSheet<_QuickAction>(
      context: context,
      showDragHandle: true,
      constraints: const BoxConstraints(maxWidth: 560),
      builder: (context) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 0, 20, 8),
              child: Text(
                entry.displayName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            for (final (value, icon, label) in items)
              ListTile(
                leading: Icon(icon),
                title: Text(label),
                onTap: () => Navigator.of(context).pop(value),
              ),
            const SizedBox(height: 8),
          ],
        ),
      ),
    );
  }
  if (action == null || !context.mounted) {
    return;
  }
  final messenger = ScaffoldMessenger.of(context);
  try {
    switch (action) {
      case _QuickAction.open:
        openExtensionDetail(kind: entry.kind, id: entry.id);
      case _QuickAction.toggle:
        await controller.setEnabled(entry, !enabled);
      case _QuickAction.update:
        await controller.update(entry);
      case _QuickAction.copyId:
        await Clipboard.setData(ClipboardData(text: entry.id));
        messenger.showSnackBar(
          const SnackBar(
            content: Text('已复制 ID'),
            behavior: SnackBarBehavior.floating,
          ),
        );
    }
  } catch (error, stackTrace) {
    debugPrint('Extension quick action failed: $error\n$stackTrace');
    messenger.showSnackBar(
      SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
    );
  }
}
