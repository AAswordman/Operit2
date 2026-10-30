// ignore_for_file: file_names

import 'dart:typed_data';

/// Incrementally applies the thinking header's text cleanup to append-only
/// snapshots. Only the latest [capacity] UTF-16 units are retained for display;
/// the source snapshot is referenced, rather than copied into another buffer.
class ThinkingPreviewBuffer {
  static const int capacity = 1000;
  static final _codePlaceholder = ' [代码] '.codeUnits;

  String _source = '';
  String _preview = '';
  _TagCleaner _tags = _TagCleaner();
  _TagCleaner? _fenceStart;
  bool _insideFence = false;
  int _ticks = 0;

  /// Position of the retained window in the cleaned text. Syntax checkpoints
  /// restore this too, so a closed code block can move the window backwards.
  int get textOffset => _tags.textOffset;

  String update(String text) {
    if (identical(text, _source) || text == _source) return _preview;
    var start = _source.length;
    // Snapshots may be replaced or rolled back, so verify the existing prefix
    // before reusing state. Do not allocate a substring for the appended text.
    if (!text.startsWith(_source)) {
      _tags = _TagCleaner();
      _fenceStart = null;
      _insideFence = false;
      _ticks = 0;
      start = 0;
    }
    for (var index = start; index < text.length; index++) {
      _append(text.codeUnitAt(index));
    }
    _source = text;
    return _preview = _tags.text;
  }

  void _append(int unit) {
    if (unit != 0x60) {
      _ticks = 0;
      if (!_insideFence) _fenceStart = null;
      _tags.append(unit);
      return;
    }

    if (!_insideFence && _ticks == 0) _fenceStart = _tags.copy();
    _ticks++;
    if (_ticks == 3) {
      _ticks = 0;
      if (_insideFence) {
        // Unclosed fences are displayed literally, as in the original regex.
        // Once closed, restore the bounded state before the opening fence.
        _tags = _fenceStart!;
        _fenceStart = null;
        _insideFence = false;
        for (final placeholderUnit in _codePlaceholder) {
          _tags.append(placeholderUnit);
        }
        return;
      }
      _insideFence = true;
    }
    _tags.append(unit);
  }
}

/// Implements <[^>]+>, including the provisional display of incomplete tags.
class _TagCleaner {
  _LineCleaner _line = _LineCleaner();
  _LineCleaner? _tagStart;
  bool _tagHasBody = false;

  String get text => _line.text;
  int get textOffset => _line.textOffset;

  _TagCleaner copy() => _TagCleaner()
    .._line = _line.copy()
    .._tagStart = _tagStart?.copy()
    .._tagHasBody = _tagHasBody;

  void append(int unit) {
    if (_tagStart != null) {
      if (unit == 0x3e) {
        if (_tagHasBody) {
          _line = _tagStart!;
          _line.append(0x20);
        } else {
          _line.append(unit);
        }
        _tagStart = null;
        _tagHasBody = false;
        return;
      }
      _tagHasBody = true;
    } else if (unit == 0x3c) {
      _tagStart = _line.copy();
    }
    _line.append(unit);
  }
}

/// Implements ^[#*>\-\d.]+\s+ with multiline anchors before collapsing spaces.
class _LineCleaner {
  _PreviewText _output = _PreviewText();
  _PreviewText? _prefixStart;
  bool _lineStart = true;
  bool _skipWhitespace = false;

  String get text => _output.text;
  int get textOffset => _output.textOffset;

  _LineCleaner copy() => _LineCleaner()
    .._output = _output.copy()
    .._prefixStart = _prefixStart?.copy()
    .._lineStart = _lineStart
    .._skipWhitespace = _skipWhitespace;

  void append(int unit) {
    final whitespace = _isWhitespace(unit);
    if (_prefixStart != null) {
      if (whitespace) {
        _output = _prefixStart!;
        _prefixStart = null;
        _skipWhitespace = true;
      } else {
        if (!_isMarker(unit)) _prefixStart = null;
        _output.append(unit);
      }
    } else if (_skipWhitespace && whitespace) {
      // The heading/list regex consumes whitespace across line boundaries.
    } else {
      _skipWhitespace = false;
      if (_lineStart && _isMarker(unit)) _prefixStart = _output.copy();
      _output.append(unit);
    }
    _lineStart =
        unit == 0x0a || unit == 0x0d || unit == 0x2028 || unit == 0x2029;
  }
}

/// Collapses regex whitespace and trims the result without evicting visible
/// text for trailing spaces. Dart trim also removes NEL (U+0085), unlike \s.
class _PreviewText {
  _CodeUnitRing _units = _CodeUnitRing();
  _CodeUnitRing? _pendingTrim;
  bool _pendingSpace = false;

  String get text => _units.text;
  int get textOffset => _units.totalLength - _units.length;

  _PreviewText copy() => _PreviewText()
    .._units = _units.copy()
    .._pendingTrim = _pendingTrim?.copy()
    .._pendingSpace = _pendingSpace;

  void append(int unit) {
    if (_isWhitespace(unit)) {
      _pendingSpace = true;
      return;
    }
    if (unit == 0x85) {
      if (_units.isNotEmpty) {
        final pending = _pendingTrim ??= _CodeUnitRing();
        if (_pendingSpace) pending.append(0x20);
        pending.append(unit);
      }
      _pendingSpace = false;
      return;
    }
    if (_units.isNotEmpty) {
      final pending = _pendingTrim;
      if (pending != null) {
        _units.totalLength += pending.totalLength - pending.length;
        for (var index = 0; index < pending.length; index++) {
          _units.append(pending.unitAt(index));
        }
        _pendingTrim = null;
      }
      if (_pendingSpace) _units.append(0x20);
    }
    _pendingSpace = false;
    _units.append(unit);
  }
}

/// Eviction moves the head in O(1), without shifting or slicing the old text.
/// Syntax checkpoints copy bounded state only when a tentative token starts.
class _CodeUnitRing {
  final Uint16List _units = Uint16List(ThinkingPreviewBuffer.capacity);
  int _head = 0;
  int length = 0;
  int totalLength = 0;

  bool get isNotEmpty => length != 0;
  int unitAt(int index) => _units[(_head + index) % _units.length];

  void append(int unit) {
    totalLength++;
    if (length == _units.length) {
      _units[_head] = unit;
      _head = (_head + 1) % _units.length;
    } else {
      _units[(_head + length) % _units.length] = unit;
      length++;
    }
  }

  _CodeUnitRing copy() => _CodeUnitRing()
    .._units.setAll(0, _units)
    .._head = _head
    ..length = length
    ..totalLength = totalLength;

  String get text =>
      String.fromCharCodes(Iterable<int>.generate(length, unitAt));
}

bool _isMarker(int unit) =>
    unit == 0x23 ||
    unit == 0x2a ||
    unit == 0x3e ||
    unit == 0x2d ||
    unit == 0x2e ||
    (unit >= 0x30 && unit <= 0x39);

bool _isWhitespace(int unit) =>
    unit == 0x20 ||
    (unit >= 0x09 && unit <= 0x0d) ||
    unit == 0xa0 ||
    unit == 0x1680 ||
    (unit >= 0x2000 && unit <= 0x200a) ||
    unit == 0x2028 ||
    unit == 0x2029 ||
    unit == 0x202f ||
    unit == 0x205f ||
    unit == 0x3000 ||
    unit == 0xfeff;
