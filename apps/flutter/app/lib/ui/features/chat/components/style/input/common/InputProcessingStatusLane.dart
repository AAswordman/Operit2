// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../../l10n/generated/app_localizations.dart';

/// The fixed vertical space reserved at the bottom of the transcript.
const double inputProcessingStatusLaneHeight = 32;

/// Paints the input-processing status inside the transcript overlay lane.
class InputProcessingStatusLane extends StatelessWidget {
  const InputProcessingStatusLane({
    super.key,
    required this.visible,
    required this.status,
    required this.textStyle,
  });

  final bool visible;
  final String status;
  final TextStyle? textStyle;

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
                  child: Text(status, style: textStyle),
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
  const memberReplyingPrefix = 'role_response_planner_member_replying|';
  if (key.startsWith(memberReplyingPrefix)) {
    return l10n.roleResponsePlannerMemberReplying(
      key.substring(memberReplyingPrefix.length),
    );
  }
  return switch (key) {
    '' => '',
    'enhanced_processing_input' => l10n.processingInput,
    'enhanced_processing_message' => l10n.processingMessage,
    'enhanced_connecting_service' => l10n.connectingAiService,
    'enhanced_receiving_response' => l10n.receivingAiResponse,
    'enhanced_receiving_tool_result' => l10n.receivingToolResultAiResponse,
    'role_response_planner_planning' => l10n.roleResponsePlannerPlanning,
    'role_response_planner_failed' => l10n.roleResponsePlannerFailed,
    'message_processing' => l10n.processingMessage,
    'message_summarizing' => l10n.summarizingMemories,
    'chat_summarizing_generating' => l10n.summaryGenerating,
    'chat_compressing_history' => l10n.chatCompressingHistory,
    _ => key.trim(),
  };
}
