// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../viewmodel/ChatViewModel.dart';
import 'BubbleAiMessageComposable.dart';
import 'BubbleSurface.dart';
import 'BubbleUserMessageComposable.dart';

class BubbleStyleChatMessage extends StatelessWidget {
  const BubbleStyleChatMessage({
    super.key,
    required this.message,
    required this.isStreaming,
    required this.userMessageColor,
    required this.aiMessageColor,
    required this.userTextColor,
    required this.aiTextColor,
    required this.systemMessageColor,
    required this.systemTextColor,
    this.transparentSurface = false,
    this.userBubbleImageStyle,
    this.aiBubbleImageStyle,
    this.bubbleUserRoundedCornersEnabled = true,
    this.bubbleAiRoundedCornersEnabled = true,
    this.bubbleUserContentPaddingLeft = 12,
    this.bubbleUserContentPaddingRight = 12,
    this.bubbleAiContentPaddingLeft = 12,
    this.bubbleAiContentPaddingRight = 12,
    this.initialThinkingExpanded = false,
    this.allowExpandedThinkingFullHeight = false,
    this.expandThinkToolsGroups = false,
    this.forceShowThinkingProcess = false,
    this.isHidden = false,
    this.enableDialogs = true,
    this.onRoleAvatarLongPress,
  });

  final ChatUiMessage message;
  final bool isStreaming;
  final Color userMessageColor;
  final Color aiMessageColor;
  final Color userTextColor;
  final Color aiTextColor;
  final Color systemMessageColor;
  final Color systemTextColor;
  final bool transparentSurface;
  final BubbleImageStyle? userBubbleImageStyle;
  final BubbleImageStyle? aiBubbleImageStyle;
  final bool bubbleUserRoundedCornersEnabled;
  final bool bubbleAiRoundedCornersEnabled;
  final double bubbleUserContentPaddingLeft;
  final double bubbleUserContentPaddingRight;
  final double bubbleAiContentPaddingLeft;
  final double bubbleAiContentPaddingRight;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final bool expandThinkToolsGroups;
  final bool forceShowThinkingProcess;
  final bool isHidden;
  final bool enableDialogs;
  final void Function(String roleName)? onRoleAvatarLongPress;

  @override
  Widget build(BuildContext context) {
    switch (message.sender) {
      case 'user':
        return BubbleUserMessageComposable(
          message: message,
          backgroundColor: userMessageColor,
          textColor: userTextColor,
          transparentSurface: transparentSurface,
          bubbleImageStyle: userBubbleImageStyle,
          bubbleRoundedCornersEnabled: bubbleUserRoundedCornersEnabled,
          bubbleContentPaddingLeft: bubbleUserContentPaddingLeft,
          bubbleContentPaddingRight: bubbleUserContentPaddingRight,
          enableDialogs: enableDialogs,
        );
      case 'ai':
        return BubbleAiMessageComposable(
          message: message,
          isStreaming: isStreaming,
          backgroundColor: aiMessageColor,
          textColor: aiTextColor,
          transparentSurface: transparentSurface,
          bubbleImageStyle: aiBubbleImageStyle,
          bubbleRoundedCornersEnabled: bubbleAiRoundedCornersEnabled,
          bubbleContentPaddingLeft: bubbleAiContentPaddingLeft,
          bubbleContentPaddingRight: bubbleAiContentPaddingRight,
          initialThinkingExpanded: initialThinkingExpanded,
          allowExpandedThinkingFullHeight: allowExpandedThinkingFullHeight,
          expandThinkToolsGroups: expandThinkToolsGroups,
          forceShowThinkingProcess: forceShowThinkingProcess,
          isHidden: isHidden,
          enableDialogs: enableDialogs,
          onAvatarLongPressMention: onRoleAvatarLongPress,
        );
      case 'summary':
        return _SummaryMessageComposable(
          message: message,
          backgroundColor: systemMessageColor.withValues(alpha: 0.7),
          textColor: systemTextColor,
        );
    }
    return _SystemMessageComposable(
      message: message,
      textColor: systemTextColor,
    );
  }
}

class _SummaryMessageComposable extends StatelessWidget {
  const _SummaryMessageComposable({
    required this.message,
    required this.backgroundColor,
    required this.textColor,
  });

  final ChatUiMessage message;
  final Color backgroundColor;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
      decoration: BoxDecoration(
        color: backgroundColor,
        borderRadius: BorderRadius.circular(8),
      ),
      child: SelectableText(
        message.content,
        style: theme.textTheme.bodySmall?.copyWith(
          color: textColor,
          height: 1.4,
        ),
      ),
    );
  }
}

class _SystemMessageComposable extends StatelessWidget {
  const _SystemMessageComposable({
    required this.message,
    required this.textColor,
  });

  final ChatUiMessage message;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: SelectableText(
        message.content,
        style: theme.textTheme.bodySmall?.copyWith(color: textColor),
      ),
    );
  }
}
