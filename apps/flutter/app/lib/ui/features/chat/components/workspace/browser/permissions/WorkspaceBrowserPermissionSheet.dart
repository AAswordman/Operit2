// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../chrome/WorkspaceBrowserPopupWidgets.dart';
import 'WorkspaceBrowserPermissionStore.dart';

class WorkspaceBrowserPermissionSheet extends StatelessWidget {
  const WorkspaceBrowserPermissionSheet({super.key, required this.store});

  final WorkspaceBrowserPermissionStore store;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: store,
      builder: (context, child) {
        final l10n = AppLocalizations.of(context)!;
        final theme = Theme.of(context);
        final records = store.records;
        return WorkspaceBrowserPopupBody(
          children: <Widget>[
            WorkspaceBrowserPopupHeader(
              title: l10n.permissionsTitle,
              trailing: IconButton(
                tooltip: l10n.clear,
                onPressed: records.isEmpty ? null : store.clear,
                icon: const Icon(Icons.delete_sweep_outlined, size: 18),
                visualDensity: VisualDensity.compact,
                constraints: const BoxConstraints.tightFor(
                  width: 32,
                  height: 32,
                ),
                padding: EdgeInsets.zero,
              ),
            ),
            if (records.isEmpty)
              WorkspaceBrowserPopupEmpty(
                icon: Icons.lock_outline,
                text: l10n.noPermissionRecords,
              )
            else
              for (final record in records)
                WorkspaceBrowserPopupRow(
                  icon: record.allowed
                      ? Icons.check_circle_outline
                      : Icons.block,
                  iconColor: record.allowed
                      ? theme.colorScheme.primary
                      : theme.colorScheme.error,
                  title: record.origin,
                  subtitle: record.types
                      .map((type) => _permissionTypeLabel(l10n, type))
                      .join(', '),
                  trailing: Text(
                    record.allowed ? l10n.allow : l10n.deny,
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: record.allowed
                          ? theme.colorScheme.primary
                          : theme.colorScheme.error,
                    ),
                  ),
                ),
          ],
        );
      },
    );
  }
}

String _permissionTypeLabel(
  AppLocalizations l10n,
  WebViewPermissionResourceType type,
) {
  switch (type.name) {
    case 'camera':
      return l10n.camera;
    case 'microphone':
      return l10n.microphone;
    case 'protectedMediaId':
      return l10n.protectedMedia;
    case 'midiSysex':
      return l10n.midiDevice;
    default:
      return type.name;
  }
}
