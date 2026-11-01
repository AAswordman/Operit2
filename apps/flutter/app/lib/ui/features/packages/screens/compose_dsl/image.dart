// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Remembers the decoded image source across unrelated Compose recompositions.
class _ComposeStableImage extends StatefulWidget {
  const _ComposeStableImage({
    required this.source,
    required this.fit,
    required this.hostContext,
    this.animationKey,
  });
  final String source;
  final BoxFit fit;
  final ComposeDslWebViewHostContext hostContext;
  final String? animationKey;

  @override
  State<_ComposeStableImage> createState() => _ComposeStableImageState();
}

class _ComposeStableImageState extends State<_ComposeStableImage> {
  late Future<ImageProvider> _provider;

  @override
  void initState() {
    super.initState();
    _resolveSource();
  }

  @override
  void didUpdateWidget(covariant _ComposeStableImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.source != widget.source ||
        oldWidget.animationKey != widget.animationKey ||
        oldWidget.hostContext.packageName != widget.hostContext.packageName ||
        oldWidget.hostContext.executionContextKey !=
            widget.hostContext.executionContextKey ||
        oldWidget.hostContext.readFileBytes !=
            widget.hostContext.readFileBytes) {
      _resolveSource();
    }
  }

  /// Resolves only a changed source; VFS providers share the owning session key.
  void _resolveSource() {
    _provider = resolveComposeDslImage(
      widget.source,
      readVfsBytes: widget.hostContext.readResourceBytes,
      // A changed playback identity must get a fresh stream/codec. A new
      // Flutter widget key alone would still reuse the completed image stream
      // from the shared cache, leaving one-shot animations frozen on replay.
      vfsOwner: widget.animationKey == null
          ? (
              widget.hostContext.packageName,
              widget.hostContext.executionContextKey,
            )
          : (
              widget.hostContext.packageName,
              widget.hostContext.executionContextKey,
              widget.animationKey,
            ),
    );
  }

  @override
  Widget build(BuildContext context) => RepaintBoundary(
    child: FutureBuilder<ImageProvider>(
      future: _provider,
      builder: (context, snapshot) {
        if (snapshot.hasError) return const Icon(Icons.broken_image_outlined);
        if (!snapshot.hasData) return const SizedBox.shrink();
        return Image(
          key: ValueKey(widget.animationKey),
          image: snapshot.requireData,
          fit: widget.fit,
          gaplessPlayback: true,
          errorBuilder: (_, error, stack) =>
              const Icon(Icons.broken_image_outlined),
        );
      },
    ),
  );
}
