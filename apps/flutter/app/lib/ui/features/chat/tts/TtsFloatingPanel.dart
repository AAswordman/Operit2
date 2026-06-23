// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'TtsPlaybackController.dart';

class TtsFloatingPanel extends StatefulWidget {
  const TtsFloatingPanel({super.key});

  @override
  State<TtsFloatingPanel> createState() => _TtsFloatingPanelState();
}

class _TtsFloatingPanelState extends State<TtsFloatingPanel> {
  Offset _bottomRight = const Offset(16, 92);

  @override
  Widget build(BuildContext context) {
    final controller = TtsPlaybackController.instance;
    return AnimatedBuilder(
      animation: controller,
      builder: (context, _) {
        final state = controller.state;
        if (state.phase == TtsPlaybackPhase.idle) {
          return const SizedBox.shrink();
        }
        return Positioned(
          right: _bottomRight.dx,
          bottom: _bottomRight.dy,
          child: GestureDetector(
            onPanUpdate: (details) {
              setState(() {
                _bottomRight = Offset(
                  (_bottomRight.dx - details.delta.dx)
                      .clamp(0.0, 360.0)
                      .toDouble(),
                  (_bottomRight.dy - details.delta.dy)
                      .clamp(0.0, 620.0)
                      .toDouble(),
                );
              });
            },
            child: Material(
              elevation: 10,
              borderRadius: BorderRadius.circular(22),
              color: Theme.of(context).colorScheme.surfaceContainerHigh,
              child: Container(
                width: 270,
                padding: const EdgeInsets.fromLTRB(14, 12, 10, 10),
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(22),
                  border: Border.all(
                    color: Theme.of(context).colorScheme.outlineVariant,
                  ),
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Row(
                      children: <Widget>[
                        Icon(
                          Icons.graphic_eq,
                          size: 18,
                          color: Theme.of(context).colorScheme.primary,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            state.title.isEmpty ? '语音朗读' : state.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.titleSmall,
                          ),
                        ),
                        IconButton(
                          tooltip: '关闭',
                          visualDensity: VisualDensity.compact,
                          icon: const Icon(Icons.close, size: 18),
                          onPressed: () {
                            if (state.phase == TtsPlaybackPhase.playing ||
                                state.phase == TtsPlaybackPhase.paused ||
                                state.phase == TtsPlaybackPhase.preparing) {
                              controller.stop();
                            } else {
                              controller.clearStopped();
                            }
                          },
                        ),
                      ],
                    ),
                    const SizedBox(height: 6),
                    Text(
                      _statusText(state),
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                    if (state.currentText.isNotEmpty) ...<Widget>[
                      const SizedBox(height: 6),
                      Text(
                        state.currentText,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                    if (state.error != null) ...<Widget>[
                      const SizedBox(height: 6),
                      Text(
                        state.error!,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                    ],
                    const SizedBox(height: 8),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: <Widget>[
                        IconButton.filledTonal(
                          tooltip: state.phase == TtsPlaybackPhase.paused
                              ? '继续'
                              : '暂停',
                          icon: Icon(
                            state.phase == TtsPlaybackPhase.paused
                                ? Icons.play_arrow
                                : Icons.pause,
                            size: 18,
                          ),
                          onPressed: state.phase == TtsPlaybackPhase.playing
                              ? controller.pause
                              : state.phase == TtsPlaybackPhase.paused
                              ? controller.resume
                              : null,
                        ),
                        const SizedBox(width: 8),
                        IconButton.filledTonal(
                          tooltip: '停止',
                          icon: const Icon(Icons.stop, size: 18),
                          onPressed: state.phase == TtsPlaybackPhase.playing ||
                                  state.phase == TtsPlaybackPhase.paused ||
                                  state.phase == TtsPlaybackPhase.preparing
                              ? controller.stop
                              : null,
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

String _statusText(TtsPlaybackState state) {
  final segment = state.audioCount > 0
      ? ' · ${state.audioIndex}/${state.audioCount}'
      : '';
  final queued = state.queueLength > 0 ? ' · 队列 ${state.queueLength}' : '';
  return '${_phaseText(state.phase)}$segment$queued';
}

String _phaseText(TtsPlaybackPhase phase) {
  return switch (phase) {
    TtsPlaybackPhase.idle => '空闲',
    TtsPlaybackPhase.preparing => '生成中',
    TtsPlaybackPhase.playing => '播放中',
    TtsPlaybackPhase.paused => '已暂停',
    TtsPlaybackPhase.stopped => '已停止',
    TtsPlaybackPhase.error => '出错',
  };
}
