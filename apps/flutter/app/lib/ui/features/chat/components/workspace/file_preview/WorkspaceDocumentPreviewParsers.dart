// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:archive/archive.dart';
import 'package:xml/xml.dart';

String workspaceDocxPreviewText(Uint8List bytes) {
  final archive = ZipDecoder().decodeBytes(bytes);
  final document = archive.findFile('word/document.xml');
  if (document == null) {
    throw const FormatException('无法读取 Word 文档内容');
  }
  final xml = XmlDocument.parse(utf8.decode(document.content as List<int>));
  final paragraphs = xml
      .findAllElements('w:p')
      .map(_docxParagraphText)
      .where((line) => line.trim().isNotEmpty)
      .toList(growable: false);
  return paragraphs.join('\n');
}

String workspacePptxPreviewText(Uint8List bytes) {
  final archive = ZipDecoder().decodeBytes(bytes);
  final slideFiles =
      archive.files
          .where((file) {
            return file.name.startsWith('ppt/slides/slide') &&
                file.name.endsWith('.xml');
          })
          .toList(growable: false)
        ..sort((a, b) => a.name.compareTo(b.name));
  if (slideFiles.isEmpty) {
    throw const FormatException('无法读取 PowerPoint 幻灯片内容');
  }
  final buffer = StringBuffer();
  for (var index = 0; index < slideFiles.length; index++) {
    final xml = XmlDocument.parse(
      utf8.decode(slideFiles[index].content as List<int>),
    );
    final lines = xml
        .findAllElements('a:t')
        .map((node) => node.innerText)
        .where((line) => line.trim().isNotEmpty)
        .toList(growable: false);
    if (lines.isEmpty) {
      continue;
    }
    if (buffer.isNotEmpty) {
      buffer.writeln();
    }
    buffer.writeln('幻灯片 ${index + 1}');
    buffer.write(lines.join('\n'));
  }
  return buffer.toString();
}

List<List<String>> workspaceSpreadsheetPreviewRows(
  Uint8List bytes,
  String fileName,
) {
  final lowerName = fileName.toLowerCase();
  if (lowerName.endsWith('.csv')) {
    return _delimitedRows(utf8.decode(bytes, allowMalformed: true), ',');
  }
  if (lowerName.endsWith('.tsv')) {
    return _delimitedRows(utf8.decode(bytes, allowMalformed: true), '\t');
  }
  final archive = ZipDecoder().decodeBytes(bytes);
  final sharedStrings = _xlsxSharedStrings(archive);
  final sheet = archive.findFile('xl/worksheets/sheet1.xml');
  if (sheet == null) {
    throw const FormatException('无法读取 Excel 工作表');
  }
  final xml = XmlDocument.parse(utf8.decode(sheet.content as List<int>));
  return xml
      .findAllElements('row')
      .map((row) {
        final cells = <String>[];
        for (final cell in row.findElements('c')) {
          final columnIndex = _xlsxColumnIndex(cell.getAttribute('r'));
          while (columnIndex != null && cells.length < columnIndex) {
            cells.add('');
          }
          cells.add(_xlsxCellText(cell, sharedStrings));
        }
        return cells;
      })
      .toList(growable: false);
}

String _docxParagraphText(XmlElement paragraph) {
  final buffer = StringBuffer();
  for (final child in paragraph.descendants.whereType<XmlElement>()) {
    if (child.name.qualified == 'w:t') {
      buffer.write(child.innerText);
    } else if (child.name.qualified == 'w:tab') {
      buffer.write('\t');
    } else if (child.name.qualified == 'w:br') {
      buffer.write('\n');
    }
  }
  return buffer.toString();
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

String _xlsxCellText(XmlElement cell, List<String> sharedStrings) {
  final type = cell.getAttribute('t');
  final values = cell.findElements('v').toList(growable: false);
  final rawValue = values.isEmpty ? '' : values.first.innerText;
  if (type == 's') {
    final index = int.tryParse(rawValue);
    if (index != null && index >= 0 && index < sharedStrings.length) {
      return sharedStrings[index];
    }
  }
  if (type == 'inlineStr') {
    return cell.findAllElements('t').map((node) => node.innerText).join();
  }
  return rawValue;
}

int? _xlsxColumnIndex(String? reference) {
  if (reference == null || reference.isEmpty) {
    return null;
  }
  var value = 0;
  for (var index = 0; index < reference.length; index++) {
    final code = reference.codeUnitAt(index);
    if (code < 65 || code > 90) {
      break;
    }
    value = value * 26 + code - 64;
  }
  return value == 0 ? null : value - 1;
}

List<List<String>> _delimitedRows(String content, String delimiter) {
  return const LineSplitter()
      .convert(content)
      .map((line) => _delimitedCells(line, delimiter))
      .toList(growable: false);
}

List<String> _delimitedCells(String line, String delimiter) {
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
    } else if (char == delimiter && !quoted) {
      cells.add(buffer.toString());
      buffer.clear();
    } else {
      buffer.write(char);
    }
  }
  cells.add(buffer.toString());
  return cells;
}
