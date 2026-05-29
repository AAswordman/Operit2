// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../main/TopBarController.dart';
import '../../../main/components/TopBarTitleText.dart';
import '../components/ChatScreenContent.dart';
import '../components/WorkspaceShell.dart';
import '../components/workspace/WorkspaceTopBarButton.dart';
import '../viewmodel/ChatViewModel.dart';

bool _chatWorkspaceOpen = false;

class AIChatScreen extends StatefulWidget {
  AIChatScreen({super.key, ChatViewModel? viewModel})
    : viewModel = viewModel ?? ChatViewModel();

  final ChatViewModel viewModel;

  @override
  State<AIChatScreen> createState() => _AIChatScreenState();
}

class _AIChatScreenState extends State<AIChatScreen>
    with WidgetsBindingObserver {
  final TextEditingController _messageController = TextEditingController();
  final FocusNode _inputFocusNode = FocusNode();
  final ScrollController _scrollController = ScrollController();
  final List<ChatUiMessage> _messages = <ChatUiMessage>[];

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
  StreamSubscription<ChatResponseStreamEvent>? _responseStreamSubscription;
  StreamSubscription<String?>? _toastEventSubscription;
  TopBarController? _topBarController;
  final Object _topBarTitleOwner = Object();
  final Object _topBarActionsOwner = Object();
  String _currentChatTitle = '';
  String? _currentCharacterCardName;
  String? _activeCharacterCardName;
  String? _currentChatId;
  String? _currentWorkspacePath;
  String? _toastMessage;
  StreamController<String>? _responseChunkController;
  int? _streamingAiMessageTimestamp;
  bool _autoScrollToBottom = true;
  bool _hasOlderDisplayHistory = false;
  bool _hasNewerDisplayHistory = false;
  bool _isLoadingDisplayWindow = false;
  late bool _workspaceOpen;
  bool _topBarActionsUpdateScheduled = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _workspaceOpen = _chatWorkspaceOpen;
    _loadSnapshot();
    _watchToastEvent();
    _messageController.addListener(_onInputChanged);
    _inputFocusNode.addListener(_onInputFocusChanged);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _topBarController = TopBarScope.of(context);
    _scheduleTopBarActionsUpdate();
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
    _responseChunkController?.close();
    _toastEventSubscription?.cancel();
    _topBarController?.clearActions(owner: _topBarActionsOwner);
    _topBarController?.clearTitleContent(owner: _topBarTitleOwner);
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
    _toastEventSubscription = widget.viewModel.watchToastEvent().listen(
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
    widget.viewModel.clearToastEvent().catchError((
      Object error,
      StackTrace stackTrace,
    ) {
      debugPrint('Failed to clear toast event: $error\n$stackTrace');
    });
  }

  Future<ChatViewModelSnapshot?> _loadSnapshot({
    bool showLoading = true,
  }) async {
    setState(() {
      if (showLoading) {
        _loading = true;
      }
      _errorMessage = null;
    });

    try {
      final snapshot = await widget.viewModel.loadMainSnapshot();
      if (!mounted) {
        return null;
      }
      setState(() {
        _responseChunkController?.close();
        _responseChunkController = null;
        _streamingAiMessageTimestamp = null;
        _messages
          ..clear()
          ..addAll(snapshot.messages);
        _loading = snapshot.isLoading;
        _inputProcessingState = snapshot.inputProcessingState;
        _modelLabel = _resolveModelLabel(snapshot.messages);
        _currentChatId = snapshot.currentChatId;
        _currentWorkspacePath = snapshot.currentWorkspacePath;
        _currentChatTitle = snapshot.currentChatTitle;
        _currentCharacterCardName = snapshot.currentCharacterCardName;
        _activeCharacterCardName = snapshot.activeCharacterCardName;
        _hasOlderDisplayHistory = snapshot.hasOlderDisplayHistory;
        _hasNewerDisplayHistory = snapshot.hasNewerDisplayHistory;
        _isLoadingDisplayWindow = snapshot.isLoadingDisplayWindow;
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
      return;
    }

    _messageController.clear();
    setState(() {
      _autoScrollToBottom = true;
      _messages.add(
        ChatUiMessage(
          sender: 'user',
          content: text,
          timestamp: DateTime.now().microsecondsSinceEpoch,
          roleName: '',
          provider: '',
          modelName: '',
          displayMode: 'NORMAL',
          isFavorite: false,
        ),
      );
      _loading = true;
      _errorMessage = null;
    });
    _scheduleScrollToBottom();

    final request = widget.viewModel.sendUserMessage(text);
    request
        .then((_) async {
          final snapshot = await _loadSnapshot(showLoading: false);
          final chatId = snapshot?.currentChatId;
          if (chatId != null && snapshot?.isLoading == true) {
            _watchResponseStream(chatId);
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
    _responseStreamSubscription?.cancel();
    _responseChunkController?.close();
    _responseChunkController = StreamController<String>();
    _streamingAiMessageTimestamp = null;
    _responseStreamSubscription = widget.viewModel
        .watchResponseStream(chatId)
        .listen(
          (event) {
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
    await _responseChunkController?.close();
    _responseChunkController = null;
    _streamingAiMessageTimestamp = null;
    await Future<void>.delayed(const Duration(milliseconds: 220));
    await _loadSnapshot(showLoading: false);
  }

  void _appendAiStreamChunk(String chunk) {
    if (!mounted) {
      return;
    }
    final controller = _responseChunkController;
    if (controller == null || controller.isClosed) {
      return;
    }
    if (_streamingAiMessageTimestamp == null) {
      final lastMessageIndex = _messages.length - 1;
      setState(() {
        if (lastMessageIndex >= 0 &&
            _messages[lastMessageIndex].sender == 'ai' &&
            _messages[lastMessageIndex].content.isEmpty) {
          final message = _messages[lastMessageIndex];
          _streamingAiMessageTimestamp = message.timestamp;
          _messages[lastMessageIndex] = message.copyWithContentStream(
            controller.stream,
          );
        } else {
          final timestamp = DateTime.now().microsecondsSinceEpoch;
          _streamingAiMessageTimestamp = timestamp;
          _messages.add(
            ChatUiMessage(
              sender: 'ai',
              content: '',
              timestamp: timestamp,
              roleName: 'Operit',
              provider: '',
              modelName: '',
              displayMode: 'NORMAL',
              isFavorite: false,
              contentStream: controller.stream,
            ),
          );
        }
        _loading = true;
      });
      _scheduleScrollToBottom();
    }
    controller.add(chunk);
  }

  void _cancelMessage() {
    widget.viewModel.cancelCurrentMessage().catchError((
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
    if (!_autoScrollToBottom) {
      return;
    }
    if (_hasNewerDisplayHistory && !_isLoadingDisplayWindow) {
      widget.viewModel
          .showLatestMessagesForCurrentChat()
          .then((_) {
            _loadSnapshot(showLoading: false);
          })
          .catchError((Object error, StackTrace stackTrace) {
            debugPrint('Failed to show latest messages: $error\n$stackTrace');
          });
      return;
    }
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

  void _setAutoScrollToBottom(bool value) {
    if (_autoScrollToBottom == value) {
      return;
    }
    setState(() {
      _autoScrollToBottom = value;
    });
  }

  Future<List<ChatMessageLocatorPreview>> _loadMessageLocatorEntries(
    String chatId,
    String query,
  ) {
    return widget.viewModel.loadChatMessageLocatorPreviews(chatId, query);
  }

  Future<void> _setMessageFavorite(int timestamp, bool isFavorite) async {
    await widget.viewModel.setMessageFavorite(timestamp, isFavorite);
    if (!mounted) {
      return;
    }
    setState(() {
      for (var index = 0; index < _messages.length; index++) {
        final message = _messages[index];
        if (message.timestamp == timestamp) {
          _messages[index] = message.copyWith(isFavorite: isFavorite);
          break;
        }
      }
    });
  }

  Future<void> _loadOlderDisplayWindow() async {
    await widget.viewModel.loadOlderMessagesForCurrentChat();
    await _loadSnapshot(showLoading: false);
  }

  Future<void> _loadNewerDisplayWindow() async {
    await widget.viewModel.loadNewerMessagesForCurrentChat();
    await _loadSnapshot(showLoading: false);
  }

  Future<void> _showLatestDisplayWindow() async {
    await widget.viewModel.showLatestMessagesForCurrentChat();
    await _loadSnapshot(showLoading: false);
  }

  String _resolveModelLabel(List<ChatUiMessage> messages) {
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
    final modelName = await widget.viewModel.currentModelName();
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
      return;
    }
    final characterCardName = _currentCharacterCardName?.trim();
    final activeCharacterCardName = _activeCharacterCardName?.trim();
    final primaryText =
        characterCardName != null && characterCardName.isNotEmpty
        ? characterCardName
        : activeCharacterCardName != null && activeCharacterCardName.isNotEmpty
        ? activeCharacterCardName
        : 'Operit';
    final secondaryText = _currentChatTitle.trim();
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

  void _updateTopBarActions() {
    final controller = _topBarController;
    if (controller == null) {
      return;
    }
    controller.setActions((context) {
      return <Widget>[
        WorkspaceTopBarButton(
          open: _workspaceOpen,
          onPressed: _toggleWorkspace,
        ),
      ];
    }, owner: _topBarActionsOwner);
  }

  void _scheduleTopBarActionsUpdate() {
    if (_topBarActionsUpdateScheduled) {
      return;
    }
    _topBarActionsUpdateScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _topBarActionsUpdateScheduled = false;
      if (!mounted) {
        return;
      }
      _updateTopBarActions();
    });
  }

  void _toggleWorkspace() {
    _setWorkspaceOpen(!_workspaceOpen);
  }

  void _setWorkspaceOpen(bool value) {
    if (_workspaceOpen == value) {
      return;
    }
    setState(() {
      _workspaceOpen = value;
      _chatWorkspaceOpen = value;
    });
    _updateTopBarActions();
  }

  @override
  Widget build(BuildContext context) {
    return WorkspaceShell(
      workspaceOpen: _workspaceOpen,
      onWorkspaceOpenChanged: _setWorkspaceOpen,
      hasBoundWorkspace: _currentWorkspacePath?.trim().isNotEmpty == true,
      workspacePath: _currentWorkspacePath,
      onListWorkspaceFiles: widget.viewModel.listWorkspaceFiles,
      onReadWorkspaceTextFile: widget.viewModel.readWorkspaceTextFile,
      onReadWorkspaceFileBytes: widget.viewModel.readWorkspaceFileBytes,
      onOpenWorkspaceFile: widget.viewModel.openWorkspaceFile,
      onCreateDefaultWorkspace: _createDefaultWorkspace,
      onBindWorkspace: _bindWorkspace,
      child: ChatScreenContent(
        messages: _messages,
        loading: _loading,
        errorMessage: _errorMessage,
        messageController: _messageController,
        inputFocusNode: _inputFocusNode,
        scrollController: _scrollController,
        inputProcessingState: _inputProcessingState,
        modelLabel: _modelLabel,
        viewModel: widget.viewModel,
        currentChatId: _currentChatId,
        autoScrollToBottom: _autoScrollToBottom,
        hasOlderDisplayHistory: _hasOlderDisplayHistory,
        hasNewerDisplayHistory: _hasNewerDisplayHistory,
        isLoadingDisplayWindow: _isLoadingDisplayWindow,
        loadLocatorEntries: _loadMessageLocatorEntries,
        onAutoScrollToBottomChanged: _setAutoScrollToBottom,
        onLoadOlderDisplayWindow: _loadOlderDisplayWindow,
        onLoadNewerDisplayWindow: _loadNewerDisplayWindow,
        onShowLatestDisplayWindow: _showLatestDisplayWindow,
        onToggleFavoriteMessage: _setMessageFavorite,
        onSendMessage: _sendMessage,
        onCancelMessage: _cancelMessage,
        onModelChanged: _setModelLabel,
        toastMessage: _toastMessage,
        onDismissToast: _dismissToast,
      ),
    );
  }

  Future<void> _createDefaultWorkspace(String? projectType) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      throw StateError('No current chat');
    }
    await widget.viewModel.createAndBindDefaultWorkspace(chatId, projectType);
    await _loadSnapshot(showLoading: false);
  }

  Future<void> _bindWorkspace(String workspace, String? workspaceEnv) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      throw StateError('No current chat');
    }
    await widget.viewModel.bindChatToWorkspace(chatId, workspace, workspaceEnv);
    await _loadSnapshot(showLoading: false);
  }
}
