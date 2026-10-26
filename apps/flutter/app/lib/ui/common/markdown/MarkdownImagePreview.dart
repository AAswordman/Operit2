// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

/// Full-screen image inspection, independent of external link navigation.
class MarkdownImagePreview extends StatefulWidget {
  const MarkdownImagePreview({super.key, required this.image, this.alt = ''});

  final Widget image;
  final String alt;

  @override
  State<MarkdownImagePreview> createState() => _MarkdownImagePreviewState();
}

class _MarkdownImagePreviewState extends State<MarkdownImagePreview> {
  final _transformation = TransformationController();

  @override
  void dispose() {
    _transformation.dispose();
    super.dispose();
  }

  void _zoom(double factor) {
    final current = _transformation.value;
    final scale = current.getMaxScaleOnAxis();
    final target = (scale * factor).clamp(0.1, 10.0);
    final viewport = context.size!;
    final center = Offset(viewport.width / 2, viewport.height / 2);
    final scene = _transformation.toScene(center);
    _transformation.value = Matrix4.identity()
      ..translateByDouble(
        center.dx - scene.dx * target,
        center.dy - scene.dy * target,
        0,
        1,
      )
      ..scaleByDouble(target, target, 1, 1);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Dialog.fullscreen(
      backgroundColor: Colors.black,
      child: Stack(
        fit: StackFit.expand,
        children: <Widget>[
          InteractiveViewer(
            transformationController: _transformation,
            minScale: 0.1,
            maxScale: 10,
            boundaryMargin: const EdgeInsets.all(double.infinity),
            trackpadScrollCausesScale: true,
            child: Center(child: widget.image),
          ),
          Positioned(
            top: 0,
            left: 0,
            right: 0,
            child: SafeArea(
              child: ColoredBox(
                color: Colors.black54,
                child: Row(
                  children: <Widget>[
                    Expanded(
                      child: Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 16),
                        child: Text(
                          widget.alt.trim().isEmpty
                              ? (l10n?.imagePreview ?? 'Image preview')
                              : widget.alt.trim(),
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(color: Colors.white),
                        ),
                      ),
                    ),
                    IconButton(
                      color: Colors.white,
                      tooltip: MaterialLocalizations.of(
                        context,
                      ).closeButtonTooltip,
                      onPressed: () => Navigator.of(context).pop(),
                      icon: const Icon(Icons.close),
                    ),
                  ],
                ),
              ),
            ),
          ),
          Positioned(
            bottom: 16,
            left: 0,
            right: 0,
            child: SafeArea(
              child: Center(
                child: Material(
                  color: Colors.black54,
                  borderRadius: BorderRadius.circular(24),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      IconButton(
                        color: Colors.white,
                        tooltip: l10n?.zoomOut ?? 'Zoom out',
                        onPressed: () => _zoom(1 / 1.5),
                        icon: const Icon(Icons.zoom_out),
                      ),
                      IconButton(
                        color: Colors.white,
                        tooltip: '1×',
                        onPressed: () =>
                            _transformation.value = Matrix4.identity(),
                        icon: const Icon(Icons.fit_screen),
                      ),
                      IconButton(
                        color: Colors.white,
                        tooltip: l10n?.zoomIn ?? 'Zoom in',
                        onPressed: () => _zoom(1.5),
                        icon: const Icon(Icons.zoom_in),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
