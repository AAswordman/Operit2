// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';

/// Keeps application targets readable and confirmation reachable on narrow screens.
Future<bool> showWorkspaceBrowserExternalNavigationDialog(
  BuildContext context,
  String url,
) async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) {
      final l10n = AppLocalizations.of(context)!;
      return AlertDialog(
        title: Text(l10n.openExternalApplication),
        content: SingleChildScrollView(child: SelectableText(url)),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.open),
          ),
        ],
      );
    },
  );
  return confirmed == true;
}
