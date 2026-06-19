// ignore_for_file: file_names

import 'dart:async';

import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/material.dart';

import '../interactions/MessagePressShield.dart';
import 'MarkdownImageRenderer.dart';

const Set<String> _markdownAudioExtensions = <String>{
  'mp3',
  'wav',
  'ogg',
  'oga',
  'm4a',
  'aac',
  'flac',
  'opus',
  'weba',
};

String normalizeMarkdownMediaUrl(String url) {
  return url.split('#').first.split('?').first.trim().toLowerCase();
}

bool isLikelyAudioUrl(String url) {
  final extension = normalizeMarkdownMediaUrl(url).split('.').last;
  return _markdownAudioExtensions.contains(extension);
}

class MarkdownAudioRenderer extends StatefulWidget {
  const MarkdownAudioRenderer({
    super.key,
    required this.audioMarkdown,
    required this.textColor,
  });

  final String audioMarkdown;
  final Color textColor;

  @override
  State<MarkdownAudioRenderer> createState() => _MarkdownAudioRendererState();
}

class _MarkdownAudioRendererState extends State<MarkdownAudioRenderer> {
  late final AudioPlayer _player;
  StreamSubscription<Duration>? _durationSubscription;
  StreamSubscription<Duration>? _positionSubscription;
  StreamSubscription<PlayerState>? _stateSubscription;
  Duration _duration = Duration.zero;
  Duration _position = Duration.zero;
  PlayerState _playerState = PlayerState.stopped;

  bool get _isPlaying => _playerState == PlayerState.playing;

  @override
  void initState() {
    super.initState();
    _player = AudioPlayer();
    final audioUrl = extractMarkdownImageUrl(widget.audioMarkdown);
    unawaited(_player.setSourceUrl(audioUrl));
    _durationSubscription = _player.onDurationChanged.listen((duration) {
      if (mounted) {
        setState(() => _duration = duration);
      }
    });
    _positionSubscription = _player.onPositionChanged.listen((position) {
      if (mounted) {
        setState(() => _position = position);
      }
    });
    _stateSubscription = _player.onPlayerStateChanged.listen((state) {
      if (mounted) {
        setState(() => _playerState = state);
      }
    });
  }

  @override
  void didUpdateWidget(covariant MarkdownAudioRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.audioMarkdown != widget.audioMarkdown) {
      final audioUrl = extractMarkdownImageUrl(widget.audioMarkdown);
      unawaited(_player.setSourceUrl(audioUrl));
      setState(() {
        _duration = Duration.zero;
        _position = Duration.zero;
        _playerState = PlayerState.stopped;
      });
    }
  }

  @override
  void dispose() {
    unawaited(_durationSubscription?.cancel());
    unawaited(_positionSubscription?.cancel());
    unawaited(_stateSubscription?.cancel());
    unawaited(_player.dispose());
    super.dispose();
  }

  Future<void> _togglePlayback() async {
    if (_isPlaying) {
      await _player.pause();
    } else {
      await _player.resume();
    }
  }

  @override
  Widget build(BuildContext context) {
    if (!isCompleteImageMarkdown(widget.audioMarkdown)) {
      return const SizedBox.shrink();
    }

    final audioAlt = extractMarkdownImageAlt(widget.audioMarkdown);
    final audioUrl = extractMarkdownImageUrl(widget.audioMarkdown);
    if (audioUrl.isEmpty || !isLikelyAudioUrl(audioUrl)) {
      return const SizedBox.shrink();
    }

    final theme = Theme.of(context);
    final maxSeconds = _duration.inMilliseconds <= 0
        ? 1.0
        : _duration.inMilliseconds.toDouble();
    final currentSeconds = _position.inMilliseconds
        .clamp(0, maxSeconds.toInt())
        .toDouble();

    return Semantics(
      label: audioAlt.isNotEmpty ? 'Audio: $audioAlt' : 'Audio',
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Container(
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.18,
                ),
                borderRadius: BorderRadius.circular(12),
              ),
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
              child: Row(
                children: <Widget>[
                  MessagePressShieldRegion(
                    child: IconButton(
                      onPressed: _togglePlayback,
                      icon: Icon(_isPlaying ? Icons.pause : Icons.play_arrow),
                      tooltip: _isPlaying ? 'Pause' : 'Play',
                    ),
                  ),
                  Expanded(
                    child: MessagePressShieldRegion(
                      child: Slider(
                        value: currentSeconds,
                        min: 0,
                        max: maxSeconds,
                        onChanged: (value) {
                          unawaited(
                            _player.seek(Duration(milliseconds: value.toInt())),
                          );
                        },
                      ),
                    ),
                  ),
                ],
              ),
            ),
            if (audioAlt.isNotEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 1),
                child: Text(
                  audioAlt,
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
