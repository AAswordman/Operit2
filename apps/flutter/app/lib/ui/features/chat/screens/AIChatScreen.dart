// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/chat/OperitChatRuntime.dart';
import '../components/ChatScreenContent.dart';

class AIChatScreen extends StatefulWidget {
  const AIChatScreen({super.key, this.runtime = const OperitChatRuntime()});

  final OperitChatRuntime runtime;

  @override
  State<AIChatScreen> createState() => _AIChatScreenState();
}

class _AIChatScreenState extends State<AIChatScreen> {
  final TextEditingController _messageController = TextEditingController();
  final FocusNode _inputFocusNode = FocusNode();
  final ScrollController _scrollController = ScrollController();
  final List<ChatRuntimeMessage> _messages = <ChatRuntimeMessage>[];

  bool _loading = true;
  ChatInputProcessingState _inputProcessingState =
      const ChatInputProcessingState(
        kind: 'Idle',
        message: '',
        progress: 0,
        toolName: '',
      );
  String _modelLabel = 'Model';
  String? _errorMessage;
  String? _currentChatId;
  StreamSubscription<ChatResponseStreamEvent>? _responseStreamSubscription;

  @override
  void initState() {
    super.initState();
    _loadSnapshot();
    _messageController.addListener(_onInputChanged);
  }

  @override
  void dispose() {
    _messageController.removeListener(_onInputChanged);
    _messageController.dispose();
    _inputFocusNode.dispose();
    _scrollController.dispose();
    _responseStreamSubscription?.cancel();
    super.dispose();
  }

  Future<void> _loadSnapshot({bool showLoading = true}) async {
    setState(() {
      if (showLoading) {
        _loading = true;
      }
      _errorMessage = null;
    });

    try {
      final snapshot = await widget.runtime.loadMainSnapshot();
      if (!mounted) {
        return;
      }
      setState(() {
        _messages
          ..clear()
          ..addAll(snapshot.messages);
        _loading = snapshot.isLoading;
        _inputProcessingState = snapshot.inputProcessingState;
        _modelLabel = _resolveModelLabel(snapshot.messages);
        _currentChatId = snapshot.currentChatId;
      });
      _scheduleScrollToBottom();
    } catch (error, stackTrace) {
      debugPrint('Failed to load chat snapshot: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  void _sendMessage() {
    final text = _messageController.text.trim();
    if (text.isEmpty) {
      debugPrint('[AIChatScreen] send ignored: empty input');
      return;
    }

    debugPrint(
      '[AIChatScreen] send tapped textLength=${text.length} '
      'currentChatId=$_currentChatId',
    );
    _messageController.clear();
    setState(() {
      _messages.add(
        ChatRuntimeMessage(
          sender: 'user',
          content: text,
          timestamp: DateTime.now().microsecondsSinceEpoch,
          roleName: '',
          provider: '',
          modelName: '',
        ),
      );
      _loading = true;
      _errorMessage = null;
    });
    _scheduleScrollToBottom();

    final request = widget.runtime.sendUserMessage(text);
    request
        .then((_) async {
          debugPrint('[AIChatScreen] send completed, refreshing snapshot');
          await _loadSnapshot(showLoading: false);
          final chatId = _currentChatId;
          if (chatId != null) {
            debugPrint('[AIChatScreen] start response stream chatId=$chatId');
            _watchResponseStream(chatId);
          } else {
            debugPrint('[AIChatScreen] response stream skipped: currentChatId is null');
          }
        })
        .catchError((Object error, StackTrace stackTrace) {
          debugPrint('Failed to send chat message: $error\n$stackTrace');
          if (!mounted) {
            return;
          }
          setState(() {
            _errorMessage = error.toString();
            _loading = false;
            _inputProcessingState = ChatInputProcessingState(
              kind: 'Error',
              message: error.toString(),
              progress: 0,
              toolName: '',
            );
          });
        });
  }

  void _watchResponseStream(String chatId) {
    debugPrint('[AIChatScreen] watch stream subscribe chatId=$chatId');
    _responseStreamSubscription?.cancel();
    _responseStreamSubscription = widget.runtime
        .watchResponseStream(chatId)
        .listen(
          (event) {
            debugPrint(
              '[AIChatScreen] stream event chatId=${event.chatId} '
              'type=${event.type} valueLength=${event.value?.length ?? 0}',
            );
            if (event.type == 'chunk') {
              final chunk = event.value;
              if (chunk == null) {
                return;
              }
              _appendAiStreamChunk(chunk);
            } else if (event.type == 'completed') {
              _loadSnapshotAfterStreamCompleted();
            }
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint('Failed to watch response stream: $error\n$stackTrace');
          },
          onDone: () {
            _loadSnapshotAfterStreamCompleted();
          },
        );
  }

  Future<void> _loadSnapshotAfterStreamCompleted() async {
    await Future<void>.delayed(const Duration(milliseconds: 80));
    await _loadSnapshot(showLoading: false);
  }

  void _appendAiStreamChunk(String chunk) {
    if (!mounted) {
      return;
    }
    setState(() {
      final lastAiIndex = _messages.lastIndexWhere(
        (message) => message.sender == 'ai',
      );
      if (lastAiIndex >= 0) {
        final message = _messages[lastAiIndex];
        _messages[lastAiIndex] = message.copyWithContent(
          message.content + chunk,
        );
      } else {
        _messages.add(
          ChatRuntimeMessage(
            sender: 'ai',
            content: chunk,
            timestamp: DateTime.now().microsecondsSinceEpoch,
            roleName: 'Operit',
            provider: '',
            modelName: '',
          ),
        );
      }
      _loading = true;
    });
    _scheduleScrollToBottom();
  }

  void _cancelMessage() {
    widget.runtime.cancelCurrentMessage().catchError((
      Object error,
      StackTrace stackTrace,
    ) {
      debugPrint('Failed to cancel chat message: $error\n$stackTrace');
    });
  }

  void _onInputChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  void _scheduleScrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scrollController.hasClients) {
        return;
      }
      _scrollController.animateTo(
        _scrollController.position.maxScrollExtent,
        duration: const Duration(milliseconds: 220),
        curve: Curves.easeOutCubic,
      );
    });
  }

  String _resolveModelLabel(List<ChatRuntimeMessage> messages) {
    for (final message in messages.reversed) {
      if (message.modelName.isNotEmpty) {
        return message.modelName.length > 26
            ? '${message.modelName.substring(0, 26)}...'
            : message.modelName;
      }
    }
    return 'Model';
  }

  @override
  Widget build(BuildContext context) {
    return ChatScreenContent(
      messages: _messages,
      loading: _loading,
      errorMessage: _errorMessage,
      messageController: _messageController,
      inputFocusNode: _inputFocusNode,
      scrollController: _scrollController,
      inputProcessingState: _inputProcessingState,
      modelLabel: _modelLabel,
      onSendMessage: _sendMessage,
      onCancelMessage: _cancelMessage,
    );
  }
}
