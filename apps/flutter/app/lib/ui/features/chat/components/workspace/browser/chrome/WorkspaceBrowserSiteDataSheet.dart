// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import 'WorkspaceBrowserPopupWidgets.dart';

class WorkspaceBrowserSiteDataSheet extends StatefulWidget {
  const WorkspaceBrowserSiteDataSheet({super.key, required this.controller});

  final WebViewController controller;

  @override
  State<WorkspaceBrowserSiteDataSheet> createState() =>
      _WorkspaceBrowserSiteDataSheetState();
}

class _WorkspaceBrowserSiteDataSheetState
    extends State<WorkspaceBrowserSiteDataSheet> {
  Future<_WorkspaceBrowserSiteData>? _dataFuture;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<_WorkspaceBrowserSiteData>(
      future: _dataFuture,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const SizedBox(
            height: 96,
            child: Center(child: CircularProgressIndicator()),
          );
        }
        if (snapshot.hasError) {
          return WorkspaceBrowserPopupBody(
            children: <Widget>[
              WorkspaceBrowserPopupHeader(
                title: l10n.siteData,
                trailing: _refreshButton(l10n),
              ),
              Padding(
                padding: const EdgeInsets.all(12),
                child: Text(
                  snapshot.error.toString(),
                  style: theme.textTheme.bodySmall,
                ),
              ),
            ],
          );
        }
        final data = snapshot.data!;
        return WorkspaceBrowserPopupBody(
          children: <Widget>[
            WorkspaceBrowserPopupHeader(
              title: l10n.siteData,
              trailing: _refreshButton(l10n),
            ),
            _StorageSection(
              title: 'localStorage',
              entries: data.localStorage,
              onClear: _clearLocalStorage,
            ),
            _StorageSection(
              title: 'sessionStorage',
              entries: data.sessionStorage,
              onClear: _clearSessionStorage,
            ),
            WorkspaceBrowserPopupRow(
              icon: Icons.cookie_outlined,
              title: 'Cookie',
              subtitle: l10n.clearAllWebViewCookies,
              trailing: IconButton(
                tooltip: l10n.clearCookies,
                onPressed: _clearCookies,
                icon: const Icon(Icons.delete_outline, size: 18),
                visualDensity: VisualDensity.compact,
                constraints: const BoxConstraints.tightFor(
                  width: 32,
                  height: 32,
                ),
                padding: EdgeInsets.zero,
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _refreshButton(AppLocalizations l10n) {
    return IconButton(
      tooltip: l10n.refresh,
      onPressed: _refresh,
      icon: const Icon(Icons.refresh, size: 18),
      visualDensity: VisualDensity.compact,
      constraints: const BoxConstraints.tightFor(width: 32, height: 32),
      padding: EdgeInsets.zero,
    );
  }

  void _refresh() {
    setState(() {
      _dataFuture = _loadData();
    });
  }

  Future<_WorkspaceBrowserSiteData> _loadData() async {
    final result = await widget.controller.runJavaScriptReturningResult(r'''
JSON.stringify({
  localStorage: Object.fromEntries(Object.keys(localStorage).map(function(key) {
    return [key, localStorage.getItem(key)];
  })),
  sessionStorage: Object.fromEntries(Object.keys(sessionStorage).map(function(key) {
    return [key, sessionStorage.getItem(key)];
  }))
})
''');
    return _WorkspaceBrowserSiteData.fromJsonText(result as String);
  }

  Future<void> _clearLocalStorage() async {
    await widget.controller.runJavaScript('localStorage.clear();');
    _refresh();
  }

  Future<void> _clearSessionStorage() async {
    await widget.controller.runJavaScript('sessionStorage.clear();');
    _refresh();
  }

  Future<void> _clearCookies() async {
    await WebViewCookieManager().clearCookies();
    _refresh();
  }
}

class _WorkspaceBrowserSiteData {
  const _WorkspaceBrowserSiteData({
    required this.localStorage,
    required this.sessionStorage,
  });

  final Map<String, String> localStorage;
  final Map<String, String> sessionStorage;

  factory _WorkspaceBrowserSiteData.fromJsonText(String text) {
    final json = jsonDecode(text) as Map<String, Object?>;
    return _WorkspaceBrowserSiteData(
      localStorage: _stringMap(json['localStorage']),
      sessionStorage: _stringMap(json['sessionStorage']),
    );
  }
}

class _StorageSection extends StatelessWidget {
  const _StorageSection({
    required this.title,
    required this.entries,
    required this.onClear,
  });

  final String title;
  final Map<String, String> entries;
  final Future<void> Function() onClear;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        WorkspaceBrowserPopupHeader(
          title: title,
          trailing: IconButton(
            tooltip: l10n.clear,
            onPressed: onClear,
            icon: const Icon(Icons.delete_outline, size: 18),
            visualDensity: VisualDensity.compact,
            constraints: const BoxConstraints.tightFor(width: 32, height: 32),
            padding: EdgeInsets.zero,
          ),
        ),
        if (entries.isEmpty)
          WorkspaceBrowserPopupRow(
            icon: Icons.storage_outlined,
            title: l10n.noData,
            iconColor: theme.colorScheme.onSurfaceVariant,
          )
        else
          for (final entry in entries.entries.take(30))
            WorkspaceBrowserPopupRow(
              icon: Icons.data_object_outlined,
              title: entry.key,
              subtitle: entry.value,
            ),
      ],
    );
  }
}

Map<String, String> _stringMap(Object? value) {
  final map = value as Map<String, Object?>;
  return map.map((key, value) => MapEntry(key, value?.toString() ?? ''));
}
