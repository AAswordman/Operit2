// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:file_selector/file_selector.dart';

import '../../../../core/logging/ClientLogger.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
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
import '../components/style/input/common/MentionSuggestionPanel.dart';
import '../components/style/input/common/MentionTokenUtils.dart';
import '../components/style/input/common/PendingQueueMessageItem.dart';
import '../components/workspace/WorkspaceLayoutMetrics.dart';
import '../components/workspace/WorkspaceTopBarButton.dart';
import '../speech/LocalSpeechRecorder.dart';
import '../viewmodel/ChatSelectionTransition.dart';
import '../viewmodel/ChatViewModel.dart';

bool _chatWorkspaceOpen = false;
const String _localSttLogTag = 'LocalSTT';
const String _packageAttachmentPrefix = 'package_attach:';
const String _workspaceMentionAttachmentPrefix = 'workspace_mention:';

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
  const AIChatScreen({super.key, this.viewModel});

  final ChatViewModel? viewModel;

  /// Builds the full chat surface owned by the main application host.
  @override
  Widget build(BuildContext context) {
    return _AIChatSurface(viewModel: viewModel, embedded: false);
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
    return _AIChatSurface(viewModel: viewModel, embedded: true);
  }
}

class _AIChatSurface extends StatefulWidget {
  /// Creates the shared implementation for a full chat screen or embedded chat.
  const _AIChatSurface({required this.viewModel, required this.embedded});

  final ChatViewModel? viewModel;
  final bool embedded;

  /// Creates the state shared by the full and embedded chat surfaces.
  @override
  State<_AIChatSurface> createState() => _AIChatSurfaceState();
}

final Map<String, Map<String?, TextEditingValue>> _chatInputDraftStores =
    <String, Map<String?, TextEditingValue>>{};

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
    required this.selectedMessageTimestamps,
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
  final core_proxy.InputProcessingState inputProcessingState;
  final String? currentChatId;
  final bool hasOlderDisplayHistory;
  final bool hasNewerDisplayHistory;
  final bool isLoadingDisplayWindow;
  final bool isMultiSelectMode;
  final Set<int> selectedMessageTimestamps;
  final String? currentCharacterCardAvatarUri;
  final bool isPreparingChatSwitch;
  final List<PendingQueueMessageItem> pendingQueueMessages;
  final bool isPendingQueueExpanded;
  final List<AttachmentInfo> attachments;
  final bool isSpeechRecording;
  final bool isSpeechTranscribing;
}

class _MentionDeletionNormalization {
  /// Creates the normalized input value and removed mention token metadata.
  const _MentionDeletionNormalization({
    required this.value,
    this.removedMentionToken,
  });

  final TextEditingValue value;
  final String? removedMentionToken;
}

class _AIChatSurfaceState extends State<_AIChatSurface> {
  late final ChatViewModel _viewModel = widget.viewModel ?? ChatViewModel();
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
  core_proxy.InputProcessingState _inputProcessingState =
      core_proxy.InputProcessingState.idle();
  String? _errorMessage;
  StreamSubscription<String?>? _currentChatIdSubscription;
  StreamSubscription<List<ChatUiMessage>>? _messagesSubscription;
  StreamSubscription<core_proxy.ChatState>? _chatStateSubscription;
  String? _requestedChatFlowChatId;
  int _chatFlowBindingGeneration = 0;
  StreamSubscription<String?>? _toastEventSubscription;
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
  Set<int> _selectedMessageTimestamps = const <int>{};
  bool _autoScrollToBottom = true;
  bool _hasOlderDisplayHistory = false;
  bool _hasNewerDisplayHistory = false;
  bool _isLoadingDisplayWindow = false;
  bool _isPreparingChatSwitch = false;
  String? _pendingChatSwitchTargetId;
  bool _bottomScrollScheduled = false;
  late bool _workspaceOpen;
  bool _isCurrentMainScreen = true;
  bool _topBarActionsUpdateScheduled = false;
  bool _pendingQueueEnqueueInFlight = false;
  bool _isApplyingChatDraft = false;
  bool _isSpeechRecording = false;
  bool _isSpeechTranscribing = false;
  bool _showMentionSuggestionPanel = false;
  String _mentionSearchQuery = '';
  String? _mentionSuggestionTriggerChar;

  /// Initializes chat state and subscriptions.
  @override
  void initState() {
    super.initState();
    _inputDraftsByChatId = _chatInputDraftStores.putIfAbsent(
      'main',
      () => <String?, TextEditingValue>{},
    );
    _chatContentDataNotifier = ValueNotifier<_ChatContentData>(
      _currentChatContentData(),
    );
    _autoScrollToBottomNotifier = ValueNotifier<bool>(_autoScrollToBottom);
    _toastMessageNotifier = ValueNotifier<String?>(_toastMessage);
    _workspaceOpen = _chatWorkspaceOpen;
    _watchChatFlows();
    _watchToastEvent();
    ChatSelectionTransition.requests.addListener(_onChatSelectionTransition);
    PendingChatDraftHandler.revision.addListener(_consumePendingChatDraft);
    _onChatSelectionTransition();
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
    ChatSelectionTransition.requests.removeListener(_onChatSelectionTransition);
    PendingChatDraftHandler.revision.removeListener(_consumePendingChatDraft);
    _messageController.removeListener(_onMessageControllerChanged);
    _messageController.dispose();
    _inputFocusNode.dispose();
    _scrollController.dispose();
    _chatContentDataNotifier.dispose();
    _autoScrollToBottomNotifier.dispose();
    _toastMessageNotifier.dispose();
    _currentChatIdSubscription?.cancel();
    _messagesSubscription?.cancel();
    _chatStateSubscription?.cancel();
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
    if (_isApplyingChatDraft) {
      _previousMessageInputValue = proposedValue;
      return;
    }
    final normalization = _normalizeMentionDeletion(
      previousValue,
      proposedValue,
    );
    final acceptedValue = normalization.value;
    if (acceptedValue != proposedValue) {
      _applyMessageControllerValue(acceptedValue);
    }
    _previousMessageInputValue = acceptedValue;
    final removedMentionToken = normalization.removedMentionToken;
    if (removedMentionToken != null &&
        !_hasMentionToken(acceptedValue.text, removedMentionToken)) {
      unawaited(
        _removeMentionAttachments(removedMentionToken).catchError((
          Object error,
          StackTrace stackTrace,
        ) {
          ClientLogger.e(
            'Failed to remove mention attachments',
            tag: 'AIChatScreen',
            error: error,
            stackTrace: stackTrace,
          );
        }),
      );
    }
    final settings = UserPreferencesManager.longPastedTextInputSettings.value;
    final insertedText = _insertedTextFromInputChange(
      previousValue: previousValue,
      proposedValue: acceptedValue,
    );
    if (settings.enabled &&
        insertedText != null &&
        insertedText.runes.length > settings.threshold) {
      unawaited(
        _convertLongPastedText(
          previousValue: previousValue,
          proposedValue: acceptedValue,
          chatId: _currentChatId,
        ),
      );
      return;
    }
    _recordMessageInputChange(acceptedValue);
  }

  /// Persists and dispatches one accepted message input editing value.
  void _recordMessageInputChange(TextEditingValue value) {
    _saveInputDraft(value);
    _updateMentionSuggestionState(value);
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

  /// Applies a controller value without re-entering accepted input processing.
  void _applyMessageControllerValue(TextEditingValue value) {
    _isApplyingChatDraft = true;
    try {
      _messageController.value = value;
    } finally {
      _isApplyingChatDraft = false;
    }
  }

  /// Expands a single backspace at a committed mention into full-token deletion.
  _MentionDeletionNormalization _normalizeMentionDeletion(
    TextEditingValue previous,
    TextEditingValue proposed,
  ) {
    if (!previous.selection.isValid ||
        !proposed.selection.isValid ||
        previous.selection.start != previous.selection.end ||
        proposed.selection.start != proposed.selection.end ||
        previous.text.length != proposed.text.length + 1) {
      return _MentionDeletionNormalization(value: proposed);
    }

    final oldCursor = _clampedTextOffset(
      previous.selection.start,
      previous.text.length,
    );
    final newCursor = _clampedTextOffset(
      proposed.selection.start,
      proposed.text.length,
    );
    if (newCursor != oldCursor - 1) {
      return _MentionDeletionNormalization(value: proposed);
    }

    final expectedText = previous.text.replaceRange(newCursor, oldCursor, '');
    if (expectedText != proposed.text) {
      return _MentionDeletionNormalization(value: proposed);
    }

    final mentionToken = findMentionTokenEndingAtCursor(
      previous.text,
      oldCursor,
    );
    if (mentionToken == null) {
      return _MentionDeletionNormalization(value: proposed);
    }

    final removedMentionToken = previous.text
        .substring(mentionToken.start + 1, mentionToken.contentEndExclusive)
        .trim();
    final normalizedText = previous.text.replaceRange(
      mentionToken.start,
      mentionToken.endExclusive,
      '',
    );
    return _MentionDeletionNormalization(
      value: proposed.copyWith(
        text: normalizedText,
        selection: TextSelection.collapsed(offset: mentionToken.start),
        composing: TextRange.empty,
      ),
      removedMentionToken: removedMentionToken.isEmpty
          ? null
          : removedMentionToken,
    );
  }

  /// Clamps an editing offset into the available text range.
  int _clampedTextOffset(int offset, int textLength) {
    if (offset < 0) {
      return 0;
    }
    if (offset > textLength) {
      return textLength;
    }
    return offset;
  }

  /// Updates the visible mention suggestion state from the accepted input.
  void _updateMentionSuggestionState(TextEditingValue value) {
    final activeMention = findActiveMentionTrigger(value);
    if (activeMention == null) {
      _clearMentionSuggestionState();
      return;
    }
    final changed =
        !_showMentionSuggestionPanel ||
        _mentionSuggestionTriggerChar != activeMention.triggerChar ||
        _mentionSearchQuery != activeMention.query;
    if (!changed) {
      return;
    }
    if (!mounted) {
      _showMentionSuggestionPanel = true;
      _mentionSuggestionTriggerChar = activeMention.triggerChar;
      _mentionSearchQuery = activeMention.query;
      return;
    }
    setState(() {
      _showMentionSuggestionPanel = true;
      _mentionSuggestionTriggerChar = activeMention.triggerChar;
      _mentionSearchQuery = activeMention.query;
    });
  }

  /// Clears the active mention suggestion state.
  void _clearMentionSuggestionState() {
    if (!_showMentionSuggestionPanel &&
        _mentionSuggestionTriggerChar == null &&
        _mentionSearchQuery.isEmpty) {
      return;
    }
    if (!mounted) {
      _showMentionSuggestionPanel = false;
      _mentionSuggestionTriggerChar = null;
      _mentionSearchQuery = '';
      return;
    }
    setState(() {
      _showMentionSuggestionPanel = false;
      _mentionSuggestionTriggerChar = null;
      _mentionSearchQuery = '';
    });
  }

  /// Replaces the active mention token with the selected token text.
  bool _replaceCurrentMentionToken(String token) {
    final trimmedToken = token.trim();
    if (trimmedToken.isEmpty) {
      return false;
    }

    final current = _messageController.value;
    final activeMention = findActiveMentionTrigger(current);
    if (activeMention == null) {
      return false;
    }
    final text = current.text;
    final cursor = _clampedTextOffset(current.selection.start, text.length);
    final before = text.substring(0, activeMention.triggerIndex);
    final after = text.substring(cursor);
    final insertion = StringBuffer()
      ..write(activeMention.triggerChar)
      ..write(trimmedToken);
    if (after.isEmpty || !isMentionWhitespace(after.substring(0, 1))) {
      insertion.write(' ');
    }
    final insertedText = insertion.toString();
    final nextText = before + insertedText + after;
    _messageController.value = TextEditingValue(
      text: nextText,
      selection: TextSelection.collapsed(
        offset: before.length + insertedText.length,
      ),
      composing: TextRange.empty,
    );
    _inputFocusNode.requestFocus();
    return true;
  }

  /// Selects a package suggestion from the active mention panel.
  Future<void> _selectMentionPackage(String packageName) async {
    final trimmedPackageName = packageName.trim();
    if (trimmedPackageName.isEmpty) {
      return;
    }
    if (!_replaceCurrentMentionToken(trimmedPackageName)) {
      return;
    }
    _clearMentionSuggestionState();
    final attachmentPath = '$_packageAttachmentPrefix$trimmedPackageName';
    await _handleSpecialAttachment(attachmentPath);
    if (!_hasMentionToken(_messageController.text, trimmedPackageName)) {
      await _removeAttachment(attachmentPath);
    }
  }

  /// Selects a workspace entry suggestion from the active mention panel.
  Future<void> _selectMentionWorkspaceEntry(String relativePath) async {
    final normalizedRelativePath = _normalizeWorkspaceMentionPath(relativePath);
    if (normalizedRelativePath.isEmpty) {
      return;
    }
    if (!_replaceCurrentMentionToken(normalizedRelativePath)) {
      return;
    }
    _clearMentionSuggestionState();
    final attachmentPath =
        '$_workspaceMentionAttachmentPrefix$normalizedRelativePath';
    await _handleSpecialAttachment(attachmentPath);
    if (!_hasMentionToken(_messageController.text, normalizedRelativePath)) {
      await _removeAttachment(attachmentPath);
    }
  }

  /// Removes package and workspace attachments linked to one mention token.
  Future<void> _removeMentionAttachments(String token) async {
    final trimmedToken = token.trim();
    if (trimmedToken.isEmpty) {
      return;
    }
    await _viewModel.removeAttachment('$_packageAttachmentPrefix$trimmedToken');
    await _viewModel.removeAttachment(
      '$_workspaceMentionAttachmentPrefix$trimmedToken',
    );
    await _refreshAttachments();
  }

  /// Reports whether the text still has an exact mention token.
  bool _hasMentionToken(String text, String token) {
    final trimmedToken = token.trim();
    if (trimmedToken.isEmpty) {
      return false;
    }
    for (final mentionToken in findMentionTokens(text)) {
      final value = text.substring(
        mentionToken.start + 1,
        mentionToken.contentEndExclusive,
      );
      if (value == trimmedToken) {
        return true;
      }
    }
    return false;
  }

  /// Normalizes a workspace mention path for the input token and attachment id.
  String _normalizeWorkspaceMentionPath(String relativePath) {
    var normalizedPath = relativePath.trim().replaceAll(r'\', '/');
    while (normalizedPath.startsWith('/')) {
      normalizedPath = normalizedPath.substring(1);
    }
    while (normalizedPath.endsWith('/')) {
      normalizedPath = normalizedPath.substring(0, normalizedPath.length - 1);
    }
    return normalizedPath;
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
    _showMentionSuggestionPanel = false;
    _mentionSuggestionTriggerChar = null;
    _mentionSearchQuery = '';
  }

  bool get _isQueueBlocked {
    return _loading || _inputProcessingState.isProcessing;
  }

  /// Requests a Rust-owned automatic dequeue after the active chat becomes ready.
  void _syncPendingQueueAfterSnapshot() {
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
        _clearMentionSuggestionState();
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

  /// Opens the independent chat-scoped message and state streams for this surface.
  void _watchChatFlows() {
    _currentChatIdSubscription?.cancel();
    _currentChatIdSubscription = _viewModel.watchCurrentChatId().listen(
      _bindChatFlows,
      onError: _handleChatFlowError,
    );
  }

  /// Rebinds the two per-chat Core watches after the selected chat changes.
  void _bindChatFlows(String? chatId) {
    if (chatId == _requestedChatFlowChatId) {
      return;
    }
    _requestedChatFlowChatId = chatId;
    final generation = ++_chatFlowBindingGeneration;
    unawaited(_rebindChatFlows(chatId, generation));
  }

  /// Cancels the previous chat watches before opening the requested pair.
  Future<void> _rebindChatFlows(String? chatId, int generation) async {
    final previousMessagesSubscription = _messagesSubscription;
    final previousChatStateSubscription = _chatStateSubscription;
    _messagesSubscription = null;
    _chatStateSubscription = null;
    // Native watch shutdown waits for the routed stream task to observe its
    // close signal.  It must not delay opening the replacement chat watches.
    if (previousMessagesSubscription != null) {
      unawaited(
        previousMessagesSubscription.cancel().catchError((Object error) {
          ClientLogger.e(
            'Failed to close previous chat message watch',
            tag: 'AIChatScreen',
            error: error,
          );
        }),
      );
    }
    if (previousChatStateSubscription != null) {
      unawaited(
        previousChatStateSubscription.cancel().catchError((Object error) {
          ClientLogger.e(
            'Failed to close previous chat state watch',
            tag: 'AIChatScreen',
            error: error,
          );
        }),
      );
    }
    if (!mounted || generation != _chatFlowBindingGeneration) {
      return;
    }
    if (chatId == null || chatId.isEmpty) {
      return;
    }
    _messagesSubscription = _viewModel
        .watchMessages(chatId)
        .listen(
          (messages) {
            if (generation == _chatFlowBindingGeneration &&
                _requestedChatFlowChatId == chatId) {
              _applyMessages(messages);
            }
          },
          onError: (Object error, StackTrace stackTrace) {
            _handleBoundChatFlowError(
              error: error,
              stackTrace: stackTrace,
              chatId: chatId,
              generation: generation,
            );
          },
        );
    _chatStateSubscription = _viewModel
        .watchChatState(chatId)
        .listen(
          (state) {
            if (generation == _chatFlowBindingGeneration &&
                _requestedChatFlowChatId == chatId &&
                state.currentChatId == chatId) {
              _applyChatState(state);
            }
          },
          onError: (Object error, StackTrace stackTrace) {
            _handleBoundChatFlowError(
              error: error,
              stackTrace: stackTrace,
              chatId: chatId,
              generation: generation,
            );
          },
        );
  }

  /// Applies a message-window change without changing chat execution state.
  void _applyMessages(List<ChatUiMessage> messages) {
    if (!mounted) {
      return;
    }
    _errorMessage = null;
    _messages
      ..clear()
      ..addAll(messages);
    if (_selectedMessageTimestamps.isNotEmpty) {
      final selectableTimestamps = messages
          .where((message) {
            final sender = message.sender;
            return sender == 'user' || sender == 'ai';
          })
          .map((message) => message.timestamp)
          .toSet();
      _selectedMessageTimestamps = _selectedMessageTimestamps
          .where(selectableTimestamps.contains)
          .toSet();
    }
    _publishChatContentData();
    _scheduleScrollToBottom();
  }

  /// Applies a locally requested visual transition before routed Core state arrives.
  void _onChatSelectionTransition() {
    final request = ChatSelectionTransition.requests.value;
    if (request == null) {
      if (_isPreparingChatSwitch) {
        _pendingChatSwitchTargetId = null;
        _mutateChatContentData(() {
          _isPreparingChatSwitch = false;
        });
      }
      return;
    }
    if (request.chatId == _currentChatId && !_isPreparingChatSwitch) {
      ChatSelectionTransition.complete(request.chatId);
      return;
    }
    _pendingChatSwitchTargetId = request.chatId;
    _setAutoScrollToBottom(true);
    _mutateChatContentData(() {
      _errorMessage = null;
      _isPreparingChatSwitch = true;
      _isMultiSelectMode = false;
      _selectedMessageTimestamps = const <int>{};
    });
  }

  /// Applies one routed chat-state change without replacing the message window.
  void _applyChatState(core_proxy.ChatState state) {
    if (!mounted) {
      return;
    }
    final chatChanged = _currentChatId != state.currentChatId;
    final workspaceChanged =
        _currentWorkspacePath != state.currentWorkspacePath;
    if (chatChanged) {
      _saveCurrentInputDraft();
      _currentChatId = state.currentChatId;
      _isMultiSelectMode = false;
      _selectedMessageTimestamps = const <int>{};
      _restoreInputDraftForChat(state.currentChatId);
    }
    _errorMessage = null;
    _currentChatTitle = state.currentChatTitle;
    _currentCharacterCardName = state.currentCharacterCardName;
    _currentCharacterCardAvatarUri = state.currentCharacterCardAvatarUri;
    _currentWorkspacePath = state.currentWorkspacePath;
    _loading = state.isLoading;
    _inputProcessingState = state.inputProcessingState;
    _hasOlderDisplayHistory = state.hasOlderDisplayHistory;
    _hasNewerDisplayHistory = state.hasNewerDisplayHistory;
    _isLoadingDisplayWindow = state.isLoadingDisplayWindow;
    if (_pendingChatSwitchTargetId == state.currentChatId) {
      _pendingChatSwitchTargetId = null;
      _isPreparingChatSwitch = false;
      ChatSelectionTransition.complete(state.currentChatId);
    }
    _publishChatContentData();
    _updateTopBarTitle();
    if (workspaceChanged && mounted) {
      setState(() {});
      _mainLayoutController?.refreshAttachment(owner: _mainLayoutOwner);
    }
    _syncPendingQueueAfterSnapshot();
  }

  /// Surfaces an unrecoverable routed-flow failure to this chat surface.
  void _handleChatFlowError(Object error, StackTrace stackTrace) {
    ClientLogger.e(
      'chat routed flow failed',
      tag: 'AIChatScreen',
      error: error,
      stackTrace: stackTrace,
    );
    if (!mounted) {
      return;
    }
    _errorMessage = error.toString();
    _loading = false;
    _publishChatContentData();
  }

  /// Surfaces an error only while its chat-flow binding remains current.
  void _handleBoundChatFlowError({
    required Object error,
    required StackTrace stackTrace,
    required String chatId,
    required int generation,
  }) {
    if (!mounted ||
        generation != _chatFlowBindingGeneration ||
        _requestedChatFlowChatId != chatId) {
      return;
    }
    _handleChatFlowError(error, stackTrace);
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
      _clearMentionSuggestionState();
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

    final chatId = _currentChatId;
    final inputValue = _messageController.value;
    final decision = await _viewModel.dispatchChatInputSubmitRequested(
      chatId: chatId,
      text: text,
      selectionStart: inputValue.selection.start,
      selectionEnd: inputValue.selection.end,
      attachmentCount: _attachments.length,
    );
    if (!mounted || _currentChatId != chatId) {
      return;
    }
    if (decision != null) {
      final timeoutMessage = decision.message;
      if (decision.timedOut && timeoutMessage != null) {
        _showLocalToast(timeoutMessage);
      }
      if (decision.action == 'block' || decision.action == 'consume') {
        if (decision.action == 'consume' && decision.clearInput) {
          _clearMentionSuggestionState();
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
    _clearMentionSuggestionState();
    _messageController.clear();
    _inputFocusNode.unfocus();
    _startSendMessageText(submittedText, chatId!);
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

  /// Starts one send while retaining the chat id that accepted the submit hook.
  void _startSendMessageText(String text, String chatId) {
    if (_currentChatId != chatId) {
      return;
    }
    _mutateChatContentData(() {
      _autoScrollToBottom = true;
      _autoScrollToBottomNotifier.value = true;
      _errorMessage = null;
      _loading = true;
      _inputProcessingState = core_proxy.InputProcessingState.processing(
        message: 'message_processing',
      );
    });
    _scheduleScrollToBottom();
    _sendMessageAfterNextFrame(text, chatId);
  }

  /// Schedules one automatic alignment with the latest message for this frame.
  void _scheduleScrollToBottom() {
    if (!_autoScrollToBottom) {
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

  /// Sends the submitted text after layout has accepted the optimistic UI state.
  void _sendMessageAfterNextFrame(String text, String chatId) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      _viewModel
          .sendUserMessage(
            text,
            replyToMessage: _replyToMessage,
            chatIdOverride: chatId,
          )
          .then((_) async {
            if (!mounted || _currentChatId != chatId) {
              return null;
            }
            _replyToMessage = null;
            await _refreshAttachments();
            return null;
          })
          .catchError((Object error, StackTrace stackTrace) {
            debugPrint('Failed to send chat message: $error\n$stackTrace');
            if (!mounted) {
              return null;
            }
            if (_currentChatId != chatId) {
              return null;
            }
            _mutateChatContentData(() {
              _errorMessage = error.toString();
              _loading = false;
              _inputProcessingState = core_proxy.InputProcessingState.error(
                message: error.toString(),
              );
            });
            return null;
          });
    });
  }

  /// Cancels the active turn for the selected chat.
  void _cancelMessage() {
    final chatId = _currentChatId;
    if (chatId == null ||
        _isPreparingChatSwitch ||
        _requestedChatFlowChatId != chatId) {
      return;
    }
    _viewModel.cancelMessage(chatId).catchError((
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
  }

  /// Returns the active chat identifier for a message action.
  String? _activeChatIdForMessageAction() {
    final chatId = _currentChatId;
    if (chatId == null || chatId.trim().isEmpty) {
      return null;
    }
    return chatId;
  }

  /// Deletes one message identified by its stable timestamp.
  Future<void> _deleteMessage(int messageTimestamp) async {
    final chatId = _activeChatIdForMessageAction();
    if (chatId == null) {
      return;
    }
    await _viewModel.deleteMessage(chatId, messageTimestamp);
  }

  /// Deletes the selected message and all following messages in one chat.
  Future<bool> _deleteMessagesFrom(int messageTimestamp) async {
    final chatId = _activeChatIdForMessageAction();
    if (chatId == null) {
      return false;
    }
    return _viewModel.deleteMessagesFrom(chatId, messageTimestamp);
  }

  Future<void> _deleteMessageVariant(int timestamp, int variantIndex) async {
    await _viewModel.deleteMessageVariant(timestamp, variantIndex);
  }

  /// Selects the displayed response variant for a message.
  Future<void> _selectMessageVariant(
    int timestamp,
    int selectedVariantIndex,
  ) async {
    await _viewModel.selectMessageVariant(timestamp, selectedVariantIndex);
  }

  /// Requests a workspace rollback anchored to a message timestamp.
  void _requestRollbackToMessage(int messageTimestamp) {
    final chatId = _activeChatIdForMessageAction();
    if (chatId == null) {
      return;
    }
    _showWorkspaceChangeConfirm(
      mode: WorkspaceChangeConfirmMode.rollback,
      chatId: chatId,
      messageTimestamp: messageTimestamp,
      onConfirm: () async {
        final draftText = await _viewModel.rollbackToMessage(
          chatId,
          messageTimestamp,
        );
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

  /// Opens the editor for a message identified by its timestamp.
  void _selectMessageToEdit(ChatUiMessage message) {
    final chatId = _activeChatIdForMessageAction();
    if (chatId == null) {
      return;
    }
    showDialog<void>(
      context: context,
      builder: (context) {
        return MessageEditorDialog(
          initialText: message.editableText,
          showResendButton: message.sender == 'user',
          onSave: (content) async {
            await _viewModel.updateMessage(chatId, message.timestamp, content);
          },
          onResend: (content) async {
            if (_currentWorkspacePath != null &&
                _currentWorkspacePath!.trim().isNotEmpty) {
              await _showWorkspaceChangeConfirm(
                mode: WorkspaceChangeConfirmMode.editAndResend,
                chatId: chatId,
                messageTimestamp: message.timestamp,
                onConfirm: () async {
                  await _viewModel.rewindAndResendMessage(
                    chatId,
                    message.timestamp,
                    content,
                  );
                },
              );
            } else {
              await _viewModel.rewindAndResendMessage(
                chatId,
                message.timestamp,
                content,
              );
            }
          },
        );
      },
    );
  }

  Future<void> _showWorkspaceChangeConfirm({
    required WorkspaceChangeConfirmMode mode,
    required String chatId,
    required int messageTimestamp,
    required Future<void> Function() onConfirm,
  }) async {
    final changes = await _viewModel.previewWorkspaceChangesForMessage(
      chatId,
      messageTimestamp,
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

  /// Regenerates one AI message identified by its timestamp.
  Future<void> _regenerateMessage(int messageTimestamp) async {
    final chatId = _activeChatIdForMessageAction();
    if (chatId == null) {
      return;
    }
    await _viewModel.regenerateSingleAiMessage(chatId, messageTimestamp);
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

  /// Starts multi-select mode with one message timestamp selected.
  void _toggleMultiSelectMode(int messageTimestamp) {
    _mutateChatContentData(() {
      _isMultiSelectMode = true;
      _selectedMessageTimestamps = <int>{messageTimestamp};
    });
  }

  /// Toggles one message timestamp in the current selection.
  void _toggleMessageSelection(int messageTimestamp) {
    _mutateChatContentData(() {
      final next = Set<int>.of(_selectedMessageTimestamps);
      if (next.contains(messageTimestamp)) {
        next.remove(messageTimestamp);
      } else {
        next.add(messageTimestamp);
      }
      _selectedMessageTimestamps = next;
    });
  }

  void _exitMultiSelectMode() {
    _mutateChatContentData(() {
      _isMultiSelectMode = false;
      _selectedMessageTimestamps = const <int>{};
    });
  }

  void _clearMessageSelection() {
    _mutateChatContentData(() {
      _selectedMessageTimestamps = const <int>{};
    });
  }

  void _selectAllMessages() {
    _mutateChatContentData(() {
      _isMultiSelectMode = true;
      _selectedMessageTimestamps = _messages
          .where((message) {
            final sender = message.sender;
            return sender == 'user' || sender == 'ai';
          })
          .map((message) => message.timestamp)
          .toSet();
    });
  }

  Future<void> _deleteSelectedMessages() async {
    final messageTimestamps = Set<int>.of(_selectedMessageTimestamps);
    if (messageTimestamps.isEmpty) {
      return;
    }
    final chatId = _activeChatIdForMessageAction();
    if (chatId == null) {
      return;
    }
    await _viewModel.deleteMessages(chatId, messageTimestamps);
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

  /// Builds the active mention suggestion panel for the current input value.
  Widget? _buildMentionSuggestionPanel() {
    final triggerChar = _mentionSuggestionTriggerChar;
    if (!_showMentionSuggestionPanel || triggerChar == null) {
      return null;
    }
    return MentionSuggestionPanel(
      viewModel: _viewModel,
      searchQuery: _mentionSearchQuery,
      triggerChar: triggerChar,
      hasBoundWorkspace: _currentWorkspacePath?.trim().isNotEmpty == true,
      onPackageSelected: (packageName) {
        unawaited(
          _selectMentionPackage(packageName).catchError((
            Object error,
            StackTrace stackTrace,
          ) {
            ClientLogger.e(
              'Failed to select mention package',
              tag: 'AIChatScreen',
              error: error,
              stackTrace: stackTrace,
            );
          }),
        );
      },
      onFileSelected: (relativePath) {
        unawaited(
          _selectMentionWorkspaceEntry(relativePath).catchError((
            Object error,
            StackTrace stackTrace,
          ) {
            ClientLogger.e(
              'Failed to select mention workspace entry',
              tag: 'AIChatScreen',
              error: error,
              stackTrace: stackTrace,
            );
          }),
        );
      },
    );
  }

  /// Builds the chat content with the current message input overlays.
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
          mentionSuggestionPanel: _buildMentionSuggestionPanel(),
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
          onSelectMessageVariant: _selectMessageVariant,
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
          selectedMessageTimestamps: data.selectedMessageTimestamps,
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
      selectedMessageTimestamps: _selectedMessageTimestamps,
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
