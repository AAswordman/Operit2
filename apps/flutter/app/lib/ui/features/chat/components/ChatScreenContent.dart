// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/chat/OperitChatRuntime.dart';
import 'AgentChatInputSection.dart';
import 'ChatArea.dart';

class ChatScreenContent extends StatelessWidget {
  const ChatScreenContent({
    super.key,
    required this.messages,
    required this.loading,
    required this.errorMessage,
    required this.messageController,
    required this.inputFocusNode,
    required this.scrollController,
    required this.inputProcessingState,
    required this.modelLabel,
    required this.onSendMessage,
    required this.onCancelMessage,
  });

  final List<ChatRuntimeMessage> messages;
  final bool loading;
  final String? errorMessage;
  final TextEditingController messageController;
  final FocusNode inputFocusNode;
  final ScrollController scrollController;
  final ChatInputProcessingState inputProcessingState;
  final String modelLabel;
  final VoidCallback onSendMessage;
  final VoidCallback onCancelMessage;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: <Widget>[
        Expanded(
          child: ChatArea(
            messages: messages,
            isLoading: loading,
            errorMessage: errorMessage,
            scrollController: scrollController,
          ),
        ),
        AgentChatInputSection(
          controller: messageController,
          focusNode: inputFocusNode,
          isLoading: loading,
          inputState: inputProcessingState,
          modelLabel: modelLabel,
          onSendMessage: onSendMessage,
          onCancelMessage: onCancelMessage,
        ),
      ],
    );
  }
}
