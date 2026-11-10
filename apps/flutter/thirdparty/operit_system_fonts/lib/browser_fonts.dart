import 'dart:js_interop';
import 'dart:typed_data';

import 'package:flutter_web_plugins/flutter_web_plugins.dart';

import 'operit_system_fonts.dart';
import 'src/font_face.dart';

@JS('window')
external _FontWindow get _window;

extension type _FontWindow(JSObject _) implements JSObject {
  external JSFunction? get queryLocalFonts;
}

extension type _BrowserFont(JSObject _) implements JSObject {
  external String get family;
  external String get style;
  external String get postscriptName;
  external JSPromise<_FontBlob> blob();
}

extension type _FontBlob(JSObject _) implements JSObject {
  external int get size;
  external JSPromise<JSArrayBuffer> arrayBuffer();
}

/// Uses the browser's permission-gated Local Font Access catalog.
class BrowserSystemFonts implements SystemFontsPlatform {
  final _families = <String, _BrowserFont>{};

  /// Registers through Flutter's web plugin system rather than runtime platform branches.
  static void registerWith(Registrar registrar) {
    SystemFontsPlatform.instance = BrowserSystemFonts();
  }

  /// Requests permission directly from the user's font-picker gesture.
  @override
  Future<List<String>> listFamilies() async {
    final query = _window.queryLocalFonts;
    if (query == null) {
      throw UnsupportedError('This browser does not support Local Font Access');
    }
    final fonts =
        await (query.callAsFunction(_window)
                as JSPromise<JSArray<_BrowserFont>>)
            .toDart;
    _families.clear();
    for (final font in fonts.toDart) {
      final previous = _families[font.family];
      if (previous == null ||
          _regularStyles.contains(font.style.toLowerCase())) {
        _families[font.family] = font;
      }
    }
    return _families.keys.toList()..sort();
  }

  static const _regularStyles = {'regular', 'normal', 'roman', 'book'};

  /// Reads only the selected permitted font and preserves its collection face.
  @override
  Future<Uint8List> readFamily(String family) async {
    final font = _families[family];
    if (font == null) throw StateError('Unknown installed font');
    final blob = await font.blob().toDart;
    if (blob.size <= 0 || blob.size > 256 * 1024 * 1024) {
      throw StateError('Invalid installed font size');
    }
    final buffer = await blob.arrayBuffer().toDart;
    return extractFontFace(
      buffer.toDart.asUint8List(),
      postscriptName: font.postscriptName,
    );
  }
}
