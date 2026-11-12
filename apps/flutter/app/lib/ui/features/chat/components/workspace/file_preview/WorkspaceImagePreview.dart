// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

class WorkspaceImagePreview extends StatelessWidget {
  /// Creates an image viewport with shared raster and vector interactions.
  const WorkspaceImagePreview({
    super.key,
    required this.bytes,
    required this.fileName,
  });

  final Uint8List bytes;
  final String fileName;

  /// Supports wheel, trackpad, and touch zoom across the entire viewport.
  @override
  Widget build(BuildContext context) {
    return InteractiveViewer(
      minScale: 0.2,
      maxScale: 6,
      boundaryMargin: const EdgeInsets.all(double.infinity),
      trackpadScrollCausesScale: true,
      child: Center(
        child: fileName.toLowerCase().endsWith('.svg')
            ? SvgPicture.memory(bytes, fit: BoxFit.contain)
            : Image.memory(bytes, fit: BoxFit.contain),
      ),
    );
  }
}
