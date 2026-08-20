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

  /// Watches messages for the chat selected by this runtime surface.
  Stream<List<ChatUiMessage>> watchMessages() {
    return _chat.chatMessagesFlow(chatId: null);
  }

  /// Watches non-message state for the chat selected by this runtime surface.
  Stream<core_proxy.ChatState> watchChatState() {
    return _chat.chatStateFlow(chatId: null);
  }

  Future<void> sendUserMessage(
    String text, {
    ChatUiMessage? replyToMessage,
    String? chatIdOverride,
  }) async {
    debugPrint(
      'Chat send requested chatId=${chatIdOverride ?? 'current'} '
      'chars=${text.length}',
    );
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
      replyToMessage: replyToMessage,
      turnOptions: const core_proxy.ChatTurnOptions(
        persistTurn: true,
        notifyReply: null,
        hideUserMessage: false,
        disableWarning: false,
        chatInputSubmitRequestedHandled: true,
      ),
    );
    debugPrint('Chat send accepted chatId=${chatIdOverride ?? 'current'}');
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

  /// Splits Markdown with the runtime surface that owns this chat.
  Future<List<ChatResponseStreamEvent>> splitMarkdownContent(String content) {
    return _chat.splitMarkdownContent(content: content);
  }

  Stream<String?> watchToastEvent() {
    return _chat.toastEventFlow();
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
    return _chat.insertSummary(message: message);
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
    return entries;
  }

  Future<List<WorkspaceFileEntry>> listWorkspaceBindingDirectories(
    String path,
  ) async {
    final entries = await clients.servicesWorkspaceService
        .listWorkspaceBindingDirectories(path: path);
    return entries;
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

  /// Returns the selected chat id required by workspace operations.
  Future<String> _requiredCurrentChatId() async {
    final chatId = await _chat.currentChatIdFlow().first;
    if (chatId == null || chatId.isEmpty) {
      throw StateError('当前没有对话');
    }
    return chatId;
  }
}

/// Uses the generated Core message state directly throughout chat UI widgets.
typedef ChatUiMessage = core_proxy.ChatMessage;

/// Exposes chat presentation helpers directly on the generated Core message state.
extension ChatMessagePresentation on core_proxy.ChatMessage {
  /// Returns the legacy variant-preview flag for a normal Core message.
  bool get isVariantPreview => false;

  /// Returns text from Core-owned parts rendered directly in the transcript.
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

  /// Returns the current Markdown source visible in the transcript.
  String get copySourceText => displayText;

  /// Reconstructs the complete assistant protocol markup from Core parts.
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

  /// Returns a stable UI key without copying the Core message.
  String get stableKey => '$sender-$timestamp';
}

/// Appends sorted and escaped XML-like protocol attributes.
void _writeProtocolAttributes(
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
String _escapeProtocolAttribute(String value) {
  return value
      .replaceAll('&', '&amp;')
      .replaceAll('"', '&quot;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
}

extension InputProcessingStatePresentation on core_proxy.InputProcessingState {
  /// Returns the generated enum tag under the established UI property name.
  String get kind => tag;

  /// Returns whether the state represents active processing.
  bool get isProcessing {
    return tag != 'Idle' && tag != 'Completed' && tag != 'Error';
  }

  /// Returns whether the state represents a processing failure.
  bool get isError {
    return tag == 'Error';
  }

  /// Returns the text presented for the current processing state.
  String get displayMessage {
    if (message.isNotEmpty) {
      return message;
    }
    if (tag == 'ExecutingTool') {
      return 'Executing tool $toolName';
    }
    if (tag == 'ProcessingToolResult') {
      return 'Processing tool result $toolName';
    }
    return '';
  }
}
