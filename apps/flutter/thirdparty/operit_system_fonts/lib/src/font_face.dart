import 'dart:convert';
import 'dart:typed_data';

const _collectionTag = 0x74746366;
const _headTag = 0x68656164;
const _nameTag = 0x6e616d65;
const _signatureTag = 0x44534947;

/// Produces a standalone SFNT so a collection never silently selects its first face.
Uint8List extractFontFace(
  Uint8List bytes, {
  int? index,
  String? postscriptName,
}) {
  if (bytes.length < 12 || bytes.length > 256 * 1024 * 1024) {
    throw FormatException('Invalid device font size');
  }
  final source = ByteData.sublistView(bytes);
  if (source.getUint32(0) != _collectionTag) return bytes;
  final count = source.getUint32(8);
  if (count == 0 || 12 + count * 4 > bytes.length) {
    throw FormatException('Invalid font collection directory');
  }
  final int face;
  if (index != null) {
    if (index < 0 || index >= count) throw FormatException('Invalid font face');
    face = source.getUint32(12 + index * 4);
  } else {
    if (postscriptName == null)
      throw FormatException('Font face is unspecified');
    final faces = [
      for (var i = 0; i < count; i++) source.getUint32(12 + i * 4),
    ];
    face = faces.singleWhere(
      (offset) => _postscriptNames(source, offset).contains(postscriptName),
    );
  }
  final tables =
      _tables(
          source,
          face,
        ).where((table) => table.tag != _signatureTag).toList()
        ..sort((a, b) => a.tag.compareTo(b.tag));
  var size = 12 + tables.length * 16;
  for (final table in tables) {
    size += (table.length + 3) & ~3;
  }
  final result = Uint8List(size);
  final output = ByteData.sublistView(result);
  output.setUint32(0, source.getUint32(face));
  output.setUint16(4, tables.length);
  var power = 1;
  var selector = 0;
  while (power * 2 <= tables.length) {
    power *= 2;
    selector++;
  }
  output.setUint16(6, power * 16);
  output.setUint16(8, selector);
  output.setUint16(10, tables.length * 16 - power * 16);
  var cursor = 12 + tables.length * 16;
  int? head;
  for (var i = 0; i < tables.length; i++) {
    final table = tables[i];
    result.setRange(cursor, cursor + table.length, bytes, table.offset);
    if (table.tag == _headTag) {
      if (table.length < 12) throw FormatException('Invalid head table');
      head = cursor;
      output.setUint32(cursor + 8, 0);
    }
    final record = 12 + i * 16;
    output.setUint32(record, table.tag);
    output.setUint32(record + 4, _checksum(output, cursor, table.length));
    output.setUint32(record + 8, cursor);
    output.setUint32(record + 12, table.length);
    cursor += (table.length + 3) & ~3;
  }
  if (head == null) throw FormatException('Font has no head table');
  output.setUint32(
    head + 8,
    (0xb1b0afba - _checksum(output, 0, size)) & 0xffffffff,
  );
  return result;
}

/// Validates table ranges before reading any collection data.
List<({int tag, int offset, int length})> _tables(ByteData data, int face) {
  if (face < 0 || face + 12 > data.lengthInBytes) {
    throw FormatException('Invalid SFNT directory');
  }
  final count = data.getUint16(face + 4);
  if (count == 0 || face + 12 + count * 16 > data.lengthInBytes) {
    throw FormatException('Invalid SFNT table count');
  }
  return [for (var i = 0; i < count; i++) _table(data, face + 12 + i * 16)];
}

/// Reads one bounded table-directory entry.
({int tag, int offset, int length}) _table(ByteData data, int record) {
  final offset = data.getUint32(record + 8);
  final length = data.getUint32(record + 12);
  if (offset + length > data.lengthInBytes)
    throw FormatException('Invalid table range');
  return (tag: data.getUint32(record), offset: offset, length: length);
}

/// Resolves the exact CoreText PostScript name to its collection face.
Set<String> _postscriptNames(ByteData data, int face) {
  final table = _tables(
    data,
    face,
  ).singleWhere((table) => table.tag == _nameTag);
  final start = table.offset;
  if (table.length < 6) throw FormatException('Invalid name table');
  final count = data.getUint16(start + 2);
  final strings = start + data.getUint16(start + 4);
  final names = <String>{};
  for (var i = 0; i < count; i++) {
    final record = start + 6 + i * 12;
    if (record + 12 > start + table.length)
      throw FormatException('Invalid name record');
    if (data.getUint16(record + 6) != 6) continue;
    final length = data.getUint16(record + 8);
    final offset = strings + data.getUint16(record + 10);
    if (offset + length > start + table.length)
      throw FormatException('Invalid font name');
    final platform = data.getUint16(record);
    if (platform == 0 || platform == 3) {
      names.add(
        String.fromCharCodes([
          for (var j = 0; j + 1 < length; j += 2) data.getUint16(offset + j),
        ]),
      );
    } else if (platform == 1) {
      names.add(
        latin1.decode(
          data.buffer.asUint8List(data.offsetInBytes + offset, length),
        ),
      );
    }
  }
  return names;
}

/// Calculates the OpenType unsigned checksum over zero-padded table words.
int _checksum(ByteData data, int start, int length) {
  var sum = 0;
  for (var offset = start; offset < start + length; offset += 4) {
    sum = (sum + data.getUint32(offset)) & 0xffffffff;
  }
  return sum;
}
