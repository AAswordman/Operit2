// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/chat/OperitChatRuntime.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../main/TopBarController.dart';
import '../../../main/components/TopBarTitleText.dart';
import '../components/ChatScreenContent.dart';

class AIChatScreen extends StatefulWidget {
  const AIChatScreen({super.key, this.runtime = const OperitChatRuntime()});

  final OperitChatRuntime runtime;

  @override
  State<AIChatScreen> createState() => _AIChatScreenState();
}

class _AIChatScreenState extends State<AIChatScreen>
    with WidgetsBindingObserver {
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
  StreamSubscription<String?>? _toastEventSubscription;
  TopBarController? _topBarController;
  final Object _topBarTitleOwner = Object();
  String _currentChatTitle = '';
  String? _currentCharacterCardName;
  String? _activeCharacterCardName;
  String? _toastMessage;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _loadSnapshot();
    _watchToastEvent();
    _messageController.addListener(_onInputChanged);
    _inputFocusNode.addListener(_onInputFocusChanged);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _topBarController = TopBarScope.of(context);
    debugPrint('[TopBarTitleTrace] didChangeDependencies bind controller');
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _messageController.removeListener(_onInputChanged);
    _inputFocusNode.removeListener(_onInputFocusChanged);
    _messageController.dispose();
    _inputFocusNode.dispose();
    _scrollController.dispose();
    _responseStreamSubscription?.cancel();
    _toastEventSubscription?.cancel();
    super.dispose();
  }

  @override
  void didChangeMetrics() {
    super.didChangeMetrics();
    if (_inputFocusNode.hasFocus) {
      _scheduleScrollToBottomAcrossKeyboardAnimation();
    }
  }

  void _watchToastEvent() {
    _toastEventSubscription?.cancel();
    _toastEventSubscription = widget.runtime.watchToastEvent().listen(
      (message) {
        if (!mounted || message == null || message.trim().isEmpty) {
          return;
        }
        setState(() {
          _toastMessage = message;
        });
      },
      onError: (Object error, StackTrace stackTrace) {
        debugPrint('Failed to watch toast event: $error\n$stackTrace');
      },
    );
  }

  void _dismissToast() {
    if (mounted) {
      setState(() {
        _toastMessage = null;
      });
    }
    widget.runtime.clearToastEvent().catchError((
      Object error,
      StackTrace stackTrace,
    ) {
      debugPrint('Failed to clear toast event: $error\n$stackTrace');
    });
  }

  Future<ChatRuntimeSnapshot?> _loadSnapshot({bool showLoading = true}) async {
    debugPrint(
      '[TopBarTitleTrace] loadSnapshot begin showLoading=$showLoading',
    );
    setState(() {
      if (showLoading) {
        _loading = true;
      }
      _errorMessage = null;
    });

    try {
      final snapshot = await widget.runtime.loadMainSnapshot();
      if (!mounted) {
        return null;
      }
      debugPrint(
        '[TopBarTitleTrace] loadSnapshot data '
        'chatId=${snapshot.currentChatId} '
        'chatTitle="${snapshot.currentChatTitle}" '
        'currentCard="${snapshot.currentCharacterCardName}" '
        'activeCard="${snapshot.activeCharacterCardName}"',
      );
      setState(() {
        _messages
          ..clear()
          ..addAll(snapshot.messages);
        _loading = snapshot.isLoading;
        _inputProcessingState = snapshot.inputProcessingState;
        _modelLabel = _resolveModelLabel(snapshot.messages);
        _currentChatId = snapshot.currentChatId;
        _currentChatTitle = snapshot.currentChatTitle;
        _currentCharacterCardName = snapshot.currentCharacterCardName;
        _activeCharacterCardName = snapshot.activeCharacterCardName;
      });
      _refreshCurrentModelLabel();
      _updateTopBarTitle();
      _scheduleScrollToBottom();
      return snapshot;
    } catch (error, stackTrace) {
      debugPrint('Failed to load chat snapshot: $error\n$stackTrace');
      if (!mounted) {
        return null;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
      return null;
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
          final snapshot = await _loadSnapshot(showLoading: false);
          final chatId = snapshot?.currentChatId;
          if (chatId != null && snapshot?.isLoading == true) {
            debugPrint('[AIChatScreen] start response stream chatId=$chatId');
            _watchResponseStream(chatId);
          } else {
            debugPrint(
              '[AIChatScreen] response stream skipped: '
              'chatId=$chatId isLoading=${snapshot?.isLoading}',
            );
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

  void _onInputFocusChanged() {
    if (_inputFocusNode.hasFocus) {
      _scheduleScrollToBottomAcrossKeyboardAnimation();
    }
  }

  void _scheduleScrollToBottomAcrossKeyboardAnimation() {
    _scheduleScrollToBottom();
    for (final delay in const <Duration>[
      Duration(milliseconds: 80),
      Duration(milliseconds: 180),
      Duration(milliseconds: 320),
    ]) {
      Future<void>.delayed(delay, () {
        if (mounted && _inputFocusNode.hasFocus) {
          _scheduleScrollToBottom();
        }
      });
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
    return AppLocalizations.of(context)!.model;
  }

  Future<void> _refreshCurrentModelLabel() async {
    final clients = GeneratedCoreProxyClients(widget.runtime.bridge);
    final mapping = await clients.preferencesFunctionalConfigManager
        .getConfigMappingForFunction(functionType: 'CHAT');
    final modelName = await clients.preferencesModelConfigManager
        .getModelNameByIndex(
          configId: mapping.configId,
          modelIndex: mapping.modelIndex,
        );
    if (!mounted) {
      return;
    }
    _setModelLabel(modelName);
  }

  void _setModelLabel(String modelName) {
    setState(() {
      _modelLabel = modelName.length > 26
          ? '${modelName.substring(0, 26)}...'
          : modelName;
    });
  }

  void _updateTopBarTitle() {
    final controller = _topBarController;
    if (controller == null) {
      debugPrint('[TopBarTitleTrace] update skipped controller=null');
      return;
    }
    final characterCardName = _currentCharacterCardName?.trim();
    final activeCharacterCardName = _activeCharacterCardName?.trim();
    debugPrint(
      '[TopBarTitleTrace] update input '
      'chatTitle="${_currentChatTitle.trim()}" '
      'currentCard="$characterCardName" '
      'activeCard="$activeCharacterCardName"',
    );
    final primaryText =
        characterCardName != null && characterCardName.isNotEmpty
        ? characterCardName
        : activeCharacterCardName != null && activeCharacterCardName.isNotEmpty
        ? activeCharacterCardName
        : 'Operit';
    final secondaryText = _currentChatTitle.trim();
    debugPrint(
      '[TopBarTitleTrace] set titleContent '
      'primary="$primaryText" secondary="$secondaryText"',
    );
    controller.setTitleContent(
      TopBarTitleContent((context) {
        return TopBarTitleText(
          primaryText: primaryText,
          secondaryText: secondaryText,
          contentColor: Theme.of(context).colorScheme.onSurface,
        );
      }),
      owner: _topBarTitleOwner,
    );
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
      bridge: widget.runtime.bridge,
      onSendMessage: _sendMessage,
      onCancelMessage: _cancelMessage,
      onModelChanged: _setModelLabel,
      toastMessage: _toastMessage,
      onDismissToast: _dismissToast,
    );
  }
}
