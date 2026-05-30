// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import 'file_preview/WorkspaceDocumentPreviewWidgets.dart';
import 'file_preview/WorkspaceFilePreviewActionBar.dart';
import 'file_preview/WorkspaceImagePreview.dart';
import 'file_preview/WorkspaceMediaPreviewWidgets.dart';
import 'file_preview/WorkspaceOpenOnlyPreview.dart';
import 'file_preview/WorkspaceTextPreview.dart';
import 'html_preview/WorkspaceHtmlPreviewWidget.dart';
import 'WorkspaceTabModels.dart';

class WorkspaceFilePreviewContent extends StatelessWidget {
  const WorkspaceFilePreviewContent({
    super.key,
    required this.tab,
    required this.onReadWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onOpenBrowser,
  });

  final WorkspaceTab tab;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final kind = tab.previewKind ?? WorkspaceFilePreviewKind.binary;
    switch (kind) {
      case WorkspaceFilePreviewKind.text:
        return WorkspaceTextPreview(tab: tab, onOpenBrowser: onOpenBrowser);
      case WorkspaceFilePreviewKind.markdown:
        return WorkspaceMarkdownPreview(tab: tab, onOpenBrowser: onOpenBrowser);
      case WorkspaceFilePreviewKind.html:
        return _WorkspaceHtmlPreview(
          tab: tab,
          onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
        );
      case WorkspaceFilePreviewKind.image:
      case WorkspaceFilePreviewKind.audio:
      case WorkspaceFilePreviewKind.video:
      case WorkspaceFilePreviewKind.pdf:
      case WorkspaceFilePreviewKind.word:
      case WorkspaceFilePreviewKind.spreadsheet:
      case WorkspaceFilePreviewKind.presentation:
        return _WorkspaceBytesPreview(
          tab: tab,
          onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
          onOpenBrowser: onOpenBrowser,
        );
      case WorkspaceFilePreviewKind.binary:
        return WorkspaceOpenOnlyPreview(
          tab: tab,
          icon: Icons.insert_drive_file_outlined,
          title: l10n.filePreview,
          detail: l10n.unsupportedReadOnlyPreview,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
        );
    }
  }
}

class _WorkspaceBytesPreview extends StatelessWidget {
  const _WorkspaceBytesPreview({
    required this.tab,
    required this.onReadWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onOpenBrowser,
  });

  final WorkspaceTab tab;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;

  @override
  Widget build(BuildContext context) {
    final filePath = tab.filePath;
    if (filePath == null) {
      return const SizedBox.shrink();
    }
    return FutureBuilder<Uint8List>(
      future: onReadWorkspaceFileBytes(filePath),
      builder: (context, snapshot) {
        final l10n = AppLocalizations.of(context)!;
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(child: CircularProgressIndicator());
        }
        if (snapshot.hasError) {
          return WorkspaceOpenOnlyPreviewBody(
            tab: tab,
            icon: Icons.error_outline,
            title: l10n.cannotPreview,
            detail: snapshot.error.toString(),
            onOpenWorkspaceFile: onOpenWorkspaceFile,
          );
        }
        try {
          return Column(
            children: <Widget>[
              WorkspaceFilePreviewActionBar(
                canOpenInBrowser: tab.absolutePath != null,
                onOpenInBrowser: () {
                  onOpenBrowser(localFilePath: tab.absolutePath);
                },
                canOpenWorkspaceFile: filePath.isNotEmpty,
                onOpenWorkspaceFile: () {
                  onOpenWorkspaceFile(filePath);
                },
              ),
              Expanded(child: _buildPreview(l10n, snapshot.data!)),
            ],
          );
        } on Object catch (error) {
          return WorkspaceOpenOnlyPreviewBody(
            tab: tab,
            icon: Icons.error_outline,
            title: l10n.cannotPreview,
            detail: error.toString(),
            onOpenWorkspaceFile: onOpenWorkspaceFile,
          );
        }
      },
    );
  }

  Widget _buildPreview(AppLocalizations l10n, Uint8List bytes) {
    switch (tab.previewKind) {
      case WorkspaceFilePreviewKind.image:
        return WorkspaceImagePreview(bytes: bytes, fileName: tab.title);
      case WorkspaceFilePreviewKind.audio:
        return WorkspaceAudioPreview(bytes: bytes, title: tab.title);
      case WorkspaceFilePreviewKind.video:
        return WorkspaceVideoPreview(bytes: bytes, fileName: tab.title);
      case WorkspaceFilePreviewKind.pdf:
        return WorkspacePdfPreview(bytes: bytes);
      case WorkspaceFilePreviewKind.word:
        return WorkspaceWordPreview(bytes: bytes);
      case WorkspaceFilePreviewKind.spreadsheet:
        return WorkspaceSpreadsheetPreview(bytes: bytes, fileName: tab.title);
      case WorkspaceFilePreviewKind.presentation:
        return WorkspacePresentationPreview(bytes: bytes);
      case WorkspaceFilePreviewKind.text:
      case WorkspaceFilePreviewKind.markdown:
      case WorkspaceFilePreviewKind.html:
      case WorkspaceFilePreviewKind.binary:
      case null:
        return WorkspaceOpenOnlyPreview(
          tab: tab,
          icon: Icons.insert_drive_file_outlined,
          title: l10n.filePreview,
          detail: l10n.unsupportedReadOnlyPreview,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
        );
    }
  }
}

class _WorkspaceHtmlPreview extends StatelessWidget {
  const _WorkspaceHtmlPreview({
    required this.tab,
    required this.onReadWorkspaceFileBytes,
  });

  final WorkspaceTab tab;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;

  @override
  Widget build(BuildContext context) {
    final filePath = tab.filePath;
    if (filePath == null) {
      return const SizedBox.shrink();
    }
    return WorkspaceHtmlPreviewWidget(
      relativePath: filePath,
      onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
    );
  }
}
