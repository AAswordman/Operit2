import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:xterm/src/core/buffer/line.dart';
import 'package:xterm/src/ui/painter.dart';

/// Retains visible terminal rows as independently cacheable display lists.
///
/// Rows are keyed by buffer identity so scrolling moves existing layers. Exact
/// cell snapshots detect all mutations, including edits through BufferLine.data.
/// Only rows used in the current viewport retain pictures and raster cache hints.
class TerminalLineCache {
  final _lines = <BufferLine, _CachedTerminalLine>{};
  final _visible = <BufferLine>{};

  /// Returns the number of rows currently retaining native drawing resources.
  int get length => _lines.length;

  /// Starts tracking the rows needed for one viewport paint.
  void beginFrame() {
    _visible.clear();
  }

  /// Returns a retained row layer, recording only when its cells changed.
  OffsetLayer layerFor(BufferLine line, TerminalPainter painter) {
    _visible.add(line);
    final cached = _lines.putIfAbsent(line, _CachedTerminalLine.new);
    if (!cached.matches(line)) {
      cached.record(line, painter);
    }
    return cached.handle.layer!;
  }

  /// Releases rows outside the viewport instead of retaining scrollback layers.
  void endFrame() {
    _lines.removeWhere((line, cached) {
      if (_visible.contains(line)) return false;
      cached.dispose();
      return true;
    });
    _visible.clear();
  }

  /// Invalidates all rows when fonts, colors, or the terminal owner change.
  void clear() {
    for (final cached in _lines.values) {
      cached.dispose();
    }
    _lines.clear();
    _visible.clear();
  }
}

class _CachedTerminalLine {
  final handle = LayerHandle<OffsetLayer>();
  Uint32List? _cells;
  int _length = -1;

  /// Compares exact cell values without allocating hashes or cell objects.
  bool matches(BufferLine line) {
    final cells = _cells;
    if (cells == null || _length != line.length) return false;
    final data = line.data;
    for (var i = 0; i < cells.length; i++) {
      if (cells[i] != data[i]) return false;
    }
    return true;
  }

  /// Records the fixed-grid row once and allows the engine to cache its pixels.
  void record(BufferLine line, TerminalPainter painter) {
    final layer = OffsetLayer();
    final bounds = Rect.fromLTWH(
      -painter.cellSize.width,
      0,
      (line.length + 2) * painter.cellSize.width,
      painter.cellSize.height * 2,
    );
    final recorder = ui.PictureRecorder();
    painter.paintLine(ui.Canvas(recorder, bounds), Offset.zero, line);
    layer.append(PictureLayer(bounds)
      ..picture = recorder.endRecording()
      ..isComplexHint = true);
    handle.layer = layer;
    _length = line.length;
    _cells = Uint32List.fromList(line.data);
  }

  /// Releases the owned layer and its native pictures when no longer visible.
  void dispose() {
    handle.layer = null;
    _cells = null;
  }
}
