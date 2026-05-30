// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';

class WorkspaceBrowserPermissionDialog extends StatelessWidget {
  const WorkspaceBrowserPermissionDialog({
    super.key,
    required this.origin,
    required this.types,
  });

  final String origin;
  final Set<WebViewPermissionResourceType> types;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.browserPermissionRequestTitle),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(origin, style: theme.textTheme.bodyMedium),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              for (final type in types)
                Chip(
                  avatar: Icon(_iconForType(type), size: 18),
                  label: Text(_labelForType(l10n, type)),
                ),
            ],
          ),
        ],
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.deny),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.allow),
        ),
      ],
    );
  }
}

IconData _iconForType(WebViewPermissionResourceType type) {
  switch (type.name) {
    case 'camera':
      return Icons.photo_camera_outlined;
    case 'microphone':
      return Icons.mic_none_outlined;
    case 'protectedMediaId':
      return Icons.verified_user_outlined;
    case 'midiSysex':
      return Icons.piano_outlined;
    default:
      return Icons.security_outlined;
  }
}

String _labelForType(
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
