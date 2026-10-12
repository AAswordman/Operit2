import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_svg/flutter_svg.dart' show SvgBytesLoader, vg;

/// Decodes theme images through Flutter's shared renderer.
///
/// SVG packages pass the Rust contract's bounded static-SVG subset check
/// before reaching this function. They are rasterized at the same intrinsic
/// pixel size on native and web so bubble nine-slice painting uses one path.
Future<ui.Image> decodeThemeImageBytes(
  String reference,
  Uint8List bytes,
) async {
  if (reference.toLowerCase().endsWith('.svg')) {
    final pictureInfo = await vg.loadPicture(SvgBytesLoader(bytes), null);
    try {
      final size = pictureInfo.size;
      if (!size.width.isFinite ||
          !size.height.isFinite ||
          size.width <= 0 ||
          size.height <= 0) {
        throw FormatException(
          'SVG intrinsic dimensions are invalid: $reference',
        );
      }
      final width = size.width.ceil();
      final height = size.height.ceil();
      if (width > 4096 || height > 4096 || width * height > 16_777_216) {
        throw FormatException(
          'SVG pixel dimensions exceed v1 bounds: $reference',
        );
      }
      return pictureInfo.picture.toImage(width, height);
    } finally {
      pictureInfo.picture.dispose();
    }
  }

  final codec = await ui.instantiateImageCodec(bytes);
  try {
    if (codec.frameCount != 1) {
      throw FormatException(
        'Theme images must contain one static frame: $reference',
      );
    }
    final frame = await codec.getNextFrame();
    final image = frame.image;
    if (image.width <= 0 ||
        image.height <= 0 ||
        image.width > 4096 ||
        image.height > 4096 ||
        image.width * image.height > 16_777_216) {
      image.dispose();
      throw FormatException(
        'Theme image dimensions exceed v1 bounds: $reference',
      );
    }
    return image;
  } finally {
    codec.dispose();
  }
}
