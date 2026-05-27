// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import '../bridge/OperitRuntimeBridge.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../link/CoreLinkProtocol.dart';

class OperitChatRuntime {
  const OperitChatRuntime({this.bridge = const ProxyCoreRuntimeBridge()});

  static const mainTargetPath = 'chatRuntimeHolder.main';

  final OperitRuntimeBridge bridge;

  Future<ChatRuntimeSnapshot> loadMainSnapshot() async {
    final currentChatId = await bridge.watch(
      mainTargetPath,
      'currentChatIdFlow',
    );
    final chatHistory = await bridge.watch(mainTargetPath, 'chatHistoryFlow');
    final chatHistories = await bridge.watch(
      mainTargetPath,
      'chatHistoriesFlow',
    );
    final isLoading = await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse(mainTargetPath),
        methodName: 'currentChatIsLoading',
        args: const {},
      ),
    );
    final inputProcessingState = await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse(mainTargetPath),
        methodName: 'currentChatInputProcessingState',
        args: const {},
      ),
    );
    final messages = await Future.wait(
      (chatHistory.value as List<Object?>).cast<Map<String, Object?>>().map(
        (json) => _messageFromSnapshotJson(json),
      ),
    );
    final currentChatMetadata = _currentChatMetadataFromSnapshot(
      currentChatId.value as String?,
      (chatHistories.value as List<Object?>).cast<Map<String, Object?>>(),
    );
    final activeCharacterCardName = await _activeCharacterCardName();
    return ChatRuntimeSnapshot(
      currentChatId: currentChatId.value as String?,
      currentChatTitle: currentChatMetadata.title,
      currentCharacterCardName: currentChatMetadata.characterCardName,
      activeCharacterCardName: activeCharacterCardName,
      isLoading: isLoading as bool,
      inputProcessingState: ChatInputProcessingState.fromJson(
        inputProcessingState,
      ),
      messages: messages,
    );
  }

  ChatRuntimeChatMetadata _currentChatMetadataFromSnapshot(
    String? currentChatId,
    List<Map<String, Object?>> chatHistories,
  ) {
    for (final history in chatHistories) {
      if (history['id'] == currentChatId) {
        return ChatRuntimeChatMetadata(
          title: history['title'] as String,
          characterCardName: history['characterCardName'] as String?,
        );
      }
    }
    return const ChatRuntimeChatMetadata(title: '', characterCardName: null);
  }

  Future<String?> _activeCharacterCardName() async {
    final activePrompt = await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('preferences.activePromptManager'),
        methodName: 'getActivePrompt',
        args: const {},
      ),
    );
    final prompt = activePrompt as Map<String, Object?>;
    final characterCard = prompt['CharacterCard'] as Map<String, Object?>?;
    if (characterCard == null) {
      return null;
    }
    final id = characterCard['id'] as String;
    final card = await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('preferences.characterCardManager'),
        methodName: 'getCharacterCard',
        args: {'id': id},
      ),
    );
    return (card as Map<String, Object?>)['name'] as String;
  }

  Future<ChatRuntimeMessage> _messageFromSnapshotJson(
    Map<String, Object?> json,
  ) async {
    final message = ChatRuntimeMessage.fromJson(json);
    if (message.sender != 'ai' || message.content.isEmpty) {
      return message;
    }
    final state = await splitMarkdownContent(message.content);
    return message.copyWithMarkdownStreamState(state);
  }

  Future<void> createNewChat() {
    return bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse(mainTargetPath),
        methodName: 'createNewChat',
        args: const {
          'characterCardName': null,
          'group': null,
          'inheritGroupFromCurrent': true,
          'setAsCurrentChat': true,
          'characterGroupId': null,
        },
      ),
    );
  }

  Future<void> sendUserMessage(String text) {
    return _sendUserMessage(text);
  }

  Future<void> _sendUserMessage(String text) async {
    debugPrint('[OperitChatRuntime] send begin textLength=${text.length}');
    await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse(mainTargetPath),
        methodName: 'updateUserMessage',
        args: {'message': text},
      ),
    );
    debugPrint('[OperitChatRuntime] updateUserMessage ok');

    final mappingJson = await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('preferences.functionalConfigManager'),
        methodName: 'getConfigMappingForFunction',
        args: const {'functionType': 'CHAT'},
      ),
    );
    final mapping = mappingJson as Map<String, Object?>;
    final configId = mapping['configId'] as String;
    final modelIndex = mapping['modelIndex'] as int;
    debugPrint(
      '[OperitChatRuntime] function mapping configId=$configId '
      'modelIndex=$modelIndex',
    );

    await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse(mainTargetPath),
        methodName: 'sendUserMessage',
        args: {
          'promptFunctionType': 'CHAT',
          'roleCardIdOverride': null,
          'chatIdOverride': null,
          'messageTextOverride': null,
          'proxySenderNameOverride': null,
          'chatModelConfigIdOverride': configId,
          'chatModelIndexOverride': modelIndex,
          'attachments': const [],
          'replyToMessage': null,
          'turnOptions': const {
            'persistTurn': true,
            'notifyReply': null,
            'hideUserMessage': false,
            'disableWarning': false,
          },
        },
      ),
    );
    debugPrint('[OperitChatRuntime] sendUserMessage ok');
  }

  Future<void> cancelCurrentMessage() {
    return bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse(mainTargetPath),
        methodName: 'cancelCurrentMessage',
        args: const {},
      ),
    );
  }

  Stream<ChatResponseStreamEvent> watchResponseStream(String chatId) {
    return bridge
        .watchChanges(
          mainTargetPath,
          'getResponseStream',
          args: {'chatId': chatId},
        )
        .map((event) => ChatResponseStreamEvent.fromJson(event.value));
  }

  Stream<String?> watchToastEvent() {
    return bridge
        .watchChanges(mainTargetPath, 'toastEventFlow')
        .map((event) => event.value as String?);
  }

  Future<void> clearToastEvent() {
    return bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse(mainTargetPath),
        methodName: 'clearToastEvent',
        args: const {},
      ),
    );
  }

  Future<ChatMarkdownStreamState> splitMarkdownContent(String content) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse(mainTargetPath),
        methodName: 'splitMarkdownContent',
        args: {'content': content},
      ),
    );
    return ChatMarkdownStreamState.fromJsonEvents(
      (value as List<Object?>).cast<Map<String, Object?>>(),
    );
  }

  String _requestId() {
    return 'flutter-${DateTime.now().microsecondsSinceEpoch}';
  }
}

class ChatRuntimeSnapshot {
  const ChatRuntimeSnapshot({
    required this.currentChatId,
    required this.currentChatTitle,
    required this.currentCharacterCardName,
    required this.activeCharacterCardName,
    required this.isLoading,
    required this.inputProcessingState,
    required this.messages,
  });

  final String? currentChatId;
  final String currentChatTitle;
  final String? currentCharacterCardName;
  final String? activeCharacterCardName;
  final bool isLoading;
  final ChatInputProcessingState inputProcessingState;
  final List<ChatRuntimeMessage> messages;
}

class ChatRuntimeChatMetadata {
  const ChatRuntimeChatMetadata({
    required this.title,
    required this.characterCardName,
  });

  final String title;
  final String? characterCardName;
}

class ChatRuntimeMessage {
  const ChatRuntimeMessage({
    required this.sender,
    required this.content,
    required this.timestamp,
    required this.roleName,
    required this.provider,
    required this.modelName,
    this.markdownStreamState,
  });

  factory ChatRuntimeMessage.fromJson(Map<String, Object?> json) {
    return ChatRuntimeMessage(
      sender: json['sender'] as String,
      content: json['content'] as String,
      timestamp: json['timestamp'] as int,
      roleName: json['roleName'] as String,
      provider: json['provider'] as String,
      modelName: json['modelName'] as String,
    );
  }

  ChatRuntimeMessage copyWithContent(String value) {
    return ChatRuntimeMessage(
      sender: sender,
      content: value,
      timestamp: timestamp,
      roleName: roleName,
      provider: provider,
      modelName: modelName,
      markdownStreamState: markdownStreamState,
    );
  }

  ChatRuntimeMessage copyWithMarkdownStreamState(
    ChatMarkdownStreamState streamState,
  ) {
    return ChatRuntimeMessage(
      sender: sender,
      content: content,
      timestamp: timestamp,
      roleName: roleName,
      provider: provider,
      modelName: modelName,
      markdownStreamState: streamState,
    );
  }

  final String sender;
  final String content;
  final int timestamp;
  final String roleName;
  final String provider;
  final String modelName;
  final ChatMarkdownStreamState? markdownStreamState;
}

class ChatMarkdownStreamState {
  ChatMarkdownStreamState();

  factory ChatMarkdownStreamState.fromJsonEvents(
    List<Map<String, Object?>> events,
  ) {
    final state = ChatMarkdownStreamState();
    for (final eventJson in events) {
      final event = ChatResponseStreamEvent.fromJson(eventJson);
      if (event.isMarkdownEvent) {
        state.apply(event);
      }
    }
    return state;
  }

  final List<ChatMarkdownBlockNode> blocks = <ChatMarkdownBlockNode>[];

  void apply(ChatResponseStreamEvent event) {
    switch (event.type) {
      case 'markdownBlockStart':
        final blockId = event.blockId;
        if (blockId == null) {
          return;
        }
        blocks.add(
          ChatMarkdownBlockNode(
            id: blockId,
            nodeType: event.nodeType,
            headerLevel: event.headerLevel,
          ),
        );
      case 'markdownBlockChunk':
        final block = _blockById(event.blockId);
        final value = event.value;
        if (block == null || value == null) {
          return;
        }
        block.content.write(value);
      case 'markdownInlineStart':
        final block = _blockById(event.blockId);
        final inlineId = event.inlineId;
        if (block == null || inlineId == null) {
          return;
        }
        block.children.add(
          ChatMarkdownInlineNode(id: inlineId, nodeType: event.nodeType),
        );
      case 'markdownInlineChunk':
        final block = _blockById(event.blockId);
        final child = block?._inlineById(event.inlineId);
        final value = event.value;
        if (block == null || child == null || value == null) {
          return;
        }
        block.content.write(value);
        child.content.write(value);
    }
  }

  ChatMarkdownBlockNode? _blockById(int? id) {
    if (id == null) {
      return null;
    }
    for (final block in blocks.reversed) {
      if (block.id == id) {
        return block;
      }
    }
    return null;
  }
}

class ChatMarkdownBlockNode {
  ChatMarkdownBlockNode({
    required this.id,
    required this.nodeType,
    required this.headerLevel,
  });

  final int id;
  final String? nodeType;
  final int? headerLevel;
  final StringBuffer content = StringBuffer();
  final List<ChatMarkdownInlineNode> children = <ChatMarkdownInlineNode>[];
}

class ChatMarkdownInlineNode {
  ChatMarkdownInlineNode({required this.id, required this.nodeType});

  final int id;
  final String? nodeType;
  final StringBuffer content = StringBuffer();
}

extension on ChatMarkdownBlockNode {
  ChatMarkdownInlineNode? _inlineById(int? id) {
    if (id == null) {
      return null;
    }
    for (final child in children.reversed) {
      if (child.id == id) {
        return child;
      }
    }
    return null;
  }
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

  bool get isMarkdownEvent {
    return type == 'markdownBlockStart' ||
        type == 'markdownBlockChunk' ||
        type == 'markdownInlineStart' ||
        type == 'markdownInlineChunk';
  }
}
