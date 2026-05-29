// ignore_for_file: file_names

import 'dart:io';
import 'dart:typed_data';

import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:video_player/video_player.dart';

class WorkspaceAudioPreview extends StatefulWidget {
  const WorkspaceAudioPreview({
    super.key,
    required this.bytes,
    required this.title,
  });

  final Uint8List bytes;
  final String title;

  @override
  State<WorkspaceAudioPreview> createState() => _WorkspaceAudioPreviewState();
}

class _WorkspaceAudioPreviewState extends State<WorkspaceAudioPreview> {
  final AudioPlayer _player = AudioPlayer();
  PlayerState _state = PlayerState.stopped;
  Duration _position = Duration.zero;
  Duration _duration = Duration.zero;

  @override
  void initState() {
    super.initState();
    _player.onPlayerStateChanged.listen((state) {
      if (mounted) {
        setState(() => _state = state);
      }
    });
    _player.onPositionChanged.listen((position) {
      if (mounted) {
        setState(() => _position = position);
      }
    });
    _player.onDurationChanged.listen((duration) {
      if (mounted) {
        setState(() => _duration = duration);
      }
    });
    _player.setSource(BytesSource(widget.bytes));
  }

  @override
  void dispose() {
    _player.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final playing = _state == PlayerState.playing;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(
              Icons.audio_file_outlined,
              size: 54,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(height: 14),
            Text(
              widget.title,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.onSurface,
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 18),
            Slider(
              value: _duration.inMilliseconds == 0
                  ? 0
                  : _position.inMilliseconds
                        .clamp(0, _duration.inMilliseconds)
                        .toDouble(),
              max: _duration.inMilliseconds == 0
                  ? 1
                  : _duration.inMilliseconds.toDouble(),
              onChanged: (value) {
                _player.seek(Duration(milliseconds: value.round()));
              },
            ),
            IconButton.filled(
              onPressed: () {
                if (playing) {
                  _player.pause();
                } else {
                  _player.resume();
                }
              },
              icon: Icon(playing ? Icons.pause : Icons.play_arrow),
            ),
          ],
        ),
      ),
    );
  }
}

class WorkspaceVideoPreview extends StatefulWidget {
  const WorkspaceVideoPreview({
    super.key,
    required this.bytes,
    required this.fileName,
  });

  final Uint8List bytes;
  final String fileName;

  @override
  State<WorkspaceVideoPreview> createState() => _WorkspaceVideoPreviewState();
}

class _WorkspaceVideoPreviewState extends State<WorkspaceVideoPreview> {
  Future<VideoPlayerController>? _controllerFuture;
  VideoPlayerController? _controller;

  @override
  void initState() {
    super.initState();
    _controllerFuture = _createController();
  }

  @override
  void dispose() {
    _controller?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<VideoPlayerController>(
      future: _controllerFuture,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(child: CircularProgressIndicator());
        }
        if (snapshot.hasError) {
          return Center(child: Text(snapshot.error.toString()));
        }
        final controller = snapshot.data!;
        return Center(
          child: AspectRatio(
            aspectRatio: controller.value.aspectRatio,
            child: Stack(
              alignment: Alignment.center,
              children: <Widget>[
                VideoPlayer(controller),
                IconButton.filled(
                  onPressed: () {
                    setState(() {
                      if (controller.value.isPlaying) {
                        controller.pause();
                      } else {
                        controller.play();
                      }
                    });
                  },
                  icon: Icon(
                    controller.value.isPlaying ? Icons.pause : Icons.play_arrow,
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  Future<VideoPlayerController> _createController() async {
    final directory = await getTemporaryDirectory();
    final file = File(
      '${directory.path}/operit-workspace-${DateTime.now().microsecondsSinceEpoch}-${widget.fileName}',
    );
    await file.writeAsBytes(widget.bytes, flush: true);
    final controller = VideoPlayerController.file(file);
    await controller.initialize();
    _controller = controller;
    return controller;
  }
}
