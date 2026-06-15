// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../main/MainLayoutController.dart';
import '../../../main/TopBarController.dart';
import '../../../main/components/TopBarTitleText.dart';
import '../PendingChatDraftHandler.dart';
import '../components/ChatScreenContent.dart';
import '../components/MessageEditorDialog.dart';
import '../components/WorkspaceChangeConfirmDialog.dart';
import '../components/WorkspaceShell.dart';
import '../components/workspace/WorkspaceLayoutMetrics.dart';
import '../components/workspace/WorkspaceTopBarButton.dart';
import '../viewmodel/ChatSwitchRenderCoordinator.dart';
import '../viewmodel/ChatViewModel.dart';

bool _chatWorkspaceOpen = false;

class AIChatScreen extends StatefulWidget {
  const AIChatScreen({super.key, this.viewModel});

  final ChatViewModel? viewModel;

  @override
  State<AIChatScreen> createState() => _AIChatScreenState();
}

class _ChatContentData {
  const _ChatContentData({
    required this.messages,
    required this.loading,
    required this.errorMessage,
    required this.inputProcessingState,
    required this.currentChatId,
    required this.hasOlderDisplayHistory,
    required this.hasNewerDisplayHistory,
    required this.isLoadingDisplayWindow,
    required this.isMultiSelectMode,
    required this.selectedMessageIndices,
    required this.currentCharacterCardAvatarUri,
    required this.isPreparingChatSwitch,
  });

  final List<ChatUiMessage> messages;
  final bool loading;
  final String? errorMessage;
  final ChatInputProcessingState inputProcessingState;
  final String? currentChatId;
  final bool hasOlderDisplayHistory;
  final bool hasNewerDisplayHistory;
  final bool isLoadingDisplayWindow;
  final bool isMultiSelectMode;
  final Set<int> selectedMessageIndices;
  final String? currentCharacterCardAvatarUri;
  final bool isPreparingChatSwitch;
}

class _AIChatScreenState extends State<AIChatScreen>
    with WidgetsBindingObserver {
  late final ChatViewModel _viewModel = widget.viewModel ?? ChatViewModel();
  final TextEditingController _messageController = TextEditingController();
  final FocusNode _inputFocusNode = FocusNode();
  final ScrollController _scrollController = ScrollController();
  final List<ChatUiMessage> _messages = <ChatUiMessage>[];
  late final ValueNotifier<_ChatContentData> _chatContentDataNotifier;
  late final ValueNotifier<bool> _autoScrollToBottomNotifier;
  late final ValueNotifier<String> _modelLabelNotifier;
  late final ValueNotifier<String?> _toastMessageNotifier;

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
  ChatSwitchRenderRequest? _activeChatSwitchRequest;
  TopBarController? _topBarController;
  MainLayoutController? _mainLayoutController;
  final Object _topBarTitleOwner = Object();
  final Object _topBarActionsOwner = Object();
  final Object _mainLayoutOwner = Object();
  late final MainLayoutAttachmentBuilder _workspaceMainLayoutAttachment =
      _buildWorkspaceMainLayoutAttachment;
  String _currentChatTitle = '';
  String? _currentCharacterCardName;
  String? _currentCharacterCardAvatarUri;
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
  bool _isPreparingChatSwitch = false;
  int _chatSwitchRenderGeneration = 0;
  ChatViewModelSnapshot? _pendingChatSwitchSnapshot;
  late bool _workspaceOpen;
  bool _isCurrentMainScreen = true;
  bool _topBarActionsUpdateScheduled = false;

  @override
  void initState() {
    super.initState();
    _chatContentDataNotifier = ValueNotifier<_ChatContentData>(
      _currentChatContentData(),
    );
    _autoScrollToBottomNotifier = ValueNotifier<bool>(_autoScrollToBottom);
    _modelLabelNotifier = ValueNotifier<String>(_modelLabel);
    _toastMessageNotifier = ValueNotifier<String?>(_toastMessage);
    WidgetsBinding.instance.addObserver(this);
    _workspaceOpen = _chatWorkspaceOpen;
    _watchMainState();
    _watchToastEvent();
    ChatSwitchRenderCoordinator.requests.addListener(
      _onChatSwitchRenderRequest,
    );
    PendingChatDraftHandler.revision.addListener(_consumePendingChatDraft);
    _onChatSwitchRenderRequest();
    _inputFocusNode.addListener(_onInputFocusChanged);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _consumePendingChatDraft();
    });
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _topBarController = TopBarScope.of(context);
    _mainLayoutController = MainLayoutScope.of(context);
    _isCurrentMainScreen = MainScreenActivityScope.isCurrentScreenOf(context);
    if (_isCurrentMainScreen) {
      _scheduleTopBarActionsUpdate();
    } else {
      _topBarController?.clearActions(owner: _topBarActionsOwner);
      _topBarController?.clearTitleContent(owner: _topBarTitleOwner);
      _mainLayoutController?.clearAttachment(owner: _mainLayoutOwner);
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    ChatSwitchRenderCoordinator.requests.removeListener(
      _onChatSwitchRenderRequest,
    );
    PendingChatDraftHandler.revision.removeListener(_consumePendingChatDraft);
    _inputFocusNode.removeListener(_onInputFocusChanged);
    _messageController.dispose();
    _inputFocusNode.dispose();
    _scrollController.dispose();
    _chatContentDataNotifier.dispose();
    _autoScrollToBottomNotifier.dispose();
    _modelLabelNotifier.dispose();
    _toastMessageNotifier.dispose();
    _mainStateSubscription?.cancel();
    _toastEventSubscription?.cancel();
    _topBarController?.clearActions(owner: _topBarActionsOwner);
    _topBarController?.clearTitleContent(owner: _topBarTitleOwner);
    _mainLayoutController?.clearAttachment(owner: _mainLayoutOwner);
    super.dispose();
  }

  void _consumePendingChatDraft() {
    if (!mounted) {
      return;
    }
    final draft = PendingChatDraftHandler.takePendingDraft();
    if (draft == null || draft.isEmpty) {
      return;
    }
    _messageController.text = draft;
    _messageController.selection = TextSelection.collapsed(
      offset: draft.length,
    );
    _inputFocusNode.requestFocus();
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
    _toastEventSubscription = _viewModel.watchToastEvent().listen(
      (message) {
        if (!mounted || message == null || message.trim().isEmpty) {
          return;
        }
        _toastMessage = message;
        _toastMessageNotifier.value = message;
      },
      onError: (Object error, StackTrace stackTrace) {
        debugPrint('Failed to watch toast event: $error\n$stackTrace');
      },
    );
  }

  void _dismissToast() {
    if (mounted) {
      _toastMessage = null;
      _toastMessageNotifier.value = null;
    }
    _viewModel.clearToastEvent().catchError((
      Object error,
      StackTrace stackTrace,
    ) {
      debugPrint('Failed to clear toast event: $error\n$stackTrace');
    });
  }

  void _watchMainState() {
    _mainStateSubscription?.cancel();
    _mainStateSubscription = _viewModel.watchMainState().listen(
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
        _errorMessage = error.toString();
        _loading = false;
        _publishChatContentData();
      },
    );
  }

  void _onChatSwitchRenderRequest() {
    final request = ChatSwitchRenderCoordinator.requests.value;
    if (request == null) {
      _activeChatSwitchRequest = null;
      _pendingChatSwitchSnapshot = null;
      if (_isPreparingChatSwitch) {
        _chatSwitchRenderGeneration += 1;
        _mutateChatContentData(() {
          _isPreparingChatSwitch = false;
        });
      }
      return;
    }
    if (request.targetChatId == _currentChatId) {
      return;
    }
    _activeChatSwitchRequest = request;
    _pendingChatSwitchSnapshot = null;
    _chatSwitchRenderGeneration += 1;
    _setAutoScrollToBottom(true);
    if (!_isPreparingChatSwitch) {
      _mutateChatContentData(() {
        _isPreparingChatSwitch = true;
        _errorMessage = null;
      });
    }
  }

  void _applySnapshot(ChatViewModelSnapshot snapshot) {
    final activeRequest = _activeChatSwitchRequest;
    if (_isPreparingChatSwitch && activeRequest != null) {
      if (snapshot.currentChatId != activeRequest.targetChatId) {
        return;
      }
      _prepareChatSwitchSnapshot(snapshot);
      return;
    }
    final isChatSwitch =
        _currentChatId != null &&
        snapshot.currentChatId != null &&
        _currentChatId != snapshot.currentChatId;
    if (isChatSwitch) {
      _prepareChatSwitchSnapshot(snapshot);
      return;
    }
    _commitSnapshot(snapshot, keepPreparingChatSwitch: _isPreparingChatSwitch);
  }

  void _prepareChatSwitchSnapshot(ChatViewModelSnapshot snapshot) {
    _pendingChatSwitchSnapshot = snapshot;
    final generation = ++_chatSwitchRenderGeneration;
    if (!_isPreparingChatSwitch) {
      _mutateChatContentData(() {
        _isPreparingChatSwitch = true;
        _errorMessage = null;
      });
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _commitPreparedChatSwitchSnapshot(generation);
    });
  }

  Future<void> _commitPreparedChatSwitchSnapshot(int generation) async {
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted || generation != _chatSwitchRenderGeneration) {
      return;
    }
    final snapshot = _pendingChatSwitchSnapshot;
    if (snapshot == null) {
      return;
    }
    _pendingChatSwitchSnapshot = null;
    _commitSnapshot(snapshot, keepPreparingChatSwitch: true);
    _refreshCurrentModelLabel();
    _updateTopBarTitle();
    final renderReady = await _waitForPreparedChatSwitchRender(generation);
    if (!renderReady) {
      return;
    }
    _jumpToBottomAfterPreparedSwitch();
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted || generation != _chatSwitchRenderGeneration) {
      return;
    }
    _mutateChatContentData(() {
      _isPreparingChatSwitch = false;
    });
    _activeChatSwitchRequest = null;
    ChatSwitchRenderCoordinator.clear();
  }

  Future<bool> _waitForPreparedChatSwitchRender(int generation) async {
    for (var frame = 0; frame < 2; frame++) {
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted || generation != _chatSwitchRenderGeneration) {
        return false;
      }
    }
    return true;
  }

  void _commitSnapshot(
    ChatViewModelSnapshot snapshot, {
    required bool keepPreparingChatSwitch,
  }) {
    final workspaceChanged =
        _currentChatId != snapshot.currentChatId ||
        _currentWorkspacePath != snapshot.currentWorkspacePath;
    _mutateChatContentData(() {
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
      _currentCharacterCardAvatarUri = snapshot.currentCharacterCardAvatarUri;
      _activeCharacterCardName = snapshot.activeCharacterCardName;
      _hasOlderDisplayHistory = snapshot.hasOlderDisplayHistory;
      _hasNewerDisplayHistory = snapshot.hasNewerDisplayHistory;
      _isLoadingDisplayWindow = snapshot.isLoadingDisplayWindow;
      _isPreparingChatSwitch = keepPreparingChatSwitch;
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
    if (workspaceChanged && mounted) {
      setState(() {});
      _mainLayoutController?.refreshAttachment(owner: _mainLayoutOwner);
    }
  }

  void _jumpToBottomAfterPreparedSwitch() {
    if (!_autoScrollToBottom || !_scrollController.hasClients) {
      return;
    }
    final position = _scrollController.position;
    final target = position.maxScrollExtent;
    if ((target - position.pixels).abs() <= 1) {
      return;
    }
    _scrollController.jumpTo(target);
  }

  void _sendMessage() {
    final text = _messageController.text.trim();
    if (text.isEmpty) {
      return;
    }

    _messageController.clear();
    _mutateChatContentData(() {
      _autoScrollToBottom = true;
      _autoScrollToBottomNotifier.value = true;
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
      _viewModel
          .sendUserMessage(text, replyToMessage: _replyToMessage)
          .then((_) {
            _replyToMessage = null;
            return null;
          })
          .catchError((Object error, StackTrace stackTrace) {
            debugPrint('Failed to send chat message: $error\n$stackTrace');
            if (!mounted) {
              return null;
            }
            _mutateChatContentData(() {
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
    _viewModel.cancelCurrentMessage().catchError((
      Object error,
      StackTrace stackTrace,
    ) {
      debugPrint('Failed to cancel chat message: $error\n$stackTrace');
    });
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
    if (_isPreparingChatSwitch) {
      return;
    }
    if (!_autoScrollToBottom) {
      return;
    }
    if (_hasNewerDisplayHistory && !_isLoadingDisplayWindow) {
      _viewModel
          .showLatestMessagesForCurrentChat()
          .then((_) {
            _viewModel.requestMainStateRefresh();
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
    _autoScrollToBottom = value;
    _autoScrollToBottomNotifier.value = value;
  }

  Future<List<ChatMessageLocatorPreview>> _loadMessageLocatorEntries(
    String chatId,
    String query,
  ) {
    return _viewModel.loadChatMessageLocatorPreviews(chatId, query);
  }

  Future<void> _setMessageFavorite(int timestamp, bool isFavorite) async {
    await _viewModel.setMessageFavorite(timestamp, isFavorite);
    if (!mounted) {
      return;
    }
    _mutateChatContentData(() {
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
    await _viewModel.deleteMessage(index);
  }

  Future<bool> _deleteMessagesFrom(int index) async {
    return _viewModel.deleteMessagesFrom(index);
  }

  Future<void> _deleteMessageVariant(int timestamp, int variantIndex) async {
    await _viewModel.deleteMessageVariant(timestamp, variantIndex);
  }

  void _requestRollbackToMessage(int index) {
    _showWorkspaceChangeConfirm(
      mode: WorkspaceChangeConfirmMode.rollback,
      index: index,
      onConfirm: () async {
        await _viewModel.rollbackToMessage(index);
        _viewModel.requestMainStateRefresh();
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
            await _viewModel.updateMessage(index, content);
            _viewModel.requestMainStateRefresh();
          },
          onResend: (content) async {
            if (_currentWorkspacePath != null &&
                _currentWorkspacePath!.trim().isNotEmpty) {
              await _showWorkspaceChangeConfirm(
                mode: WorkspaceChangeConfirmMode.editAndResend,
                index: index,
                onConfirm: () async {
                  await _viewModel.rewindAndResendMessage(index, content);
                  _viewModel.requestMainStateRefresh();
                },
              );
            } else {
              await _viewModel.rewindAndResendMessage(index, content);
              _viewModel.requestMainStateRefresh();
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
    final changes = await _viewModel.previewWorkspaceChangesForMessage(index);
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
    await _viewModel.regenerateSingleAiMessage(index);
  }

  void _insertSummary(ChatUiMessage message) {
    _viewModel
        .insertSummary(message)
        .then((_) => _viewModel.requestMainStateRefresh())
        .catchError((Object error, StackTrace stackTrace) {
          debugPrint('Failed to insert summary: $error\n$stackTrace');
          return null;
        });
  }

  Future<void> _createBranch(int timestamp) async {
    await _viewModel.createBranch(timestamp);
  }

  void _replyToMessageTarget(ChatUiMessage message) {
    _mutateChatContentData(() {
      _replyToMessage = message;
    });
    _inputFocusNode.requestFocus();
  }

  void _toggleMultiSelectMode(int index) {
    _mutateChatContentData(() {
      _isMultiSelectMode = true;
      _selectedMessageIndices = <int>{index};
    });
  }

  void _toggleMessageSelection(int index) {
    _mutateChatContentData(() {
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
    _mutateChatContentData(() {
      _isMultiSelectMode = false;
      _selectedMessageIndices = const <int>{};
    });
  }

  void _clearMessageSelection() {
    _mutateChatContentData(() {
      _selectedMessageIndices = const <int>{};
    });
  }

  void _selectAllMessages() {
    _mutateChatContentData(() {
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
    await _viewModel.deleteMessages(indices);
    _exitMultiSelectMode();
    _viewModel.requestMainStateRefresh();
  }

  Future<void> _loadOlderDisplayWindow() async {
    await _viewModel.loadOlderMessagesForCurrentChat();
    _viewModel.requestMainStateRefresh();
  }

  Future<void> _loadNewerDisplayWindow() async {
    await _viewModel.loadNewerMessagesForCurrentChat();
    _viewModel.requestMainStateRefresh();
  }

  Future<void> _showLatestDisplayWindow() async {
    await _viewModel.showLatestMessagesForCurrentChat();
    _viewModel.requestMainStateRefresh();
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
    final modelName = await _viewModel.currentModelName();
    if (!mounted) {
      return;
    }
    _setModelLabel(modelName);
  }

  void _setModelLabel(String modelName) {
    _modelLabel = modelName.length > 26
        ? '${modelName.substring(0, 26)}...'
        : modelName;
    _modelLabelNotifier.value = _modelLabel;
  }

  void _updateTopBarTitle() {
    final controller = _topBarController;
    if (controller == null || !_isCurrentMainScreen) {
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
    if (controller == null || !_isCurrentMainScreen) {
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
    _mainLayoutController?.refreshAttachment(owner: _mainLayoutOwner);
  }

  @override
  Widget build(BuildContext context) {
    _isCurrentMainScreen = MainScreenActivityScope.isCurrentScreenOf(context);
    final useMainLayoutWorkspace =
        MediaQuery.sizeOf(context).width >= workspaceTabletBreakpoint;
    _syncWorkspaceMainLayoutAttachment(
      useMainLayoutWorkspace && _isCurrentMainScreen,
    );
    final content = _buildChatContent();
    if (useMainLayoutWorkspace) {
      return content;
    }
    return WorkspaceShell(
      workspaceOpen: _workspaceOpen,
      onWorkspaceOpenChanged: _setWorkspaceOpen,
      hasBoundWorkspace: _currentWorkspacePath?.trim().isNotEmpty == true,
      workspacePath: _currentWorkspacePath,
      onListWorkspaceFiles: _viewModel.listWorkspaceFiles,
      onReadWorkspaceTextFile: _viewModel.readWorkspaceTextFile,
      onReadWorkspaceFileBytes: _viewModel.readWorkspaceFileBytes,
      onWriteWorkspaceFileBytes: _viewModel.writeWorkspaceFileBytes,
      onOpenWorkspaceFile: _viewModel.openWorkspaceFile,
      onCreateDefaultWorkspace: _createDefaultWorkspace,
      onBindWorkspace: _bindWorkspace,
      child: content,
    );
  }

  Widget _buildChatContent() {
    return ValueListenableBuilder<_ChatContentData>(
      valueListenable: _chatContentDataNotifier,
      builder: (context, data, _) {
        return ChatScreenContent(
          messages: data.messages,
          loading: data.loading,
          errorMessage: data.errorMessage,
          messageController: _messageController,
          inputFocusNode: _inputFocusNode,
          scrollController: _scrollController,
          inputProcessingState: data.inputProcessingState,
          modelLabelListenable: _modelLabelNotifier,
          viewModel: _viewModel,
          currentChatId: data.currentChatId,
          currentCharacterCardAvatarUri: data.currentCharacterCardAvatarUri,
          autoScrollToBottomListenable: _autoScrollToBottomNotifier,
          hasOlderDisplayHistory: data.hasOlderDisplayHistory,
          hasNewerDisplayHistory: data.hasNewerDisplayHistory,
          isLoadingDisplayWindow: data.isLoadingDisplayWindow,
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
          onRefreshRequested: () async {
            _viewModel.requestMainStateRefresh();
          },
          isMultiSelectMode: data.isMultiSelectMode,
          selectedMessageIndices: data.selectedMessageIndices,
          isPreparingChatSwitch: data.isPreparingChatSwitch,
          onSendMessage: _sendMessage,
          onCancelMessage: _cancelMessage,
          onModelChanged: _setModelLabel,
          toastMessageListenable: _toastMessageNotifier,
          onDismissToast: _dismissToast,
        );
      },
    );
  }

  Widget _buildWorkspaceMainLayoutAttachment(
    BuildContext context,
    Widget child,
  ) {
    return WorkspaceShell(
      workspaceOpen: _workspaceOpen,
      onWorkspaceOpenChanged: _setWorkspaceOpen,
      hasBoundWorkspace: _currentWorkspacePath?.trim().isNotEmpty == true,
      workspacePath: _currentWorkspacePath,
      onListWorkspaceFiles: _viewModel.listWorkspaceFiles,
      onReadWorkspaceTextFile: _viewModel.readWorkspaceTextFile,
      onReadWorkspaceFileBytes: _viewModel.readWorkspaceFileBytes,
      onWriteWorkspaceFileBytes: _viewModel.writeWorkspaceFileBytes,
      onOpenWorkspaceFile: _viewModel.openWorkspaceFile,
      onCreateDefaultWorkspace: _createDefaultWorkspace,
      onBindWorkspace: _bindWorkspace,
      child: child,
    );
  }

  void _syncWorkspaceMainLayoutAttachment(bool active) {
    final controller = _mainLayoutController;
    if (controller == null) {
      return;
    }
    if (active) {
      controller.setAttachment(
        _workspaceMainLayoutAttachment,
        owner: _mainLayoutOwner,
      );
      return;
    }
    controller.clearAttachment(owner: _mainLayoutOwner);
  }

  void _mutateChatContentData(VoidCallback mutate) {
    mutate();
    _publishChatContentData();
  }

  void _publishChatContentData() {
    _chatContentDataNotifier.value = _currentChatContentData();
  }

  _ChatContentData _currentChatContentData() {
    return _ChatContentData(
      messages: List<ChatUiMessage>.unmodifiable(_messages),
      loading: _loading,
      errorMessage: _errorMessage,
      inputProcessingState: _inputProcessingState,
      currentChatId: _currentChatId,
      hasOlderDisplayHistory: _hasOlderDisplayHistory,
      hasNewerDisplayHistory: _hasNewerDisplayHistory,
      isLoadingDisplayWindow: _isLoadingDisplayWindow,
      isMultiSelectMode: _isMultiSelectMode,
      selectedMessageIndices: _selectedMessageIndices,
      currentCharacterCardAvatarUri: _currentCharacterCardAvatarUri,
      isPreparingChatSwitch: _isPreparingChatSwitch,
    );
  }

  Future<void> _createDefaultWorkspace(String? projectType) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      throw StateError('No current chat');
    }
    await _viewModel.createAndBindDefaultWorkspace(chatId, projectType);
    _viewModel.requestMainStateRefresh();
  }

  Future<void> _bindWorkspace(String workspace, String? workspaceEnv) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      throw StateError('No current chat');
    }
    await _viewModel.bindChatToWorkspace(chatId, workspace, workspaceEnv);
    _viewModel.requestMainStateRefresh();
  }
}
