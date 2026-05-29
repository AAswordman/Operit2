// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:archive/archive.dart';
import 'package:xml/xml.dart';

String workspaceDocxPreviewText(Uint8List bytes) {
  final archive = ZipDecoder().decodeBytes(bytes);
  final document = archive.findFile('word/document.xml');
  if (document == null) {
    throw FormatException('无法读取 Word 文档内容');
  }
  final xml = XmlDocument.parse(utf8.decode(document.content as List<int>));
  final paragraphs = xml
      .findAllElements('w:p')
      .map(
        (paragraph) => paragraph
            .findAllElements('w:t')
            .map((node) => node.innerText)
            .join(),
      )
      .where((line) => line.trim().isNotEmpty)
      .toList(growable: false);
  return paragraphs.join('\n');
}

List<List<String>> workspaceSpreadsheetPreviewRows(
  Uint8List bytes,
  String fileName,
) {
  if (fileName.toLowerCase().endsWith('.csv')) {
    return _csvRows(utf8.decode(bytes, allowMalformed: true));
  }
  final archive = ZipDecoder().decodeBytes(bytes);
  final sharedStrings = _xlsxSharedStrings(archive);
  final sheet = archive.findFile('xl/worksheets/sheet1.xml');
  if (sheet == null) {
    throw FormatException('无法读取 Excel 工作表');
  }
  final xml = XmlDocument.parse(utf8.decode(sheet.content as List<int>));
  return xml
      .findAllElements('row')
      .map((row) {
        return row
            .findElements('c')
            .map((cell) {
              final type = cell.getAttribute('t');
              final values = cell.findElements('v').toList(growable: false);
              final rawValue = values.isEmpty ? '' : values.first.innerText;
              if (type == 's') {
                final index = int.tryParse(rawValue);
                if (index != null &&
                    index >= 0 &&
                    index < sharedStrings.length) {
                  return sharedStrings[index];
                }
              }
              return rawValue;
            })
            .toList(growable: false);
      })
      .toList(growable: false);
}

List<String> _xlsxSharedStrings(Archive archive) {
  final file = archive.findFile('xl/sharedStrings.xml');
  if (file == null) {
    return const <String>[];
  }
  final xml = XmlDocument.parse(utf8.decode(file.content as List<int>));
  return xml
      .findAllElements('si')
      .map(
        (item) =>
            item.findAllElements('t').map((node) => node.innerText).join(),
      )
      .toList(growable: false);
}

List<List<String>> _csvRows(String content) {
  return const LineSplitter()
      .convert(content)
      .map(_csvCells)
      .toList(growable: false);
}

List<String> _csvCells(String line) {
  final cells = <String>[];
  final buffer = StringBuffer();
  var quoted = false;
  for (var index = 0; index < line.length; index++) {
    final char = line[index];
    if (char == '"') {
      if (quoted && index + 1 < line.length && line[index + 1] == '"') {
        buffer.write('"');
        index++;
      } else {
        quoted = !quoted;
      }
    } else if (char == ',' && !quoted) {
      cells.add(buffer.toString());
      buffer.clear();
    } else {
      buffer.write(char);
    }
  }
  cells.add(buffer.toString());
  return cells;
}
