// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../../core/chat/OperitChatRuntime.dart';
import 'AgentChatInputSection.dart';
import 'ChatArea.dart';
import 'ChatToastHost.dart';

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
    required this.bridge,
    required this.onSendMessage,
    required this.onCancelMessage,
    required this.onModelChanged,
    required this.toastMessage,
    required this.onDismissToast,
  });

  final List<ChatRuntimeMessage> messages;
  final bool loading;
  final String? errorMessage;
  final TextEditingController messageController;
  final FocusNode inputFocusNode;
  final ScrollController scrollController;
  final ChatInputProcessingState inputProcessingState;
  final String modelLabel;
  final OperitRuntimeBridge bridge;
  final VoidCallback onSendMessage;
  final VoidCallback onCancelMessage;
  final ValueChanged<String> onModelChanged;
  final String? toastMessage;
  final VoidCallback onDismissToast;

  @override
  Widget build(BuildContext context) {
    return Stack(
      alignment: Alignment.topCenter,
      children: <Widget>[
        Column(
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
              bridge: bridge,
              onSendMessage: onSendMessage,
              onCancelMessage: onCancelMessage,
              onModelChanged: onModelChanged,
            ),
          ],
        ),
        SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
            child: ChatToastHost(
              message: toastMessage,
              onDismiss: onDismissToast,
              maxHeight: 280,
            ),
          ),
        ),
      ],
    );
  }
}
