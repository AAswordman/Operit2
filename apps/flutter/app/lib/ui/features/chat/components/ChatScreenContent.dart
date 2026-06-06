// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../common/components/M3LoadingIndicator.dart';
import '../viewmodel/ChatViewModel.dart';
import 'AgentChatInputSection.dart';
import 'ChatArea.dart';
import 'ChatMultiSelectBar.dart';
import 'ChatScrollNavigator.dart';
import 'ChatToastHost.dart';
import 'MessageContextMenu.dart';
import 'share/ChatShareImageGenerator.dart';
import 'share/ChatShareImagePreviewDialog.dart';

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
    required this.modelLabelListenable,
    required this.viewModel,
    required this.currentChatId,
    required this.currentCharacterCardAvatarUri,
    required this.autoScrollToBottomListenable,
    required this.hasOlderDisplayHistory,
    required this.hasNewerDisplayHistory,
    required this.isLoadingDisplayWindow,
    required this.loadLocatorEntries,
    required this.onAutoScrollToBottomChanged,
    required this.onLoadOlderDisplayWindow,
    required this.onLoadNewerDisplayWindow,
    required this.onShowLatestDisplayWindow,
    required this.onToggleFavoriteMessage,
    required this.onDeleteMessage,
    required this.onDeleteMessagesFrom,
    required this.onDeleteMessageVariant,
    required this.onRollbackToMessage,
    required this.onSelectMessageToEdit,
    required this.onRegenerateMessage,
    required this.onInsertSummary,
    required this.onCreateBranch,
    required this.onReplyToMessage,
    required this.onToggleMultiSelectMode,
    required this.onToggleMessageSelection,
    required this.onExitMultiSelectMode,
    required this.onSelectAllMessages,
    required this.onClearMessageSelection,
    required this.onDeleteSelectedMessages,
    required this.onRefreshRequested,
    required this.onSendMessage,
    required this.onCancelMessage,
    required this.onModelChanged,
    required this.toastMessageListenable,
    required this.onDismissToast,
    required this.isMultiSelectMode,
    required this.isPreparingChatSwitch,
    this.selectedMessageIndices = const <int>{},
  });

  final List<ChatUiMessage> messages;
  final bool loading;
  final String? errorMessage;
  final TextEditingController messageController;
  final FocusNode inputFocusNode;
  final ScrollController scrollController;
  final ChatInputProcessingState inputProcessingState;
  final ValueListenable<String> modelLabelListenable;
  final ChatViewModel viewModel;
  final String? currentChatId;
  final String? currentCharacterCardAvatarUri;
  final ValueListenable<bool> autoScrollToBottomListenable;
  final bool hasOlderDisplayHistory;
  final bool hasNewerDisplayHistory;
  final bool isLoadingDisplayWindow;
  final LoadMessageLocatorEntries loadLocatorEntries;
  final ValueChanged<bool> onAutoScrollToBottomChanged;
  final Future<void> Function() onLoadOlderDisplayWindow;
  final Future<void> Function() onLoadNewerDisplayWindow;
  final Future<void> Function() onShowLatestDisplayWindow;
  final ToggleFavoriteMessage onToggleFavoriteMessage;
  final MessageIndexAction onDeleteMessage;
  final MessageIndexBoolAction onDeleteMessagesFrom;
  final MessageVariantAction onDeleteMessageVariant;
  final ValueChanged<int> onRollbackToMessage;
  final MessageSelectionAction onSelectMessageToEdit;
  final MessageIndexAction onRegenerateMessage;
  final ValueChanged<ChatUiMessage> onInsertSummary;
  final MessageTimestampAction onCreateBranch;
  final ValueChanged<ChatUiMessage> onReplyToMessage;
  final ValueChanged<int> onToggleMultiSelectMode;
  final ValueChanged<int> onToggleMessageSelection;
  final VoidCallback onExitMultiSelectMode;
  final VoidCallback onSelectAllMessages;
  final VoidCallback onClearMessageSelection;
  final Future<void> Function() onDeleteSelectedMessages;
  final Future<void> Function() onRefreshRequested;
  final VoidCallback onSendMessage;
  final VoidCallback onCancelMessage;
  final ValueChanged<String> onModelChanged;
  final ValueListenable<String?> toastMessageListenable;
  final VoidCallback onDismissToast;
  final bool isMultiSelectMode;
  final bool isPreparingChatSwitch;
  final Set<int> selectedMessageIndices;

  @override
  Widget build(BuildContext context) {
    return Stack(
      alignment: Alignment.topCenter,
      children: <Widget>[
        Column(
          children: <Widget>[
            Expanded(
              child: Stack(
                fit: StackFit.expand,
                children: <Widget>[
                  IgnorePointer(
                    ignoring: isPreparingChatSwitch,
                    child: Opacity(
                      opacity: isPreparingChatSwitch ? 0 : 1,
                      child: _buildChatArea(),
                    ),
                  ),
                  if (isPreparingChatSwitch) const M3LoadingPane(size: 42),
                ],
              ),
            ),
            if (!isPreparingChatSwitch) ...<Widget>[
              if (isMultiSelectMode)
                ChatMultiSelectBar(
                  selectedCount: selectedMessageIndices.length,
                  allSelected:
                      _selectableMessageIndices.isNotEmpty &&
                      _selectableMessageIndices.length ==
                          selectedMessageIndices.length,
                  onClose: onExitMultiSelectMode,
                  onToggleSelectAll:
                      _selectableMessageIndices.isNotEmpty &&
                          _selectableMessageIndices.length ==
                              selectedMessageIndices.length
                      ? onClearMessageSelection
                      : onSelectAllMessages,
                  onCopy: selectedMessageIndices.isEmpty
                      ? null
                      : () => _copySelectedMessages(context),
                  onShareImage: selectedMessageIndices.isEmpty
                      ? null
                      : () => _generateShareImage(context),
                  onDelete: selectedMessageIndices.isEmpty
                      ? null
                      : () => _confirmDeleteSelected(context),
                )
              else
                ValueListenableBuilder<String>(
                  valueListenable: modelLabelListenable,
                  builder: (context, modelLabel, _) {
                    return AgentChatInputSection(
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
                    );
                  },
                ),
            ],
          ],
        ),
        SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
            child: ValueListenableBuilder<String?>(
              valueListenable: toastMessageListenable,
              builder: (context, toastMessage, _) {
                return ChatToastHost(
                  message: toastMessage,
                  onDismiss: onDismissToast,
                  maxHeight: 280,
                );
              },
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildChatArea() {
    return ChatArea(
      messages: messages,
      isLoading: loading,
      errorMessage: errorMessage,
      scrollController: scrollController,
      currentChatId: currentChatId,
      currentCharacterCardAvatarUri: currentCharacterCardAvatarUri,
      autoScrollToBottomListenable: autoScrollToBottomListenable,
      hasOlderDisplayHistory: hasOlderDisplayHistory,
      hasNewerDisplayHistory: hasNewerDisplayHistory,
      isLoadingDisplayWindow: isLoadingDisplayWindow,
      loadLocatorEntries: loadLocatorEntries,
      onAutoScrollToBottomChanged: onAutoScrollToBottomChanged,
      onLoadOlderDisplayWindow: onLoadOlderDisplayWindow,
      onLoadNewerDisplayWindow: onLoadNewerDisplayWindow,
      onShowLatestDisplayWindow: onShowLatestDisplayWindow,
      onToggleFavoriteMessage: onToggleFavoriteMessage,
      onDeleteMessage: onDeleteMessage,
      onDeleteMessagesFrom: onDeleteMessagesFrom,
      onDeleteMessageVariant: onDeleteMessageVariant,
      onRollbackToMessage: onRollbackToMessage,
      onSelectMessageToEdit: onSelectMessageToEdit,
      onRegenerateMessage: onRegenerateMessage,
      onInsertSummary: onInsertSummary,
      onCreateBranch: onCreateBranch,
      onReplyToMessage: onReplyToMessage,
      onToggleMultiSelectMode: onToggleMultiSelectMode,
      onToggleMessageSelection: onToggleMessageSelection,
      onRefreshRequested: onRefreshRequested,
      isMultiSelectMode: isMultiSelectMode,
      selectedMessageIndices: selectedMessageIndices,
    );
  }

  List<int> get _selectableMessageIndices {
    return List<int>.generate(messages.length, (index) => index)
        .where((index) {
          final sender = messages[index].sender;
          return sender == 'user' || sender == 'ai';
        })
        .toList(growable: false);
  }

  Future<void> _copySelectedMessages(BuildContext context) async {
    final selectedMessages = selectedMessageIndices.toList()..sort();
    final text = selectedMessages
        .map((index) => messages[index])
        .map((message) => cleanMessageContent(message.content))
        .join('\n\n');
    await Clipboard.setData(ClipboardData(text: text));
  }

  Future<void> _confirmDeleteSelected(BuildContext context) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) {
        return AlertDialog(
          title: const Text('确认删除'),
          content: Text('确定删除已选的 ${selectedMessageIndices.length} 条消息？'),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('删除'),
            ),
          ],
        );
      },
    );
    if (confirmed == true) {
      await onDeleteSelectedMessages();
    }
  }

  Future<void> _generateShareImage(BuildContext context) async {
    showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (context) {
        return const AlertDialog(
          content: Row(
            children: <Widget>[
              SizedBox(
                width: 22,
                height: 22,
                child: CircularProgressIndicator(strokeWidth: 2.5),
              ),
              SizedBox(width: 14),
              Text('正在生成长图...'),
            ],
          ),
        );
      },
    );

    try {
      final selectedMessages = selectedMessageIndices.toList()..sort();
      final file = await ChatShareImageGenerator.generate(
        context: context,
        messages: selectedMessages.map((index) => messages[index]).toList(),
      );
      if (!context.mounted) {
        return;
      }
      Navigator.of(context).pop();
      showDialog<void>(
        context: context,
        builder: (context) {
          return ChatShareImagePreviewDialog(
            imageFile: file,
            onDismiss: () => Navigator.of(context).pop(),
          );
        },
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to generate share image: $error\n$stackTrace');
      if (!context.mounted) {
        return;
      }
      Navigator.of(context).pop();
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('生成长图失败：$error')));
    }
  }
}
