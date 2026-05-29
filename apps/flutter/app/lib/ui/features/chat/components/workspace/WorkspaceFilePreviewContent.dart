// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:printing/printing.dart';

import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import 'WorkspaceDocumentPreviewParsers.dart';
import 'WorkspaceMediaPreviewWidgets.dart';
import 'WorkspaceTabModels.dart';

class WorkspaceFilePreviewContent extends StatelessWidget {
  const WorkspaceFilePreviewContent({
    super.key,
    required this.tab,
    required this.onReadWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
  });

  final WorkspaceTab tab;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;

  @override
  Widget build(BuildContext context) {
    final kind = tab.previewKind ?? WorkspaceFilePreviewKind.binary;
    switch (kind) {
      case WorkspaceFilePreviewKind.text:
      case WorkspaceFilePreviewKind.html:
        return _WorkspaceTextPreview(tab: tab);
      case WorkspaceFilePreviewKind.markdown:
        return _WorkspaceMarkdownPreview(tab: tab);
      case WorkspaceFilePreviewKind.image:
      case WorkspaceFilePreviewKind.audio:
      case WorkspaceFilePreviewKind.video:
      case WorkspaceFilePreviewKind.pdf:
      case WorkspaceFilePreviewKind.word:
      case WorkspaceFilePreviewKind.spreadsheet:
        return _WorkspaceBytesPreview(
          tab: tab,
          onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
        );
      case WorkspaceFilePreviewKind.binary:
        return _WorkspaceOpenOnlyPreview(
          tab: tab,
          icon: Icons.insert_drive_file_outlined,
          title: '文件预览',
          detail: '此文件不属于内置只读预览类型。',
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
  });

  final WorkspaceTab tab;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;

  @override
  Widget build(BuildContext context) {
    final filePath = tab.filePath;
    if (filePath == null) {
      return const SizedBox.shrink();
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _WorkspacePreviewHeader(tab: tab),
        Expanded(
          child: FutureBuilder<Uint8List>(
            future: onReadWorkspaceFileBytes(filePath),
            builder: (context, snapshot) {
              if (snapshot.connectionState != ConnectionState.done) {
                return const Center(child: CircularProgressIndicator());
              }
              if (snapshot.hasError) {
                return _WorkspaceOpenOnlyPreviewBody(
                  tab: tab,
                  icon: Icons.error_outline,
                  title: '无法预览',
                  detail: snapshot.error.toString(),
                  onOpenWorkspaceFile: onOpenWorkspaceFile,
                );
              }
              try {
                return _buildPreview(context, snapshot.data!);
              } on Object catch (error) {
                return _WorkspaceOpenOnlyPreviewBody(
                  tab: tab,
                  icon: Icons.error_outline,
                  title: '无法预览',
                  detail: error.toString(),
                  onOpenWorkspaceFile: onOpenWorkspaceFile,
                );
              }
            },
          ),
        ),
      ],
    );
  }

  Widget _buildPreview(BuildContext context, Uint8List bytes) {
    switch (tab.previewKind) {
      case WorkspaceFilePreviewKind.image:
        if ((tab.filePath ?? '').toLowerCase().endsWith('.svg')) {
          return _WorkspaceTextBody(
            text: utf8.decode(bytes, allowMalformed: true),
            monospace: true,
          );
        }
        return InteractiveViewer(
          minScale: 0.2,
          maxScale: 6,
          child: Center(child: Image.memory(bytes, fit: BoxFit.contain)),
        );
      case WorkspaceFilePreviewKind.audio:
        return WorkspaceAudioPreview(bytes: bytes, title: tab.title);
      case WorkspaceFilePreviewKind.video:
        return WorkspaceVideoPreview(bytes: bytes, fileName: tab.title);
      case WorkspaceFilePreviewKind.pdf:
        return PdfPreview(
          build: (_) async => bytes,
          allowPrinting: false,
          allowSharing: false,
          canChangeOrientation: false,
          canChangePageFormat: false,
          canDebug: false,
        );
      case WorkspaceFilePreviewKind.word:
        final text = workspaceDocxPreviewText(bytes);
        return _WorkspaceTextBody(text: text, monospace: false);
      case WorkspaceFilePreviewKind.spreadsheet:
        final rows = workspaceSpreadsheetPreviewRows(bytes, tab.title);
        return _WorkspaceSpreadsheetBody(rows: rows);
      case WorkspaceFilePreviewKind.text:
      case WorkspaceFilePreviewKind.markdown:
      case WorkspaceFilePreviewKind.html:
      case WorkspaceFilePreviewKind.binary:
      case null:
        return _WorkspaceOpenOnlyPreviewBody(
          tab: tab,
          icon: Icons.insert_drive_file_outlined,
          title: '文件预览',
          detail: '此文件不属于内置只读预览类型。',
          onOpenWorkspaceFile: onOpenWorkspaceFile,
        );
    }
  }
}

class _WorkspaceTextPreview extends StatelessWidget {
  const _WorkspaceTextPreview({required this.tab});

  final WorkspaceTab tab;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _WorkspacePreviewHeader(tab: tab),
        Expanded(
          child: _WorkspaceTextBody(
            text: tab.fileContent ?? '',
            monospace: true,
          ),
        ),
      ],
    );
  }
}

class _WorkspaceMarkdownPreview extends StatelessWidget {
  const _WorkspaceMarkdownPreview({required this.tab});

  final WorkspaceTab tab;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _WorkspacePreviewHeader(tab: tab),
        Expanded(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(14),
            child: StreamMarkdownRenderer(
              content: tab.fileContent ?? '',
              isStreaming: false,
              textColor: theme.colorScheme.onSurface,
              backgroundColor: theme.colorScheme.surface,
            ),
          ),
        ),
      ],
    );
  }
}

class _WorkspaceTextBody extends StatelessWidget {
  const _WorkspaceTextBody({required this.text, required this.monospace});

  final String text;
  final bool monospace;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ColoredBox(
      color: theme.colorScheme.surface,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(12),
        child: SelectableText(
          text,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurface,
            fontFamily: monospace ? 'monospace' : null,
            height: 1.45,
          ),
        ),
      ),
    );
  }
}

class _WorkspaceSpreadsheetBody extends StatelessWidget {
  const _WorkspaceSpreadsheetBody({required this.rows});

  final List<List<String>> rows;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (rows.isEmpty) {
      return Center(
        child: Text(
          '表格为空',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      );
    }
    final columnCount = rows.fold<int>(
      0,
      (value, row) => row.length > value ? row.length : value,
    );
    return Scrollbar(
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: SingleChildScrollView(
          child: DataTable(
            columns: List<DataColumn>.generate(
              columnCount,
              (index) => DataColumn(label: Text('列 ${index + 1}')),
            ),
            rows: rows
                .take(200)
                .map((row) {
                  return DataRow(
                    cells: List<DataCell>.generate(
                      columnCount,
                      (index) => DataCell(
                        SelectableText(index < row.length ? row[index] : ''),
                      ),
                    ),
                  );
                })
                .toList(growable: false),
          ),
        ),
      ),
    );
  }
}

class _WorkspaceOpenOnlyPreview extends StatelessWidget {
  const _WorkspaceOpenOnlyPreview({
    required this.tab,
    required this.icon,
    required this.title,
    required this.detail,
    required this.onOpenWorkspaceFile,
  });

  final WorkspaceTab tab;
  final IconData icon;
  final String title;
  final String detail;
  final Future<void> Function(String path) onOpenWorkspaceFile;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _WorkspacePreviewHeader(tab: tab),
        Expanded(
          child: _WorkspaceOpenOnlyPreviewBody(
            tab: tab,
            icon: icon,
            title: title,
            detail: detail,
            onOpenWorkspaceFile: onOpenWorkspaceFile,
          ),
        ),
      ],
    );
  }
}

class _WorkspaceOpenOnlyPreviewBody extends StatelessWidget {
  const _WorkspaceOpenOnlyPreviewBody({
    required this.tab,
    required this.icon,
    required this.title,
    required this.detail,
    required this.onOpenWorkspaceFile,
  });

  final WorkspaceTab tab;
  final IconData icon;
  final String title;
  final String detail;
  final Future<void> Function(String path) onOpenWorkspaceFile;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final filePath = tab.filePath;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(icon, size: 44, color: theme.colorScheme.primary),
            const SizedBox(height: 12),
            Text(
              title,
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.onSurface,
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              detail,
              textAlign: TextAlign.center,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            if (filePath != null) ...<Widget>[
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: () {
                  onOpenWorkspaceFile(filePath);
                },
                icon: const Icon(Icons.open_in_new),
                label: const Text('打开文件'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _WorkspacePreviewHeader extends StatelessWidget {
  const _WorkspacePreviewHeader({required this.tab});

  final WorkspaceTab tab;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLow,
        border: Border(
          bottom: BorderSide(color: theme.colorScheme.outlineVariant),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 9, 12, 9),
        child: Text(
          tab.absolutePath ?? tab.filePath ?? tab.title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ),
    );
  }
}
