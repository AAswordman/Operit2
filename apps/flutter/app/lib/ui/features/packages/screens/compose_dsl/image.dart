// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Remembers the decoded image source across unrelated Compose recompositions.
class _ComposeStableImage extends StatefulWidget {
  const _ComposeStableImage({required this.source, required this.fit});
  final String source;
  final BoxFit fit;

  @override
  State<_ComposeStableImage> createState() => _ComposeStableImageState();
}

class _ComposeStableImageState extends State<_ComposeStableImage> {
  late ImageProvider _provider;

  @override
  void initState() {
    super.initState();
    _resolveSource();
  }

  @override
  void didUpdateWidget(covariant _ComposeStableImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.source != widget.source) _resolveSource();
  }

  /// Resolves only a changed source; MemoryImage equality depends on byte identity.
  void _resolveSource() {
    final uri = Uri.tryParse(widget.source);
    _provider = switch (uri?.scheme) {
      'data' => MemoryImage(UriData.parse(widget.source).contentAsBytes()),
      'file' => FileImage(File.fromUri(uri!)),
      _ =>
        widget.source.startsWith('/')
            ? FileImage(File(widget.source))
            : NetworkImage(widget.source),
    };
  }

  @override
  Widget build(BuildContext context) => RepaintBoundary(
    child: Image(image: _provider, fit: widget.fit, gaplessPlayback: true),
  );
}
