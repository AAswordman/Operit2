// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:video_player/video_player.dart';

import '../../features/packages/screens/ComposeDslImageSource.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/logging/ClientLogger.dart';
import '../../features/chat/components/style/input/common/ChatAttachmentImagePreview.dart';
import '../interactions/MessagePressShield.dart';
import 'MarkdownFileScope.dart';
import 'MarkdownAudioRenderer.dart';
import 'MarkdownImageRenderer.dart';
import 'MarkdownLink.dart';

const Set<String> _markdownVideoExtensions = <String>{
  'mp4',
  'webm',
  'mkv',
  'mov',
  'm4v',
  '3gp',
  'avi',
  'ogv',
};

/// Recognizes video destinations in Markdown image syntax.
bool isLikelyVideoUrl(String url) {
  if (url.trim().toLowerCase().startsWith('data:video/')) return true;
  final extension = normalizeMarkdownMediaUrl(url).split('.').last;
  return _markdownVideoExtensions.contains(extension);
}

/// Resolves local and VFS videos through the same file host as attachments.
Future<Uri> resolveMarkdownVideoUri(
  String source, {
  String? basePath,
  String? nodeId,
  GeneratedCoreProxyClients clients = imageSourceClients,
}) async {
  final resolved = resolveMarkdownFilePath(source, basePath);
  final uri = Uri.tryParse(resolved);
  if (uri != null &&
      (uri.scheme == 'http' || uri.scheme == 'https' || uri.scheme == 'data')) {
    return uri;
  }
  final bytes = await readChatAttachmentFileBytes(
    resolved,
    sourceNodeId: nodeId,
    clients: clients,
  );
  final extension = normalizeMarkdownMediaUrl(resolved).split('.').last;
  final mimeType = switch (extension) {
    'mp4' || 'm4v' => 'video/mp4',
    'webm' => 'video/webm',
    'mkv' => 'video/x-matroska',
    'mov' => 'video/quicktime',
    '3gp' => 'video/3gpp',
    'avi' => 'video/x-msvideo',
    'ogv' => 'video/ogg',
    _ => throw UnsupportedError('Unsupported Markdown video: $source'),
  };
  return Uri.dataFromBytes(bytes, mimeType: mimeType);
}

class MarkdownVideoRenderer extends StatefulWidget {
  const MarkdownVideoRenderer({
    super.key,
    required this.videoMarkdown,
    required this.textColor,
    this.maxVideoHeight = 220,
  });

  final String videoMarkdown;
  final Color textColor;
  final double maxVideoHeight;

  @override
  State<MarkdownVideoRenderer> createState() => _MarkdownVideoRendererState();
}

class _MarkdownVideoRendererState extends State<MarkdownVideoRenderer> {
  VideoPlayerController? _controller;
  Future<void>? _initializeFuture;
  bool _failed = false;
  MarkdownFileScope? _scope;
  int _loadGeneration = 0;

  /// Creates the controller after the message's file scope becomes available.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final scope = MarkdownFileScope.maybeOf(context);
    if (_initializeFuture == null ||
        scope?.basePath != _scope?.basePath ||
        scope?.nodeId != _scope?.nodeId ||
        scope?.clients.bridge != _scope?.clients.bridge) {
      _scope = scope;
      _releaseController();
      _createController();
    }
  }

  /// Replaces the source when streamed Markdown changes its destination.
  @override
  void didUpdateWidget(covariant MarkdownVideoRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.videoMarkdown != widget.videoMarkdown) {
      _releaseController();
      _createController();
    }
  }

  /// Starts one source request and clears the previous playback failure.
  void _createController() {
    _failed = false;
    final videoUrl = extractMarkdownImageUrl(widget.videoMarkdown);
    _initializeFuture = _initializeController(videoUrl, ++_loadGeneration);
  }

  /// Initializes only the current request and disposes obsolete asynchronous loads.
  Future<void> _initializeController(String videoUrl, int generation) async {
    VideoPlayerController? controller;
    try {
      final scope = _scope;
      final uri = await resolveMarkdownVideoUri(
        videoUrl,
        basePath: scope?.basePath,
        nodeId: scope?.nodeId,
        clients: scope?.clients ?? imageSourceClients,
      );
      if (!mounted || generation != _loadGeneration) return;
      controller = VideoPlayerController.networkUrl(uri);
      await controller.initialize();
      if (!mounted || generation != _loadGeneration) {
        await controller.dispose();
        return;
      }
      controller.addListener(_handleControllerChanged);
      setState(() => _controller = controller);
    } catch (error, stackTrace) {
      await controller?.dispose();
      if (!mounted || generation != _loadGeneration) return;
      ClientLogger.w(
        'Cannot load markdown video: $videoUrl',
        tag: 'MarkdownVideo',
        error: error,
        stackTrace: stackTrace,
      );
      if (mounted) {
        setState(() {
          _failed = true;
        });
      }
    }
  }

  /// Repaints playback controls as the player state changes.
  void _handleControllerChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  /// Invalidates pending loads and releases the active player.
  void _releaseController() {
    _loadGeneration++;
    final controller = _controller;
    _controller = null;
    _initializeFuture = null;
    if (controller == null) {
      return;
    }
    controller.removeListener(_handleControllerChanged);
    controller.dispose();
  }

  /// Releases playback and prevents late source reads from creating a player.
  @override
  void dispose() {
    _releaseController();
    super.dispose();
  }

  /// Toggles playback for the initialized video.
  Future<void> _togglePlayback() async {
    final controller = _controller;
    if (controller == null) {
      return;
    }
    try {
      if (controller.value.isPlaying) {
        await controller.pause();
      } else {
        await controller.play();
      }
    } catch (error, stackTrace) {
      ClientLogger.w(
        'Cannot play markdown video',
        tag: 'MarkdownVideo',
        error: error,
        stackTrace: stackTrace,
      );
      if (mounted) {
        setState(() {
          _failed = true;
        });
      }
    }
  }

  /// Displays loading, playback controls or the source error.
  @override
  Widget build(BuildContext context) {
    if (!isCompleteImageMarkdown(widget.videoMarkdown)) {
      return const SizedBox.shrink();
    }

    final videoAlt = extractMarkdownImageAlt(widget.videoMarkdown);
    final videoUrl = extractMarkdownImageUrl(widget.videoMarkdown);
    if (videoUrl.isEmpty || !isLikelyVideoUrl(videoUrl)) {
      return const SizedBox.shrink();
    }

    final theme = Theme.of(context);
    final initializeFuture = _initializeFuture;
    final controller = _controller;
    return Semantics(
      label: videoAlt.isNotEmpty ? 'Video: $videoAlt' : 'Video',
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            ClipRRect(
              borderRadius: BorderRadius.circular(12),
              child: ColoredBox(
                color: theme.colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.18,
                ),
                child: ConstrainedBox(
                  constraints: BoxConstraints(maxHeight: widget.maxVideoHeight),
                  child: _failed || initializeFuture == null
                      ? _MarkdownMediaFailure(
                          label: videoAlt.isEmpty ? videoUrl : videoAlt,
                          onTap: () => activateMarkdownLink(videoUrl, null),
                        )
                      : FutureBuilder<void>(
                          future: initializeFuture,
                          builder: (context, snapshot) {
                            if (_failed || snapshot.hasError) {
                              return _MarkdownMediaFailure(
                                label: videoAlt.isEmpty ? videoUrl : videoAlt,
                                onTap: () =>
                                    activateMarkdownLink(videoUrl, null),
                              );
                            }
                            final ready =
                                snapshot.connectionState ==
                                    ConnectionState.done &&
                                controller != null;
                            if (!ready) {
                              return const AspectRatio(
                                aspectRatio: 16 / 9,
                                child: Center(
                                  child: CircularProgressIndicator(),
                                ),
                              );
                            }
                            return Stack(
                              alignment: Alignment.center,
                              children: <Widget>[
                                AspectRatio(
                                  aspectRatio: controller.value.aspectRatio == 0
                                      ? 16 / 9
                                      : controller.value.aspectRatio,
                                  child: VideoPlayer(controller),
                                ),
                                MessagePressShieldRegion(
                                  child: IconButton.filledTonal(
                                    onPressed: _togglePlayback,
                                    icon: Icon(
                                      controller.value.isPlaying
                                          ? Icons.pause
                                          : Icons.play_arrow,
                                    ),
                                    tooltip: controller.value.isPlaying
                                        ? 'Pause'
                                        : 'Play',
                                  ),
                                ),
                              ],
                            );
                          },
                        ),
                ),
              ),
            ),
            if (videoAlt.isNotEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 1),
                child: Text(
                  videoAlt,
                  textAlign: TextAlign.center,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant.withValues(
                      alpha: 0.7,
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _MarkdownMediaFailure extends StatelessWidget {
  const _MarkdownMediaFailure({required this.label, required this.onTap});

  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.onSurfaceVariant;
    return MessagePressShieldRegion(
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 8),
          child: Row(
            children: <Widget>[
              Icon(Icons.broken_image_outlined, size: 18, color: color),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  label,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: color,
                    decoration: TextDecoration.underline,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
