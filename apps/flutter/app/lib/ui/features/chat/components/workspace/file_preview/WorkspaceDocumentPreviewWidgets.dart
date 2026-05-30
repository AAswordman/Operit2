// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:printing/printing.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import 'WorkspaceDocumentPreviewParsers.dart';
import 'WorkspaceTextPreview.dart';

class WorkspacePdfPreview extends StatelessWidget {
  const WorkspacePdfPreview({super.key, required this.bytes});

  final Uint8List bytes;

  @override
  Widget build(BuildContext context) {
    return PdfPreview(
      build: (_) async => bytes,
      allowPrinting: false,
      allowSharing: false,
      canChangeOrientation: false,
      canChangePageFormat: false,
      canDebug: false,
      pdfPreviewPageDecoration: const BoxDecoration(),
    );
  }
}

class WorkspaceWordPreview extends StatelessWidget {
  const WorkspaceWordPreview({super.key, required this.bytes});

  final Uint8List bytes;

  @override
  Widget build(BuildContext context) {
    return WorkspaceTextBody(
      text: workspaceDocxPreviewText(bytes),
      monospace: false,
    );
  }
}

class WorkspacePresentationPreview extends StatelessWidget {
  const WorkspacePresentationPreview({super.key, required this.bytes});

  final Uint8List bytes;

  @override
  Widget build(BuildContext context) {
    return WorkspaceTextBody(
      text: workspacePptxPreviewText(bytes),
      monospace: false,
    );
  }
}

class WorkspaceSpreadsheetPreview extends StatefulWidget {
  const WorkspaceSpreadsheetPreview({
    super.key,
    required this.bytes,
    required this.fileName,
  });

  final Uint8List bytes;
  final String fileName;

  @override
  State<WorkspaceSpreadsheetPreview> createState() =>
      _WorkspaceSpreadsheetPreviewState();
}

class _WorkspaceSpreadsheetPreviewState
    extends State<WorkspaceSpreadsheetPreview> {
  final ScrollController _horizontalController = ScrollController();
  final ScrollController _verticalController = ScrollController();

  @override
  void dispose() {
    _horizontalController.dispose();
    _verticalController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final rows = workspaceSpreadsheetPreviewRows(widget.bytes, widget.fileName);
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    if (rows.isEmpty) {
      return Center(
        child: Text(
          l10n.emptySpreadsheet,
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
      controller: _horizontalController,
      child: SingleChildScrollView(
        controller: _horizontalController,
        scrollDirection: Axis.horizontal,
        child: Scrollbar(
          controller: _verticalController,
          child: SingleChildScrollView(
            controller: _verticalController,
            primary: false,
            child: DataTable(
              headingRowHeight: 36,
              dataRowMinHeight: 34,
              dataRowMaxHeight: 64,
              columns: List<DataColumn>.generate(
                columnCount,
                (index) => DataColumn(label: Text(_columnName(index))),
              ),
              rows: rows
                  .take(300)
                  .map((row) {
                    return DataRow(
                      cells: List<DataCell>.generate(
                        columnCount,
                        (index) => DataCell(
                          ConstrainedBox(
                            constraints: const BoxConstraints(maxWidth: 260),
                            child: SelectableText(
                              index < row.length ? row[index] : '',
                              maxLines: 3,
                            ),
                          ),
                        ),
                      ),
                    );
                  })
                  .toList(growable: false),
            ),
          ),
        ),
      ),
    );
  }
}

String _columnName(int index) {
  var value = index + 1;
  final chars = <String>[];
  while (value > 0) {
    value--;
    chars.add(String.fromCharCode(65 + value % 26));
    value ~/= 26;
  }
  return chars.reversed.join();
}
