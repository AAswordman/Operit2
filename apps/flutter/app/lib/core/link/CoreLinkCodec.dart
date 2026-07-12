// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

/// Encodes one Link value using the protocol's only MessagePack representation.
Uint8List encodeCoreLink(Object? value) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeValue(value);
  return writer.takeBytes();
}

/// Decodes one complete MessagePack Link value.
Object? decodeCoreLink(Uint8List bytes) {
  final reader = _CoreLinkMessagePackReader(bytes);
  final value = reader.readValue();
  reader.expectDone();
  return value;
}

/// Decodes one complete MessagePack Link map.
Map<String, Object?> decodeCoreLinkMap(Uint8List bytes) {
  final value = decodeCoreLink(bytes);
  if (value is! Map) {
    throw FormatException(
      'Link payload must be a map, got ${value.runtimeType}',
    );
  }
  return value.cast<String, Object?>();
}

/// Writes the Link protocol's MessagePack value set without dart2js uint64 accessors.
class _CoreLinkMessagePackWriter {
  final BytesBuilder _bytes = BytesBuilder(copy: false);

  /// Writes any supported Link value.
  void writeValue(Object? value) {
    if (value == null) {
      _writeByte(0xc0);
      return;
    }
    if (value is bool) {
      _writeByte(value ? 0xc3 : 0xc2);
      return;
    }
    if (value is int) {
      _writeInt(value);
      return;
    }
    if (value is double) {
      _writeFloat64(value);
      return;
    }
    if (value is String) {
      _writeString(value);
      return;
    }
    if (value is Uint8List) {
      _writeBinary(value);
      return;
    }
    if (value is List) {
      _writeArray(value);
      return;
    }
    if (value is Map) {
      _writeMap(value);
      return;
    }
    throw FormatException('Unsupported Link value type: ${value.runtimeType}');
  }

  /// Returns the encoded bytes accumulated by this writer.
  Uint8List takeBytes() {
    return _bytes.takeBytes();
  }

  /// Writes one byte after validating its range.
  void _writeByte(int value) {
    _bytes.addByte(value & 0xff);
  }

  /// Writes a signed or unsigned integer using the smallest MessagePack form.
  void _writeInt(int value) {
    if (value >= 0) {
      _writeUint(value);
      return;
    }
    if (value >= -32) {
      _writeByte(value & 0xff);
      return;
    }
    if (value >= -0x80) {
      _writeByte(0xd0);
      _writeByte(value);
      return;
    }
    if (value >= -0x8000) {
      _writeByte(0xd1);
      _writeSigned(2, value);
      return;
    }
    if (value >= -0x80000000) {
      _writeByte(0xd2);
      _writeSigned(4, value);
      return;
    }
    _writeByte(0xd3);
    _writeSigned(8, value);
  }

  /// Writes a non-negative integer using the smallest MessagePack form.
  void _writeUint(int value) {
    if (value <= 0x7f) {
      _writeByte(value);
      return;
    }
    if (value <= 0xff) {
      _writeByte(0xcc);
      _writeByte(value);
      return;
    }
    if (value <= 0xffff) {
      _writeByte(0xcd);
      _writeUnsigned(2, value);
      return;
    }
    if (value <= 0xffffffff) {
      _writeByte(0xce);
      _writeUnsigned(4, value);
      return;
    }
    _writeByte(0xcf);
    _writeUnsigned(8, value);
  }

  /// Writes an unsigned integer as big-endian bytes.
  void _writeUnsigned(int byteCount, int value) {
    for (var shift = (byteCount - 1) * 8; shift >= 0; shift -= 8) {
      _writeByte(value >> shift);
    }
  }

  /// Writes a signed integer as two's-complement big-endian bytes.
  void _writeSigned(int byteCount, int value) {
    var encoded = value;
    if (value < 0) {
      encoded += 1 << (byteCount * 8);
    }
    _writeUnsigned(byteCount, encoded);
  }

  /// Writes a double using the MessagePack float64 form.
  void _writeFloat64(double value) {
    final data = ByteData(8)..setFloat64(0, value);
    _writeByte(0xcb);
    _bytes.add(data.buffer.asUint8List());
  }

  /// Writes a UTF-8 string with the matching MessagePack string prefix.
  void _writeString(String value) {
    final bytes = utf8.encode(value);
    final length = bytes.length;
    if (length <= 31) {
      _writeByte(0xa0 | length);
    } else if (length <= 0xff) {
      _writeByte(0xd9);
      _writeByte(length);
    } else if (length <= 0xffff) {
      _writeByte(0xda);
      _writeUnsigned(2, length);
    } else {
      _writeByte(0xdb);
      _writeUnsigned(4, length);
    }
    _bytes.add(bytes);
  }

  /// Writes native bytes using the MessagePack binary family.
  void _writeBinary(Uint8List value) {
    final length = value.length;
    if (length <= 0xff) {
      _writeByte(0xc4);
      _writeByte(length);
    } else if (length <= 0xffff) {
      _writeByte(0xc5);
      _writeUnsigned(2, length);
    } else {
      _writeByte(0xc6);
      _writeUnsigned(4, length);
    }
    _bytes.add(value);
  }

  /// Writes a Link array.
  void _writeArray(List<Object?> value) {
    final length = value.length;
    if (length <= 15) {
      _writeByte(0x90 | length);
    } else if (length <= 0xffff) {
      _writeByte(0xdc);
      _writeUnsigned(2, length);
    } else {
      _writeByte(0xdd);
      _writeUnsigned(4, length);
    }
    for (final item in value) {
      writeValue(item);
    }
  }

  /// Writes a Link map with string keys.
  void _writeMap(Map<Object?, Object?> value) {
    final length = value.length;
    if (length <= 15) {
      _writeByte(0x80 | length);
    } else if (length <= 0xffff) {
      _writeByte(0xde);
      _writeUnsigned(2, length);
    } else {
      _writeByte(0xdf);
      _writeUnsigned(4, length);
    }
    for (final entry in value.entries) {
      final key = entry.key;
      if (key is! String) {
        throw FormatException(
          'Link map key must be a string: ${key.runtimeType}',
        );
      }
      _writeString(key);
      writeValue(entry.value);
    }
  }
}

/// Reads the Link protocol's MessagePack value set without dart2js uint64 accessors.
class _CoreLinkMessagePackReader {
  final Uint8List _bytes;
  int _offset = 0;

  /// Creates a reader over one complete MessagePack payload.
  _CoreLinkMessagePackReader(this._bytes);

  /// Reads any supported Link value.
  Object? readValue() {
    final marker = _readByte();
    if (marker <= 0x7f) {
      return marker;
    }
    if (marker >= 0xe0) {
      return marker - 0x100;
    }
    if ((marker & 0xe0) == 0xa0) {
      return _readString(marker & 0x1f);
    }
    if ((marker & 0xf0) == 0x90) {
      return _readArray(marker & 0x0f);
    }
    if ((marker & 0xf0) == 0x80) {
      return _readMap(marker & 0x0f);
    }

    switch (marker) {
      case 0xc0:
        return null;
      case 0xc2:
        return false;
      case 0xc3:
        return true;
      case 0xc4:
        return _readBinary(_readUnsigned(1));
      case 0xc5:
        return _readBinary(_readUnsigned(2));
      case 0xc6:
        return _readBinary(_readUnsigned(4));
      case 0xca:
        return _readFloat32();
      case 0xcb:
        return _readFloat64();
      case 0xcc:
        return _readUnsigned(1);
      case 0xcd:
        return _readUnsigned(2);
      case 0xce:
        return _readUnsigned(4);
      case 0xcf:
        return _readUnsigned(8);
      case 0xd0:
        return _readSigned(1);
      case 0xd1:
        return _readSigned(2);
      case 0xd2:
        return _readSigned(4);
      case 0xd3:
        return _readSigned(8);
      case 0xd9:
        return _readString(_readUnsigned(1));
      case 0xda:
        return _readString(_readUnsigned(2));
      case 0xdb:
        return _readString(_readUnsigned(4));
      case 0xdc:
        return _readArray(_readUnsigned(2));
      case 0xdd:
        return _readArray(_readUnsigned(4));
      case 0xde:
        return _readMap(_readUnsigned(2));
      case 0xdf:
        return _readMap(_readUnsigned(4));
    }

    throw FormatException(
      'Unsupported MessagePack marker 0x${marker.toRadixString(16)}',
    );
  }

  /// Verifies the reader consumed the complete payload.
  void expectDone() {
    if (_offset != _bytes.length) {
      throw FormatException(
        'Trailing bytes after Link payload: ${_bytes.length - _offset}',
      );
    }
  }

  /// Reads one byte from the payload.
  int _readByte() {
    _require(1);
    return _bytes[_offset++];
  }

  /// Reads an unsigned big-endian integer with explicit byte composition.
  int _readUnsigned(int byteCount) {
    _require(byteCount);
    var value = 0;
    for (var i = 0; i < byteCount; i += 1) {
      value = (value * 0x100) + _bytes[_offset++];
    }
    return value;
  }

  /// Reads a signed two's-complement big-endian integer.
  int _readSigned(int byteCount) {
    final unsigned = _readUnsigned(byteCount);
    final signBit = 1 << ((byteCount * 8) - 1);
    if ((unsigned & signBit) == 0) {
      return unsigned;
    }
    return unsigned - (1 << (byteCount * 8));
  }

  /// Reads a MessagePack float32 value.
  double _readFloat32() {
    _require(4);
    final data = ByteData.sublistView(_bytes, _offset, _offset + 4);
    _offset += 4;
    return data.getFloat32(0);
  }

  /// Reads a MessagePack float64 value.
  double _readFloat64() {
    _require(8);
    final data = ByteData.sublistView(_bytes, _offset, _offset + 8);
    _offset += 8;
    return data.getFloat64(0);
  }

  /// Reads native bytes.
  Uint8List _readBinary(int length) {
    _require(length);
    final value = Uint8List.sublistView(_bytes, _offset, _offset + length);
    _offset += length;
    return value;
  }

  /// Reads a UTF-8 string.
  String _readString(int length) {
    _require(length);
    final value = utf8.decode(_bytes.sublist(_offset, _offset + length));
    _offset += length;
    return value;
  }

  /// Reads a Link array.
  List<Object?> _readArray(int length) {
    return List<Object?>.generate(length, (_) => readValue(), growable: false);
  }

  /// Reads a Link map with string keys.
  Map<String, Object?> _readMap(int length) {
    final value = <String, Object?>{};
    for (var i = 0; i < length; i += 1) {
      final key = readValue();
      if (key is! String) {
        throw FormatException(
          'Link map key must be a string: ${key.runtimeType}',
        );
      }
      value[key] = readValue();
    }
    return value;
  }

  /// Checks that the requested byte count is present.
  void _require(int byteCount) {
    if (_offset + byteCount > _bytes.length) {
      throw FormatException('Unexpected end of Link payload');
    }
  }
}
