// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

import '../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../theme/OperitTheme.dart';
import '../viewmodel/ChatViewModel.dart';
import 'ChatLayoutMetrics.dart';
import 'MessageContextMenu.dart';
import 'MessageCopyPreview.dart';
import 'ChatScrollNavigator.dart';
import 'style/bubble/BubbleStyleChatMessage.dart';
import 'style/bubble/BubbleSurface.dart';
import 'style/cursor/CursorStyleChatMessage.dart';

const Duration _navigatorHideDelay = Duration(milliseconds: 1200);
const Duration _viewportResizeSettleDelay = Duration(milliseconds: 120);
const Duration _messageJumpRetryDelay = Duration(milliseconds: 90);
const Duration _messageJumpSettleDelay = Duration(milliseconds: 280);
const double _messageJumpPositionTolerance = 2;

class ChatArea extends StatefulWidget {
  const ChatArea({
    super.key,
    required this.messages,
    required this.isLoading,
    required this.errorMessage,
    required this.scrollController,
    required this.currentChatId,
    required this.currentCharacterCardAvatarUri,
    required this.autoScrollToBottomListenable,
    required this.hasOlderDisplayHistory,
    required this.hasNewerDisplayHistory,
    required this.isLoadingDisplayWindow,
    required this.loadLocatorEntries,
    required this.onRevealMessageForLocator,
    required this.onAutoScrollToBottomChanged,
    required this.onLoadOlderDisplayWindow,
    required this.onLoadNewerDisplayWindow,
    required this.onShowLatestDisplayWindow,
    required this.onToggleFavoriteMessage,
    required this.onDeleteMessage,
    required this.onDeleteMessagesFrom,
    required this.onDeleteMessageVariant,
    required this.onSelectMessageVariant,
    required this.onRollbackToMessage,
    required this.onSelectMessageToEdit,
    required this.onRegenerateMessage,
    required this.onInsertSummary,
    required this.onCreateBranch,
    required this.onReplyToMessage,
    required this.onPlayVoice,
    required this.onToggleMultiSelectMode,
    required this.onToggleMessageSelection,
    required this.onRefreshRequested,
    required this.bottomContentInset,
    this.splitMarkdownContent,
    this.isMultiSelectMode = false,
    this.selectedMessageTimestamps = const <int>{},
  });

  final List<ChatUiMessage> messages;
  final bool isLoading;
  final String? errorMessage;
  final ScrollController scrollController;
  final String? currentChatId;
  final String? currentCharacterCardAvatarUri;
  final ValueListenable<bool> autoScrollToBottomListenable;
  final bool hasOlderDisplayHistory;
  final bool hasNewerDisplayHistory;
  final bool isLoadingDisplayWindow;
  final LoadMessageLocatorEntries loadLocatorEntries;
  final RevealMessageForLocator onRevealMessageForLocator;
  final ValueChanged<bool> onAutoScrollToBottomChanged;
  final Future<void> Function() onLoadOlderDisplayWindow;
  final Future<void> Function() onLoadNewerDisplayWindow;
  final Future<void> Function() onShowLatestDisplayWindow;
  final ToggleFavoriteMessage onToggleFavoriteMessage;
  final MessageTimestampAction onDeleteMessage;
  final MessageTimestampBoolAction onDeleteMessagesFrom;
  final MessageVariantAction onDeleteMessageVariant;
  final MessageVariantAction onSelectMessageVariant;
  final MessageTimestampSelectionAction onRollbackToMessage;
  final MessageSelectionAction onSelectMessageToEdit;
  final MessageTimestampAction onRegenerateMessage;
  final ValueChanged<ChatUiMessage> onInsertSummary;
  final MessageTimestampAction onCreateBranch;
  final ValueChanged<ChatUiMessage> onReplyToMessage;
  final MessageVoiceAction onPlayVoice;
  final MessageTimestampSelectionAction onToggleMultiSelectMode;
  final MessageTimestampSelectionAction onToggleMessageSelection;
  final Future<void> Function() onRefreshRequested;
  final double bottomContentInset;
  final MarkdownCopySplitter? splitMarkdownContent;
  final bool isMultiSelectMode;
  final Set<int> selectedMessageTimestamps;

  @override
  State<ChatArea> createState() => _ChatAreaState();
}

class _ChatAreaState extends State<ChatArea> {
  final GlobalKey _viewportKey = GlobalKey();
  final Map<int, GlobalKey> _messageKeys = <int, GlobalKey>{};
  final ValueNotifier<Map<int, ChatScrollMessageAnchor>>
  _messageAnchorsNotifier = ValueNotifier<Map<int, ChatScrollMessageAnchor>>(
    const <int, ChatScrollMessageAnchor>{},
  );
  final ValueNotifier<bool> _showNavigatorChipNotifier = ValueNotifier<bool>(
    false,
  );
  final Map<int, _CachedMessageRow> _messageRowCache =
      <int, _CachedMessageRow>{};
  Timer? _navigatorHideTimer;
  Timer? _viewportResizeTimer;
  bool _userScrollSessionActive = false;
  bool _userScrollsTowardHistory = false;
  bool _messageAnchorCollectionScheduled = false;
  double _viewportHeight = 0;
  double _scrollViewportDimension = 0;
  bool _bottomFollowScheduled = false;
  int? _pendingJumpToMessageTimestamp;
  Timer? _pendingMessageJumpTimer;
  double? _lastEstimatedPendingJumpOffset;
  bool _pendingMessageJumpStabilityCheckRequested = false;
  bool _pendingMessageJumpScheduled = false;
  bool _pendingMessageJumpInFlight = false;

  /// Builds the scrollable message area and its navigation overlay.
  @override
  Widget build(BuildContext context) {
    final showLoadingIndicator = _shouldShowLoadingIndicator();
    final itemCount =
        widget.messages.length +
        (widget.hasOlderDisplayHistory ? 1 : 0) +
        (widget.hasNewerDisplayHistory ? 1 : 0) +
        (showLoadingIndicator || widget.errorMessage != null ? 1 : 0);

    if (itemCount == 0) {
      return const _EmptyChatArea();
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewportHeight = constraints.maxHeight;
        if (_viewportHeight != viewportHeight) {
          _viewportHeight = viewportHeight;
          _scheduleViewportResizeUpdate();
        }
        final messageStartIndex = widget.hasOlderDisplayHistory ? 1 : 0;
        final messageEndIndex = messageStartIndex + widget.messages.length;
        return Stack(
          key: _viewportKey,
          children: <Widget>[
            NotificationListener<SizeChangedLayoutNotification>(
              onNotification: _handleSizeChangedLayoutNotification,
              child: NotificationListener<ScrollMetricsNotification>(
                onNotification: _handleScrollMetricsNotification,
                child: NotificationListener<ScrollNotification>(
                  onNotification: _handleScrollNotification,
                  child: ListView.builder(
                    controller: widget.scrollController,
                    padding: EdgeInsets.fromLTRB(
                      16,
                      16,
                      16,
                      16 + widget.bottomContentInset,
                    ),
                    itemCount: itemCount,
                    itemBuilder: (context, index) {
                      late final Widget child;
                      if (widget.hasOlderDisplayHistory && index == 0) {
                        child = _DisplayWindowAction(
                          text: 'Load more history',
                          isLoading: widget.isLoadingDisplayWindow,
                          onTap: () {
                            widget.onAutoScrollToBottomChanged(false);
                            if (!widget.isLoadingDisplayWindow) {
                              widget.onLoadOlderDisplayWindow();
                            }
                          },
                        );
                      } else if (index >= messageStartIndex &&
                          index < messageEndIndex) {
                        final message =
                            widget.messages[index - messageStartIndex];
                        final messageIndex = index - messageStartIndex;
                        child = _messageRowFor(messageIndex, message);
                      } else if (widget.hasNewerDisplayHistory &&
                          index == messageEndIndex) {
                        child = _DisplayWindowAction(
                          text: 'Load newer history',
                          isLoading: widget.isLoadingDisplayWindow,
                          onTap: () {
                            if (!widget.isLoadingDisplayWindow) {
                              widget.onLoadNewerDisplayWindow();
                            }
                          },
                        );
                      } else if (widget.errorMessage != null) {
                        child = _StatusMessage(
                          text: widget.errorMessage!,
                          isError: true,
                        );
                      } else {
                        child = const Padding(
                          padding: EdgeInsets.only(left: 16, top: 2, bottom: 2),
                          child: StreamingCursor(),
                        );
                      }
                      return Padding(
                        padding: EdgeInsets.only(
                          bottom: index == itemCount - 1 ? 0 : 8,
                        ),
                        child: SizeChangedLayoutNotifier(
                          key: _rowKeyForIndex(
                            index,
                            messageStartIndex,
                            messageEndIndex,
                          ),
                          child: _ChatAreaContentColumn(child: child),
                        ),
                      );
                    },
                  ),
                ),
              ),
            ),
            ValueListenableBuilder<Map<int, ChatScrollMessageAnchor>>(
              valueListenable: _messageAnchorsNotifier,
              builder: (context, messageAnchors, _) {
                return ValueListenableBuilder<bool>(
                  valueListenable: widget.autoScrollToBottomListenable,
                  builder: (context, autoScrollToBottom, _) {
                    return ValueListenableBuilder<bool>(
                      valueListenable: _showNavigatorChipNotifier,
                      builder: (context, showNavigatorChip, _) {
                        return ChatScrollNavigator(
                          messages: widget.messages,
                          currentChatId: widget.currentChatId,
                          scrollController: widget.scrollController,
                          messageAnchors: messageAnchors,
                          viewportHeight: _viewportHeight,
                          autoScrollToBottom: autoScrollToBottom,
                          hasNewerDisplayHistory: widget.hasNewerDisplayHistory,
                          loadLocatorEntries: widget.loadLocatorEntries,
                          onRequestLatestMessages:
                              widget.onShowLatestDisplayWindow,
                          onAutoScrollToBottomChanged:
                              widget.onAutoScrollToBottomChanged,
                          onJumpToMessageTimestamp: _jumpToMessageTimestamp,
                          onJumpToMessage: _jumpToMessageIndex,
                          onToggleFavoriteMessage:
                              widget.onToggleFavoriteMessage,
                          onRequestScrollToBottom: _scrollToBottomFromNavigator,
                          showNavigatorChip: showNavigatorChip,
                          onNavigatorChipHidden: () {
                            _showNavigatorChipNotifier.value = false;
                            _userScrollSessionActive = false;
                          },
                        );
                      },
                    );
                  },
                );
              },
            ),
          ],
        );
      },
    );
  }

  /// Keeps the chat bottom aligned while the viewport changes size.
  bool _handleScrollMetricsNotification(
    ScrollMetricsNotification notification,
  ) {
    final viewportDimension = notification.metrics.viewportDimension;
    if (_scrollViewportDimension != viewportDimension) {
      _scrollViewportDimension = viewportDimension;
      _scheduleViewportResizeUpdate();
      _scheduleBottomFollow();
      return false;
    }
    _scheduleMessageAnchorCollection();
    _scheduleBottomFollow();
    return false;
  }

  /// Rechecks a pending message jump after a chat row changes size.
  bool _handleSizeChangedLayoutNotification(
    SizeChangedLayoutNotification notification,
  ) {
    if (_pendingJumpToMessageTimestamp == null) {
      return false;
    }
    _pendingMessageJumpStabilityCheckRequested = false;
    _pendingMessageJumpTimer?.cancel();
    _pendingMessageJumpTimer = null;
    _scheduleMessageAnchorCollection();
    _schedulePendingMessageJump();
    return false;
  }

  /// Updates navigator anchors for active user scroll sessions only.
  bool _handleScrollNotification(ScrollNotification notification) {
    if (notification is UserScrollNotification) {
      if (notification.direction != ScrollDirection.idle) {
        _userScrollSessionActive = true;
        _userScrollsTowardHistory =
            notification.direction == ScrollDirection.forward;
        if (!_showNavigatorChipNotifier.value) {
          _showNavigatorChipNotifier.value = true;
        }
        if (_userScrollsTowardHistory &&
            widget.autoScrollToBottomListenable.value) {
          widget.onAutoScrollToBottomChanged(false);
        }
      } else if (_userScrollSessionActive) {
        if (_isAtBottom(notification.metrics) &&
            !widget.autoScrollToBottomListenable.value) {
          widget.onAutoScrollToBottomChanged(true);
        }
        _userScrollsTowardHistory = false;
        _scheduleNavigatorHide();
      }
    }

    if (notification is ScrollUpdateNotification) {
      if (notification.dragDetails != null) {
        if (!_showNavigatorChipNotifier.value) {
          _userScrollSessionActive = true;
          _showNavigatorChipNotifier.value = true;
        }
        _scheduleNavigatorHide();
      }
      if (_userScrollSessionActive) {
        _scheduleMessageAnchorCollection();
      }
      if (_isAtBottom(notification.metrics) &&
          !_userScrollsTowardHistory &&
          !widget.autoScrollToBottomListenable.value) {
        widget.onAutoScrollToBottomChanged(true);
      }
    }
    return false;
  }

  /// Coalesces a burst of viewport-size changes into one anchor refresh.
  void _scheduleViewportResizeUpdate() {
    _viewportResizeTimer?.cancel();
    _viewportResizeTimer = Timer(_viewportResizeSettleDelay, () {
      _viewportResizeTimer = null;
      if (!mounted) {
        return;
      }
      _scheduleMessageAnchorCollection();
    });
  }

  /// Schedules one post-layout collection of message navigation anchors.
  void _scheduleMessageAnchorCollection() {
    if (_messageAnchorCollectionScheduled) {
      return;
    }
    _messageAnchorCollectionScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _messageAnchorCollectionScheduled = false;
      if (mounted) {
        _collectMessageAnchors();
      }
    });
  }

  /// Hides the scroll navigator after the active user scroll session settles.
  void _scheduleNavigatorHide() {
    _navigatorHideTimer?.cancel();
    _navigatorHideTimer = Timer(_navigatorHideDelay, () {
      _navigatorHideTimer = null;
      if (!mounted) {
        return;
      }
      final scrollController = widget.scrollController;
      if (scrollController.hasClients &&
          scrollController.position.isScrollingNotifier.value) {
        _scheduleNavigatorHide();
        return;
      }
      _showNavigatorChipNotifier.value = false;
      _userScrollSessionActive = false;
    });
  }

  /// Reports whether the supplied scroll metrics are positioned at the bottom.
  bool _isAtBottom(ScrollMetrics metrics) {
    return metrics.pixels >= metrics.maxScrollExtent - 2;
  }

  /// Schedules a single automatic jump to the current bottom extent.
  void _scheduleBottomFollow() {
    if (_bottomFollowScheduled ||
        !widget.autoScrollToBottomListenable.value ||
        widget.hasNewerDisplayHistory ||
        widget.isLoadingDisplayWindow ||
        !widget.scrollController.hasClients) {
      return;
    }
    _bottomFollowScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _bottomFollowScheduled = false;
      if (!mounted ||
          !widget.autoScrollToBottomListenable.value ||
          widget.hasNewerDisplayHistory ||
          widget.isLoadingDisplayWindow ||
          !widget.scrollController.hasClients) {
        return;
      }
      final position = widget.scrollController.position;
      final target = position.maxScrollExtent;
      if ((target - position.pixels).abs() <= 1) {
        return;
      }
      widget.scrollController.jumpTo(target);
    });
  }

  Future<void> _scrollToBottomFromNavigator() async {
    widget.onAutoScrollToBottomChanged(true);
    if (widget.hasNewerDisplayHistory) {
      await widget.onShowLatestDisplayWindow();
      return;
    }
    await widget.scrollController.animateTo(
      widget.scrollController.position.maxScrollExtent,
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
    );
  }

  /// Starts a locator jump by timestamp and reveals the display window when needed.
  Future<void> _jumpToMessageTimestamp(int timestamp) async {
    _beginPendingMessageJump(timestamp);
    final targetIndex = widget.messages.indexWhere(
      (message) => message.timestamp == timestamp,
    );
    if (targetIndex >= 0) {
      _jumpToMessageIndex(targetIndex);
      return;
    }

    final currentChatId = widget.currentChatId;
    widget.onAutoScrollToBottomChanged(false);
    final didReveal = await widget.onRevealMessageForLocator(timestamp);
    if (!mounted || widget.currentChatId != currentChatId) {
      return;
    }
    final targetVisible = widget.messages.any(
      (message) => message.timestamp == timestamp,
    );
    if (!didReveal &&
        !targetVisible &&
        _pendingJumpToMessageTimestamp == timestamp) {
      _pendingJumpToMessageTimestamp = null;
      return;
    }
    _scheduleMessageAnchorCollection();
    _schedulePendingMessageJump();
  }

  /// Starts a locator jump by the message index visible in the active window.
  void _jumpToMessageIndex(int targetIndex) {
    if (targetIndex < 0 || targetIndex >= widget.messages.length) {
      return;
    }
    final isActualLatestMessage =
        targetIndex == widget.messages.length - 1 &&
        !widget.hasNewerDisplayHistory;
    widget.onAutoScrollToBottomChanged(isActualLatestMessage);
    _beginPendingMessageJump(widget.messages[targetIndex].timestamp);
    _scheduleMessageAnchorCollection();
    _schedulePendingMessageJump();
  }

  /// Starts tracking one message until its rendered position becomes stable.
  void _beginPendingMessageJump(int timestamp) {
    _pendingMessageJumpTimer?.cancel();
    _pendingMessageJumpTimer = null;
    _pendingJumpToMessageTimestamp = timestamp;
    _lastEstimatedPendingJumpOffset = null;
    _pendingMessageJumpStabilityCheckRequested = false;
  }

  /// Stops tracking the active message locator jump and clears its timers.
  void _clearPendingMessageJump() {
    _pendingMessageJumpTimer?.cancel();
    _pendingMessageJumpTimer = null;
    _pendingJumpToMessageTimestamp = null;
    _lastEstimatedPendingJumpOffset = null;
    _pendingMessageJumpStabilityCheckRequested = false;
  }

  /// Schedules a delayed check for Markdown layout changes during a jump.
  void _schedulePendingMessageJumpCheck(Duration delay) {
    _pendingMessageJumpTimer?.cancel();
    _pendingMessageJumpTimer = Timer(delay, () {
      _pendingMessageJumpTimer = null;
      if (!mounted || _pendingJumpToMessageTimestamp == null) {
        return;
      }
      _pendingMessageJumpStabilityCheckRequested = true;
      _schedulePendingMessageJump();
    });
  }

  /// Estimates an initial offset that brings an unbuilt message near the viewport.
  double _estimateMessageOffset(int targetIndex, double maxScrollExtent) {
    final anchors = _messageAnchorsNotifier.value.values.toList()
      ..sort((left, right) => left.index.compareTo(right.index));
    if (anchors.length >= 2) {
      final first = anchors.first;
      final last = anchors.last;
      final indexSpan = last.index - first.index;
      if (indexSpan > 0) {
        final averageMessageStep =
            (last.absoluteTopPx - first.absoluteTopPx) / indexSpan;
        final estimatedOffset =
            first.absoluteTopPx +
            (targetIndex - first.index) * averageMessageStep;
        return estimatedOffset.clamp(0, maxScrollExtent).toDouble();
      }
    }
    final messageCount = widget.messages.length;
    if (messageCount <= 1) {
      return 0;
    }
    final progress = targetIndex / (messageCount - 1);
    return (maxScrollExtent * progress).clamp(0, maxScrollExtent).toDouble();
  }

  /// Schedules one post-layout attempt to complete a pending locator jump.
  void _schedulePendingMessageJump() {
    if (_pendingMessageJumpScheduled) {
      return;
    }
    _pendingMessageJumpScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _pendingMessageJumpScheduled = false;
      if (!mounted) {
        return;
      }
      _collectMessageAnchors();
    });
  }

  /// Completes a pending locator jump once the target row anchor is available.
  Future<void> _tryCompletePendingMessageJump() async {
    final targetTimestamp = _pendingJumpToMessageTimestamp;
    if (targetTimestamp == null ||
        _pendingMessageJumpInFlight ||
        !mounted ||
        !widget.scrollController.hasClients) {
      return;
    }
    final targetIndex = widget.messages.indexWhere(
      (message) => message.timestamp == targetTimestamp,
    );
    if (targetIndex < 0) {
      return;
    }
    final anchor = _messageAnchorsNotifier.value[targetTimestamp];
    _pendingMessageJumpInFlight = true;
    try {
      final maxScrollExtent = widget.scrollController.position.maxScrollExtent;
      if (anchor == null) {
        final estimatedOffset = _estimateMessageOffset(
          targetIndex,
          maxScrollExtent,
        );
        final shouldMove =
            _lastEstimatedPendingJumpOffset == null ||
            (_lastEstimatedPendingJumpOffset! - estimatedOffset).abs() >
                _messageJumpPositionTolerance;
        if (shouldMove) {
          _lastEstimatedPendingJumpOffset = estimatedOffset;
          await widget.scrollController.animateTo(
            estimatedOffset,
            duration: const Duration(milliseconds: 260),
            curve: Curves.easeOutCubic,
          );
        }
        await WidgetsBinding.instance.endOfFrame;
        if (mounted && _pendingJumpToMessageTimestamp == targetTimestamp) {
          _collectMessageAnchors();
          _schedulePendingMessageJumpCheck(_messageJumpRetryDelay);
        }
        return;
      }
      _lastEstimatedPendingJumpOffset = null;
      final isActualLatestMessage =
          targetIndex == widget.messages.length - 1 &&
          !widget.hasNewerDisplayHistory;
      widget.onAutoScrollToBottomChanged(isActualLatestMessage);
      final targetOffset = isActualLatestMessage
          ? maxScrollExtent
          : anchor.absoluteTopPx.clamp(0, maxScrollExtent).toDouble();
      final needsCorrection =
          (targetOffset - widget.scrollController.offset).abs() >
          _messageJumpPositionTolerance;
      if (needsCorrection) {
        await widget.scrollController.animateTo(
          targetOffset,
          duration: const Duration(milliseconds: 260),
          curve: Curves.easeOutCubic,
        );
      }
      await WidgetsBinding.instance.endOfFrame;
      if (mounted) {
        _collectMessageAnchors();
        if (_pendingJumpToMessageTimestamp == targetTimestamp) {
          if (!needsCorrection && _pendingMessageJumpStabilityCheckRequested) {
            _clearPendingMessageJump();
          } else {
            _pendingMessageJumpStabilityCheckRequested = false;
            _schedulePendingMessageJumpCheck(_messageJumpSettleDelay);
          }
        }
      }
    } finally {
      _pendingMessageJumpInFlight = false;
    }
  }

  /// Collects render anchors for currently visible chat message rows.
  void _collectMessageAnchors() {
    if (!widget.scrollController.hasClients) {
      return;
    }
    final viewportContext = _viewportKey.currentContext;
    final viewportBox = viewportContext?.findRenderObject() as RenderBox?;
    if (viewportBox == null) {
      return;
    }
    final anchors = <int, ChatScrollMessageAnchor>{};
    for (var index = 0; index < widget.messages.length; index++) {
      final message = widget.messages[index];
      final key = _keyForMessage(message.timestamp);
      final rowContext = key.currentContext;
      final rowBox = rowContext?.findRenderObject() as RenderBox?;
      if (rowBox == null || !rowBox.hasSize) {
        continue;
      }
      final localTop = rowBox
          .localToGlobal(Offset.zero, ancestor: viewportBox)
          .dy;
      anchors[message.timestamp] = ChatScrollMessageAnchor(
        timestamp: message.timestamp,
        index: index,
        key: key,
        absoluteTopPx: widget.scrollController.offset + localTop,
        heightPx: rowBox.size.height,
      );
    }
    _messageAnchorsNotifier.value = anchors;
    unawaited(_tryCompletePendingMessageJump());
  }

  GlobalKey _keyForMessage(int timestamp) {
    return _messageKeys.putIfAbsent(timestamp, GlobalKey.new);
  }

  Key _rowKeyForIndex(int index, int messageStartIndex, int messageEndIndex) {
    if (index >= messageStartIndex && index < messageEndIndex) {
      return _keyForMessage(
        widget.messages[index - messageStartIndex].timestamp,
      );
    }
    if (widget.hasOlderDisplayHistory && index == 0) {
      return const ValueKey<String>('row-load-older');
    }
    if (widget.hasNewerDisplayHistory && index == messageEndIndex) {
      return const ValueKey<String>('row-load-newer');
    }
    return const ValueKey<String>('row-status');
  }

  /// Refreshes cached rows and anchors after the message window changes.
  @override
  void didUpdateWidget(ChatArea oldWidget) {
    super.didUpdateWidget(oldWidget);
    final chatChanged = oldWidget.currentChatId != widget.currentChatId;
    if (chatChanged) {
      _messageKeys.clear();
      _messageRowCache.clear();
      _messageAnchorsNotifier.value = const <int, ChatScrollMessageAnchor>{};
      _showNavigatorChipNotifier.value = false;
      _userScrollSessionActive = false;
      _userScrollsTowardHistory = false;
      _clearPendingMessageJump();
      _pendingMessageJumpScheduled = false;
      _pendingMessageJumpInFlight = false;
    }
    final messagesChanged =
        chatChanged ||
        oldWidget.messages.length != widget.messages.length ||
        oldWidget.messages.firstOrNull?.timestamp !=
            widget.messages.firstOrNull?.timestamp ||
        oldWidget.messages.lastOrNull?.timestamp !=
            widget.messages.lastOrNull?.timestamp;
    final bottomInsetChanged =
        oldWidget.bottomContentInset != widget.bottomContentInset;
    if (messagesChanged || bottomInsetChanged) {
      _scheduleBottomFollow();
      _scheduleMessageAnchorCollection();
      _schedulePendingMessageJump();
    }
    final timestamps = widget.messages
        .map((message) => message.timestamp)
        .toSet();
    _messageKeys.removeWhere(
      (timestamp, key) => !timestamps.contains(timestamp),
    );
    _messageRowCache.removeWhere(
      (timestamp, row) => !timestamps.contains(timestamp),
    );
  }

  /// Releases timers, cached rows, and navigation state.
  @override
  void dispose() {
    _navigatorHideTimer?.cancel();
    _viewportResizeTimer?.cancel();
    _pendingMessageJumpTimer?.cancel();
    _messageAnchorsNotifier.dispose();
    _showNavigatorChipNotifier.dispose();
    _messageKeys.clear();
    _messageRowCache.clear();
    super.dispose();
  }

  Widget _messageRowFor(int messageIndex, ChatUiMessage message) {
    final selected = widget.selectedMessageTimestamps.contains(
      message.timestamp,
    );
    final selectionMode = widget.isMultiSelectMode;
    final isStreaming = _isStreamingMessage(messageIndex);
    final themePreferenceSnapshot = OperitTheme.of(
      context,
    ).themePreferenceSnapshot;
    final colorScheme = Theme.of(context).colorScheme;
    final messageThemeColors = _resolveMessageRowThemeColors(
      themePreferenceSnapshot,
      colorScheme,
    );
    final cached = _messageRowCache[message.timestamp];
    if (cached != null &&
        cached.index == messageIndex &&
        cached.selected == selected &&
        cached.selectionMode == selectionMode &&
        cached.isStreaming == isStreaming &&
        cached.currentCharacterCardAvatarUri ==
            widget.currentCharacterCardAvatarUri &&
        cached.themePreferenceSnapshot == themePreferenceSnapshot &&
        cached.messageThemeColors == messageThemeColors &&
        _sameMessageForRender(cached.message, message)) {
      return cached.widget;
    }

    final chatMessage =
        themePreferenceSnapshot.chatStyle ==
            UserPreferencesManager.CHAT_STYLE_BUBBLE
        ? BubbleStyleChatMessage(
            key: ValueKey<String>(_messageWidgetKey(message)),
            message: message,
            isStreaming: isStreaming,
            userMessageColor: messageThemeColors.userMessageColor,
            aiMessageColor: messageThemeColors.aiMessageColor,
            userTextColor: messageThemeColors.userTextColor,
            aiTextColor: messageThemeColors.aiTextColor,
            systemMessageColor: messageThemeColors.systemMessageColor,
            systemTextColor: messageThemeColors.systemTextColor,
            transparentSurface:
                themePreferenceSnapshot.transparentSurfaceEnabled,
            userBubbleImageStyle: _userBubbleImageStyle(
              themePreferenceSnapshot,
            ),
            aiBubbleImageStyle: _aiBubbleImageStyle(themePreferenceSnapshot),
            bubbleUserRoundedCornersEnabled:
                themePreferenceSnapshot.bubbleUserRoundedCornersEnabled,
            bubbleAiRoundedCornersEnabled:
                themePreferenceSnapshot.bubbleAiRoundedCornersEnabled,
            bubbleUserContentPaddingLeft:
                themePreferenceSnapshot.bubbleUserContentPaddingLeft,
            bubbleUserContentPaddingRight:
                themePreferenceSnapshot.bubbleUserContentPaddingRight,
            bubbleAiContentPaddingLeft:
                themePreferenceSnapshot.bubbleAiContentPaddingLeft,
            bubbleAiContentPaddingRight:
                themePreferenceSnapshot.bubbleAiContentPaddingRight,
            currentCharacterCardAvatarUri: widget.currentCharacterCardAvatarUri,
            splitMarkdownContent: widget.splitMarkdownContent,
            onDeleteMessage: widget.onDeleteMessage,
            onEditSummary: widget.onSelectMessageToEdit,
          )
        : CursorStyleChatMessage(
            key: ValueKey<String>(_messageWidgetKey(message)),
            message: message,
            isStreaming: isStreaming,
            currentCharacterCardAvatarUri: widget.currentCharacterCardAvatarUri,
            splitMarkdownContent: widget.splitMarkdownContent,
            onDeleteMessage: widget.onDeleteMessage,
            onEditSummary: widget.onSelectMessageToEdit,
            enableDialogs: true,
          );
    final messageContent = _SelectableMessageFrame(
      selected: selected,
      selectionMode: selectionMode,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          chatMessage,
          if (message.sender == 'ai' && message.variantCount > 1)
            _MessageVariantSwitcher(
              message: message,
              onSelect: widget.onSelectMessageVariant,
            ),
        ],
      ),
    );
    final row = selectionMode
        ? GestureDetector(
            behavior: HitTestBehavior.translucent,
            onTap: () => widget.onToggleMessageSelection(message.timestamp),
            child: messageContent,
          )
        : MessageContextMenu(
            key: ValueKey<String>('menu-${_messageWidgetKey(message)}'),
            message: message,
            onToggleFavoriteMessage: widget.onToggleFavoriteMessage,
            onDeleteMessage: widget.onDeleteMessage,
            onDeleteMessagesFrom: widget.onDeleteMessagesFrom,
            onDeleteMessageVariant: widget.onDeleteMessageVariant,
            onRollbackToMessage: widget.onRollbackToMessage,
            onSelectMessageToEdit: widget.onSelectMessageToEdit,
            onRegenerateMessage: widget.onRegenerateMessage,
            onInsertSummary: widget.onInsertSummary,
            onCreateBranch: widget.onCreateBranch,
            onReplyToMessage: widget.onReplyToMessage,
            onPlayVoice: widget.onPlayVoice,
            onToggleMultiSelectMode: widget.onToggleMultiSelectMode,
            onRefresh: widget.onRefreshRequested,
            splitMarkdownContent: widget.splitMarkdownContent,
            child: messageContent,
          );
    _messageRowCache[message.timestamp] = _CachedMessageRow(
      index: messageIndex,
      message: message,
      selected: selected,
      selectionMode: selectionMode,
      isStreaming: isStreaming,
      currentCharacterCardAvatarUri: widget.currentCharacterCardAvatarUri,
      themePreferenceSnapshot: themePreferenceSnapshot,
      messageThemeColors: messageThemeColors,
      widget: row,
    );
    return row;
  }

  /// Builds an element identity that cannot be shared by different chats.
  String _messageWidgetKey(ChatUiMessage message) {
    return '${widget.currentChatId ?? '__NO_CHAT__'}-${message.stableKey}';
  }

  /// Shows the standalone cursor only before an AI response stream is attached.
  bool _shouldShowLoadingIndicator() {
    if (!widget.isLoading || widget.messages.isEmpty) {
      return widget.isLoading && widget.messages.isEmpty;
    }
    final lastMessage = widget.messages.last;
    return lastMessage.sender == 'user' ||
        (lastMessage.sender == 'ai' &&
            lastMessage.parts.isEmpty &&
            lastMessage.contentStream == null);
  }

  /// Reports whether this AI row currently owns a live response stream.
  bool _isStreamingMessage(int index) {
    if (index < 0 || index >= widget.messages.length) {
      return false;
    }
    final message = widget.messages[index];
    return message.sender == 'ai' && message.contentStream != null;
  }
}

/// Displays controls for selecting the active response variant of one AI message.
class _MessageVariantSwitcher extends StatelessWidget {
  const _MessageVariantSwitcher({
    required this.message,
    required this.onSelect,
  });

  final ChatUiMessage message;
  final MessageVariantAction onSelect;

  @override
  /// Builds the compact variant selection controls.
  Widget build(BuildContext context) {
    final hasPrevious = message.selectedVariantIndex > 0;
    final hasNext = message.selectedVariantIndex < message.variantCount - 1;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsetsDirectional.only(start: 16, top: 4),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          IconButton(
            tooltip: '上一条变体',
            visualDensity: VisualDensity.compact,
            onPressed: hasPrevious
                ? () async {
                    await onSelect(
                      message.timestamp,
                      message.selectedVariantIndex - 1,
                    );
                  }
                : null,
            icon: const Icon(Icons.arrow_back, size: 18),
          ),
          Text(
            '${message.selectedVariantIndex + 1} / ${message.variantCount}',
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
          IconButton(
            tooltip: '下一条变体',
            visualDensity: VisualDensity.compact,
            onPressed: hasNext
                ? () async {
                    await onSelect(
                      message.timestamp,
                      message.selectedVariantIndex + 1,
                    );
                  }
                : null,
            icon: const Icon(Icons.arrow_forward, size: 18),
          ),
        ],
      ),
    );
  }
}

class _ChatAreaContentColumn extends StatelessWidget {
  const _ChatAreaContentColumn({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final themePreferenceSnapshot = OperitTheme.of(
      context,
    ).themePreferenceSnapshot;
    final maxWidth = themePreferenceSnapshot.bubbleWideLayoutEnabled
        ? chatWideContentMaxWidth
        : chatContentMaxWidth;
    return Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: maxWidth),
        child: SizedBox(width: double.infinity, child: child),
      ),
    );
  }
}

class _StatusMessage extends StatelessWidget {
  const _StatusMessage({required this.text, this.isError = false});

  final String text;
  final bool isError;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: SelectableText(
        text,
        style: theme.textTheme.bodySmall?.copyWith(
          color: isError
              ? theme.colorScheme.error
              : theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _DisplayWindowAction extends StatelessWidget {
  const _DisplayWindowAction({
    required this.text,
    required this.isLoading,
    required this.onTap,
  });

  final String text;
  final bool isLoading;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 16),
        child: Center(
          child: Text(
            isLoading ? 'Loading...' : text,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.primary,
            ),
          ),
        ),
      ),
    );
  }
}

class _SelectableMessageFrame extends StatelessWidget {
  const _SelectableMessageFrame({
    required this.selected,
    required this.selectionMode,
    required this.child,
  });

  final bool selected;
  final bool selectionMode;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!selectionMode && !selected) {
      return child;
    }
    final colorScheme = Theme.of(context).colorScheme;
    return Stack(
      children: <Widget>[
        AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          decoration: BoxDecoration(
            color: selected
                ? colorScheme.primary.withValues(alpha: 0.08)
                : Colors.transparent,
            border: Border.all(
              color: selected
                  ? colorScheme.primary
                  : colorScheme.outlineVariant.withValues(alpha: 0.45),
              width: selected ? 1.5 : 1,
            ),
            borderRadius: BorderRadius.circular(12),
          ),
          child: child,
        ),
        Positioned(
          left: 6,
          top: 6,
          child: Icon(
            selected ? Icons.check_circle : Icons.radio_button_unchecked,
            size: 18,
            color: selected
                ? colorScheme.primary
                : colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
          ),
        ),
      ],
    );
  }
}

class _CachedMessageRow {
  const _CachedMessageRow({
    required this.index,
    required this.message,
    required this.selected,
    required this.selectionMode,
    required this.isStreaming,
    required this.currentCharacterCardAvatarUri,
    required this.themePreferenceSnapshot,
    required this.messageThemeColors,
    required this.widget,
  });

  final int index;
  final ChatUiMessage message;
  final bool selected;
  final bool selectionMode;
  final bool isStreaming;
  final String? currentCharacterCardAvatarUri;
  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final _MessageRowThemeColors messageThemeColors;
  final Widget widget;
}

/// Captures concrete theme-dependent colors used by one cached message row.
typedef _MessageRowThemeColors = ({
  Color userMessageColor,
  Color aiMessageColor,
  Color userTextColor,
  Color aiTextColor,
  Color systemMessageColor,
  Color systemTextColor,
});

/// Resolves concrete colors used by one cached message row.
_MessageRowThemeColors _resolveMessageRowThemeColors(
  ThemePreferenceSnapshot snapshot,
  ColorScheme colorScheme,
) {
  return (
    userMessageColor:
        _optionalColor(snapshot.bubbleUserBubbleColor) ??
        colorScheme.primaryContainer,
    aiMessageColor:
        _optionalColor(snapshot.bubbleAiBubbleColor) ??
        colorScheme.surfaceContainerHighest,
    userTextColor:
        _optionalColor(snapshot.bubbleUserTextColor) ??
        colorScheme.onPrimaryContainer,
    aiTextColor:
        _optionalColor(snapshot.bubbleAiTextColor) ?? colorScheme.onSurface,
    systemMessageColor: colorScheme.surfaceContainerHighest,
    systemTextColor: colorScheme.onSurfaceVariant,
  );
}

/// Builds user bubble image settings from the active theme snapshot.
BubbleImageStyle? _userBubbleImageStyle(ThemePreferenceSnapshot snapshot) {
  final imagePath = snapshot.bubbleUserImageUri;
  if (!snapshot.bubbleUserUseImage || imagePath == null || imagePath.isEmpty) {
    return null;
  }
  return BubbleImageStyle(
    imagePath: imagePath,
    cropLeftRatio: snapshot.bubbleUserImageCropLeft,
    cropTopRatio: snapshot.bubbleUserImageCropTop,
    cropRightRatio: snapshot.bubbleUserImageCropRight,
    cropBottomRatio: snapshot.bubbleUserImageCropBottom,
    repeatXStartRatio: snapshot.bubbleUserImageRepeatStart,
    repeatXEndRatio: snapshot.bubbleUserImageRepeatEnd,
    repeatYStartRatio: snapshot.bubbleUserImageRepeatYStart,
    repeatYEndRatio: snapshot.bubbleUserImageRepeatYEnd,
    imageScale: snapshot.bubbleUserImageScale,
    renderMode: snapshot.bubbleUserImageRenderMode,
  );
}

/// Builds AI bubble image settings from the active theme snapshot.
BubbleImageStyle? _aiBubbleImageStyle(ThemePreferenceSnapshot snapshot) {
  final imagePath = snapshot.bubbleAiImageUri;
  if (!snapshot.bubbleAiUseImage || imagePath == null || imagePath.isEmpty) {
    return null;
  }
  return BubbleImageStyle(
    imagePath: imagePath,
    cropLeftRatio: snapshot.bubbleAiImageCropLeft,
    cropTopRatio: snapshot.bubbleAiImageCropTop,
    cropRightRatio: snapshot.bubbleAiImageCropRight,
    cropBottomRatio: snapshot.bubbleAiImageCropBottom,
    repeatXStartRatio: snapshot.bubbleAiImageRepeatStart,
    repeatXEndRatio: snapshot.bubbleAiImageRepeatEnd,
    repeatYStartRatio: snapshot.bubbleAiImageRepeatYStart,
    repeatYEndRatio: snapshot.bubbleAiImageRepeatYEnd,
    imageScale: snapshot.bubbleAiImageScale,
    renderMode: snapshot.bubbleAiImageRenderMode,
  );
}

/// Converts a stored ARGB color value into a Flutter color.
Color? _optionalColor(int? value) {
  return value == null ? null : Color(value);
}

/// Reports whether one cached message row can be reused unchanged.
bool _sameMessageForRender(ChatUiMessage left, ChatUiMessage right) {
  return left.sender == right.sender &&
      _sameMessagePartsForRender(left, right) &&
      left.timestamp == right.timestamp &&
      left.roleName == right.roleName &&
      left.selectedVariantIndex == right.selectedVariantIndex &&
      left.variantCount == right.variantCount &&
      left.provider == right.provider &&
      left.modelName == right.modelName &&
      left.inputTokens == right.inputTokens &&
      left.outputTokens == right.outputTokens &&
      left.cachedInputTokens == right.cachedInputTokens &&
      left.sentAt == right.sentAt &&
      left.outputDurationMs == right.outputDurationMs &&
      left.waitDurationMs == right.waitDurationMs &&
      left.displayMode == right.displayMode &&
      left.isFavorite == right.isFavorite &&
      left.isVariantPreview == right.isVariantPreview &&
      left.completedAt == right.completedAt &&
      identical(left.contentStream, right.contentStream);
}

/// Compares canonical message parts by value for message-row cache reuse.
bool _sameMessagePartsForRender(ChatUiMessage left, ChatUiMessage right) {
  if (left.parts.length != right.parts.length) {
    return false;
  }
  for (var index = 0; index < left.parts.length; index++) {
    final leftPart = left.parts[index];
    final rightPart = right.parts[index];
    if (leftPart.partId != rightPart.partId ||
        leftPart.sequence != rightPart.sequence ||
        leftPart.kind != rightPart.kind ||
        leftPart.content != rightPart.content ||
        leftPart.toolCallId != rightPart.toolCallId ||
        leftPart.toolName != rightPart.toolName ||
        !mapEquals(leftPart.attributes, rightPart.attributes)) {
      return false;
    }
  }
  return true;
}

class _EmptyChatArea extends StatelessWidget {
  const _EmptyChatArea();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Text(
        'Operit',
        style: theme.textTheme.displaySmall?.copyWith(
          color: theme.colorScheme.primary.withValues(alpha: 0.38),
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}
