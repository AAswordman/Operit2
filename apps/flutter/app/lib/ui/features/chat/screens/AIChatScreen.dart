// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:file_selector/file_selector.dart';

import '../../../../core/logging/ClientLogger.dart';
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../main/MainLayoutController.dart';
import '../../../main/TopBarController.dart';
import '../../../main/components/TopBarTitleText.dart';
import '../PendingChatDraftHandler.dart';
import '../components/ChatScreenContent.dart';
import '../components/MessageEditorDialog.dart';
import '../components/WorkspaceChangeConfirmDialog.dart';
import '../components/WorkspaceShell.dart';
import '../components/style/input/common/PendingQueueMessageItem.dart';
import '../components/workspace/WorkspaceLayoutMetrics.dart';
import '../components/workspace/WorkspaceTopBarButton.dart';
import '../speech/LocalSpeechRecorder.dart';
import '../viewmodel/ChatSwitchRenderCoordinator.dart';
import '../viewmodel/ChatViewModel.dart';

bool _chatWorkspaceOpen = false;
const String _localSttLogTag = 'LocalSTT';

/// Derives text inserted by one editing update from the previous selection.
String? _insertedTextFromInputChange({
  required TextEditingValue previousValue,
  required TextEditingValue proposedValue,
}) {
  final selection = previousValue.selection;
  final previousText = previousValue.text;
  if (!selection.isValid ||
      selection.start > previousText.length ||
      selection.end > previousText.length) {
    return null;
  }
  final prefix = previousText.substring(0, selection.start);
  final suffix = previousText.substring(selection.end);
  final proposedText = proposedValue.text;
  if (proposedText.length < prefix.length + suffix.length ||
      !proposedText.startsWith(prefix) ||
      !proposedText.endsWith(suffix)) {
    return null;
  }
  final insertedTextEnd = proposedText.length - suffix.length;
  return proposedText.substring(selection.start, insertedTextEnd);
}

/// Returns clipboard text when it exactly matches the insertion modulo line endings.
String? _pastedTextFromClipboard({
  required TextEditingValue previousValue,
  required TextEditingValue proposedValue,
  required String clipboardText,
}) {
  final insertedText = _insertedTextFromInputChange(
    previousValue: previousValue,
    proposedValue: proposedValue,
  );
  if (insertedText == null ||
      _normalizedLineEndings(insertedText) !=
          _normalizedLineEndings(clipboardText)) {
    return null;
  }
  return clipboardText;
}

/// Normalizes platform line-ending conventions for exact clipboard comparison.
String _normalizedLineEndings(String value) {
  return value.replaceAll('\r\n', '\n').replaceAll('\r', '\n');
}

class AIChatScreen extends StatelessWidget {
  /// Creates the full host-owned AI chat screen.
  const AIChatScreen({
    super.key,
    this.viewModel,
    this.runtimeSurface = ChatRuntimeSurface.main,
  });

  final ChatViewModel? viewModel;
  final ChatRuntimeSurface runtimeSurface;

  /// Builds the full chat surface owned by the main application host.
  @override
  Widget build(BuildContext context) {
    return _AIChatSurface(
      viewModel: viewModel,
      runtimeSurface: runtimeSurface,
      embedded: false,
    );
  }
}

/// Renders AI chat content without the host workspace or top-bar integration.
class AIChatEmbed extends StatelessWidget {
  /// Creates an AI chat control for embedding in another host surface.
  const AIChatEmbed({super.key, this.viewModel});

  final ChatViewModel? viewModel;

  /// Builds the workspace-free chat control for the surrounding surface.
  @override
  Widget build(BuildContext context) {
    return _AIChatSurface(
      viewModel: viewModel,
      runtimeSurface: ChatRuntimeSurface.main,
      embedded: true,
    );
  }
}

class _AIChatSurface extends StatefulWidget {
  /// Creates the shared implementation for a full chat screen or embedded chat.
  const _AIChatSurface({
    required this.viewModel,
    required this.runtimeSurface,
    required this.embedded,
  });

  final ChatViewModel? viewModel;
  final ChatRuntimeSurface runtimeSurface;
  final bool embedded;

  /// Creates the state shared by the full and embedded chat surfaces.
  @override
  State<_AIChatSurface> createState() => _AIChatSurfaceState();
}

final Map<String, Map<String?, TextEditingValue>> _chatInputDraftStores =
    <String, Map<String?, TextEditingValue>>{};

String _chatInputDraftStoreKey(ChatRuntimeSurface surface) {
  return switch (surface) {
    MainChatRuntimeSurface() => 'main',
    FloatingChatRuntimeSurface() => 'floating',
    DetachedChatRuntimeSurface(:final slotId) => 'detached:$slotId',
  };
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
    required this.pendingQueueMessages,
    required this.isPendingQueueExpanded,
    required this.attachments,
    required this.isSpeechRecording,
    required this.isSpeechTranscribing,
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
  final List<PendingQueueMessageItem> pendingQueueMessages;
  final bool isPendingQueueExpanded;
  final List<AttachmentInfo> attachments;
  final bool isSpeechRecording;
  final bool isSpeechTranscribing;
}

class _AIChatSurfaceState extends State<_AIChatSurface> {
  late final ChatViewModel _viewModel =
      widget.viewModel ?? ChatViewModel(runtimeSurface: widget.runtimeSurface);
  final TextEditingController _messageController = TextEditingController();
  TextEditingValue _previousMessageInputValue = TextEditingValue.empty;
  final FocusNode _inputFocusNode = FocusNode();
  final ScrollController _scrollController = ScrollController();
  final LocalSpeechRecorder _speechRecorder = LocalSpeechRecorder();
  late final Map<String?, TextEditingValue> _inputDraftsByChatId;
  final List<ChatUiMessage> _messages = <ChatUiMessage>[];
  List<AttachmentInfo> _attachments = const <AttachmentInfo>[];
  late final ValueNotifier<_ChatContentData> _chatContentDataNotifier;
  late final ValueNotifier<bool> _autoScrollToBottomNotifier;
  late final ValueNotifier<String?> _toastMessageNotifier;

  bool _loading = true;
  ChatInputProcessingState _inputProcessingState =
      const ChatInputProcessingState(
        kind: 'Idle',
        message: '',
        progress: 0,
        toolName: '',
      );
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
  bool _bottomScrollScheduled = false;
  int _chatSwitchRenderGeneration = 0;
  ChatViewModelSnapshot? _pendingChatSwitchSnapshot;
  late bool _workspaceOpen;
  bool _isCurrentMainScreen = true;
  bool _topBarActionsUpdateScheduled = false;
  bool _pendingQueueEnqueueInFlight = false;
  bool _isApplyingChatDraft = false;
  bool _isSpeechRecording = false;
  bool _isSpeechTranscribing = false;

  /// Initializes chat state and subscriptions.
  @override
  void initState() {
    super.initState();
    _inputDraftsByChatId = _chatInputDraftStores.putIfAbsent(
      _chatInputDraftStoreKey(_viewModel.runtimeSurface),
      () => <String?, TextEditingValue>{},
    );
    _chatContentDataNotifier = ValueNotifier<_ChatContentData>(
      _currentChatContentData(),
    );
    _autoScrollToBottomNotifier = ValueNotifier<bool>(_autoScrollToBottom);
    _toastMessageNotifier = ValueNotifier<String?>(_toastMessage);
    _workspaceOpen = _chatWorkspaceOpen;
    _watchMainState();
    _watchToastEvent();
    ChatSwitchRenderCoordinator.requests.addListener(
      _onChatSwitchRenderRequest,
    );
    PendingChatDraftHandler.revision.addListener(_consumePendingChatDraft);
    _onChatSwitchRenderRequest();
    _messageController.addListener(_onMessageControllerChanged);
    unawaited(_loadLongPastedTextInputSettings());
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _consumePendingChatDraft();
      _refreshAttachments();
    });
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (widget.embedded) {
      _isCurrentMainScreen = false;
      return;
    }
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

  /// Releases chat state and subscriptions.
  @override
  void dispose() {
    _saveCurrentInputDraft();
    ChatSwitchRenderCoordinator.requests.removeListener(
      _onChatSwitchRenderRequest,
    );
    PendingChatDraftHandler.revision.removeListener(_consumePendingChatDraft);
    _messageController.removeListener(_onMessageControllerChanged);
    _messageController.dispose();
    _inputFocusNode.dispose();
    _scrollController.dispose();
    _chatContentDataNotifier.dispose();
    _autoScrollToBottomNotifier.dispose();
    _toastMessageNotifier.dispose();
    _mainStateSubscription?.cancel();
    _toastEventSubscription?.cancel();
    unawaited(_speechRecorder.dispose());
    _topBarController?.clearActions(owner: _topBarActionsOwner);
    _topBarController?.clearTitleContent(owner: _topBarTitleOwner);
    _mainLayoutController?.clearAttachment(owner: _mainLayoutOwner);
    super.dispose();
  }

  /// Loads the global long-paste conversion settings used by this chat surface.
  Future<void> _loadLongPastedTextInputSettings() async {
    try {
      await const UserPreferencesManager().loadLongPastedTextInputSettings();
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Unable to load long pasted text preferences',
        tag: 'AIChatScreen',
        error: error,
        stackTrace: stackTrace,
      );
    }
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

  void _onMessageControllerChanged() {
    final previousValue = _previousMessageInputValue;
    final proposedValue = _messageController.value;
    _previousMessageInputValue = proposedValue;
    if (_isApplyingChatDraft) {
      return;
    }
    final settings = UserPreferencesManager.longPastedTextInputSettings.value;
    final insertedText = _insertedTextFromInputChange(
      previousValue: previousValue,
      proposedValue: proposedValue,
    );
    if (settings.enabled &&
        insertedText != null &&
        insertedText.runes.length > settings.threshold) {
      unawaited(
        _convertLongPastedText(
          previousValue: previousValue,
          proposedValue: proposedValue,
          chatId: _currentChatId,
        ),
      );
      return;
    }
    _recordMessageInputChange(proposedValue);
  }

  /// Persists and dispatches one accepted message input editing value.
  void _recordMessageInputChange(TextEditingValue value) {
    _saveInputDraft(value);
    unawaited(
      _viewModel
          .dispatchChatInputChanged(
            chatId: _currentChatId,
            text: value.text,
            selectionStart: value.selection.start,
            selectionEnd: value.selection.end,
            attachmentCount: _attachments.length,
          )
          .catchError((Object error, StackTrace stackTrace) {
            ClientLogger.e(
              'chat input change hook failed',
              tag: 'AIChatScreen',
              error: error,
              stackTrace: stackTrace,
            );
          }),
    );
  }

  /// Converts a verified long clipboard insertion into a text attachment.
  Future<void> _convertLongPastedText({
    required TextEditingValue previousValue,
    required TextEditingValue proposedValue,
    required String? chatId,
  }) async {
    try {
      final clipboardData = await Clipboard.getData(Clipboard.kTextPlain);
      final clipboardText = clipboardData?.text;
      if (clipboardText == null) {
        _recordDeferredMessageInputChange(chatId, proposedValue);
        return;
      }
      final pastedText = _pastedTextFromClipboard(
        previousValue: previousValue,
        proposedValue: proposedValue,
        clipboardText: clipboardText,
      );
      if (pastedText == null) {
        _recordDeferredMessageInputChange(chatId, proposedValue);
        return;
      }
      if (!_matchesPendingLongPaste(chatId, proposedValue)) {
        return;
      }
      await _viewModel.attachPastedText(pastedText);
      if (!_matchesPendingLongPaste(chatId, proposedValue)) {
        return;
      }
      try {
        await _refreshAttachments();
      } catch (error, stackTrace) {
        ClientLogger.e(
          'Unable to refresh attachments after converting pasted text',
          tag: 'AIChatScreen',
          error: error,
          stackTrace: stackTrace,
        );
      }
      if (!_matchesPendingLongPaste(chatId, proposedValue)) {
        return;
      }
      _isApplyingChatDraft = true;
      _messageController.value = previousValue;
      _isApplyingChatDraft = false;
      _previousMessageInputValue = previousValue;
      _recordMessageInputChange(previousValue);
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Unable to convert pasted text to an attachment',
        tag: 'AIChatScreen',
        error: error,
        stackTrace: stackTrace,
      );
      _recordDeferredMessageInputChange(chatId, proposedValue);
    }
  }

  /// Records a deferred input update while it still belongs to the active chat.
  void _recordDeferredMessageInputChange(
    String? chatId,
    TextEditingValue proposedValue,
  ) {
    if (_matchesPendingLongPaste(chatId, proposedValue)) {
      _recordMessageInputChange(proposedValue);
    }
  }

  /// Checks whether a long-paste conversion still targets the active editor value.
  bool _matchesPendingLongPaste(
    String? chatId,
    TextEditingValue proposedValue,
  ) {
    return mounted &&
        _currentChatId == chatId &&
        _messageController.value == proposedValue;
  }

  void _saveCurrentInputDraft() {
    _saveInputDraft(_messageController.value);
  }

  /// Stores one editing value as the active chat's current draft.
  void _saveInputDraft(TextEditingValue value) {
    _inputDraftsByChatId[_currentChatId] = value;
  }

  void _restoreInputDraftForChat(String? chatId) {
    final value = _inputDraftsByChatId[chatId] ?? TextEditingValue.empty;
    _isApplyingChatDraft = true;
    _messageController.value = value;
    _isApplyingChatDraft = false;
  }

  bool get _isQueueBlocked {
    return _loading || _inputProcessingState.isProcessing;
  }

  /// Requests a Rust-owned automatic dequeue after the active chat becomes ready.
  void _syncPendingQueueAfterSnapshot() {
    if (_viewModel.runtimeSurface is! MainChatRuntimeSurface) {
      return;
    }
    final contentData = _chatContentDataNotifier.value;
    if (!_isQueueBlocked && contentData.pendingQueueMessages.isNotEmpty) {
      _schedulePendingQueueAutoDequeue();
    }
  }

  /// Schedules an atomic Rust dequeue only while the owning chat remains active.
  void _schedulePendingQueueAutoDequeue() {
    final queueChatId = _currentChatId;
    final contentData = _chatContentDataNotifier.value;
    if (queueChatId == null || contentData.pendingQueueMessages.isEmpty) {
      return;
    }
    Future<void>.delayed(const Duration(milliseconds: 250), () {
      final currentContentData = _chatContentDataNotifier.value;
      if (!mounted ||
          _currentChatId != queueChatId ||
          _isQueueBlocked ||
          currentContentData.pendingQueueMessages.isEmpty) {
        return;
      }
      unawaited(_takeNextPendingQueueMessageIfReady(queueChatId));
    });
  }

  /// Atomically takes and submits the next Rust-owned queued message when available.
  Future<void> _takeNextPendingQueueMessageIfReady(String chatId) async {
    try {
      final item = await _viewModel.takeNextPendingQueueMessageIfReady(chatId);
      if (item == null) {
        return;
      }
      await _sendQueuedItemNow(chatId, item, false);
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Failed to dequeue pending chat message',
        tag: 'AIChatScreen',
        error: error,
        stackTrace: stackTrace,
      );
    }
  }

  /// Enqueues the current draft in the Rust-owned queue for the active chat.
  void _enqueueDraftToPendingQueue() {
    unawaited(_enqueueDraftToPendingQueueInRuntime());
  }

  /// Commits the current draft to the Rust-owned queue without losing a changed draft.
  Future<void> _enqueueDraftToPendingQueueInRuntime() async {
    final draftText = _messageController.text.trim();
    final chatId = _currentChatId;
    if (_pendingQueueEnqueueInFlight || draftText.isEmpty || chatId == null) {
      return;
    }
    _pendingQueueEnqueueInFlight = true;
    try {
      await _viewModel.enqueuePendingQueueMessage(
        chatId: chatId,
        messageText: draftText,
      );
      final savedDraft = _inputDraftsByChatId[chatId];
      if (savedDraft != null && savedDraft.text.trim() == draftText) {
        _inputDraftsByChatId[chatId] = TextEditingValue.empty;
      }
      if (!mounted || _currentChatId != chatId) {
        return;
      }
      if (_messageController.text.trim() == draftText) {
        _messageController.clear();
      }
      _showLocalToast(AppLocalizations.of(context)!.chatQueueAdded);
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Failed to enqueue pending chat message',
        tag: 'AIChatScreen',
        error: error,
        stackTrace: stackTrace,
      );
    } finally {
      _pendingQueueEnqueueInFlight = false;
    }
  }

  /// Deletes one message from the Rust-owned queue for the active chat.
  void _deletePendingQueueMessage(int id) {
    unawaited(_deletePendingQueueMessageInRuntime(id));
  }

  /// Applies a pending-message deletion through the chat runtime.
  Future<void> _deletePendingQueueMessageInRuntime(int id) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      return;
    }
    try {
      await _viewModel.deletePendingQueueMessage(chatId: chatId, messageId: id);
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Failed to delete pending chat message',
        tag: 'AIChatScreen',
        error: error,
        stackTrace: stackTrace,
      );
    }
  }

  /// Moves one Rust-owned queue item into the input editor for the active chat.
  void _editPendingQueueMessage(int id) {
    unawaited(_editPendingQueueMessageInRuntime(id));
  }

  /// Atomically takes a queue item before placing it in the active input editor.
  Future<void> _editPendingQueueMessageInRuntime(int id) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      return;
    }
    try {
      final item = await _viewModel.takePendingQueueMessage(
        chatId: chatId,
        messageId: id,
        suppressNextAutoDequeue: false,
      );
      if (item == null) {
        return;
      }
      if (!mounted || _currentChatId != chatId) {
        await _viewModel.restorePendingQueueMessage(
          chatId: chatId,
          message: item,
        );
        return;
      }
      _messageController.text = item.text;
      _messageController.selection = TextSelection.collapsed(
        offset: item.text.length,
      );
      _inputFocusNode.requestFocus();
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Failed to edit pending chat message',
        tag: 'AIChatScreen',
        error: error,
        stackTrace: stackTrace,
      );
    }
  }

  /// Sends a manually selected item through the chat that owns the Rust queue.
  void _sendPendingQueueMessage(int id) {
    unawaited(_sendPendingQueueMessageInRuntime(id));
  }

  /// Atomically takes the selected queue item before submitting it.
  Future<void> _sendPendingQueueMessageInRuntime(int id) async {
    final queueChatId = _currentChatId;
    if (queueChatId == null) {
      return;
    }
    try {
      final item = await _viewModel.takePendingQueueMessage(
        chatId: queueChatId,
        messageId: id,
        suppressNextAutoDequeue: true,
      );
      if (item == null) {
        return;
      }
      await _sendQueuedItemNow(queueChatId, item, true);
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Failed to send pending chat message',
        tag: 'AIChatScreen',
        error: error,
        stackTrace: stackTrace,
      );
    }
  }

  /// Runs queue submission hooks and sends the item to its owning chat.
  Future<void> _sendQueuedItemNow(
    String queueChatId,
    PendingQueueMessageItem item,
    bool cancelCurrentConversation,
  ) async {
    var queuedText = item.text;
    final decision = await _viewModel.dispatchChatInputSubmitRequested(
      chatId: queueChatId,
      text: queuedText,
      selectionStart: queuedText.length,
      selectionEnd: queuedText.length,
      attachmentCount: 0,
    );
    if (decision != null) {
      final timeoutMessage = decision.message;
      if (decision.timedOut && timeoutMessage != null) {
        _showLocalToast(timeoutMessage);
      }
      if (decision.action == 'block') {
        if (cancelCurrentConversation) {
          await _viewModel.clearPendingQueueAutoDequeueSuppression(queueChatId);
        }
        await _viewModel.restorePendingQueueMessage(
          chatId: queueChatId,
          message: item,
        );
        final message = decision.message;
        if (mounted && message != null && message.trim().isNotEmpty) {
          _showLocalToast(message);
        }
        return;
      }
      if (decision.action == 'consume') {
        if (cancelCurrentConversation) {
          await _viewModel.clearPendingQueueAutoDequeueSuppression(queueChatId);
        }
        final message = decision.message;
        if (mounted && message != null && message.trim().isNotEmpty) {
          _showLocalToast(message);
        }
        return;
      }
      if (decision.action == 'replace') {
        final updatedText = decision.text;
        if (updatedText != null) {
          queuedText = updatedText;
        }
      }
    }
    if (cancelCurrentConversation) {
      await _viewModel.cancelMessage(queueChatId);
    }
    if (queuedText.trim().isEmpty) {
      return;
    }
    if (mounted && _currentChatId == queueChatId) {
      _inputFocusNode.unfocus();
    }
    await _viewModel.sendUserMessage(
      queuedText.trim(),
      chatIdOverride: queueChatId,
    );
  }

  /// Persists the pending-queue expanded state through the chat runtime.
  void _setPendingQueueExpanded(bool expanded) {
    unawaited(_setPendingQueueExpandedInRuntime(expanded));
  }

  /// Applies a queue-expansion change to the currently active chat.
  Future<void> _setPendingQueueExpandedInRuntime(bool expanded) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      return;
    }
    try {
      await _viewModel.setPendingQueueExpanded(
        chatId: chatId,
        isExpanded: expanded,
      );
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Failed to update pending queue expanded state',
        tag: 'AIChatScreen',
        error: error,
        stackTrace: stackTrace,
      );
    }
  }

  void _showLocalToast(String message) {
    if (!mounted || message.trim().isEmpty) {
      return;
    }
    _toastMessage = message;
    _toastMessageNotifier.value = message;
  }

  Future<void> _refreshAttachments() async {
    final attachments = await _viewModel.attachments();
    if (!mounted) {
      return;
    }
    _attachments = attachments;
    _publishChatContentData();
  }

  Future<void> _handleAttachImage() async {
    const imageGroup = XTypeGroup(
      label: 'image',
      extensions: <String>['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif', 'heic'],
    );
    final files = await openFiles(
      acceptedTypeGroups: const <XTypeGroup>[imageGroup],
    );
    await _handleSelectedAttachmentFiles(files);
  }

  Future<void> _handleAttachFile() async {
    final files = await openFiles();
    await _handleSelectedAttachmentFiles(files);
  }

  Future<void> _handleSelectedAttachmentFiles(List<XFile> files) {
    return _handleAttachmentPaths(files.map((file) => file.path).toList());
  }

  Future<void> _handleAttachmentPaths(List<String> paths) async {
    for (final path in paths) {
      await _viewModel.handleAttachment(path);
    }
    await _refreshAttachments();
  }

  Future<void> _handleSpecialAttachment(String filePath) async {
    await _viewModel.handleAttachment(filePath);
    await _refreshAttachments();
  }

  Future<void> _handleAttachPackage(String packageName) {
    return _handleSpecialAttachment('package_attach:$packageName');
  }

  void _handleTakePhoto() {
    _showLocalToast(AppLocalizations.of(context)!.attachmentCameraUnavailable);
  }

  void _handleAttachMemory() {
    _showLocalToast(AppLocalizations.of(context)!.attachmentMemoryUnavailable);
  }

  Future<void> _removeAttachment(String filePath) async {
    await _viewModel.removeAttachment(filePath);
    await _refreshAttachments();
  }

  void _insertAttachmentReference(AttachmentInfo attachment) {
    final reference = _viewModel.createAttachmentReference(attachment);
    final value = _messageController.value;
    final text = value.text;
    final selection = value.selection;
    final range = selection.isValid
        ? selection
        : TextSelection.collapsed(offset: text.length);
    final nextText = text.replaceRange(range.start, range.end, reference);
    _messageController.value = TextEditingValue(
      text: nextText,
      selection: TextSelection.collapsed(
        offset: range.start + reference.length,
      ),
      composing: TextRange.empty,
    );
    _inputFocusNode.requestFocus();
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
    final chatChanged = _currentChatId != snapshot.currentChatId;
    final workspaceChanged =
        _currentChatId != snapshot.currentChatId ||
        _currentWorkspacePath != snapshot.currentWorkspacePath;
    if (chatChanged) {
      _saveCurrentInputDraft();
    }
    _mutateChatContentData(() {
      final didSwitchChat =
          _currentChatId != null &&
          snapshot.currentChatId != null &&
          _currentChatId != snapshot.currentChatId;
      _errorMessage = null;
      _messages
        ..clear()
        ..addAll(snapshot.messages);
      _loading = snapshot.isLoading;
      _inputProcessingState = snapshot.inputProcessingState;
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
      if (didSwitchChat) {
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
    _chatContentDataNotifier.value = _currentChatContentData(
      pendingQueueMessages: snapshot.pendingQueueMessages,
      isPendingQueueExpanded: snapshot.isPendingQueueExpanded,
    );
    if (chatChanged) {
      _restoreInputDraftForChat(snapshot.currentChatId);
    }
    if (workspaceChanged && mounted) {
      setState(() {});
      _mainLayoutController?.refreshAttachment(owner: _mainLayoutOwner);
    }
    _syncPendingQueueAfterSnapshot();
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
    unawaited(_sendMessageWithHooks());
  }

  /// Dispatches submit_requested before mutating the visible input field.
  Future<void> _sendMessageWithHooks() async {
    final text = _messageController.text.trim();
    final hasAttachments = _attachments.isNotEmpty;
    if (text.isEmpty && !hasAttachments) {
      return;
    }
    if (_isQueueBlocked && text.isNotEmpty) {
      _enqueueDraftToPendingQueue();
      return;
    }
    if (_isQueueBlocked) {
      return;
    }
    if (_currentChatId == null || _currentChatId!.trim().isEmpty) {
      _showLocalToast(AppLocalizations.of(context)!.chatPleaseCreateNewChat);
      return;
    }

    final inputValue = _messageController.value;
    final decision = await _viewModel.dispatchChatInputSubmitRequested(
      chatId: _currentChatId,
      text: text,
      selectionStart: inputValue.selection.start,
      selectionEnd: inputValue.selection.end,
      attachmentCount: _attachments.length,
    );
    if (!mounted) {
      return;
    }
    if (decision != null) {
      final timeoutMessage = decision.message;
      if (decision.timedOut && timeoutMessage != null) {
        _showLocalToast(timeoutMessage);
      }
      if (decision.action == 'block' || decision.action == 'consume') {
        if (decision.action == 'consume' && decision.clearInput) {
          _messageController.clear();
          await _viewModel.clearAttachments();
        }
        final message = decision.message;
        if (message != null && message.trim().isNotEmpty) {
          _showLocalToast(message);
        }
        return;
      }
      if (decision.action == 'replace') {
        final updatedText = decision.text;
        if (updatedText != null) {
          _messageController.value = TextEditingValue(
            text: updatedText,
            selection: TextSelection.collapsed(offset: updatedText.length),
          );
        }
      }
    }

    final submittedText = _messageController.text.trim();
    _messageController.clear();
    _inputFocusNode.unfocus();
    _startSendMessageText(submittedText);
  }

  /// Starts or stops local speech input from the chat action button.
  Future<void> _toggleSpeechInput() async {
    if (_isSpeechTranscribing) {
      return;
    }
    try {
      if (_isSpeechRecording) {
        await _finishSpeechInput();
      } else {
        await _startSpeechInput();
      }
    } catch (error, stackTrace) {
      ClientLogger.e(
        'speech input failed',
        tag: _localSttLogTag,
        error: error,
        stackTrace: stackTrace,
      );
      if (!mounted) {
        return;
      }
      _mutateChatContentData(() {
        _isSpeechRecording = false;
        _isSpeechTranscribing = false;
      });
      _showLocalToast(
        AppLocalizations.of(context)!.chatSpeechInputFailed('$error'),
      );
    }
  }

  /// Starts one WAV recording after validating the selected STT provider config.
  Future<void> _startSpeechInput() async {
    final selectedConfigId = await _viewModel
        .clients
        .preferencesSttConfigManager
        .getSelectedSttConfigId();
    if (!mounted) {
      return;
    }
    if (selectedConfigId == null) {
      _showLocalToast(
        AppLocalizations.of(context)!.chatSpeechInputConfigurationRequired,
      );
      return;
    }
    await _speechRecorder.start();
    if (!mounted) {
      return;
    }
    _mutateChatContentData(() {
      _isSpeechRecording = true;
    });
  }

  /// Stops recording, transcribes its bytes, and updates the current draft.
  Future<void> _finishSpeechInput() async {
    _mutateChatContentData(() {
      _isSpeechRecording = false;
      _isSpeechTranscribing = true;
    });
    final recordedAudio = await _speechRecorder.stop();
    try {
      final response = await _viewModel.clients.servicesSttRecognitionService
          .transcribeCurrent(
            audioBytes: recordedAudio.bytes,
            fileName: recordedAudio.fileName,
            contentType: recordedAudio.contentType,
            language: null,
          );
      final text = response.text.trim();
      if (text.isEmpty) {
        if (mounted) {
          _showLocalToast(
            AppLocalizations.of(context)!.chatSpeechNoTextRecognized,
          );
        }
        return;
      }
      _messageController.value = TextEditingValue(
        text: text,
        selection: TextSelection.collapsed(offset: text.length),
      );
      _inputFocusNode.requestFocus();
    } finally {
      if (mounted) {
        _mutateChatContentData(() {
          _isSpeechTranscribing = false;
        });
      }
    }
  }

  void _startSendMessageText(String text) {
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

  /// Schedules one automatic alignment with the latest message for this frame.
  void _scheduleScrollToBottom() {
    if (_isPreparingChatSwitch || !_autoScrollToBottom) {
      return;
    }
    if (_hasNewerDisplayHistory && !_isLoadingDisplayWindow) {
      unawaited(
        _viewModel.showLatestMessagesForCurrentChat().catchError((
          Object error,
          StackTrace stackTrace,
        ) {
          debugPrint('Failed to show latest messages: $error\n$stackTrace');
        }),
      );
      return;
    }
    if (_bottomScrollScheduled) {
      return;
    }
    _bottomScrollScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _bottomScrollScheduled = false;
      if (!mounted || !_scrollController.hasClients) {
        return;
      }
      final position = _scrollController.position;
      final target = position.maxScrollExtent;
      if ((target - position.pixels).abs() > 1) {
        _scrollController.jumpTo(target);
      }
    });
  }

  void _sendMessageAfterNextFrame(String text) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      _viewModel
          .sendUserMessage(text, replyToMessage: _replyToMessage)
          .then((_) async {
            _replyToMessage = null;
            await _refreshAttachments();
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
    if (index < 0 || index >= _messages.length) {
      return;
    }
    _showWorkspaceChangeConfirm(
      mode: WorkspaceChangeConfirmMode.rollback,
      index: index,
      onConfirm: () async {
        final draftText = await _viewModel.rollbackToMessage(index);
        if (draftText != null && mounted) {
          _messageController.value = TextEditingValue(
            text: draftText,
            selection: TextSelection.collapsed(offset: draftText.length),
          );
          _inputFocusNode.requestFocus();
        }
      },
    );
  }

  void _selectMessageToEdit(int index, ChatUiMessage message) {
    showDialog<void>(
      context: context,
      builder: (context) {
        return MessageEditorDialog(
          initialText: message.editableText,
          showResendButton: message.sender == 'user',
          onSave: (content) async {
            await _viewModel.updateMessage(index, content);
          },
          onResend: (content) async {
            if (_currentWorkspacePath != null &&
                _currentWorkspacePath!.trim().isNotEmpty) {
              await _showWorkspaceChangeConfirm(
                mode: WorkspaceChangeConfirmMode.editAndResend,
                index: index,
                onConfirm: () async {
                  await _viewModel.rewindAndResendMessage(index, content);
                },
              );
            } else {
              await _viewModel.rewindAndResendMessage(index, content);
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
    unawaited(
      _viewModel.insertSummary(message).catchError((
        Object error,
        StackTrace stackTrace,
      ) {
        debugPrint('Failed to insert summary: $error\n$stackTrace');
        return false;
      }),
    );
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
  }

  Future<void> _loadOlderDisplayWindow() async {
    await _viewModel.loadOlderMessagesForCurrentChat();
  }

  Future<void> _loadNewerDisplayWindow() async {
    await _viewModel.loadNewerMessagesForCurrentChat();
  }

  Future<void> _showLatestDisplayWindow() async {
    await _viewModel.showLatestMessagesForCurrentChat();
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
    if (widget.embedded) {
      return _buildChatContent();
    }
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
      onListWorkspaceBindingDirectories:
          _viewModel.listWorkspaceBindingDirectories,
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
          onRefreshRequested: _viewModel.showLatestMessagesForCurrentChat,
          isMultiSelectMode: data.isMultiSelectMode,
          selectedMessageIndices: data.selectedMessageIndices,
          isPreparingChatSwitch: data.isPreparingChatSwitch,
          isSpeechRecording: data.isSpeechRecording,
          isSpeechTranscribing: data.isSpeechTranscribing,
          onSpeechInput: _toggleSpeechInput,
          onSendMessage: _sendMessage,
          onQueueMessage: _enqueueDraftToPendingQueue,
          onCancelMessage: _cancelMessage,
          pendingQueueMessages: data.pendingQueueMessages,
          isPendingQueueExpanded: data.isPendingQueueExpanded,
          onPendingQueueExpandedChange: _setPendingQueueExpanded,
          onDeletePendingQueueMessage: _deletePendingQueueMessage,
          onEditPendingQueueMessage: _editPendingQueueMessage,
          onSendPendingQueueMessage: _sendPendingQueueMessage,
          attachments: data.attachments,
          onAttachImage: () {
            _handleAttachImage().catchError((
              Object error,
              StackTrace stackTrace,
            ) {
              debugPrint('Failed to attach image: $error\n$stackTrace');
              return null;
            });
          },
          onTakePhoto: _handleTakePhoto,
          onAttachMemory: _handleAttachMemory,
          onAttachFile: () {
            _handleAttachFile().catchError((
              Object error,
              StackTrace stackTrace,
            ) {
              debugPrint('Failed to attach file: $error\n$stackTrace');
              return null;
            });
          },
          onAttachFiles: (paths) {
            _handleAttachmentPaths(paths).catchError((
              Object error,
              StackTrace stackTrace,
            ) {
              debugPrint('Failed to attach dropped files: $error\n$stackTrace');
              return null;
            });
          },
          onAttachScreenContent: () {
            _handleSpecialAttachment('screen_capture').catchError((
              Object error,
              StackTrace stackTrace,
            ) {
              debugPrint(
                'Failed to attach screen content: $error\n$stackTrace',
              );
              return null;
            });
          },
          onAttachNotifications: () {
            _handleSpecialAttachment('notifications_capture').catchError((
              Object error,
              StackTrace stackTrace,
            ) {
              debugPrint('Failed to attach notifications: $error\n$stackTrace');
              return null;
            });
          },
          onAttachLocation: () {
            _handleSpecialAttachment('location_capture').catchError((
              Object error,
              StackTrace stackTrace,
            ) {
              debugPrint('Failed to attach location: $error\n$stackTrace');
              return null;
            });
          },
          onAttachPackage: (packageName) {
            _handleAttachPackage(packageName).catchError((
              Object error,
              StackTrace stackTrace,
            ) {
              debugPrint('Failed to attach package: $error\n$stackTrace');
              return null;
            });
          },
          onRemoveAttachment: (filePath) {
            _removeAttachment(filePath).catchError((
              Object error,
              StackTrace stackTrace,
            ) {
              debugPrint('Failed to remove attachment: $error\n$stackTrace');
              return null;
            });
          },
          onInsertAttachment: _insertAttachmentReference,
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
      onListWorkspaceBindingDirectories:
          _viewModel.listWorkspaceBindingDirectories,
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
    final previousContentData = _chatContentDataNotifier.value;
    _chatContentDataNotifier.value = _currentChatContentData(
      pendingQueueMessages: previousContentData.pendingQueueMessages,
      isPendingQueueExpanded: previousContentData.isPendingQueueExpanded,
    );
  }

  _ChatContentData _currentChatContentData({
    List<PendingQueueMessageItem> pendingQueueMessages =
        const <PendingQueueMessageItem>[],
    bool isPendingQueueExpanded = true,
  }) {
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
      pendingQueueMessages: List<PendingQueueMessageItem>.unmodifiable(
        pendingQueueMessages,
      ),
      isPendingQueueExpanded: isPendingQueueExpanded,
      attachments: List<AttachmentInfo>.unmodifiable(_attachments),
      isSpeechRecording: _isSpeechRecording,
      isSpeechTranscribing: _isSpeechTranscribing,
    );
  }

  Future<void> _createDefaultWorkspace(String? projectType) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      throw StateError('No current chat');
    }
    await _viewModel.createAndBindDefaultWorkspace(chatId, projectType);
  }

  Future<void> _bindWorkspace(String workspace) async {
    final chatId = _currentChatId;
    if (chatId == null) {
      throw StateError('No current chat');
    }
    await _viewModel.bindChatToWorkspace(chatId, workspace);
  }
}
