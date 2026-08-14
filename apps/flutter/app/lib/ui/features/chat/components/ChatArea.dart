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
    required this.onAutoScrollToBottomChanged,
    required this.onLoadOlderDisplayWindow,
    required this.onLoadNewerDisplayWindow,
    required this.onShowLatestDisplayWindow,
    required this.onToggleFavoriteMessage,
    required this.onDeleteMessage,
    required this.onDeleteMessagesFrom,
    required this.onDeleteMessageVariant,
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
    this.splitMarkdownContent,
    this.isMultiSelectMode = false,
    this.selectedMessageIndices = const <int>{},
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
  final ValueChanged<bool> onAutoScrollToBottomChanged;
  final Future<void> Function() onLoadOlderDisplayWindow;
  final Future<void> Function() onLoadNewerDisplayWindow;
  final Future<void> Function() onShowLatestDisplayWindow;
  final ToggleFavoriteMessage onToggleFavoriteMessage;
  final MessageIndexAction onDeleteMessage;
  final MessageIndexBoolAction onDeleteMessagesFrom;
  final MessageVariantAction onDeleteMessageVariant;
  final ValueChanged<int> onRollbackToMessage;
  final MessageSelectionAction onSelectMessageToEdit;
  final MessageIndexAction onRegenerateMessage;
  final ValueChanged<ChatUiMessage> onInsertSummary;
  final MessageTimestampAction onCreateBranch;
  final ValueChanged<ChatUiMessage> onReplyToMessage;
  final MessageVoiceAction onPlayVoice;
  final ValueChanged<int> onToggleMultiSelectMode;
  final ValueChanged<int> onToggleMessageSelection;
  final Future<void> Function() onRefreshRequested;
  final MarkdownCopySplitter? splitMarkdownContent;
  final bool isMultiSelectMode;
  final Set<int> selectedMessageIndices;

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
  bool _messageAnchorCollectionScheduled = false;
  double _viewportHeight = 0;
  double _scrollViewportDimension = 0;
  bool _bottomFollowScheduled = false;

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
            NotificationListener<ScrollMetricsNotification>(
              onNotification: _handleScrollMetricsNotification,
              child: NotificationListener<ScrollNotification>(
                onNotification: _handleScrollNotification,
                child: SingleChildScrollView(
                  controller: widget.scrollController,
                  padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
                  child: Column(
                    children: List<Widget>.generate(itemCount, (index) {
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
                        child: _ChatAreaContentColumn(
                          key: _rowKeyForIndex(
                            index,
                            messageStartIndex,
                            messageEndIndex,
                          ),
                          child: child,
                        ),
                      );
                    }),
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
                          onJumpToMessage: _jumpToMessageTimestamp,
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

  /// Updates navigator anchors for active user scroll sessions only.
  bool _handleScrollNotification(ScrollNotification notification) {
    if (notification is UserScrollNotification) {
      if (notification.direction != ScrollDirection.idle) {
        _userScrollSessionActive = true;
        if (!_showNavigatorChipNotifier.value) {
          _showNavigatorChipNotifier.value = true;
        }
        if (notification.direction == ScrollDirection.forward &&
            widget.autoScrollToBottomListenable.value &&
            !_isAtBottom(notification.metrics)) {
          widget.onAutoScrollToBottomChanged(false);
        }
      } else if (_userScrollSessionActive) {
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

  Future<void> _jumpToMessageTimestamp(int timestamp) async {
    widget.onAutoScrollToBottomChanged(false);
    final targetIndex = widget.messages.indexWhere(
      (message) => message.timestamp == timestamp,
    );
    if (targetIndex < 0 || !widget.scrollController.hasClients) {
      return;
    }
    _collectMessageAnchors();
    final anchor = _messageAnchorsNotifier.value[timestamp];
    if (anchor == null) {
      return;
    }
    final targetOffset = anchor.absoluteTopPx
        .clamp(0, widget.scrollController.position.maxScrollExtent)
        .toDouble();
    await widget.scrollController.animateTo(
      targetOffset,
      duration: const Duration(milliseconds: 260),
      curve: Curves.easeOutCubic,
    );
    await WidgetsBinding.instance.endOfFrame;
    _collectMessageAnchors();
  }

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
    final messagesChanged =
        oldWidget.messages.length != widget.messages.length ||
        oldWidget.messages.firstOrNull?.timestamp !=
            widget.messages.firstOrNull?.timestamp ||
        oldWidget.messages.lastOrNull?.timestamp !=
            widget.messages.lastOrNull?.timestamp;
    if (messagesChanged) {
      _scheduleBottomFollow();
      _scheduleMessageAnchorCollection();
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
    _messageAnchorsNotifier.dispose();
    _showNavigatorChipNotifier.dispose();
    _messageKeys.clear();
    _messageRowCache.clear();
    super.dispose();
  }

  Widget _messageRowFor(int messageIndex, ChatUiMessage message) {
    final selected = widget.selectedMessageIndices.contains(messageIndex);
    final selectionMode = widget.isMultiSelectMode;
    final isStreaming = _isStreamingMessage(messageIndex);
    final themePreferenceSnapshot = OperitTheme.of(
      context,
    ).themePreferenceSnapshot;
    final cached = _messageRowCache[message.timestamp];
    if (cached != null &&
        cached.index == messageIndex &&
        cached.selected == selected &&
        cached.selectionMode == selectionMode &&
        cached.isStreaming == isStreaming &&
        cached.currentCharacterCardAvatarUri ==
            widget.currentCharacterCardAvatarUri &&
        cached.themePreferenceSnapshot == themePreferenceSnapshot &&
        _sameMessageForRender(cached.message, message)) {
      return cached.widget;
    }

    final colorScheme = Theme.of(context).colorScheme;
    final chatMessage =
        themePreferenceSnapshot.chatStyle ==
            UserPreferencesManager.CHAT_STYLE_BUBBLE
        ? BubbleStyleChatMessage(
            key: ValueKey<String>(message.stableKey),
            message: message,
            isStreaming: isStreaming,
            userMessageColor:
                _optionalColor(themePreferenceSnapshot.bubbleUserBubbleColor) ??
                colorScheme.primaryContainer,
            aiMessageColor:
                _optionalColor(themePreferenceSnapshot.bubbleAiBubbleColor) ??
                colorScheme.surfaceContainerHighest,
            userTextColor:
                _optionalColor(themePreferenceSnapshot.bubbleUserTextColor) ??
                colorScheme.onPrimaryContainer,
            aiTextColor:
                _optionalColor(themePreferenceSnapshot.bubbleAiTextColor) ??
                colorScheme.onSurface,
            systemMessageColor: colorScheme.surfaceContainerHighest,
            systemTextColor: colorScheme.onSurfaceVariant,
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
          )
        : CursorStyleChatMessage(
            key: ValueKey<String>(message.stableKey),
            message: message,
            isStreaming: isStreaming,
            currentCharacterCardAvatarUri: widget.currentCharacterCardAvatarUri,
          );
    final messageContent = _SelectableMessageFrame(
      selected: selected,
      selectionMode: selectionMode,
      child: chatMessage,
    );
    final row = selectionMode
        ? GestureDetector(
            behavior: HitTestBehavior.translucent,
            onTap: () => widget.onToggleMessageSelection(messageIndex),
            child: messageContent,
          )
        : MessageContextMenu(
            key: ValueKey<String>('menu-${message.stableKey}'),
            index: messageIndex,
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
      widget: row,
    );
    return row;
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

class _ChatAreaContentColumn extends StatelessWidget {
  const _ChatAreaContentColumn({super.key, required this.child});

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
    required this.widget,
  });

  final int index;
  final ChatUiMessage message;
  final bool selected;
  final bool selectionMode;
  final bool isStreaming;
  final String? currentCharacterCardAvatarUri;
  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final Widget widget;
}

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
