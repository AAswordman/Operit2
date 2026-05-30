// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';

Future<void> showWorkspaceBrowserAlertDialog(
  BuildContext context,
  JavaScriptAlertDialogRequest request,
) async {
  await showDialog<void>(
    context: context,
    builder: (context) {
      final l10n = AppLocalizations.of(context)!;
      return AlertDialog(
        title: Text(_hostLabel(l10n, request.url)),
        content: Text(request.message),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(l10n.ok),
          ),
        ],
      );
    },
  );
}

Future<bool> showWorkspaceBrowserConfirmDialog(
  BuildContext context,
  JavaScriptConfirmDialogRequest request,
) async {
  final result = await showDialog<bool>(
    context: context,
    builder: (context) {
      final l10n = AppLocalizations.of(context)!;
      return AlertDialog(
        title: Text(_hostLabel(l10n, request.url)),
        content: Text(request.message),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.ok),
          ),
        ],
      );
    },
  );
  return result == true;
}

Future<String> showWorkspaceBrowserPromptDialog(
  BuildContext context,
  JavaScriptTextInputDialogRequest request,
) async {
  final controller = TextEditingController(text: request.defaultText ?? '');
  final result = await showDialog<String>(
    context: context,
    builder: (context) {
      final l10n = AppLocalizations.of(context)!;
      return AlertDialog(
        title: Text(_hostLabel(l10n, request.url)),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Align(
              alignment: Alignment.centerLeft,
              child: Text(request.message),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: controller,
              autofocus: true,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                isDense: true,
              ),
              onSubmitted: (value) => Navigator.of(context).pop(value),
            ),
          ],
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(''),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(controller.text),
            child: Text(l10n.ok),
          ),
        ],
      );
    },
  );
  controller.dispose();
  return result ?? '';
}

String _hostLabel(AppLocalizations l10n, String url) {
  final uri = Uri.tryParse(url);
  if (uri == null || uri.host.isEmpty) {
    return l10n.webPage;
  }
  return uri.host;
}
