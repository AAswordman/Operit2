// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

/// Returns the laid-out size of the nearest Overlay.
Size overlaySizeOf(BuildContext context) => _overlayBoxOf(context).size;

/// Resolves a laid-out anchor rectangle in the nearest Overlay coordinate space.
Rect overlayTargetRectOf(BuildContext context, RenderBox target) {
  final overlayBox = _overlayBoxOf(context);
  if (!target.attached || !target.hasSize) {
    throw StateError('Popup target is not laid out.');
  }
  return MatrixUtils.transformRect(
    target.getTransformTo(overlayBox),
    Offset.zero & target.size,
  );
}

/// Resolves the laid-out render box that defines the popup coordinate space.
RenderBox _overlayBoxOf(BuildContext context) {
  final overlayBox = Overlay.of(context).context.findRenderObject();
  if (overlayBox is! RenderBox || !overlayBox.attached || !overlayBox.hasSize) {
    throw StateError('Popup overlay is not laid out.');
  }
  return overlayBox;
}
