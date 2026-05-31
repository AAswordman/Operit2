// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../main/TopBarController.dart';
import '../../../main/components/TopBarTitleText.dart';
import '../components/ChatScreenContent.dart';
import '../components/MessageEditorDialog.dart';
import '../components/WorkspaceChangeConfirmDialog.dart';
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
  StreamSubscription<ChatViewModelSnapshot>? _mainStateSubscription;
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
  ChatUiMessage? _replyToMessage;
  bool _isMultiSelectMode = false;
  Set<int> _selectedMessageIndices = const <int>{};
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
    _watchMainState();
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
    _mainStateSubscription?.cancel();
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

  void _watchMainState() {
    _mainStateSubscription?.cancel();
    _mainStateSubscription = widget.viewModel.watchMainState().listen(
      (snapshot) {
        if (!mounted) {
          return;
        }
        _applySnapshot(snapshot);
        _refreshCurrentModelLabel();
        _updateTopBarTitle();
        _scheduleScrollToBottom();
      },
      onError: (Object error, StackTrace stackTrace) {
        debugPrint('Failed to watch chat state: $error\n$stackTrace');
        if (!mounted) {
          return;
        }
        setState(() {
          _errorMessage = error.toString();
          _loading = false;
        });
      },
    );
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
      _applySnapshot(snapshot);
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

  void _applySnapshot(ChatViewModelSnapshot snapshot) {
    setState(() {
      final chatChanged =
          _currentChatId != null &&
          snapshot.currentChatId != null &&
          _currentChatId != snapshot.currentChatId;
      _errorMessage = null;
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
      if (chatChanged) {
        _isMultiSelectMode = false;
        _selectedMessageIndices = const <int>{};
      } else if (_selectedMessageIndices.isNotEmpty) {
        _selectedMessageIndices = _selectedMessageIndices.where((index) {
          if (index < 0 || index >= snapshot.messages.length) {
            return false;
          }
          final sender = snapshot.messages[index].sender;
          return sender == 'user' || sender == 'ai';
        }).toSet();
      }
    });
  }

  void _sendMessage() {
    final text = _messageController.text.trim();
    if (text.isEmpty) {
      return;
    }

    _messageController.clear();
    setState(() {
      _autoScrollToBottom = true;
      _errorMessage = null;
      _loading = true;
      _inputProcessingState = const ChatInputProcessingState(
        kind: 'Processing',
        message: 'message_processing',
        progress: 0,
        toolName: '',
      );
    });
    _scheduleScrollToBottom();
    _sendMessageAfterNextFrame(text);
  }

  void _sendMessageAfterNextFrame(String text) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      widget.viewModel
          .sendUserMessage(text, replyToMessage: _replyToMessage)
          .then((_) {
            _replyToMessage = null;
            return _loadSnapshot(showLoading: false);
          })
          .catchError((Object error, StackTrace stackTrace) {
            debugPrint('Failed to send chat message: $error\n$stackTrace');
            if (!mounted) {
              return null;
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
            return null;
          });
    });
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

  Future<void> _deleteMessage(int index) async {
    await widget.viewModel.deleteMessage(index);
  }

  Future<bool> _deleteMessagesFrom(int index) async {
    return widget.viewModel.deleteMessagesFrom(index);
  }

  Future<void> _deleteMessageVariant(int timestamp, int variantIndex) async {
    await widget.viewModel.deleteMessageVariant(timestamp, variantIndex);
  }

  void _requestRollbackToMessage(int index) {
    _showWorkspaceChangeConfirm(
      mode: WorkspaceChangeConfirmMode.rollback,
      index: index,
      onConfirm: () async {
        await widget.viewModel.rollbackToMessage(index);
        await _loadSnapshot(showLoading: false);
      },
    );
  }

  void _selectMessageToEdit(int index, ChatUiMessage message) {
    showDialog<void>(
      context: context,
      builder: (context) {
        return MessageEditorDialog(
          initialText: message.content,
          showResendButton: message.sender == 'user',
          onSave: (content) async {
            await widget.viewModel.updateMessage(index, content);
            await _loadSnapshot(showLoading: false);
          },
          onResend: (content) async {
            if (_currentWorkspacePath != null &&
                _currentWorkspacePath!.trim().isNotEmpty) {
              await _showWorkspaceChangeConfirm(
                mode: WorkspaceChangeConfirmMode.editAndResend,
                index: index,
                onConfirm: () async {
                  await widget.viewModel.rewindAndResendMessage(index, content);
                  await _loadSnapshot(showLoading: false);
                },
              );
            } else {
              await widget.viewModel.rewindAndResendMessage(index, content);
              await _loadSnapshot(showLoading: false);
            }
          },
        );
      },
    );
  }

  Future<void> _showWorkspaceChangeConfirm({
    required WorkspaceChangeConfirmMode mode,
    required int index,
    required Future<void> Function() onConfirm,
  }) async {
    final changes = await widget.viewModel.previewWorkspaceChangesForMessage(
      index,
    );
    if (!mounted) {
      return;
    }
    await showDialog<void>(
      context: context,
      builder: (context) {
        return WorkspaceChangeConfirmDialog(
          mode: mode,
          changes: changes,
          onConfirm: onConfirm,
        );
      },
    );
  }

  Future<void> _regenerateMessage(int index) async {
    await widget.viewModel.regenerateSingleAiMessage(index);
  }

  void _insertSummary(ChatUiMessage message) {
    widget.viewModel
        .insertSummary(message)
        .then((_) => _loadSnapshot(showLoading: false))
        .catchError((Object error, StackTrace stackTrace) {
          debugPrint('Failed to insert summary: $error\n$stackTrace');
          return null;
        });
  }

  Future<void> _createBranch(int timestamp) async {
    await widget.viewModel.createBranch(timestamp);
  }

  void _replyToMessageTarget(ChatUiMessage message) {
    setState(() {
      _replyToMessage = message;
    });
    _inputFocusNode.requestFocus();
  }

  void _toggleMultiSelectMode(int index) {
    setState(() {
      _isMultiSelectMode = true;
      _selectedMessageIndices = <int>{index};
    });
  }

  void _toggleMessageSelection(int index) {
    setState(() {
      final next = Set<int>.of(_selectedMessageIndices);
      if (next.contains(index)) {
        next.remove(index);
      } else {
        next.add(index);
      }
      _selectedMessageIndices = next;
    });
  }

  void _exitMultiSelectMode() {
    setState(() {
      _isMultiSelectMode = false;
      _selectedMessageIndices = const <int>{};
    });
  }

  void _clearMessageSelection() {
    setState(() {
      _selectedMessageIndices = const <int>{};
    });
  }

  void _selectAllMessages() {
    setState(() {
      _isMultiSelectMode = true;
      _selectedMessageIndices = Set<int>.from(
        List<int>.generate(_messages.length, (index) => index).where((index) {
          final sender = _messages[index].sender;
          return sender == 'user' || sender == 'ai';
        }),
      );
    });
  }

  Future<void> _deleteSelectedMessages() async {
    final indices = Set<int>.of(_selectedMessageIndices);
    if (indices.isEmpty) {
      return;
    }
    await widget.viewModel.deleteMessages(indices);
    _exitMultiSelectMode();
    await _loadSnapshot(showLoading: false);
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
      onWriteWorkspaceFileBytes: widget.viewModel.writeWorkspaceFileBytes,
      onOpenWorkspaceFile: widget.viewModel.openWorkspaceFile,
      onCreateDefaultWorkspace: _createDefaultWorkspace,
      onBindWorkspace: _bindWorkspace,
      child: ChatScreenContent(
        messages: List<ChatUiMessage>.unmodifiable(_messages),
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
        onDeleteMessage: _deleteMessage,
        onDeleteMessagesFrom: _deleteMessagesFrom,
        onDeleteMessageVariant: _deleteMessageVariant,
        onRollbackToMessage: _requestRollbackToMessage,
        onSelectMessageToEdit: _selectMessageToEdit,
        onRegenerateMessage: _regenerateMessage,
        onInsertSummary: _insertSummary,
        onCreateBranch: _createBranch,
        onReplyToMessage: _replyToMessageTarget,
        onToggleMultiSelectMode: _toggleMultiSelectMode,
        onToggleMessageSelection: _toggleMessageSelection,
        onExitMultiSelectMode: _exitMultiSelectMode,
        onSelectAllMessages: _selectAllMessages,
        onClearMessageSelection: _clearMessageSelection,
        onDeleteSelectedMessages: _deleteSelectedMessages,
        onRefreshRequested: () =>
            _loadSnapshot(showLoading: false).then((_) {}),
        isMultiSelectMode: _isMultiSelectMode,
        selectedMessageIndices: _selectedMessageIndices,
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
