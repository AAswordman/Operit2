// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/chat/OperitChatRuntime.dart';
import '../../../../l10n/generated/app_localizations.dart';
import 'ChatLayoutMetrics.dart';

class AgentChatInputSection extends StatelessWidget {
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
    final l10n = AppLocalizations.of(context)!;
    final processing = isLoading || inputState.isProcessing;
    final hasDraftText = controller.text.trim().isNotEmpty;
    final showCancelAction = processing && !hasDraftText;
    final showQueueAction = processing && hasDraftText;
    final processingStatus = _inputProcessingStatus(l10n, inputState);
    final showProcessingStatus =
        inputState.isProcessing && processingStatus.isNotEmpty;
    final inputCardShape = const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
    );

    return Material(
      color: Colors.transparent,
      child: Align(
        alignment: Alignment.bottomCenter,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: chatContentMaxWidth),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (showProcessingStatus)
                Padding(
                  padding: const EdgeInsets.fromLTRB(12, 4, 12, 0),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      processingStatus,
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
                  color: colorScheme.surfaceContainerLow,
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
                          hintText: l10n.askOperitHint,
                          hintStyle: theme.textTheme.bodyMedium?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                            fontSize: 14,
                          ),
                          suffixIcon: IconButton(
                            onPressed: () {},
                            icon: const Icon(Icons.fullscreen),
                            color: colorScheme.onSurfaceVariant,
                            tooltip: l10n.fullscreenInput,
                          ),
                          border: InputBorder.none,
                          enabledBorder: InputBorder.none,
                          focusedBorder: InputBorder.none,
                          contentPadding: const EdgeInsets.fromLTRB(
                            16,
                            10,
                            8,
                            8,
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
                          const SizedBox(width: 6),
                          _IconTapTarget(
                            icon: Icons.tune_outlined,
                            color: colorScheme.onSurfaceVariant,
                            onTap: onSettings,
                            targetSize: 34,
                            tooltip: l10n.settings,
                          ),
                          const SizedBox(width: 8),
                          _IconTapTarget(
                            icon: Icons.add,
                            color: colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.9,
                            ),
                            onTap: onAttach,
                            size: 24,
                            tooltip: l10n.addAttachment,
                          ),
                          const SizedBox(width: 6),
                          _ActionButton(
                            processing: processing,
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
                            tooltip: showCancelAction
                                ? l10n.cancel
                                : (hasDraftText ? l10n.send : ''),
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

String _inputProcessingStatus(
  AppLocalizations l10n,
  ChatInputProcessingState state,
) {
  final message = _inputProcessingMessage(l10n, state.message);
  if (message.isNotEmpty) {
    return message;
  }
  return switch (state.kind) {
    'Processing' => l10n.processingMessage,
    'Connecting' => l10n.connectingAiService,
    'Receiving' => l10n.receivingAiResponse,
    'Summarizing' => l10n.summarizingMemories,
    'ExecutingPlan' => l10n.executingPlan,
    'ExecutingTool' => l10n.executingTool(state.toolName),
    'ProcessingToolResult' => l10n.processingToolResult(state.toolName),
    'ToolProgress' => _toolProgressStatus(l10n, state),
    _ => '',
  };
}

String _toolProgressStatus(
  AppLocalizations l10n,
  ChatInputProcessingState state,
) {
  final message = _inputProcessingMessage(l10n, state.message);
  if (message.isNotEmpty) {
    return state.toolName.isEmpty
        ? message
        : l10n.toolStatusWithName(state.toolName, message);
  }
  if (state.toolName.isEmpty) {
    return l10n.toolRunning;
  }
  return l10n.toolRunningWithName(state.toolName);
}

String _inputProcessingMessage(AppLocalizations l10n, String key) {
  return switch (key) {
    'enhanced_processing_input' => l10n.processingInput,
    'enhanced_processing_message' => l10n.processingMessage,
    'enhanced_connecting_service' => l10n.connectingAiService,
    'enhanced_receiving_response' => l10n.receivingAiResponse,
    'enhanced_receiving_tool_result' => l10n.receivingToolResultAiResponse,
    'message_processing' => l10n.processingMessage,
    'message_summarizing' => l10n.summarizingMemories,
    _ => key,
  };
}

class _IconTapTarget extends StatelessWidget {
  const _IconTapTarget({
    required this.icon,
    required this.color,
    required this.onTap,
    required this.tooltip,
    this.size = 20,
    this.targetSize = 36,
  });

  final IconData icon;
  final Color color;
  final VoidCallback? onTap;
  final String tooltip;
  final double size;
  final double targetSize;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InkResponse(
        onTap: onTap,
        radius: 20,
        child: SizedBox(
          width: targetSize,
          height: targetSize,
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
    required this.tooltip,
    required this.onPressed,
  });

  final bool processing;
  final double progress;
  final Color background;
  final Color foreground;
  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final button = SizedBox(
      width: 40,
      height: 40,
      child: Stack(
        alignment: Alignment.center,
        children: <Widget>[
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
          if (processing)
            Positioned.fill(
              child: IgnorePointer(
                child: CircularProgressIndicator(
                  value: progress,
                  strokeWidth: 2.4,
                  color: foreground.withValues(alpha: 0.9),
                  backgroundColor: foreground.withValues(alpha: 0.24),
                ),
              ),
            ),
        ],
      ),
    );
    if (tooltip.isEmpty) {
      return button;
    }
    return Tooltip(message: tooltip, child: button);
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
