// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'WorkspaceFileModels.dart';

typedef ChatMessageLocatorPreview = core_proxy.ChatMessageLocatorPreview;

class ChatViewModel {
  ChatViewModel({this.bridge = const ProxyCoreRuntimeBridge()})
    : clients = GeneratedCoreProxyClients(bridge);

  final OperitRuntimeBridge bridge;
  final GeneratedCoreProxyClients clients;

  GeneratedChatRuntimeHolderMainCoreProxy get _chat =>
      clients.chatRuntimeHolderMain;

  Future<ChatViewModelSnapshot> loadMainSnapshot() async {
    final currentChatId = await _chat.currentChatIdFlowSnapshot();
    final chatHistory = await _chatHistoryFlowSnapshot();
    final chatHistories = await _chat.chatHistoriesFlowSnapshot();
    final isLoading = await _chat.currentChatIsLoading();
    final inputProcessingState = await _chat.currentChatInputProcessingState();
    final activeCharacterCardName = await _activeCharacterCardName();
    final currentChatMetadata = _currentChatMetadataFromSnapshot(
      currentChatId,
      chatHistories,
    );

    return ChatViewModelSnapshot(
      currentChatId: currentChatId,
      currentChatTitle: currentChatMetadata.title,
      currentCharacterCardName: currentChatMetadata.characterCardName,
      currentWorkspacePath: currentChatMetadata.workspacePath,
      activeCharacterCardName: activeCharacterCardName,
      isLoading: isLoading,
      inputProcessingState: ChatInputProcessingState.fromJson(
        inputProcessingState,
      ),
      messages: chatHistory,
      hasOlderDisplayHistory: await _chat.hasOlderDisplayHistory(),
      hasNewerDisplayHistory: await _chat.hasNewerDisplayHistory(),
      isLoadingDisplayWindow: await _chat.isLoadingDisplayWindow(),
    );
  }

  Future<void> sendUserMessage(String text) async {
    debugPrint('[ChatViewModel] send begin textLength=${text.length}');
    await _chat.updateUserMessage(message: text);
    debugPrint('[ChatViewModel] updateUserMessage ok');

    final mapping = await clients.preferencesFunctionalConfigManager
        .getConfigMappingForFunction(functionType: 'CHAT');
    debugPrint(
      '[ChatViewModel] function mapping configId=${mapping.configId} '
      'modelIndex=${mapping.modelIndex}',
    );

    await _chat.sendUserMessage(
      promptFunctionType: 'CHAT',
      roleCardIdOverride: null,
      chatIdOverride: null,
      messageTextOverride: null,
      proxySenderNameOverride: null,
      chatModelConfigIdOverride: mapping.configId,
      chatModelIndexOverride: mapping.modelIndex,
      attachments: const <core_proxy.AttachmentInfo>[],
      replyToMessage: null,
      turnOptions: const core_proxy.ChatTurnOptions(
        persistTurn: true,
        notifyReply: null,
        hideUserMessage: false,
        disableWarning: false,
      ),
    );
  }

  Future<void> cancelCurrentMessage() {
    return _chat.cancelCurrentMessage();
  }

  Stream<ChatResponseStreamEvent> watchResponseStream(String chatId) {
    return _chat
        .getResponseStreamChanges(chatId: chatId)
        .map(ChatResponseStreamEvent.fromJson);
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
    final mapping = await clients.preferencesFunctionalConfigManager
        .getConfigMappingForFunction(functionType: 'CHAT');
    return clients.preferencesModelConfigManager.getModelNameByIndex(
      configId: mapping.configId,
      modelIndex: mapping.modelIndex,
    );
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

  Future<void> bindChatToWorkspace(
    String chatId,
    String workspace,
    String? workspaceEnv,
  ) {
    return _chat.bindChatToWorkspace(
      chatId: chatId,
      workspace: workspace,
      workspaceEnv: workspaceEnv,
    );
  }

  Future<List<WorkspaceFileEntry>> listWorkspaceFiles(
    String relativePath,
  ) async {
    final chatId = await _requiredCurrentChatId();
    final entries = await clients.repositoryWorkspaceService.listWorkspaceFiles(
      chatId: chatId,
      relativePath: relativePath,
    );
    return entries.map(WorkspaceFileEntry.fromProxy).toList(growable: false);
  }

  Future<String> readWorkspaceTextFile(String relativePath) async {
    final chatId = await _requiredCurrentChatId();
    return clients.repositoryWorkspaceService.readWorkspaceTextFile(
      chatId: chatId,
      relativePath: relativePath,
    );
  }

  Future<Uint8List> readWorkspaceFileBytes(String relativePath) async {
    final chatId = await _requiredCurrentChatId();
    final bytes = await clients.repositoryWorkspaceService
        .readWorkspaceFileBytes(chatId: chatId, relativePath: relativePath);
    return base64Decode(bytes.base64Content);
  }

  Future<void> openWorkspaceFile(String relativePath) async {
    final chatId = await _requiredCurrentChatId();
    await clients.repositoryWorkspaceService.openWorkspaceFile(
      chatId: chatId,
      relativePath: relativePath,
    );
  }

  Future<List<ChatUiMessage>> _chatHistoryFlowSnapshot() async {
    final event = await bridge.watch(
      'chatRuntimeHolder.main',
      'chatHistoryFlow',
    );
    return (event.value as List<Object?>)
        .map((item) => ChatUiMessage.fromJson(item as Map<String, Object?>))
        .toList(growable: false);
  }

  Future<String> _requiredCurrentChatId() async {
    final chatId = await _chat.currentChatIdFlowSnapshot();
    if (chatId == null || chatId.isEmpty) {
      throw StateError('当前没有对话');
    }
    return chatId;
  }

  ChatViewModelChatMetadata _currentChatMetadataFromSnapshot(
    String? currentChatId,
    List<core_proxy.ChatHistory> chatHistories,
  ) {
    for (final history in chatHistories) {
      if (history.id == currentChatId) {
        return ChatViewModelChatMetadata(
          title: history.title,
          characterCardName: history.characterCardName,
          workspacePath: history.workspace,
        );
      }
    }
    return const ChatViewModelChatMetadata(
      title: '',
      characterCardName: null,
      workspacePath: null,
    );
  }

  Future<String?> _activeCharacterCardName() async {
    final activePrompt = await clients.preferencesActivePromptManager
        .getActivePrompt();
    final prompt = activePrompt as Map<String, Object?>;
    final characterCard = prompt['CharacterCard'] as Map<String, Object?>?;
    if (characterCard == null) {
      return null;
    }
    final id = characterCard['id'] as String;
    final card = await clients.preferencesCharacterCardManager.getCharacterCard(
      id: id,
    );
    return card.name;
  }
}

class ChatViewModelSnapshot {
  const ChatViewModelSnapshot({
    required this.currentChatId,
    required this.currentChatTitle,
    required this.currentCharacterCardName,
    required this.currentWorkspacePath,
    required this.activeCharacterCardName,
    required this.isLoading,
    required this.inputProcessingState,
    required this.messages,
    required this.hasOlderDisplayHistory,
    required this.hasNewerDisplayHistory,
    required this.isLoadingDisplayWindow,
  });

  final String? currentChatId;
  final String currentChatTitle;
  final String? currentCharacterCardName;
  final String? currentWorkspacePath;
  final String? activeCharacterCardName;
  final bool isLoading;
  final ChatInputProcessingState inputProcessingState;
  final List<ChatUiMessage> messages;
  final bool hasOlderDisplayHistory;
  final bool hasNewerDisplayHistory;
  final bool isLoadingDisplayWindow;
}

class ChatViewModelChatMetadata {
  const ChatViewModelChatMetadata({
    required this.title,
    required this.characterCardName,
    required this.workspacePath,
  });

  final String title;
  final String? characterCardName;
  final String? workspacePath;
}

class ChatUiMessage {
  const ChatUiMessage({
    required this.sender,
    required this.content,
    required this.timestamp,
    required this.roleName,
    required this.provider,
    required this.modelName,
    required this.displayMode,
    required this.isFavorite,
    this.contentStream,
  });

  factory ChatUiMessage.fromProxy(core_proxy.ChatMessage message) {
    return ChatUiMessage(
      sender: message.sender,
      content: message.content,
      timestamp: message.timestamp,
      roleName: message.roleName,
      provider: message.provider,
      modelName: message.modelName,
      displayMode: message.displayMode as String,
      isFavorite: message.isFavorite,
    );
  }

  factory ChatUiMessage.fromJson(Map<String, Object?> json) {
    return ChatUiMessage(
      sender: json['sender'] as String,
      content: json['content'] as String,
      timestamp: json['timestamp'] as int,
      roleName: json['roleName'] as String,
      provider: json['provider'] as String,
      modelName: json['modelName'] as String,
      displayMode: json['displayMode'] as String,
      isFavorite: json['isFavorite'] as bool,
    );
  }

  ChatUiMessage copyWith({String? content, bool? isFavorite}) {
    return ChatUiMessage(
      sender: sender,
      content: content ?? this.content,
      timestamp: timestamp,
      roleName: roleName,
      provider: provider,
      modelName: modelName,
      displayMode: displayMode,
      isFavorite: isFavorite ?? this.isFavorite,
      contentStream: contentStream,
    );
  }

  ChatUiMessage copyWithContentStream(Stream<String>? value) {
    return ChatUiMessage(
      sender: sender,
      content: content,
      timestamp: timestamp,
      roleName: roleName,
      provider: provider,
      modelName: modelName,
      displayMode: displayMode,
      isFavorite: isFavorite,
      contentStream: value,
    );
  }

  final String sender;
  final String content;
  final int timestamp;
  final String roleName;
  final String provider;
  final String modelName;
  final String displayMode;
  final bool isFavorite;
  final Stream<String>? contentStream;

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

class ChatResponseStreamEvent {
  const ChatResponseStreamEvent({
    required this.chatId,
    required this.type,
    required this.value,
    required this.blockId,
    required this.inlineId,
    required this.nodeType,
    required this.headerLevel,
  });

  factory ChatResponseStreamEvent.fromJson(Object? json) {
    final data = json as Map<String, Object?>;
    return ChatResponseStreamEvent(
      chatId: data['chatId'] as String,
      type: data['type'] as String,
      value: data['value'] as String?,
      blockId: data['blockId'] as int?,
      inlineId: data['inlineId'] as int?,
      nodeType: data['nodeType'] as String?,
      headerLevel: data['headerLevel'] as int?,
    );
  }

  final String chatId;
  final String type;
  final String? value;
  final int? blockId;
  final int? inlineId;
  final String? nodeType;
  final int? headerLevel;
}
