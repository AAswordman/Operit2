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
    return ChatRuntimeSnapshot(
      currentChatId: currentChatId.value as String?,
      isLoading: isLoading as bool,
      inputProcessingState: ChatInputProcessingState.fromJson(
        inputProcessingState,
      ),
      messages: (chatHistory.value as List<Object?>)
          .cast<Map<String, Object?>>()
          .map(ChatRuntimeMessage.fromJson)
          .toList(growable: false),
    );
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

  String _requestId() {
    return 'flutter-${DateTime.now().microsecondsSinceEpoch}';
  }
}

class ChatRuntimeSnapshot {
  const ChatRuntimeSnapshot({
    required this.currentChatId,
    required this.isLoading,
    required this.inputProcessingState,
    required this.messages,
  });

  final String? currentChatId;
  final bool isLoading;
  final ChatInputProcessingState inputProcessingState;
  final List<ChatRuntimeMessage> messages;
}

class ChatRuntimeMessage {
  const ChatRuntimeMessage({
    required this.sender,
    required this.content,
    required this.timestamp,
    required this.roleName,
    required this.provider,
    required this.modelName,
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
    );
  }

  final String sender;
  final String content;
  final int timestamp;
  final String roleName;
  final String provider;
  final String modelName;
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
  });

  factory ChatResponseStreamEvent.fromJson(Object? json) {
    final data = json as Map<String, Object?>;
    return ChatResponseStreamEvent(
      chatId: data['chatId'] as String,
      type: data['type'] as String,
      value: data['value'] as String?,
    );
  }

  final String chatId;
  final String type;
  final String? value;
}
