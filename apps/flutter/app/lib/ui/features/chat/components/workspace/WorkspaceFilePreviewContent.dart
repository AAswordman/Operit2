// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';
import '../../../packages/screens/ComposeDslImageSource.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';

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
  /// Creates a file view with shared workspace read and write capabilities.
  const WorkspaceFilePreviewContent({
    super.key,
    required this.tab,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onOpenBrowser,
    required this.splitMarkdownContent,
  });

  final WorkspaceTab tab;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path, Uint8List bytes)
  onWriteWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;
  final MarkdownContentSplitter splitMarkdownContent;

  /// Routes editable text formats to the editor and other formats to previews.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final kind = tab.previewKind ?? WorkspaceFilePreviewKind.binary;
    switch (kind) {
      case WorkspaceFilePreviewKind.text:
      case WorkspaceFilePreviewKind.markdown:
        return WorkspaceTextPreview(
          tab: tab,
          onWriteWorkspaceFileBytes: onWriteWorkspaceFileBytes,
          onOpenBrowser: onOpenBrowser,
          splitMarkdownContent: splitMarkdownContent,
        );
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

class _WorkspaceBytesPreview extends StatefulWidget {
  /// Creates a byte preview whose file read survives parent rebuilds.
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

  /// Keeps the pending read and loaded content for this file preview.
  @override
  State<_WorkspaceBytesPreview> createState() => _WorkspaceBytesPreviewState();
}

class _WorkspaceBytesPreviewState extends State<_WorkspaceBytesPreview> {
  Future<Uint8List>? _bytesFuture;

  /// Starts one read for the mounted file preview.
  @override
  void initState() {
    super.initState();
    _loadPreview();
  }

  /// Reloads only when the file identity or its decoding requirements change.
  @override
  void didUpdateWidget(covariant _WorkspaceBytesPreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.tab.filePath != widget.tab.filePath ||
        oldWidget.tab.absolutePath != widget.tab.absolutePath ||
        oldWidget.tab.identityToken != widget.tab.identityToken ||
        oldWidget.tab.previewKind != widget.tab.previewKind) {
      _loadPreview();
    }
  }

  /// Captures the current reader and format for the lifetime of this request.
  void _loadPreview() {
    final path = widget.tab.filePath;
    _bytesFuture = path == null
        ? null
        : _readPreviewBytes(
            path,
            widget.tab.previewKind,
            widget.onReadWorkspaceFileBytes,
          );
  }

  /// Builds the loaded preview without replacing its read on layout updates.
  @override
  Widget build(BuildContext context) {
    final tab = widget.tab;
    final filePath = tab.filePath;
    if (filePath == null) {
      return const SizedBox.shrink();
    }
    return FutureBuilder<Uint8List>(
      future: _bytesFuture,
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
            onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
          );
        }
        try {
          return Column(
            children: <Widget>[
              WorkspaceFilePreviewActionBar(
                canOpenInBrowser: tab.absolutePath != null,
                onOpenInBrowser: () {
                  widget.onOpenBrowser(localFilePath: tab.absolutePath);
                },
                canOpenWorkspaceFile: filePath.isNotEmpty,
                onOpenWorkspaceFile: () {
                  widget.onOpenWorkspaceFile(filePath);
                },
              ),
              Expanded(
                child: KeyedSubtree(
                  key: ObjectKey(_bytesFuture),
                  child: _buildPreview(l10n, snapshot.data!),
                ),
              ),
            ],
          );
        } on Object catch (error) {
          return WorkspaceOpenOnlyPreviewBody(
            tab: tab,
            icon: Icons.error_outline,
            title: l10n.cannotPreview,
            detail: error.toString(),
            onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
          );
        }
      },
    );
  }

  /// Prepares image bytes at the file read boundary before building a preview.
  Future<Uint8List> _readPreviewBytes(
    String path,
    WorkspaceFilePreviewKind? kind,
    Future<Uint8List> Function(String path) readBytes,
  ) async {
    final bytes = await readBytes(path);
    return kind == WorkspaceFilePreviewKind.image
        ? readImageDisplayBytes(bytes)
        : bytes;
  }

  /// Selects a renderer for the bytes owned by the current preview request.
  Widget _buildPreview(AppLocalizations l10n, Uint8List bytes) {
    final tab = widget.tab;
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
          onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
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
      onReadWorkspaceFileBytes: (path) => onReadWorkspaceFileBytes(
        // The HTTP preview server returns URL paths without a leading slash.
        filePath.startsWith('/') && !path.startsWith('/') ? '/$path' : path,
      ),
    );
  }
}
