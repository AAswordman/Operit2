// ignore_for_file: file_names
import 'dart:typed_data';
import 'package:image/image.dart' as image;
import 'package:zxing2/qrcode.dart';

/// Runs in a worker isolate. Decode dimensions before allocating large image buffers.
String? readPeerQrImage(Uint8List bytes) {
  if (bytes.length < 12 || bytes.length > 16 * 1024 * 1024) return null;
  try {
    return _decode(bytes);
  } on image.ImageException {
    return null;
  } on RangeError {
    return null;
  } on FormatException {
    return null;
  }
}

String? _decode(Uint8List bytes) {
  // Only the advertised import formats are decoded, rather than probing arbitrary containers.
  final decoder = <image.Decoder>[
    image.PngDecoder(),
    image.JpegDecoder(),
    image.WebPDecoder(),
  ].where((candidate) => candidate.isValidFile(bytes)).firstOrNull;
  final header = decoder?.startDecode(bytes);
  if (header == null ||
      header.width <= 0 ||
      header.height <= 0 ||
      header.width * header.height > 24000000) {
    return null;
  }
  final decoded = decoder!.decodeFrame(0);
  if (decoded == null) return null;
  var frame = image.bakeOrientation(decoded);
  if (frame.width > 2048 || frame.height > 2048) {
    frame = image.copyResize(
      frame,
      width: frame.width >= frame.height ? 2048 : null,
      height: frame.height > frame.width ? 2048 : null,
    );
  }
  final rgba = frame
      .convert(numChannels: 4)
      .getBytes(order: image.ChannelOrder.rgba);
  final pixels = Int32List.view(
    rgba.buffer,
    rgba.offsetInBytes,
    rgba.lengthInBytes ~/ 4,
  );
  final source = RGBLuminanceSource(frame.width, frame.height, pixels);
  final hints = DecodeHints()..put(DecodeHintType.tryHarder);
  for (final luminance in [source, source.invert()]) {
    try {
      return QRCodeReader()
          .decode(BinaryBitmap(HybridBinarizer(luminance)), hints: hints)
          .text;
    } on ReaderException {
      // A light-on-dark code gets the same chance as a conventional code.
    }
  }
  return null;
}
