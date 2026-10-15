// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Remembers the decoded image source across unrelated Compose recompositions.
class _ComposeStableImage extends StatefulWidget {
  const _ComposeStableImage({
    required this.source,
    required this.fit,
    required this.hostContext,
  });
  final String source;
  final BoxFit fit;
  final ComposeDslWebViewHostContext hostContext;

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
      vfsOwner: (
        widget.hostContext.packageName,
        widget.hostContext.executionContextKey,
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
