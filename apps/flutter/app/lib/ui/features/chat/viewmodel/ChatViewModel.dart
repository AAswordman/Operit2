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
typedef WorkspaceFileChange = core_proxy.WorkspaceFileChange;

class ChatViewModel {
  ChatViewModel({this.bridge = const ProxyCoreRuntimeBridge()})
    : clients = GeneratedCoreProxyClients(bridge);

  final OperitRuntimeBridge bridge;
  final GeneratedCoreProxyClients clients;
  final Map<int, _ReplayTextStream<ChatResponseStreamEvent>>
  _boundMessageStreams = <int, _ReplayTextStream<ChatResponseStreamEvent>>{};
  final Map<int, StreamSubscription<ChatResponseStreamEvent>>
  _boundResponseSubscriptions =
      <int, StreamSubscription<ChatResponseStreamEvent>>{};
  final StreamController<void> _stateRefreshRequests =
      StreamController<void>.broadcast();

  GeneratedChatRuntimeHolderMainCoreProxy get _chat =>
      clients.chatRuntimeHolderMain;

  Stream<ChatViewModelSnapshot> watchMainState() async* {
    final changes = StreamController<void>();
    final subscriptions = <StreamSubscription<dynamic>>[];
    var disposed = false;
    var scheduled = false;

    void requestSnapshot(String source) {
      if (disposed || scheduled) {
        return;
      }
      scheduled = true;
      scheduleMicrotask(() {
        scheduled = false;
        if (!disposed && !changes.isClosed) {
          changes.add(null);
        }
      });
    }

    void forwardError(Object error, StackTrace stackTrace) {
      if (!disposed && !changes.isClosed) {
        changes.addError(error, stackTrace);
      }
    }

    void listen<T>(String source, Stream<T> stream) {
      subscriptions.add(
        stream.listen((_) => requestSnapshot(source), onError: forwardError),
      );
    }

    listen('chatHistoryFlow', _rawChatHistoryFlowChanges());
    listen('currentChatIdFlow', _chat.currentChatIdFlowChanges());
    listen('chatHistoriesFlow', _chat.chatHistoriesFlowChanges());
    listen(
      'activeStreamingChatIdsFlow',
      _chat.activeStreamingChatIdsFlowChanges(),
    );
    listen(
      'inputProcessingStateByChatIdFlow',
      _chat.inputProcessingStateByChatIdFlowChanges(),
    );
    listen('responseStreamCompletedRefresh', _stateRefreshRequests.stream);

    try {
      yield await loadMainSnapshot();
      await for (final _ in changes.stream) {
        yield await loadMainSnapshot();
      }
    } finally {
      disposed = true;
      for (final subscription in subscriptions) {
        await subscription.cancel();
      }
      await changes.close();
      await _closeAllBoundResponseStreams();
    }
  }

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
    return _bindActiveResponseStream(
      ChatViewModelSnapshot(
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
      ),
    );
  }

  Future<void> sendUserMessage(
    String text, {
    ChatUiMessage? replyToMessage,
  }) async {
    await _chat.updateUserMessage(message: text);

    final mapping = await clients.preferencesFunctionalConfigManager
        .getConfigMappingForFunction(functionType: 'CHAT');

    await _chat.sendUserMessage(
      promptFunctionType: 'CHAT',
      roleCardIdOverride: null,
      chatIdOverride: null,
      messageTextOverride: null,
      proxySenderNameOverride: null,
      chatModelConfigIdOverride: mapping.configId,
      chatModelIndexOverride: mapping.modelIndex,
      attachments: const <core_proxy.AttachmentInfo>[],
      replyToMessage: replyToMessage?.toProxy(),
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

  Future<bool> rollbackToMessage(int index) {
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

  Future<void> writeWorkspaceFileBytes(
    String relativePath,
    Uint8List bytes,
  ) async {
    final chatId = await _requiredCurrentChatId();
    await clients.repositoryWorkspaceService.writeWorkspaceFileBytes(
      chatId: chatId,
      relativePath: relativePath,
      base64Content: base64Encode(bytes),
    );
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

  Stream<List<ChatUiMessage>> _rawChatHistoryFlowChanges() {
    return bridge.watchChanges('chatRuntimeHolder.main', 'chatHistoryFlow').map(
      (event) {
        return (event.value as List<Object?>)
            .map((item) => ChatUiMessage.fromJson(item as Map<String, Object?>))
            .toList(growable: false);
      },
    );
  }

  ChatViewModelSnapshot _bindActiveResponseStream(
    ChatViewModelSnapshot snapshot,
  ) {
    final activeTimestamp = _activeStreamingMessageTimestamp(snapshot);
    final currentChatId = snapshot.currentChatId;
    final activeKeys = activeTimestamp == null
        ? const <int>{}
        : <int>{activeTimestamp};

    _closeInactiveBoundResponseStreams(activeKeys);

    if (activeTimestamp != null && currentChatId != null) {
      final stream = _boundMessageStreams.putIfAbsent(activeTimestamp, () {
        return _ReplayTextStream<ChatResponseStreamEvent>(activeTimestamp);
      });
      _boundResponseSubscriptions.putIfAbsent(activeTimestamp, () {
        return watchResponseStream(currentChatId).listen(
          (event) {
            stream.add(event);
            if (event.type == 'completed') {
              stream.close();
              _requestStateRefreshAfterStreamCompleted();
            }
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint('Failed to watch response stream: $error\n$stackTrace');
            _requestStateRefreshAfterStreamCompleted();
          },
          onDone: () {
            stream.close();
            _requestStateRefreshAfterStreamCompleted();
          },
        );
      });
    }

    return snapshot.copyWith(
      messages: <ChatUiMessage>[
        for (final message in snapshot.messages)
          if (message.timestamp == activeTimestamp)
            message
                .copyWith(content: '')
                .copyWithContentStream(_boundMessageStreams[message.timestamp])
          else
            message,
      ],
    );
  }

  int? _activeStreamingMessageTimestamp(ChatViewModelSnapshot snapshot) {
    if (!snapshot.isLoading || snapshot.currentChatId == null) {
      return null;
    }
    for (final message in snapshot.messages.reversed) {
      if (message.sender == 'ai' && message.content.isEmpty) {
        return message.timestamp;
      }
    }
    return null;
  }

  void _requestStateRefreshAfterStreamCompleted() {
    Future<void>.delayed(const Duration(milliseconds: 220), () {
      if (!_stateRefreshRequests.isClosed) {
        _stateRefreshRequests.add(null);
      }
    });
  }

  void _closeInactiveBoundResponseStreams(Set<int> activeKeys) {
    final staleKeys = _boundMessageStreams.keys
        .where((timestamp) => !activeKeys.contains(timestamp))
        .toList(growable: false);
    for (final timestamp in staleKeys) {
      _boundResponseSubscriptions.remove(timestamp)?.cancel();
      _boundMessageStreams.remove(timestamp)?.close();
    }
  }

  Future<void> _closeAllBoundResponseStreams() async {
    final subscriptions = _boundResponseSubscriptions.values.toList(
      growable: false,
    );
    _boundResponseSubscriptions.clear();
    for (final subscription in subscriptions) {
      await subscription.cancel();
    }
    final streams = _boundMessageStreams.values.toList(growable: false);
    _boundMessageStreams.clear();
    for (final stream in streams) {
      await stream.close();
    }
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

  ChatViewModelSnapshot copyWith({List<ChatUiMessage>? messages}) {
    return ChatViewModelSnapshot(
      currentChatId: currentChatId,
      currentChatTitle: currentChatTitle,
      currentCharacterCardName: currentCharacterCardName,
      currentWorkspacePath: currentWorkspacePath,
      activeCharacterCardName: activeCharacterCardName,
      isLoading: isLoading,
      inputProcessingState: inputProcessingState,
      messages: messages ?? this.messages,
      hasOlderDisplayHistory: hasOlderDisplayHistory,
      hasNewerDisplayHistory: hasNewerDisplayHistory,
      isLoadingDisplayWindow: isLoadingDisplayWindow,
    );
  }
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
      content: message.content,
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
      displayMode: message.displayMode as String,
      isFavorite: message.isFavorite,
      isVariantPreview: message.isVariantPreview,
      completedAt: message.completedAt,
    );
  }

  factory ChatUiMessage.fromJson(Map<String, Object?> json) {
    return ChatUiMessage(
      sender: json['sender'] as String,
      content: json['content'] as String,
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

  ChatUiMessage copyWith({String? content, bool? isFavorite}) {
    return ChatUiMessage(
      sender: sender,
      content: content ?? this.content,
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

  ChatUiMessage copyWithContentStream(Stream<ChatResponseStreamEvent>? value) {
    return ChatUiMessage(
      sender: sender,
      content: content,
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
      content: content,
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
      displayMode: displayMode,
      isFavorite: isFavorite,
      isVariantPreview: isVariantPreview,
      contentStream: null,
    );
  }

  final String sender;
  final String content;
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

class ChatResponseStreamEvent {
  const ChatResponseStreamEvent({
    required this.chatId,
    required this.type,
    required this.id,
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
      id: data['id'] as String?,
      value: data['value'] as String?,
      blockId: data['blockId'] as int?,
      inlineId: data['inlineId'] as int?,
      nodeType: data['nodeType'] as String?,
      headerLevel: data['headerLevel'] as int?,
    );
  }

  final String chatId;
  final String type;
  final String? id;
  final String? value;
  final int? blockId;
  final int? inlineId;
  final String? nodeType;
  final int? headerLevel;
}

class _ReplayTextStream<T> extends Stream<T> {
  _ReplayTextStream(this.timestamp);

  final int timestamp;
  final List<T> _cache = <T>[];
  final StreamController<T> _liveController = StreamController<T>.broadcast();
  Object? _error;
  StackTrace? _stackTrace;
  bool _closed = false;

  void add(T chunk) {
    if (_closed) {
      return;
    }
    _cache.add(chunk);
    _liveController.add(chunk);
  }

  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;
    await _liveController.close();
  }

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
      final error = _error;
      if (error != null) {
        replayController.addError(error, _stackTrace);
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
