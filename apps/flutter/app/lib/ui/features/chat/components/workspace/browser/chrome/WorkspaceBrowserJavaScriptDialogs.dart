// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';

Future<void> showWorkspaceBrowserAlertDialog(
  BuildContext context,
  JavaScriptAlertDialogRequest request,
  Future<void> dismissed,
) async {
  await _showBrowserDialog<void>(
    context: context,
    dismissed: dismissed,
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
  Future<void> dismissed,
) async {
  final result = await _showBrowserDialog<bool>(
    context: context,
    dismissed: dismissed,
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

Future<String?> showWorkspaceBrowserPromptDialog(
  BuildContext context,
  JavaScriptTextInputDialogRequest request,
  Future<void> dismissed,
) async {
  final controller = TextEditingController(text: request.defaultText ?? '');
  final result = await _showBrowserDialog<String>(
    context: context,
    dismissed: dismissed,
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
            onPressed: () => Navigator.of(context).pop(),
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
  return result;
}

/// Removes only the originating dialog when automation resolves its request.
Future<T?> _showBrowserDialog<T>({
  required BuildContext context,
  required Future<void> dismissed,
  required WidgetBuilder builder,
}) {
  final navigator = Navigator.of(context, rootNavigator: true);
  final route = DialogRoute<T>(context: context, builder: builder);
  dismissed.then((_) {
    if (route.isActive) navigator.removeRoute(route);
  });
  return navigator.push(route);
}

String _hostLabel(AppLocalizations l10n, String url) {
  final uri = Uri.tryParse(url);
  if (uri == null || uri.host.isEmpty) {
    return l10n.webPage;
  }
  return uri.host;
}
