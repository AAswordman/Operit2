// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/chat/OperitChatRuntime.dart';

class AgentChatInputSection extends StatelessWidget {
  static const double _maxInputWidth = 860;

  const AgentChatInputSection({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.isLoading,
    required this.inputState,
    required this.modelLabel,
    required this.onSendMessage,
    required this.onCancelMessage,
    this.onAttach,
    this.onSettings,
    this.onModelSelector,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool isLoading;
  final ChatInputProcessingState inputState;
  final String modelLabel;
  final VoidCallback onSendMessage;
  final VoidCallback onCancelMessage;
  final VoidCallback? onAttach;
  final VoidCallback? onSettings;
  final VoidCallback? onModelSelector;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final processing = isLoading || inputState.isProcessing;
    final hasDraftText = controller.text.trim().isNotEmpty;
    final showCancelAction = processing && !hasDraftText;
    final showQueueAction = processing && hasDraftText;
    final showProcessingStatus =
        inputState.isProcessing && inputState.displayMessage.isNotEmpty;
    final inputCardShape = const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
    );

    return Material(
      color: Colors.transparent,
      child: Align(
        alignment: Alignment.bottomCenter,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: _maxInputWidth),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (showProcessingStatus)
                Padding(
                  padding: const EdgeInsets.fromLTRB(12, 4, 12, 0),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      inputState.displayMessage,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurface.withValues(alpha: 0.8),
                      ),
                    ),
                  ),
                ),
              Container(
                width: double.infinity,
                margin: const EdgeInsets.only(top: 4),
                decoration: ShapeDecoration(
                  color: colorScheme.surface,
                  shape: inputCardShape,
                  shadows: <BoxShadow>[
                    BoxShadow(
                      color: Colors.black.withValues(alpha: 0.08),
                      blurRadius: 18,
                      spreadRadius: 1,
                      offset: const Offset(0, -4),
                    ),
                    BoxShadow(
                      color: Colors.black.withValues(alpha: 0.035),
                      blurRadius: 5,
                      spreadRadius: 0,
                      offset: const Offset(0, -1),
                    ),
                  ],
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 8,
                  ),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      TextField(
                        controller: controller,
                        focusNode: focusNode,
                        minLines: 1,
                        maxLines: 6,
                        enabled: true,
                        textInputAction: TextInputAction.newline,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          fontSize: 14,
                          height: 20 / 14,
                        ),
                        decoration: InputDecoration(
                          hintText: '向 Operit 提问',
                          hintStyle: theme.textTheme.bodyMedium?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                            fontSize: 14,
                          ),
                          suffixIcon: IconButton(
                            onPressed: () {},
                            icon: const Icon(Icons.fullscreen),
                            color: colorScheme.onSurfaceVariant,
                            tooltip: 'Fullscreen input',
                          ),
                          border: InputBorder.none,
                          enabledBorder: InputBorder.none,
                          focusedBorder: InputBorder.none,
                          contentPadding: const EdgeInsets.symmetric(
                            horizontal: 0,
                            vertical: 10,
                          ),
                        ),
                        onSubmitted: (_) {
                          if (hasDraftText && !processing) {
                            onSendMessage();
                          }
                        },
                      ),
                      const SizedBox(height: 8),
                      Row(
                        children: <Widget>[
                          Expanded(
                            child: Align(
                              alignment: Alignment.centerLeft,
                              child: InkWell(
                                borderRadius: BorderRadius.circular(12),
                                onTap: onModelSelector,
                                child: Container(
                                  constraints: const BoxConstraints(
                                    maxWidth: 220,
                                  ),
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 10,
                                    vertical: 6,
                                  ),
                                  decoration: BoxDecoration(
                                    border: Border.all(
                                      color: colorScheme.outline.withValues(
                                        alpha: 0.2,
                                      ),
                                    ),
                                    borderRadius: BorderRadius.circular(12),
                                  ),
                                  child: Row(
                                    mainAxisSize: MainAxisSize.min,
                                    children: <Widget>[
                                      Flexible(
                                        child: Text(
                                          modelLabel,
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                          style: theme.textTheme.bodyMedium
                                              ?.copyWith(
                                                color: colorScheme.onSurface,
                                              ),
                                        ),
                                      ),
                                      const SizedBox(width: 4),
                                      Icon(
                                        Icons.keyboard_arrow_down,
                                        size: 18,
                                        color: colorScheme.onSurfaceVariant,
                                      ),
                                    ],
                                  ),
                                ),
                              ),
                            ),
                          ),
                          _IconTapTarget(
                            icon: Icons.tune_outlined,
                            color: colorScheme.onSurfaceVariant,
                            onTap: onSettings,
                            tooltip: 'Settings',
                          ),
                          const SizedBox(width: 8),
                          _IconTapTarget(
                            icon: Icons.add,
                            color: colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.9,
                            ),
                            onTap: onAttach,
                            size: 24,
                            tooltip: 'Add attachment',
                          ),
                          const SizedBox(width: 6),
                          _ActionButton(
                            processing: showProcessingStatus,
                            progress: _progressFor(inputState),
                            background: _actionBackground(
                              colorScheme,
                              showCancelAction: showCancelAction,
                              showQueueAction: showQueueAction,
                              canSend: hasDraftText,
                            ),
                            foreground: _actionForeground(
                              colorScheme,
                              showCancelAction: showCancelAction,
                              showQueueAction: showQueueAction,
                              canSend: hasDraftText,
                            ),
                            icon: _actionIcon(
                              showCancelAction: showCancelAction,
                              showQueueAction: showQueueAction,
                              canSend: hasDraftText,
                            ),
                            onPressed: () {
                              if (showCancelAction) {
                                onCancelMessage();
                              } else if (hasDraftText) {
                                onSendMessage();
                              }
                            },
                          ),
                        ],
                      ),
                    ],
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

class _IconTapTarget extends StatelessWidget {
  const _IconTapTarget({
    required this.icon,
    required this.color,
    required this.onTap,
    required this.tooltip,
    this.size = 20,
  });

  final IconData icon;
  final Color color;
  final VoidCallback? onTap;
  final String tooltip;
  final double size;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InkResponse(
        onTap: onTap,
        radius: 20,
        child: SizedBox(
          width: 36,
          height: 36,
          child: Icon(icon, size: size, color: color),
        ),
      ),
    );
  }
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({
    required this.processing,
    required this.progress,
    required this.background,
    required this.foreground,
    required this.icon,
    required this.onPressed,
  });

  final bool processing;
  final double progress;
  final Color background;
  final Color foreground;
  final IconData icon;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 40,
      height: 40,
      child: Stack(
        alignment: Alignment.center,
        children: <Widget>[
          if (processing)
            CircularProgressIndicator(
              value: progress,
              strokeWidth: 2,
              color: background,
              backgroundColor: background.withValues(alpha: 0.2),
            ),
          Material(
            color: background,
            shape: const CircleBorder(),
            child: InkWell(
              customBorder: const CircleBorder(),
              onTap: onPressed,
              child: SizedBox(
                width: 36,
                height: 36,
                child: Icon(icon, size: 18, color: foreground),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

double _progressFor(ChatInputProcessingState state) {
  return switch (state.kind) {
    'Processing' => 0.3,
    'Connecting' => 0.6,
    'Summarizing' => 0.05,
    'ToolProgress' => state.progress.clamp(0, 1),
    _ => 1,
  };
}

Color _actionBackground(
  ColorScheme colorScheme, {
  required bool showCancelAction,
  required bool showQueueAction,
  required bool canSend,
}) {
  if (showCancelAction) {
    return colorScheme.error;
  }
  if (showQueueAction) {
    return colorScheme.tertiary;
  }
  if (canSend) {
    return colorScheme.primary;
  }
  return colorScheme.primary;
}

Color _actionForeground(
  ColorScheme colorScheme, {
  required bool showCancelAction,
  required bool showQueueAction,
  required bool canSend,
}) {
  if (showCancelAction) {
    return colorScheme.onError;
  }
  if (showQueueAction) {
    return colorScheme.onTertiary;
  }
  return colorScheme.onPrimary;
}

IconData _actionIcon({
  required bool showCancelAction,
  required bool showQueueAction,
  required bool canSend,
}) {
  if (showCancelAction) {
    return Icons.close;
  }
  if (showQueueAction) {
    return Icons.add;
  }
  if (canSend) {
    return Icons.send;
  }
  return Icons.mic;
}
