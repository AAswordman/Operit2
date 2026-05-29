// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../viewmodel/ChatViewModel.dart';
import 'AgentChatInputSection.dart';
import 'ChatArea.dart';
import 'ChatScrollNavigator.dart';
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
    required this.viewModel,
    required this.currentChatId,
    required this.autoScrollToBottom,
    required this.hasOlderDisplayHistory,
    required this.hasNewerDisplayHistory,
    required this.isLoadingDisplayWindow,
    required this.loadLocatorEntries,
    required this.onAutoScrollToBottomChanged,
    required this.onLoadOlderDisplayWindow,
    required this.onLoadNewerDisplayWindow,
    required this.onShowLatestDisplayWindow,
    required this.onToggleFavoriteMessage,
    required this.onSendMessage,
    required this.onCancelMessage,
    required this.onModelChanged,
    required this.toastMessage,
    required this.onDismissToast,
  });

  final List<ChatUiMessage> messages;
  final bool loading;
  final String? errorMessage;
  final TextEditingController messageController;
  final FocusNode inputFocusNode;
  final ScrollController scrollController;
  final ChatInputProcessingState inputProcessingState;
  final String modelLabel;
  final ChatViewModel viewModel;
  final String? currentChatId;
  final bool autoScrollToBottom;
  final bool hasOlderDisplayHistory;
  final bool hasNewerDisplayHistory;
  final bool isLoadingDisplayWindow;
  final LoadMessageLocatorEntries loadLocatorEntries;
  final ValueChanged<bool> onAutoScrollToBottomChanged;
  final Future<void> Function() onLoadOlderDisplayWindow;
  final Future<void> Function() onLoadNewerDisplayWindow;
  final Future<void> Function() onShowLatestDisplayWindow;
  final ToggleFavoriteMessage onToggleFavoriteMessage;
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
                currentChatId: currentChatId,
                autoScrollToBottom: autoScrollToBottom,
                hasOlderDisplayHistory: hasOlderDisplayHistory,
                hasNewerDisplayHistory: hasNewerDisplayHistory,
                isLoadingDisplayWindow: isLoadingDisplayWindow,
                loadLocatorEntries: loadLocatorEntries,
                onAutoScrollToBottomChanged: onAutoScrollToBottomChanged,
                onLoadOlderDisplayWindow: onLoadOlderDisplayWindow,
                onLoadNewerDisplayWindow: onLoadNewerDisplayWindow,
                onShowLatestDisplayWindow: onShowLatestDisplayWindow,
                onToggleFavoriteMessage: onToggleFavoriteMessage,
              ),
            ),
            AgentChatInputSection(
              controller: messageController,
              focusNode: inputFocusNode,
              isLoading: loading,
              inputState: inputProcessingState,
              modelLabel: modelLabel,
              viewModel: viewModel,
              currentChatId: currentChatId,
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
