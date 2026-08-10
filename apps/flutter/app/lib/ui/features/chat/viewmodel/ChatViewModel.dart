// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'WorkspaceFileModels.dart';

typedef ChatMessageLocatorPreview = core_proxy.ChatMessageLocatorPreview;
typedef WorkspaceFileChange = core_proxy.WorkspaceFileChange;
typedef ChatResponseStreamEvent = core_proxy.MarkdownStreamEvent;
typedef AttachmentInfo = core_proxy.AttachmentInfo;

const String _pastedTextAttachmentPrefix = 'pasted_text:';

class ChatInputSubmitDecision {
  const ChatInputSubmitDecision({
    required this.action,
    required this.text,
    required this.message,
    required this.clearInput,
    required this.timedOut,
  });

  /// Parses the JSON decision returned by the Core ToolPkg chat input bridge.
  factory ChatInputSubmitDecision.fromJson(Map<String, Object?> json) {
    return ChatInputSubmitDecision(
      action: json['action'] as String,
      text: json['text'] == null ? null : json['text'] as String,
      message: json['message'] == null ? null : json['message'] as String,
      clearInput: json['clearInput'] == null
          ? false
          : json['clearInput'] as bool,
      timedOut: json['timedOut'] as bool,
    );
  }

  final String action;
  final String? text;
  final String? message;
  final bool clearInput;
  final bool timedOut;
}

sealed class ChatRuntimeSurface {
  const ChatRuntimeSurface();

  static const ChatRuntimeSurface main = MainChatRuntimeSurface();
  static const ChatRuntimeSurface floating = FloatingChatRuntimeSurface();
}

class MainChatRuntimeSurface extends ChatRuntimeSurface {
  const MainChatRuntimeSurface();
}

class FloatingChatRuntimeSurface extends ChatRuntimeSurface {
  const FloatingChatRuntimeSurface();
}

class DetachedChatRuntimeSurface extends ChatRuntimeSurface {
  const DetachedChatRuntimeSurface(this.slotId);

  final String slotId;
}

class ChatViewModel {
  ChatViewModel({
    this.bridge = const ProxyCoreRuntimeBridge(),
    this.runtimeSurface = ChatRuntimeSurface.main,
  }) : clients = GeneratedCoreProxyClients(bridge),
       _chat = _chatProxyFor(bridge, runtimeSurface);

  final OperitRuntimeBridge bridge;
  final ChatRuntimeSurface runtimeSurface;
  final GeneratedCoreProxyClients clients;
  final GeneratedChatRuntimeHolderMainCoreProxy _chat;

  static GeneratedChatRuntimeHolderMainCoreProxy _chatProxyFor(
    OperitRuntimeBridge bridge,
    ChatRuntimeSurface runtimeSurface,
  ) {
    final clients = GeneratedCoreProxyClients(bridge);
    return switch (runtimeSurface) {
      MainChatRuntimeSurface() => clients.chatRuntimeHolderMain,
      FloatingChatRuntimeSurface() => clients.chatRuntimeHolderFloating,
      DetachedChatRuntimeSurface(:final slotId) =>
        GeneratedChatRuntimeHolderMainCoreProxy(
          bridge,
          CoreObjectPath.parse('chatRuntimeHolder.detached.$slotId'),
        ),
    };
  }

  /// Watches structured chat-state snapshots for this runtime surface.
  Stream<ChatViewModelSnapshot> watchMainState() {
    final controller = StreamController<ChatViewModelSnapshot>();
    StreamSubscription<core_proxy.ChatMainState>? subscription;
    final boundMessageStreams =
        <int, _ReplayTextStream<ChatResponseStreamEvent>>{};
    final boundResponseSubscriptions =
        <int, StreamSubscription<ChatResponseStreamEvent>>{};

    controller.onListen = () {
      subscription = _chat.chatMainStateFlowChanges().listen(
        (state) {
          final snapshot = _snapshotFromMainState(
            state,
            boundMessageStreams: boundMessageStreams,
            boundResponseSubscriptions: boundResponseSubscriptions,
          );
          if (!controller.isClosed) {
            controller.add(snapshot);
          }
        },
        onError: (Object error, StackTrace stackTrace) {
          if (!controller.isClosed) {
            controller.addError(error, stackTrace);
          }
        },
      );
    };
    controller.onCancel = () async {
      await subscription?.cancel();
      await _closeAllBoundResponseStreams(
        boundMessageStreams: boundMessageStreams,
        boundResponseSubscriptions: boundResponseSubscriptions,
      );
    };
    return controller.stream;
  }

  /// Converts core state and binds its active AI turn to the live response stream.
  ChatViewModelSnapshot _snapshotFromMainState(
    core_proxy.ChatMainState state, {
    required Map<int, _ReplayTextStream<ChatResponseStreamEvent>>
    boundMessageStreams,
    required Map<int, StreamSubscription<ChatResponseStreamEvent>>
    boundResponseSubscriptions,
  }) {
    return _bindActiveResponseStream(
      ChatViewModelSnapshot(
        currentChatId: state.currentChatId,
        currentChatTitle: state.currentChatTitle,
        currentModelName: state.currentModelName,
        currentCharacterCardName: state.currentCharacterCardName,
        currentCharacterCardAvatarUri: state.currentCharacterCardAvatarUri,
        currentWorkspacePath: state.currentWorkspacePath,
        activeCharacterCardName: state.activeCharacterCardName,
        isLoading: state.isLoading,
        inputProcessingState: ChatInputProcessingState.fromJson(
          state.inputProcessingState,
        ),
        messages: state.messages
            .map((item) => ChatUiMessage.fromJson(item as Map<String, Object?>))
            .toList(growable: false),
        hasOlderDisplayHistory: state.hasOlderDisplayHistory,
        hasNewerDisplayHistory: state.hasNewerDisplayHistory,
        isLoadingDisplayWindow: state.isLoadingDisplayWindow,
        pendingQueueMessages: state.pendingQueueMessages,
        isPendingQueueExpanded: state.isPendingQueueExpanded,
      ),
      boundMessageStreams: boundMessageStreams,
      boundResponseSubscriptions: boundResponseSubscriptions,
    );
  }

  Future<void> sendUserMessage(
    String text, {
    ChatUiMessage? replyToMessage,
    String? chatIdOverride,
  }) async {
    final attachments = await _chat.attachments();
    await _chat.sendUserMessage(
      promptFunctionType: core_proxy.PromptFunctionType.chat,
      roleCardIdOverride: null,
      chatIdOverride: chatIdOverride,
      messageText: text,
      proxySenderNameOverride: null,
      chatProviderIdOverride: null,
      chatModelIdOverride: null,
      attachments: attachments,
      replyToMessage: replyToMessage?.toProxy(),
      turnOptions: const core_proxy.ChatTurnOptions(
        persistTurn: true,
        notifyReply: null,
        hideUserMessage: false,
        disableWarning: false,
        chatInputSubmitRequestedHandled: true,
      ),
    );
    if (attachments.isNotEmpty) {
      await _chat.clearAttachments();
    }
  }

  Future<void> dispatchChatInputChanged({
    required String? chatId,
    required String text,
    required int selectionStart,
    required int selectionEnd,
    required int attachmentCount,
  }) {
    return _chat.dispatchChatInputChanged(
      chatIdOverride: chatId,
      messageText: text,
      selectionStart: selectionStart,
      selectionEnd: selectionEnd,
      attachmentCount: attachmentCount,
    );
  }

  Future<ChatInputSubmitDecision?> dispatchChatInputSubmitRequested({
    required String? chatId,
    required String text,
    required int selectionStart,
    required int selectionEnd,
    required int attachmentCount,
  }) async {
    final result = await _chat.dispatchChatInputSubmitRequested(
      chatIdOverride: chatId,
      messageText: text,
      selectionStart: selectionStart,
      selectionEnd: selectionEnd,
      attachmentCount: attachmentCount,
    );
    if (result == null) {
      return null;
    }
    return ChatInputSubmitDecision.fromJson(result as Map<String, Object?>);
  }

  Future<void> cancelCurrentMessage() {
    return _chat.cancelCurrentMessage();
  }

  /// Cancels generation for the specified chat without changing the active UI selection.
  Future<void> cancelMessage(String chatId) {
    return _chat.cancelMessage(chatId: chatId);
  }

  /// Adds a message to the runtime-owned queue for one chat.
  Future<void> enqueuePendingQueueMessage({
    required String chatId,
    required String messageText,
  }) {
    return _chat.enqueuePendingQueueMessage(
      chatId: chatId,
      messageText: messageText,
    );
  }

  /// Deletes a queued message from the runtime-owned queue for one chat.
  Future<void> deletePendingQueueMessage({
    required String chatId,
    required int messageId,
  }) {
    return _chat.deletePendingQueueMessage(
      chatId: chatId,
      messageId: messageId,
    );
  }

  /// Atomically takes a queued message for a local edit or explicit send action.
  Future<core_proxy.PendingQueueMessageItem?> takePendingQueueMessage({
    required String chatId,
    required int messageId,
    required bool suppressNextAutoDequeue,
  }) {
    return _chat.takePendingQueueMessage(
      chatId: chatId,
      messageId: messageId,
      suppressNextAutoDequeue: suppressNextAutoDequeue,
    );
  }

  /// Clears the one-shot automatic dequeue suppression for a manually claimed item.
  Future<void> clearPendingQueueAutoDequeueSuppression(String chatId) {
    return _chat.clearPendingQueueAutoDequeueSuppression(chatId: chatId);
  }

  /// Atomically takes the next queued message when its chat has become ready.
  Future<core_proxy.PendingQueueMessageItem?>
  takeNextPendingQueueMessageIfReady(String chatId) {
    return _chat.takeNextPendingQueueMessageIfReady(chatId: chatId);
  }

  /// Restores a queued message after its submit hook rejects delivery.
  Future<void> restorePendingQueueMessage({
    required String chatId,
    required core_proxy.PendingQueueMessageItem message,
  }) {
    return _chat.restorePendingQueueMessage(chatId: chatId, message: message);
  }

  /// Saves the expanded state for a chat's runtime-owned queue.
  Future<void> setPendingQueueExpanded({
    required String chatId,
    required bool isExpanded,
  }) {
    return _chat.setPendingQueueExpanded(
      chatId: chatId,
      isExpanded: isExpanded,
    );
  }

  Future<List<AttachmentInfo>> attachments() {
    return _chat.attachments();
  }

  Future<void> handleAttachment(String filePath) {
    return _chat.handleAttachment(filePath: filePath);
  }

  /// Adds pasted text through the runtime's virtual plain-text attachment path.
  Future<void> attachPastedText(String text) {
    return handleAttachment('$_pastedTextAttachmentPrefix$text');
  }

  Future<void> removeAttachment(String filePath) {
    return _chat.removeAttachment(filePath: filePath);
  }

  Future<void> clearAttachments() {
    return _chat.clearAttachments();
  }

  String createAttachmentReference(AttachmentInfo attachment) {
    final buffer = StringBuffer('<attachment ');
    buffer.write('id="${attachment.filePath}" ');
    buffer.write('filename="${attachment.fileName}" ');
    buffer.write('type="${attachment.mimeType}" ');
    if (attachment.fileSize > 0) {
      buffer.write('size="${attachment.fileSize}" ');
    }
    if (attachment.content.isNotEmpty) {
      buffer.write('content="${attachment.content}" ');
    }
    buffer.write('/>');
    return buffer.toString();
  }

  /// Watches the raw Markdown revision events for one active chat turn.
  Stream<ChatResponseStreamEvent> watchResponseStream(String chatId) {
    return _chat.getResponseStreamChanges(chatId: chatId).map((event) {
      return core_proxy.MarkdownStreamEvent.fromJson(
        event as Map<String, Object?>,
      );
    });
  }

  /// Splits Markdown with the runtime surface that owns this chat.
  Future<List<ChatResponseStreamEvent>> splitMarkdownContent(String content) {
    return _chat.splitMarkdownContent(content: content);
  }

  Stream<String?> watchToastEvent() {
    return _chat.toastEventFlowChanges();
  }

  Future<void> clearToastEvent() {
    return _chat.clearToastEvent();
  }

  Future<List<ChatMessageLocatorPreview>> loadChatMessageLocatorPreviews(
    String chatId,
    String query,
  ) {
    return _chat.loadChatMessageLocatorPreviews(chatId: chatId, query: query);
  }

  Future<void> setMessageFavorite(int timestamp, bool isFavorite) {
    return _chat.setMessageFavorite(
      timestamp: timestamp,
      isFavorite: isFavorite,
    );
  }

  Future<void> deleteMessage(int index) {
    return _chat.deleteMessage(index: index);
  }

  Future<bool> deleteMessages(Set<int> indices) {
    return _chat.deleteMessages(indices: indices.toList(growable: false));
  }

  Future<bool> updateMessage(int index, String editedContent) {
    return _chat.updateMessage(index: index, editedContent: editedContent);
  }

  Future<bool> deleteMessagesFrom(int index) {
    return _chat.deleteMessagesFrom(index: index);
  }

  Future<void> deleteMessageVariant(int timestamp, int variantIndex) {
    return _chat.deleteMessageVariant(
      timestamp: timestamp,
      variantIndex: variantIndex,
    );
  }

  Future<String?> rollbackToMessage(int index) {
    return _chat.rollbackToMessage(index: index);
  }

  Future<bool> rewindAndResendMessage(int index, String editedContent) {
    return _chat.rewindAndResendMessage(
      index: index,
      editedContent: editedContent,
    );
  }

  Future<List<WorkspaceFileChange>> previewWorkspaceChangesForMessage(
    int index,
  ) {
    return _chat.previewWorkspaceChangesForMessage(index: index);
  }

  Future<void> regenerateSingleAiMessage(int index) {
    return _chat.regenerateSingleAiMessage(index: index);
  }

  Future<void> createBranch(int timestamp) {
    return _chat.createBranch(upToMessageTimestamp: timestamp);
  }

  Future<bool> insertSummary(ChatUiMessage message) {
    return _chat.insertSummary(message: message.toProxy());
  }

  Future<void> loadOlderMessagesForCurrentChat() {
    return _chat.loadOlderMessagesForCurrentChat();
  }

  Future<void> loadNewerMessagesForCurrentChat() {
    return _chat.loadNewerMessagesForCurrentChat();
  }

  Future<void> showLatestMessagesForCurrentChat() {
    return _chat.showLatestMessagesForCurrentChat();
  }

  Future<String> currentModelName() async {
    return (await _chat.chatMainStateFlowSnapshot()).currentModelName;
  }

  Future<String> createAndBindDefaultWorkspace(
    String chatId,
    String? projectType,
  ) {
    return _chat.createAndBindDefaultWorkspace(
      chatId: chatId,
      projectType: projectType,
    );
  }

  Future<void> bindChatToWorkspace(String chatId, String workspace) {
    return _chat.bindChatToWorkspace(chatId: chatId, workspace: workspace);
  }

  Future<List<WorkspaceFileEntry>> listWorkspaceFiles(
    String relativePath,
  ) async {
    final chatId = await _requiredCurrentChatId();
    final entries = await clients.servicesWorkspaceService.listWorkspaceFiles(
      chatId: chatId,
      relativePath: relativePath,
    );
    return entries.map(WorkspaceFileEntry.fromProxy).toList(growable: false);
  }

  Future<List<WorkspaceFileEntry>> listWorkspaceBindingDirectories(
    String path,
  ) async {
    final entries = await clients.servicesWorkspaceService
        .listWorkspaceBindingDirectories(path: path);
    return entries.map(WorkspaceFileEntry.fromProxy).toList(growable: false);
  }

  Future<String> readWorkspaceTextFile(String relativePath) async {
    final chatId = await _requiredCurrentChatId();
    return clients.servicesWorkspaceService.readWorkspaceTextFile(
      chatId: chatId,
      relativePath: relativePath,
    );
  }

  Future<Uint8List> readWorkspaceFileBytes(String relativePath) async {
    final chatId = await _requiredCurrentChatId();
    final bytes = await clients.servicesWorkspaceService.readWorkspaceFileBytes(
      chatId: chatId,
      relativePath: relativePath,
    );
    return base64Decode(bytes.base64Content);
  }

  Future<void> writeWorkspaceFileBytes(
    String relativePath,
    Uint8List bytes,
  ) async {
    final chatId = await _requiredCurrentChatId();
    await clients.servicesWorkspaceService.writeWorkspaceFileBytes(
      chatId: chatId,
      relativePath: relativePath,
      base64Content: base64Encode(bytes),
    );
  }

  Future<void> openWorkspaceFile(String relativePath) async {
    final chatId = await _requiredCurrentChatId();
    await clients.servicesWorkspaceService.openWorkspaceFile(
      chatId: chatId,
      relativePath: relativePath,
    );
  }

  /// Attaches the active turn stream to its in-progress AI message.
  ChatViewModelSnapshot _bindActiveResponseStream(
    ChatViewModelSnapshot snapshot, {
    required Map<int, _ReplayTextStream<ChatResponseStreamEvent>>
    boundMessageStreams,
    required Map<int, StreamSubscription<ChatResponseStreamEvent>>
    boundResponseSubscriptions,
  }) {
    final activeTimestamp = _activeStreamingMessageTimestamp(snapshot);
    final currentChatId = snapshot.currentChatId;
    final activeKeys = activeTimestamp == null
        ? const <int>{}
        : <int>{activeTimestamp};

    _closeInactiveBoundResponseStreams(
      activeKeys,
      boundMessageStreams: boundMessageStreams,
      boundResponseSubscriptions: boundResponseSubscriptions,
    );

    if (activeTimestamp != null && currentChatId != null) {
      final stream = boundMessageStreams.putIfAbsent(activeTimestamp, () {
        return _ReplayTextStream<ChatResponseStreamEvent>(activeTimestamp);
      });
      boundResponseSubscriptions.putIfAbsent(activeTimestamp, () {
        return watchResponseStream(currentChatId).listen(
          (event) {
            stream.add(event);
            if (event.eventType == 'completed' && event.parentBlockId == null) {
              stream.close();
            }
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint('Failed to watch response stream: $error\n$stackTrace');
          },
          onDone: stream.close,
        );
      });
    }

    return snapshot.copyWith(
      messages: <ChatUiMessage>[
        for (final message in snapshot.messages)
          if (message.timestamp == activeTimestamp)
            message
                .copyWith(parts: const <core_proxy.MessagePart>[])
                .copyWithContentStream(boundMessageStreams[message.timestamp])
          else
            message,
      ],
    );
  }

  /// Finds the unfinished AI message owned by the active chat turn.
  int? _activeStreamingMessageTimestamp(ChatViewModelSnapshot snapshot) {
    if (!snapshot.isLoading || snapshot.currentChatId == null) {
      return null;
    }
    for (final message in snapshot.messages.reversed) {
      if (message.sender == 'ai' && message.completedAt <= 0) {
        return message.timestamp;
      }
    }
    return null;
  }

  /// Closes response bindings that no longer belong to an active message.
  void _closeInactiveBoundResponseStreams(
    Set<int> activeKeys, {
    required Map<int, _ReplayTextStream<ChatResponseStreamEvent>>
    boundMessageStreams,
    required Map<int, StreamSubscription<ChatResponseStreamEvent>>
    boundResponseSubscriptions,
  }) {
    final staleKeys = boundMessageStreams.keys
        .where((timestamp) => !activeKeys.contains(timestamp))
        .toList(growable: false);
    for (final timestamp in staleKeys) {
      boundResponseSubscriptions.remove(timestamp)?.cancel();
      boundMessageStreams.remove(timestamp)?.close();
    }
  }

  /// Closes every response binding owned by this main-state watcher.
  Future<void> _closeAllBoundResponseStreams({
    required Map<int, _ReplayTextStream<ChatResponseStreamEvent>>
    boundMessageStreams,
    required Map<int, StreamSubscription<ChatResponseStreamEvent>>
    boundResponseSubscriptions,
  }) async {
    final subscriptions = boundResponseSubscriptions.values.toList(
      growable: false,
    );
    boundResponseSubscriptions.clear();
    for (final subscription in subscriptions) {
      await subscription.cancel();
    }
    final streams = boundMessageStreams.values.toList(growable: false);
    boundMessageStreams.clear();
    for (final stream in streams) {
      await stream.close();
    }
  }

  /// Returns the selected chat id required by workspace operations.
  Future<String> _requiredCurrentChatId() async {
    final chatId = await _chat.currentChatIdFlowSnapshot();
    if (chatId == null || chatId.isEmpty) {
      throw StateError('当前没有对话');
    }
    return chatId;
  }
}

class ChatViewModelSnapshot {
  const ChatViewModelSnapshot({
    required this.currentChatId,
    required this.currentChatTitle,
    required this.currentModelName,
    required this.currentCharacterCardName,
    required this.currentCharacterCardAvatarUri,
    required this.currentWorkspacePath,
    required this.activeCharacterCardName,
    required this.isLoading,
    required this.inputProcessingState,
    required this.messages,
    required this.hasOlderDisplayHistory,
    required this.hasNewerDisplayHistory,
    required this.isLoadingDisplayWindow,
    required this.pendingQueueMessages,
    required this.isPendingQueueExpanded,
  });

  final String? currentChatId;
  final String currentChatTitle;
  final String currentModelName;
  final String? currentCharacterCardName;
  final String? currentCharacterCardAvatarUri;
  final String? currentWorkspacePath;
  final String? activeCharacterCardName;
  final bool isLoading;
  final ChatInputProcessingState inputProcessingState;
  final List<ChatUiMessage> messages;
  final bool hasOlderDisplayHistory;
  final bool hasNewerDisplayHistory;
  final bool isLoadingDisplayWindow;
  final List<core_proxy.PendingQueueMessageItem> pendingQueueMessages;
  final bool isPendingQueueExpanded;

  ChatViewModelSnapshot copyWith({List<ChatUiMessage>? messages}) {
    return ChatViewModelSnapshot(
      currentChatId: currentChatId,
      currentChatTitle: currentChatTitle,
      currentModelName: currentModelName,
      currentCharacterCardName: currentCharacterCardName,
      currentCharacterCardAvatarUri: currentCharacterCardAvatarUri,
      currentWorkspacePath: currentWorkspacePath,
      activeCharacterCardName: activeCharacterCardName,
      isLoading: isLoading,
      inputProcessingState: inputProcessingState,
      messages: messages ?? this.messages,
      hasOlderDisplayHistory: hasOlderDisplayHistory,
      hasNewerDisplayHistory: hasNewerDisplayHistory,
      isLoadingDisplayWindow: isLoadingDisplayWindow,
      pendingQueueMessages: pendingQueueMessages,
      isPendingQueueExpanded: isPendingQueueExpanded,
    );
  }
}

class ChatUiMessage {
  const ChatUiMessage({
    required this.sender,
    required this.parts,
    required this.timestamp,
    required this.roleName,
    required this.selectedVariantIndex,
    required this.variantCount,
    required this.provider,
    required this.modelName,
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedInputTokens,
    required this.sentAt,
    required this.outputDurationMs,
    required this.waitDurationMs,
    required this.displayMode,
    required this.isFavorite,
    required this.isVariantPreview,
    required this.completedAt,
    this.contentStream,
  });

  factory ChatUiMessage.fromProxy(core_proxy.ChatMessage message) {
    return ChatUiMessage(
      sender: message.sender,
      parts: message.parts,
      timestamp: message.timestamp,
      roleName: message.roleName,
      selectedVariantIndex: message.selectedVariantIndex,
      variantCount: message.variantCount,
      provider: message.provider,
      modelName: message.modelName,
      inputTokens: message.inputTokens,
      outputTokens: message.outputTokens,
      cachedInputTokens: message.cachedInputTokens,
      sentAt: message.sentAt,
      outputDurationMs: message.outputDurationMs,
      waitDurationMs: message.waitDurationMs,
      displayMode: message.displayMode.value,
      isFavorite: message.isFavorite,
      isVariantPreview: message.isVariantPreview,
      completedAt: message.completedAt,
    );
  }

  factory ChatUiMessage.fromJson(Map<String, Object?> json) {
    return ChatUiMessage(
      sender: json['sender'] as String,
      parts: (json['parts'] as List<Object?>)
          .map(
            (item) => core_proxy.MessagePart.fromJson(
              Map<String, Object?>.from(item! as Map<Object?, Object?>),
            ),
          )
          .toList(growable: false),
      timestamp: json['timestamp'] as int,
      roleName: json['roleName'] as String,
      selectedVariantIndex: json['selectedVariantIndex'] as int,
      variantCount: json['variantCount'] as int,
      provider: json['provider'] as String,
      modelName: json['modelName'] as String,
      inputTokens: json['inputTokens'] as int,
      outputTokens: json['outputTokens'] as int,
      cachedInputTokens: json['cachedInputTokens'] as int,
      sentAt: json['sentAt'] as int,
      outputDurationMs: json['outputDurationMs'] as int,
      waitDurationMs: json['waitDurationMs'] as int,
      displayMode: json['displayMode'] as String,
      isFavorite: json['isFavorite'] as bool,
      isVariantPreview: json['isVariantPreview'] as bool? ?? false,
      completedAt: json['completedAt'] as int,
    );
  }

  /// Creates a copy with changed editable message properties.
  ChatUiMessage copyWith({
    List<core_proxy.MessagePart>? parts,
    bool? isFavorite,
  }) {
    return ChatUiMessage(
      sender: sender,
      parts: parts ?? this.parts,
      timestamp: timestamp,
      roleName: roleName,
      selectedVariantIndex: selectedVariantIndex,
      variantCount: variantCount,
      provider: provider,
      modelName: modelName,
      inputTokens: inputTokens,
      outputTokens: outputTokens,
      cachedInputTokens: cachedInputTokens,
      sentAt: sentAt,
      outputDurationMs: outputDurationMs,
      waitDurationMs: waitDurationMs,
      displayMode: displayMode,
      isFavorite: isFavorite ?? this.isFavorite,
      isVariantPreview: isVariantPreview,
      completedAt: completedAt,
      contentStream: contentStream,
    );
  }

  /// Creates a copy with a live response stream attached to this UI message.
  ChatUiMessage copyWithContentStream(Stream<ChatResponseStreamEvent>? value) {
    return ChatUiMessage(
      sender: sender,
      parts: parts,
      timestamp: timestamp,
      roleName: roleName,
      selectedVariantIndex: selectedVariantIndex,
      variantCount: variantCount,
      provider: provider,
      modelName: modelName,
      inputTokens: inputTokens,
      outputTokens: outputTokens,
      cachedInputTokens: cachedInputTokens,
      sentAt: sentAt,
      outputDurationMs: outputDurationMs,
      waitDurationMs: waitDurationMs,
      displayMode: displayMode,
      isFavorite: isFavorite,
      isVariantPreview: isVariantPreview,
      completedAt: completedAt,
      contentStream: value,
    );
  }

  core_proxy.ChatMessage toProxy() {
    return core_proxy.ChatMessage(
      sender: sender,
      parts: parts,
      timestamp: timestamp,
      roleName: roleName,
      selectedVariantIndex: selectedVariantIndex,
      variantCount: variantCount,
      provider: provider,
      modelName: modelName,
      inputTokens: inputTokens,
      outputTokens: outputTokens,
      cachedInputTokens: cachedInputTokens,
      sentAt: sentAt,
      outputDurationMs: outputDurationMs,
      waitDurationMs: waitDurationMs,
      completedAt: completedAt,
      displayMode: core_proxy.ChatMessageDisplayMode.fromJson(displayMode),
      isFavorite: isFavorite,
      isVariantPreview: isVariantPreview,
    );
  }

  final String sender;
  final List<core_proxy.MessagePart> parts;

  /// Returns text from parts rendered directly in the chat transcript.
  String get displayText {
    final orderedParts = parts.toList(growable: false)
      ..sort((left, right) => left.sequence.compareTo(right.sequence));
    return orderedParts
        .where(
          (part) =>
              part.kind == core_proxy.MessagePartKind.markdown ||
              part.kind == core_proxy.MessagePartKind.status,
        )
        .map((part) => part.content)
        .join();
  }

  /// Returns the current Markdown source visible in the transcript for copying.
  String get copySourceText {
    final stream = contentStream;
    if (stream is _ReplayTextStream<ChatResponseStreamEvent>) {
      return stream.cachedEvents
          .whereType<ChatResponseStreamEvent>()
          .where(
            (event) =>
                event.eventType == 'chunk' && event.parentBlockId == null,
          )
          .map((event) => event.value ?? '')
          .join();
    }
    return displayText;
  }

  /// Reconstructs the complete assistant protocol markup from semantic parts.
  String get assistantProtocolMarkup {
    final orderedParts = parts.toList(growable: false)
      ..sort((left, right) => left.sequence.compareTo(right.sequence));
    final markup = StringBuffer();
    for (final part in orderedParts) {
      switch (part.kind) {
        case core_proxy.MessagePartKind.markdown:
          markup.write(part.content);
        case core_proxy.MessagePartKind.thinking:
          markup
            ..write('<think>')
            ..write(part.content)
            ..write('</think>');
        case core_proxy.MessagePartKind.toolCall:
          markup
            ..write('<tool name="')
            ..write(_escapeProtocolAttribute(part.toolName!))
            ..write('" call_id="')
            ..write(_escapeProtocolAttribute(part.toolCallId!))
            ..write('">');
          final parameterNames = part.attributes.keys.toList(growable: false)
            ..sort();
          for (final name in parameterNames) {
            markup
              ..write('<param name="')
              ..write(_escapeProtocolAttribute(name))
              ..write('">')
              ..write(part.attributes[name]!)
              ..write('</param>');
          }
          markup.write('</tool>');
        case core_proxy.MessagePartKind.toolResult:
          markup
            ..write('<tool_result name="')
            ..write(_escapeProtocolAttribute(part.toolName!))
            ..write('"');
          final toolCallId = part.toolCallId;
          if (toolCallId != null) {
            markup
              ..write(' call_id="')
              ..write(_escapeProtocolAttribute(toolCallId))
              ..write('"');
          }
          _writeProtocolAttributes(markup, part.attributes);
          markup
            ..write('><content>')
            ..write(part.content)
            ..write('</content></tool_result>');
        case core_proxy.MessagePartKind.status:
          markup.write('<status');
          _writeProtocolAttributes(markup, part.attributes);
          markup
            ..write('>')
            ..write(part.content)
            ..write('</status>');
      }
    }
    return markup.toString();
  }

  /// Returns the complete text representation accepted by message editing.
  String get editableText {
    return switch (sender) {
      'ai' => assistantProtocolMarkup,
      'user' => displayText,
      _ => throw StateError('Message sender cannot be edited: $sender'),
    };
  }

  /// Appends sorted and escaped XML-like protocol attributes.
  static void _writeProtocolAttributes(
    StringBuffer markup,
    Map<String, String> attributes,
  ) {
    final names = attributes.keys.toList(growable: false)..sort();
    for (final name in names) {
      markup
        ..write(' ')
        ..write(name)
        ..write('="')
        ..write(_escapeProtocolAttribute(attributes[name]!))
        ..write('"');
    }
  }

  /// Escapes one XML-like protocol attribute value.
  static String _escapeProtocolAttribute(String value) {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('"', '&quot;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;');
  }

  final int timestamp;
  final String roleName;
  final int selectedVariantIndex;
  final int variantCount;
  final String provider;
  final String modelName;
  final int inputTokens;
  final int outputTokens;
  final int cachedInputTokens;
  final int sentAt;
  final int outputDurationMs;
  final int waitDurationMs;
  final String displayMode;
  final bool isFavorite;
  final bool isVariantPreview;
  final int completedAt;
  final Stream<ChatResponseStreamEvent>? contentStream;

  String get stableKey => '$sender-$timestamp';
}

class ChatInputProcessingState {
  const ChatInputProcessingState({
    required this.kind,
    required this.message,
    required this.progress,
    required this.toolName,
  });

  factory ChatInputProcessingState.fromJson(Object? json) {
    if (json is String) {
      return ChatInputProcessingState(
        kind: json,
        message: '',
        progress: 0,
        toolName: '',
      );
    }
    final tagged = json as Map<String, Object?>;
    final kind = tagged.keys.single;
    final payload = tagged[kind] as Map<String, Object?>;
    switch (kind) {
      case 'Processing':
      case 'Connecting':
      case 'Receiving':
      case 'Summarizing':
      case 'ExecutingPlan':
      case 'Error':
        return ChatInputProcessingState(
          kind: kind,
          message: payload['message'] as String,
          progress: 0,
          toolName: '',
        );
      case 'ExecutingTool':
      case 'ProcessingToolResult':
        return ChatInputProcessingState(
          kind: kind,
          message: '',
          progress: 0,
          toolName: payload['toolName'] as String,
        );
      case 'ToolProgress':
        return ChatInputProcessingState(
          kind: kind,
          message: payload['message'] as String,
          progress: (payload['progress'] as num).toDouble(),
          toolName: payload['toolName'] as String,
        );
    }
    throw ArgumentError.value(kind, 'kind', 'unknown input processing state');
  }

  final String kind;
  final String message;
  final double progress;
  final String toolName;

  bool get isProcessing {
    return kind != 'Idle' && kind != 'Completed' && kind != 'Error';
  }

  bool get isError {
    return kind == 'Error';
  }

  String get displayMessage {
    if (message.isNotEmpty) {
      return message;
    }
    if (kind == 'ExecutingTool') {
      return 'Executing tool $toolName';
    }
    if (kind == 'ProcessingToolResult') {
      return 'Processing tool result $toolName';
    }
    return '';
  }
}

/// Replays all received response events to each renderer subscription.
class _ReplayTextStream<T> extends Stream<T> {
  _ReplayTextStream(this.timestamp);

  final int timestamp;
  final List<T> _cache = <T>[];
  final StreamController<T> _liveController = StreamController<T>.broadcast();
  bool _closed = false;

  /// Exposes the replayed event snapshot without subscribing to live output.
  List<T> get cachedEvents => List<T>.unmodifiable(_cache);

  /// Appends one event to the replay cache and live subscribers.
  void add(T chunk) {
    if (_closed) {
      return;
    }
    _cache.add(chunk);
    _liveController.add(chunk);
  }

  /// Closes the live event channel once generation has ended.
  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;
    await _liveController.close();
  }

  /// Creates a subscription that receives cached events before live events.
  @override
  StreamSubscription<T> listen(
    void Function(T event)? onData, {
    Function? onError,
    void Function()? onDone,
    bool? cancelOnError,
  }) {
    final replayController = StreamController<T>(sync: true);
    StreamSubscription<T>? liveSubscription;

    replayController.onListen = () {
      for (final chunk in _cache) {
        replayController.add(chunk);
      }
      if (_closed) {
        replayController.close();
        return;
      }
      liveSubscription = _liveController.stream.listen(
        replayController.add,
        onError: replayController.addError,
        onDone: replayController.close,
      );
    };
    replayController.onPause = () {
      liveSubscription?.pause();
    };
    replayController.onResume = () {
      liveSubscription?.resume();
    };
    replayController.onCancel = () {
      return liveSubscription?.cancel();
    };

    return replayController.stream.listen(
      onData,
      onError: onError,
      onDone: onDone,
      cancelOnError: cancelOnError,
    );
  }
}
