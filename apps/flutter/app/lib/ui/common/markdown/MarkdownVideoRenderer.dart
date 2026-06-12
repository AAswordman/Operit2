// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:video_player/video_player.dart';

import 'MarkdownAudioRenderer.dart';
import 'MarkdownImageRenderer.dart';

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

bool isLikelyVideoUrl(String url) {
  final extension = normalizeMarkdownMediaUrl(url).split('.').last;
  return _markdownVideoExtensions.contains(extension);
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
  late VideoPlayerController _controller;
  late Future<void> _initializeFuture;

  @override
  void initState() {
    super.initState();
    _createController();
  }

  @override
  void didUpdateWidget(covariant MarkdownVideoRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.videoMarkdown != widget.videoMarkdown) {
      _controller.dispose();
      _createController();
    }
  }

  void _createController() {
    final videoUrl = extractMarkdownImageUrl(widget.videoMarkdown);
    _controller = VideoPlayerController.networkUrl(Uri.parse(videoUrl));
    _initializeFuture = _controller.initialize().then((_) {
      if (mounted) {
        setState(() {});
      }
    });
    _controller.addListener(_handleControllerChanged);
  }

  void _handleControllerChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  @override
  void dispose() {
    _controller
      ..removeListener(_handleControllerChanged)
      ..dispose();
    super.dispose();
  }

  Future<void> _togglePlayback() async {
    if (_controller.value.isPlaying) {
      await _controller.pause();
    } else {
      await _controller.play();
    }
  }

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
                  child: FutureBuilder<void>(
                    future: _initializeFuture,
                    builder: (context, snapshot) {
                      final ready =
                          snapshot.connectionState == ConnectionState.done;
                      if (!ready) {
                        return const AspectRatio(
                          aspectRatio: 16 / 9,
                          child: Center(child: CircularProgressIndicator()),
                        );
                      }
                      return Stack(
                        alignment: Alignment.center,
                        children: <Widget>[
                          AspectRatio(
                            aspectRatio: _controller.value.aspectRatio == 0
                                ? 16 / 9
                                : _controller.value.aspectRatio,
                            child: VideoPlayer(_controller),
                          ),
                          IconButton.filledTonal(
                            onPressed: _togglePlayback,
                            icon: Icon(
                              _controller.value.isPlaying
                                  ? Icons.pause
                                  : Icons.play_arrow,
                            ),
                            tooltip: _controller.value.isPlaying
                                ? 'Pause'
                                : 'Play',
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
