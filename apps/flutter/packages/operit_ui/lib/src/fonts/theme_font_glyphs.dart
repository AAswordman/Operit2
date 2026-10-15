import 'dart:typed_data';

import 'theme_font_registry.dart';

/// Bounded cmap coverage probe for fixed fixture text. This is not the complete
/// Rust L4 font validator: it does not audit outlines, instructions or checksums.
class ThemeFontGlyphs {
  ThemeFontGlyphs(Uint8List bytes) : _data = ByteData.sublistView(bytes) {
    if (bytes.length < 12 ||
        bytes.length > themeFontMaxBytes ||
        _u32(0) != 0x00010000) {
      throw FormatException('Fixture font must be a static TrueType sfnt');
    }
    final tableCount = _u16(4);
    if (tableCount == 0 || tableCount > 64) {
      throw FormatException('Fixture font table budget exceeded');
    }
    _range(12, tableCount * 16, _data.lengthInBytes);
    final tables = <String, (int, int)>{};
    for (var index = 0; index < tableCount; index++) {
      final record = 12 + index * 16;
      final tag = String.fromCharCodes(bytes.sublist(record, record + 4));
      final offset = _u32(record + 8);
      final length = _u32(record + 12);
      _range(offset, length, _data.lengthInBytes);
      if (tables.containsKey(tag))
        throw FormatException('Duplicate font table: $tag');
      tables[tag] = (offset, length);
    }
    const unsupported = <String>{
      'fvar',
      'gvar',
      'avar',
      'cvar',
      'HVAR',
      'MVAR',
      'CFF ',
      'CFF2',
      'COLR',
      'CPAL',
      'CBDT',
      'CBLC',
      'sbix',
      'SVG ',
    };
    if (tables.keys.any(unsupported.contains)) {
      throw FormatException(
        'Variable, CFF or color fixture fonts are not supported',
      );
    }
    for (final tag in <String>[
      'head',
      'hhea',
      'hmtx',
      'maxp',
      'loca',
      'glyf',
      'cmap',
    ]) {
      if (!tables.containsKey(tag))
        throw FormatException('Missing font table: $tag');
    }
    final maxp = tables['maxp']!;
    _range(0, 6, maxp.$2);
    _glyphCount = _u16(maxp.$1 + 4);
    if (_glyphCount == 0) throw FormatException('Fixture font has no glyphs');
    final cmap = tables['cmap']!;
    _range(0, 4, cmap.$2);
    final records = _u16(cmap.$1 + 2);
    if (records == 0 || records > 64)
      throw FormatException('Invalid cmap record budget');
    _range(4, records * 8, cmap.$2);
    var selectedRank = -1;
    for (var index = 0; index < records; index++) {
      final record = cmap.$1 + 4 + index * 8;
      final platform = _u16(record);
      final encoding = _u16(record + 2);
      final relative = _u32(record + 4);
      _range(relative, 2, cmap.$2);
      final format = _u16(cmap.$1 + relative);
      // Choose the declared Unicode mapping by repertoire, not on decode errors.
      final rank = switch ((platform, encoding, format)) {
        (3, 10, 12) => 400,
        (0, _, 12) => 300,
        (3, 1, 4) => 200,
        (0, _, 4) => 100,
        _ => -1,
      };
      if (rank > selectedRank) {
        selectedRank = rank;
        _base = cmap.$1 + relative;
        _format = format;
      }
    }
    if (selectedRank < 0)
      throw FormatException('Unicode cmap 4 or 12 is required');
    final relative = _base - cmap.$1;
    if (_format == 12) {
      _range(relative, 16, cmap.$2);
      _length = _u32(_base + 4);
      _range(relative, _length, cmap.$2);
      _segments = _u32(_base + 12);
      if (_segments > 65535) throw FormatException('Too many cmap groups');
      _range(16, _segments * 12, _length);
      var previousEnd = -1;
      for (var index = 0; index < _segments; index++) {
        final group = _base + 16 + index * 12;
        final start = _u32(group);
        final end = _u32(group + 4);
        final glyph = _u32(group + 8);
        if (start <= previousEnd ||
            end < start ||
            end > 0x10ffff ||
            glyph + end - start >= _glyphCount) {
          throw FormatException('Invalid cmap group geometry');
        }
        previousEnd = end;
      }
    } else {
      _range(relative, 16, cmap.$2);
      _length = _u16(_base + 2);
      _range(relative, _length, cmap.$2);
      final doubledSegments = _u16(_base + 6);
      if (doubledSegments == 0 || doubledSegments.isOdd) {
        throw FormatException('Invalid cmap segment count');
      }
      _segments = doubledSegments ~/ 2;
      _range(0, 16 + _segments * 8, _length);
      var previousEnd = -1;
      for (var index = 0; index < _segments; index++) {
        final end = _u16(_base + 14 + index * 2);
        final start = _u16(_base + 16 + _segments * 2 + index * 2);
        if (start > end || start <= previousEnd) {
          throw FormatException('Invalid cmap segment geometry');
        }
        previousEnd = end;
      }
    }
  }

  final ByteData _data;
  late final int _glyphCount;
  late int _base;
  late int _format;
  late int _length;
  late int _segments;

  int _u16(int offset) => _data.getUint16(offset, Endian.big);
  int _u32(int offset) => _data.getUint32(offset, Endian.big);

  static void _range(int start, int length, int limit) {
    if (start < 0 || length < 0 || start > limit || length > limit - start) {
      throw FormatException('Font table/cmap range is out of bounds');
    }
  }

  bool hasGlyph(int scalar) {
    if (scalar < 0 || scalar > 0x10ffff) return false;
    if (_format == 4 && scalar > 0xffff) return false;
    var low = 0;
    var high = _segments - 1;
    while (low <= high) {
      final index = (low + high) ~/ 2;
      final int start;
      final int end;
      if (_format == 12) {
        final group = _base + 16 + index * 12;
        start = _u32(group);
        end = _u32(group + 4);
        if (scalar >= start && scalar <= end) {
          return _u32(group + 8) + scalar - start != 0;
        }
      } else {
        start = _u16(_base + 16 + _segments * 2 + index * 2);
        end = _u16(_base + 14 + index * 2);
        if (scalar >= start && scalar <= end) {
          final delta = _data.getInt16(
            _base + 16 + _segments * 4 + index * 2,
            Endian.big,
          );
          final rangeSlot = _base + 16 + _segments * 6 + index * 2;
          final rangeOffset = _u16(rangeSlot);
          int glyph;
          if (rangeOffset == 0) {
            glyph = (scalar + delta) & 0xffff;
          } else {
            final offset = rangeSlot + rangeOffset + (scalar - start) * 2;
            _range(offset - _base, 2, _length);
            glyph = _u16(offset);
            if (glyph == 0) return false;
            glyph = (glyph + delta) & 0xffff;
          }
          return glyph != 0 && glyph < _glyphCount;
        }
      }
      if (scalar < start) {
        high = index - 1;
      } else {
        low = index + 1;
      }
    }
    return false;
  }

  void requireText(String text, {required String reference}) {
    final scalars = text.runes.toSet();
    if (scalars.length > 4096)
      throw ArgumentError('Fixture glyph budget exceeded');
    for (final scalar in scalars) {
      if (scalar == 9 || scalar == 10 || scalar == 13) continue;
      if (!hasGlyph(scalar)) {
        throw StateError(
          'Fixture glyph U+${scalar.toRadixString(16).toUpperCase()} '
          'is not covered by $reference',
        );
      }
    }
  }
}
