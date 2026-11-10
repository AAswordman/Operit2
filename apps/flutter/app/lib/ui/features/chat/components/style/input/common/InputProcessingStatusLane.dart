// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../../l10n/generated/app_localizations.dart';

/// The fixed vertical space reserved at the bottom of the transcript.
const double inputProcessingStatusLaneHeight = 56;

/// Paints the input-processing status inside the transcript overlay lane.
class InputProcessingStatusLane extends StatelessWidget {
  const InputProcessingStatusLane({
    super.key,
    required this.visible,
    required this.status,
    required this.textStyle,
    this.isSummarizing = false,
  });

  final bool visible;
  final String status;
  final TextStyle? textStyle;
  final bool isSummarizing;

  /// Builds the always-reserved status lane and its optional floating label.
  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: double.infinity,
      height: inputProcessingStatusLaneHeight,
      child: Stack(
        children: <Widget>[
          if (visible)
            Positioned(
              left: 12,
              right: 12,
              bottom: 4,
              child: IgnorePointer(
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (isSummarizing) ...[
                        const SizedBox(
                          key: ValueKey('summary-processing-indicator'),
                          width: 14,
                          height: 14,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                        const SizedBox(width: 8),
                      ],
                      Flexible(child: Text(status, style: textStyle)),
                    ],
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Converts a processing state into a visible status message.
String inputProcessingStatusText(
  AppLocalizations l10n,
  core_proxy.InputProcessingState state,
) {
  final message = _inputProcessingMessageText(l10n, state.message);
  if (message.isNotEmpty) {
    return message;
  }
  return switch (state.tag) {
    'Idle' => '',
    'Completed' => '',
    'Error' => state.message.trim(),
    'Processing' => l10n.processingMessage,
    'Connecting' => l10n.connectingAiService,
    'Receiving' => l10n.receivingAiResponse,
    'Summarizing' => l10n.summarizingMemories,
    'ExecutingPlan' => l10n.executingPlan,
    'ExecutingTool' => l10n.executingTool(state.toolName),
    'ProcessingToolResult' => l10n.processingToolResult(state.toolName),
    'ToolProgress' => _toolProgressStatusText(l10n, state),
    _ => throw ArgumentError(
      'Unsupported input processing state tag: ${state.tag}',
    ),
  };
}

/// Converts tool progress into a visible status message.
String _toolProgressStatusText(
  AppLocalizations l10n,
  core_proxy.InputProcessingState state,
) {
  final message = _inputProcessingMessageText(l10n, state.message);
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

/// Maps internal processing message keys to localized text.
String _inputProcessingMessageText(AppLocalizations l10n, String key) {
  return switch (key) {
    '' => '',
    'enhanced_processing_input' => l10n.processingInput,
    'enhanced_processing_message' => l10n.processingMessage,
    'enhanced_connecting_service' => l10n.connectingAiService,
    'enhanced_receiving_response' => l10n.receivingAiResponse,
    'enhanced_receiving_tool_result' => l10n.receivingToolResultAiResponse,
    'message_processing' => l10n.processingMessage,
    'message_summarizing' => l10n.summarizingMemories,
    'chat_summarizing_generating' => l10n.summaryGenerating,
    'summary_preparing' => l10n.summaryGenerating,
    'summary_core_task' => l10n.summaryStageCore,
    'summary_user_constraints' => l10n.summaryStageConstraints,
    'summary_key_files' => l10n.summaryStageFiles,
    'summary_errors' => l10n.summaryStageErrors,
    'summary_interaction' => l10n.summaryStageInteraction,
    'summary_next_steps' => l10n.summaryStageNext,
    'summary_formatting' => l10n.summaryStageFormatting,
    'chat_compressing_history' => l10n.chatCompressingHistory,
    _ => key.trim(),
  };
}

/// Estimated stage within a streamed summary, not a percentage of model work.
double summaryStageProgress(String message) => switch (message) {
  'summary_core_task' => 0.2,
  'summary_user_constraints' => 0.32,
  'summary_key_files' => 0.44,
  'summary_errors' => 0.56,
  'summary_interaction' => 0.68,
  'summary_next_steps' => 0.8,
  'summary_formatting' => 0.9,
  _ => 0.05,
};
