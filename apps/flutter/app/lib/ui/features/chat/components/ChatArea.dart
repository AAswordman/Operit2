// ignore_for_file: file_names

import 'dart:async';
import 'dart:collection';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';

import '../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../theme/OperitTheme.dart';
import '../viewmodel/ChatViewModel.dart';
import 'ChatLayoutMetrics.dart';
import 'ChatMessageExtent.dart';
import 'MessageContextMenu.dart';
import 'MessageCopyPreview.dart';
import 'ChatScrollNavigator.dart';
import 'NewChatIntro.dart';
import 'style/ThemedChatMessage.dart';

const Duration _navigatorHideDelay = Duration(milliseconds: 1200);
const Duration _viewportResizeSettleDelay = Duration(milliseconds: 120);
const Duration _bottomFollowRateWindow = Duration(milliseconds: 600);
const double _bottomFollowOutputVelocityGain = 0.65;
const double _bottomFollowGapCorrectionRate = 0.8;
const double _bottomFollowPositionTolerance = 1;

class ChatArea extends StatefulWidget {
  const ChatArea({
    super.key,
    required this.messages,
    required this.isLoading,
    required this.errorMessage,
    required this.scrollController,
    required this.currentChatId,
    required this.identityAvatarUri,
    this.onIdentityTap,
    required this.clients,
    required this.packageManager,
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
  final String? identityAvatarUri;
  final VoidCallback? onIdentityTap;
  final GeneratedCoreProxyClients clients;
  final GeneratedApplicationPackageManagerCoreProxy packageManager;
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

class _ChatAreaState extends State<ChatArea>
    with SingleTickerProviderStateMixin {
  final GlobalKey _viewportKey = GlobalKey();
  final Map<int, GlobalKey> _messageKeys = <int, GlobalKey>{};
  final Map<Key, ChatMessageExtent> _rowExtents = <Key, ChatMessageExtent>{};
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
  ScrollDirection _userScrollDirection = ScrollDirection.idle;
  final Set<int> _activeUserScrollPointers = <int>{};
  bool _messageAnchorCollectionScheduled = false;
  double _viewportHeight = 0;
  final ValueNotifier<double> _viewportHeightNotifier = ValueNotifier<double>(
    0,
  );
  double _scrollViewportDimension = 0;
  final Stopwatch _bottomFollowClock = Stopwatch();
  final Queue<_BottomGrowthSample> _bottomGrowthSamples =
      Queue<_BottomGrowthSample>();
  late final Ticker _bottomFollowTicker;
  int? _bottomFollowLastFrameMicroseconds;
  bool _bottomFollowCompleting = false;
  double? _lastScrollMaxExtent;
  int? _pendingJumpToMessageTimestamp;
  int _messageNavigationGeneration = 0;
  int? _layoutAnchorTimestamp;
  Key _scrollLayoutKey = UniqueKey();
  static const _centerSliverKey = ValueKey<String>('chat-message-center');
  late final _ChatLayoutScrollController _layoutScrollController;

  /// Initializes the frame-driven live-output follower.
  @override
  void initState() {
    super.initState();
    _layoutScrollController = _ChatLayoutScrollController(
      delegate: widget.scrollController,
      shouldAlignBottom: _shouldAlignBottomDuringLayout,
      shouldInitiallyAlignBottom: _ownsBottomScroll,
    );
    _bottomFollowClock.start();
    _bottomFollowTicker = createTicker(_tickBottomFollow);
  }

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
      _disposeRowExtents();
      return ValueListenableBuilder<bool>(
        valueListenable: newChatIntroActive,
        builder: (context, introActive, _) =>
            _EmptyChatArea(showMark: !introActive),
      );
    }

    final messageStartIndex = widget.hasOlderDisplayHistory ? 1 : 0;
    final messageEndIndex = messageStartIndex + widget.messages.length;
    final rowExtents = List<ChatMessageExtent>.generate(itemCount, (index) {
      final key = _extentKeyForIndex(index, messageStartIndex, messageEndIndex);
      final extent = _rowExtents.putIfAbsent(key, ChatMessageExtent.new);
      extent.updateTrailingSpacing(index == itemCount - 1 ? 0 : 8);
      return extent;
    }, growable: false);
    final anchorIndex = widget.messages.indexWhere(
      (message) => message.timestamp == _layoutAnchorTimestamp,
    );
    final centerIndex = anchorIndex < 0
        ? 0
        : anchorIndex + (widget.hasOlderDisplayHistory ? 1 : 0);
    // A sidebar width animation changes constraints, not message data. Reuse
    // this viewport so LayoutBuilder does not recreate the scroll view's delegate and
    // rebuild all visible rows on every animation frame.
    final viewport = Stack(
      key: _viewportKey,
      children: <Widget>[
        NotificationListener<SizeChangedLayoutNotification>(
          onNotification: _handleSizeChangedLayoutNotification,
          child: NotificationListener<ScrollMetricsNotification>(
            onNotification: _handleScrollMetricsNotification,
            child: NotificationListener<ScrollNotification>(
              onNotification: _handleScrollNotification,
              child: Listener(
                onPointerDown: _handleUserPointerStart,
                onPointerUp: _handleUserPointerEnd,
                onPointerCancel: _handleUserPointerEnd,
                onPointerPanZoomStart: _handleUserPointerStart,
                onPointerPanZoomEnd: _handleUserPointerEnd,
                child: CustomScrollView(
                  key: _scrollLayoutKey,
                  controller: _layoutScrollController,
                  // Chat history should stop hard at its boundaries. The
                  // platform default can use bouncing physics, which lets the
                  // transcript move past the bottom and snap back on release.
                  physics: const ClampingScrollPhysics(),
                  center: _centerSliverKey,
                  slivers: [
                    if (centerIndex > 0)
                      SliverPadding(
                        padding: const EdgeInsets.fromLTRB(16, 16, 16, 0),
                        sliver: ChatMessageSliverList(
                          extents: rowExtents
                              .take(centerIndex)
                              .toList()
                              .reversed
                              .toList(),
                          preserveViewport: _shouldPreserveManualViewport,
                          itemBuilder: (context, index) =>
                              _buildListRow(centerIndex - index - 1, itemCount),
                        ),
                      ),
                    SliverPadding(
                      key: _centerSliverKey,
                      padding: EdgeInsets.fromLTRB(
                        16,
                        _layoutAnchorTimestamp == null ? 16 : 0,
                        16,
                        16 + widget.bottomContentInset,
                      ),
                      sliver: ChatMessageSliverList(
                        extents: rowExtents.skip(centerIndex).toList(),
                        preserveViewport: _shouldPreserveManualViewport,
                        itemBuilder: (context, index) =>
                            _buildListRow(centerIndex + index, itemCount),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
        AnimatedBuilder(
          animation: Listenable.merge(<Listenable>[
            _messageAnchorsNotifier,
            _viewportHeightNotifier,
          ]),
          builder: (context, _) {
            final messageAnchors = _messageAnchorsNotifier.value;
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
                      onRequestLatestMessages: widget.onShowLatestDisplayWindow,
                      onAutoScrollToBottomChanged:
                          widget.onAutoScrollToBottomChanged,
                      onJumpToMessageTimestamp: _jumpToMessageTimestamp,
                      onJumpToMessage: _jumpToMessageIndex,
                      onToggleFavoriteMessage: widget.onToggleFavoriteMessage,
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

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewportHeight = constraints.maxHeight;
        if (_viewportHeight != viewportHeight) {
          _viewportHeight = viewportHeight;
          _scheduleViewportResizeUpdate();
        }
        return viewport;
      },
    );
  }

  /// Builds one chronological row shared by both growth directions.
  Widget _buildListRow(int index, int itemCount) {
    final messageStartIndex = widget.hasOlderDisplayHistory ? 1 : 0;
    final messageEndIndex = messageStartIndex + widget.messages.length;
    late final Widget child;
    var observesLiveBottomGrowth = false;
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
    } else if (index >= messageStartIndex && index < messageEndIndex) {
      final message = widget.messages[index - messageStartIndex];
      final messageIndex = index - messageStartIndex;
      child = _messageRowFor(messageIndex, message);
      if (messageIndex == widget.messages.length - 1 &&
          _isStreamingMessage(messageIndex)) {
        observesLiveBottomGrowth = true;
      }
    } else if (widget.hasNewerDisplayHistory && index == messageEndIndex) {
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
      child = _StatusMessage(text: widget.errorMessage!, isError: true);
    } else {
      child = const Padding(
        padding: EdgeInsets.only(left: 16, top: 2, bottom: 2),
        child: StreamingCursor(),
      );
    }
    final rowKey = _rowKeyForIndex(index, messageStartIndex, messageEndIndex);
    return observeChatMessageExtent(
      key: ValueKey<Key>(rowKey),
      extent:
          _rowExtents[_extentKeyForIndex(
            index,
            messageStartIndex,
            messageEndIndex,
          )]!,
      child: Padding(
        padding: EdgeInsets.only(bottom: index == itemCount - 1 ? 0 : 8),
        child: SizeChangedLayoutNotifier(
          key: rowKey,
          child: _LiveBottomStreamSizeObserver(
            observesGrowth: observesLiveBottomGrowth,
            onSizeGrown: _scheduleBottomFollow,
            child: _ChatAreaContentColumn(child: child),
          ),
        ),
      ),
    );
  }

  /// Keeps the chat bottom aligned while the viewport changes size.
  bool _handleScrollMetricsNotification(
    ScrollMetricsNotification notification,
  ) {
    if (notification.depth != 0) {
      return false;
    }
    final maxScrollExtentDelta = _updateLastScrollMaxExtent(
      notification.metrics.maxScrollExtent,
    );
    final viewportDimension = notification.metrics.viewportDimension;
    _handleCompletedStreamExtentChange(maxScrollExtentDelta);
    if (_scrollViewportDimension != viewportDimension) {
      _scrollViewportDimension = viewportDimension;
      _scheduleViewportResizeUpdate();
      return false;
    }
    _scheduleMessageAnchorCollection();
    return false;
  }

  /// Updates the remembered bottom extent and returns the measured delta.
  double _updateLastScrollMaxExtent(double maxScrollExtent) {
    final previousMaxScrollExtent = _lastScrollMaxExtent;
    _lastScrollMaxExtent = maxScrollExtent;
    return previousMaxScrollExtent == null
        ? 0
        : maxScrollExtent - previousMaxScrollExtent;
  }

  /// Routes completed stream extent changes through the frame follower.
  bool _handleCompletedStreamExtentChange(double maxScrollExtentDelta) {
    if (!_isCompletingBottomFollow()) {
      return false;
    }
    _scheduleBottomFollow(maxScrollExtentDelta);
    return true;
  }

  /// Refreshes navigation geometry after asynchronously rendered content changes.
  bool _handleSizeChangedLayoutNotification(
    SizeChangedLayoutNotification notification,
  ) {
    _scheduleMessageAnchorCollection();
    return false;
  }

  /// Updates navigator anchors for active user scroll sessions only.
  bool _handleScrollNotification(ScrollNotification notification) {
    if (notification.depth != 0) {
      return false;
    }
    if (notification is UserScrollNotification) {
      if (notification.direction != ScrollDirection.idle) {
        _clearPendingMessageJump();
        _userScrollSessionActive = true;
        _userScrollDirection = notification.direction;
        if (!_showNavigatorChipNotifier.value) {
          _showNavigatorChipNotifier.value = true;
        }
        if (widget.autoScrollToBottomListenable.value) {
          _stopBottomFollow();
          widget.onAutoScrollToBottomChanged(false);
        }
      } else {
        if (_userScrollSessionActive &&
            _userScrollDirection == ScrollDirection.reverse &&
            _isAtBottom(notification.metrics) &&
            !widget.autoScrollToBottomListenable.value) {
          widget.onAutoScrollToBottomChanged(true);
        }
        _userScrollDirection = ScrollDirection.idle;
        if (_userScrollSessionActive) {
          _scheduleNavigatorHide();
        }
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
      if (_userScrollDirection == ScrollDirection.reverse &&
          _isAtBottom(notification.metrics) &&
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
      _viewportHeightNotifier.value = _viewportHeight;
      _scheduleMessageAnchorCollection();
    });
  }

  /// Pauses automatic jumps before a touch or trackpad drag takes ownership.
  /// Trackpads send pan/zoom events rather than pointer-down/up events; a
  /// follower jumpTo between pan start and drag acceptance cancels their hold.
  void _handleUserPointerStart(PointerEvent event) {
    _clearPendingMessageJump();
    _activeUserScrollPointers.add(event.pointer);
    if (_bottomFollowTicker.isActive) {
      _bottomFollowLastFrameMicroseconds =
          _bottomFollowClock.elapsedMicroseconds;
    }
  }

  /// Resumes follow scheduling only after all touch/trackpad gestures end.
  void _handleUserPointerEnd(PointerEvent event) {
    _activeUserScrollPointers.remove(event.pointer);
    if (_activeUserScrollPointers.isNotEmpty) {
      if (_bottomFollowTicker.isActive) {
        _bottomFollowLastFrameMicroseconds =
            _bottomFollowClock.elapsedMicroseconds;
      }
      return;
    }
    if (_bottomFollowTicker.isActive) {
      _bottomFollowLastFrameMicroseconds =
          _bottomFollowClock.elapsedMicroseconds;
      return;
    }
    if (!widget.autoScrollToBottomListenable.value) {
      return;
    }
    _scheduleBottomFollow(0);
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

  /// Reports whether the settled display window has reached the latest bottom.
  bool _isAtBottom(ScrollMetrics metrics) {
    return !widget.hasNewerDisplayHistory &&
        !widget.isLoadingDisplayWindow &&
        metrics.pixels >= metrics.maxScrollExtent - 2;
  }

  /// Reports whether the viewport currently belongs to automatic bottom following.
  bool _ownsBottomScroll() {
    return mounted &&
        _activeUserScrollPointers.isEmpty &&
        widget.autoScrollToBottomListenable.value &&
        !widget.hasNewerDisplayHistory &&
        !widget.isLoadingDisplayWindow;
  }

  /// Preserves visible message geometry while the user owns transcript scrolling.
  bool _shouldPreserveManualViewport() {
    return !widget.autoScrollToBottomListenable.value ||
        _activeUserScrollPointers.isNotEmpty;
  }

  /// Aligns static history before paint without taking over live output following.
  bool _shouldAlignBottomDuringLayout() {
    return _ownsBottomScroll() &&
        !_hasLiveBottomStream() &&
        !_isCompletingBottomFollow();
  }

  /// Records measured live growth and starts the frame-driven bottom follower.
  void _scheduleBottomFollow(double heightDelta) {
    final nowMicroseconds = _bottomFollowClock.elapsedMicroseconds;
    if (!mounted ||
        !_shouldRunBottomFollow() ||
        !widget.autoScrollToBottomListenable.value ||
        widget.hasNewerDisplayHistory ||
        widget.isLoadingDisplayWindow ||
        !widget.scrollController.hasClients) {
      return;
    }
    if (heightDelta > _bottomFollowPositionTolerance) {
      _bottomGrowthSamples.add(
        _BottomGrowthSample(
          timestampMicroseconds: nowMicroseconds,
          heightDelta: heightDelta,
        ),
      );
    }
    _pruneBottomGrowthSamples(nowMicroseconds);
    if (!_bottomFollowTicker.isActive) {
      _bottomFollowLastFrameMicroseconds = nowMicroseconds;
      _bottomFollowTicker.start();
    }
  }

  /// Advances the scroll position from recent output growth and baseline error.
  void _tickBottomFollow(Duration elapsed) {
    final nowMicroseconds = _bottomFollowClock.elapsedMicroseconds;
    if (!mounted ||
        !_shouldRunBottomFollow() ||
        !widget.autoScrollToBottomListenable.value ||
        widget.hasNewerDisplayHistory ||
        widget.isLoadingDisplayWindow ||
        !widget.scrollController.hasClients) {
      _stopBottomFollow();
      return;
    }
    if (_activeUserScrollPointers.isNotEmpty) {
      _bottomFollowLastFrameMicroseconds = nowMicroseconds;
      _pruneBottomGrowthSamples(nowMicroseconds);
      return;
    }
    final previousMicroseconds = _bottomFollowLastFrameMicroseconds!;
    _bottomFollowLastFrameMicroseconds = nowMicroseconds;
    _pruneBottomGrowthSamples(nowMicroseconds);

    final position = widget.scrollController.position;
    final gap = position.maxScrollExtent - position.pixels;
    if (gap <= _bottomFollowPositionTolerance) {
      if (_bottomGrowthSamples.isEmpty) {
        _stopBottomFollow();
      }
      return;
    }
    final elapsedSeconds =
        (nowMicroseconds - previousMicroseconds) /
        Duration.microsecondsPerSecond;
    final outputDelta =
        _bottomOutputVelocity(nowMicroseconds) *
        _bottomFollowOutputVelocityGain *
        elapsedSeconds;
    final gapCorrectionDelta =
        gap * _bottomFollowGapCorrectionRate * elapsedSeconds;
    final scrollDelta = outputDelta + gapCorrectionDelta;
    final target = (position.pixels + scrollDelta).clamp(
      position.pixels,
      position.maxScrollExtent,
    );
    widget.scrollController.jumpTo(target);
  }

  /// Computes a linearly weighted output velocity over the active time window.
  double _bottomOutputVelocity(int nowMicroseconds) {
    final windowMicroseconds = _bottomFollowRateWindow.inMicroseconds;
    var weightedGrowth = 0.0;
    for (final sample in _bottomGrowthSamples) {
      final ageMicroseconds = nowMicroseconds - sample.timestampMicroseconds;
      final remainingWeight = 1 - ageMicroseconds / windowMicroseconds;
      weightedGrowth += sample.heightDelta * remainingWeight;
    }
    return weightedGrowth *
        2 *
        Duration.microsecondsPerSecond /
        windowMicroseconds;
  }

  /// Removes growth samples that no longer contribute to the time window.
  void _pruneBottomGrowthSamples(int nowMicroseconds) {
    final oldestTimestamp =
        nowMicroseconds - _bottomFollowRateWindow.inMicroseconds;
    while (_bottomGrowthSamples.isNotEmpty &&
        _bottomGrowthSamples.first.timestampMicroseconds <= oldestTimestamp) {
      _bottomGrowthSamples.removeFirst();
    }
  }

  /// Stops live following and clears its temporal growth model.
  void _stopBottomFollow() {
    _bottomFollowTicker.stop();
    _bottomFollowLastFrameMicroseconds = null;
    _bottomFollowCompleting = false;
    _bottomGrowthSamples.clear();
  }

  /// Reports whether bottom following should continue for live or completing rows.
  bool _shouldRunBottomFollow() {
    return _hasLiveBottomStream() || _isCompletingBottomFollow();
  }

  /// Reports whether a finished bottom stream is still settling its final layout.
  bool _isCompletingBottomFollow() {
    return _bottomFollowCompleting;
  }

  /// Starts the final smooth-follow window for a completed bottom stream.
  void _beginBottomFollowCompletion() {
    if (!mounted ||
        !widget.autoScrollToBottomListenable.value ||
        widget.hasNewerDisplayHistory ||
        widget.isLoadingDisplayWindow ||
        !widget.scrollController.hasClients) {
      return;
    }
    final nowMicroseconds = _bottomFollowClock.elapsedMicroseconds;
    _bottomFollowCompleting = true;
    if (!_bottomFollowTicker.isActive) {
      _bottomFollowLastFrameMicroseconds = nowMicroseconds;
      _bottomFollowTicker.start();
    }
  }

  /// Reports whether the visible bottom message is receiving live AI output.
  bool _hasLiveBottomStream() {
    final message = widget.messages.lastOrNull;
    return message != null &&
        message.sender == 'ai' &&
        message.contentStream != null;
  }

  /// Replaces any message locator target with an explicit bottom request.
  Future<void> _scrollToBottomFromNavigator() async {
    _clearPendingMessageJump();
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

  /// Reveals a timestamp and ignores completions superseded by user navigation.
  Future<void> _jumpToMessageTimestamp(int timestamp) async {
    _clearPendingMessageJump();
    final generation = _messageNavigationGeneration;
    _pendingJumpToMessageTimestamp = timestamp;
    _stopBottomFollow();
    widget.onAutoScrollToBottomChanged(false);
    if (widget.messages.any((message) => message.timestamp == timestamp)) {
      setState(_applyPendingMessageJump);
      return;
    }

    final didReveal = await widget.onRevealMessageForLocator(timestamp);
    if (!mounted || generation != _messageNavigationGeneration) {
      return;
    }
    if (!didReveal) {
      _clearPendingMessageJump();
      return;
    }
    setState(_applyPendingMessageJump);
  }

  /// Resolves navigator indices to stable message identities.
  void _jumpToMessageIndex(int targetIndex) {
    if (targetIndex < 0 || targetIndex >= widget.messages.length) {
      return;
    }
    unawaited(_jumpToMessageTimestamp(widget.messages[targetIndex].timestamp));
  }

  /// Makes the target the layout origin so earlier rows grow away from it.
  void _applyPendingMessageJump() {
    final timestamp = _pendingJumpToMessageTimestamp;
    if (timestamp == null) {
      return;
    }
    final index = widget.messages.indexWhere(
      (message) => message.timestamp == timestamp,
    );
    if (index < 0) {
      return;
    }
    _pendingJumpToMessageTimestamp = null;
    _layoutAnchorTimestamp = timestamp;
    _scrollLayoutKey = UniqueKey();
    _lastScrollMaxExtent = null;
    _messageAnchorsNotifier.value = const <int, ChatScrollMessageAnchor>{};
    final isLatest =
        index == widget.messages.length - 1 && !widget.hasNewerDisplayHistory;
    widget.onAutoScrollToBottomChanged(isLatest);
    _scheduleMessageAnchorCollection();
  }

  /// Cancels pending reveals without changing the established layout origin.
  void _clearPendingMessageJump() {
    _messageNavigationGeneration++;
    _pendingJumpToMessageTimestamp = null;
  }

  /// Collects render anchors for currently visible chat message rows.
  void _collectMessageAnchors() {
    if (!widget.scrollController.hasClients) {
      return;
    }
    final viewportContext = _viewportKey.currentContext;
    final viewportBox = viewportContext?.findRenderObject() as RenderBox?;
    if (viewportBox == null || !viewportBox.attached) {
      return;
    }
    final anchors = <int, ChatScrollMessageAnchor>{};
    for (var index = 0; index < widget.messages.length; index++) {
      final message = widget.messages[index];
      final key = _keyForMessage(message.timestamp);
      final rowContext = key.currentContext;
      final rowBox = rowContext?.findRenderObject() as RenderBox?;
      if (rowBox == null || !rowBox.attached || !rowBox.hasSize) {
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

  /// Returns the persistent render key for one message identity.
  GlobalKey _keyForMessage(int timestamp) {
    return _messageKeys.putIfAbsent(timestamp, GlobalKey.new);
  }

  /// Keeps measurements keyed by timestamps across display-window changes.
  Key _extentKeyForIndex(
    int index,
    int messageStartIndex,
    int messageEndIndex,
  ) {
    if (index >= messageStartIndex && index < messageEndIndex) {
      return ValueKey<int>(
        widget.messages[index - messageStartIndex].timestamp,
      );
    }
    return _rowKeyForIndex(index, messageStartIndex, messageEndIndex);
  }

  /// Identifies message and action rows independently of their list index.
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
    _layoutScrollController.delegate = widget.scrollController;
    final chatChanged = oldWidget.currentChatId != widget.currentChatId;
    if (chatChanged) {
      _stopBottomFollow();
      _lastScrollMaxExtent = null;
      _messageKeys.clear();
      _messageRowCache.clear();
      _disposeRowExtents();
      _messageAnchorsNotifier.value = const <int, ChatScrollMessageAnchor>{};
      _showNavigatorChipNotifier.value = false;
      _userScrollSessionActive = false;
      _userScrollDirection = ScrollDirection.idle;
      _clearPendingMessageJump();
      _layoutAnchorTimestamp = null;
      _scrollLayoutKey = UniqueKey();
    }
    if (_bottomStreamCompleted(oldWidget)) {
      _beginBottomFollowCompletion();
    }
    final messagesChanged =
        chatChanged ||
        oldWidget.messages.length != widget.messages.length ||
        oldWidget.messages.firstOrNull?.timestamp !=
            widget.messages.firstOrNull?.timestamp ||
        oldWidget.messages.lastOrNull?.timestamp !=
            widget.messages.lastOrNull?.timestamp;
    if (_layoutAnchorTimestamp != null &&
        !widget.messages.any(
          (message) => message.timestamp == _layoutAnchorTimestamp,
        )) {
      _layoutAnchorTimestamp = null;
      _scrollLayoutKey = UniqueKey();
    }
    _applyPendingMessageJump();
    final bottomInsetChanged =
        oldWidget.bottomContentInset != widget.bottomContentInset;
    if (messagesChanged || bottomInsetChanged) {
      _scheduleMessageAnchorCollection();
    } else if (oldWidget.isLoading != widget.isLoading ||
        oldWidget.errorMessage != widget.errorMessage ||
        oldWidget.hasNewerDisplayHistory != widget.hasNewerDisplayHistory ||
        oldWidget.isLoadingDisplayWindow != widget.isLoadingDisplayWindow) {
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

  /// Releases measurements after their old row render trees have detached.
  void _releaseRowExtents(List<ChatMessageExtent> extents) {
    if (extents.isEmpty) {
      return;
    }
    scheduleMicrotask(() {
      for (final extent in extents) {
        extent.dispose();
      }
    });
  }

  /// Retires all measurements when their chat identity leaves the transcript.
  void _disposeRowExtents() {
    _releaseRowExtents(_rowExtents.values.toList());
    _rowExtents.clear();
  }

  /// Releases timers, cached rows, and navigation state.
  @override
  void dispose() {
    _navigatorHideTimer?.cancel();
    _viewportResizeTimer?.cancel();
    _layoutScrollController.dispose();
    _bottomFollowTicker.dispose();
    _bottomFollowClock.stop();
    _bottomGrowthSamples.clear();
    _messageAnchorsNotifier.dispose();
    _viewportHeightNotifier.dispose();
    _showNavigatorChipNotifier.dispose();
    _messageKeys.clear();
    _messageRowCache.clear();
    _disposeRowExtents();
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
    final messageThemeColors = resolveChatMessageThemeColors(
      themePreferenceSnapshot,
      colorScheme,
    );
    final cached = _messageRowCache[message.timestamp];
    final cachedMessageMatches =
        cached != null &&
        cached.index == messageIndex &&
        cached.selected == selected &&
        cached.selectionMode == selectionMode &&
        cached.isStreaming == isStreaming &&
        cached.onIdentityTap == widget.onIdentityTap &&
        cached.chatId == widget.currentChatId &&
        cached.identityAvatarUri == widget.identityAvatarUri &&
        cached.themePreferenceSnapshot == themePreferenceSnapshot &&
        cached.messageThemeColors == messageThemeColors &&
        _sameMessageForRender(cached.message, message);
    if (cachedMessageMatches) {
      return cached.widget;
    }

    final chatMessage = buildThemedChatMessage(
      snapshot: themePreferenceSnapshot,
      colorScheme: colorScheme,
      key: ValueKey<String>(_messageWidgetKey(message)),
      message: message,
      identityAvatarUri: widget.identityAvatarUri,
      onIdentityTap: widget.onIdentityTap,
      splitMarkdownContent: widget.splitMarkdownContent,
      onDeleteMessage: widget.onDeleteMessage,
      onEditSummary: widget.onSelectMessageToEdit,
      enableDialogs: !selectionMode,
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
        : widget.currentChatId == null
        ? messageContent
        : MessageContextMenu(
            key: ValueKey<String>('menu-${_messageWidgetKey(message)}'),
            message: message,
            chatId: widget.currentChatId!,
            messageIndex: messageIndex,
            clients: widget.clients,
            packageManager: widget.packageManager,
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
      identityAvatarUri: widget.identityAvatarUri,
      onIdentityTap: widget.onIdentityTap,
      themePreferenceSnapshot: themePreferenceSnapshot,
      messageThemeColors: messageThemeColors,
      chatId: widget.currentChatId,
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

  /// Reports whether the same bottom AI message just finished streaming.
  bool _bottomStreamCompleted(ChatArea oldWidget) {
    final oldMessage = oldWidget.messages.lastOrNull;
    final message = widget.messages.lastOrNull;
    return oldWidget.currentChatId == widget.currentChatId &&
        oldMessage != null &&
        message != null &&
        oldMessage.timestamp == message.timestamp &&
        oldMessage.sender == 'ai' &&
        message.sender == 'ai' &&
        oldMessage.contentStream != null &&
        message.contentStream == null;
  }
}

/// Creates layout-aware positions while keeping the public controller attached.
class _ChatLayoutScrollController extends ScrollController {
  /// Retains the external navigation controller and the current ownership policy.
  _ChatLayoutScrollController({
    required ScrollController delegate,
    required this.shouldAlignBottom,
    required this.shouldInitiallyAlignBottom,
  }) : _delegate = delegate,
       super(keepScrollOffset: false);

  ScrollController _delegate;
  final ValueGetter<bool> shouldAlignBottom;
  final ValueGetter<bool> shouldInitiallyAlignBottom;

  /// Transfers attached positions when the owner replaces its controller.
  set delegate(ScrollController value) {
    if (identical(value, _delegate)) {
      return;
    }
    for (final position in positions) {
      _delegate.detach(position);
      value.attach(position);
    }
    _delegate = value;
  }

  /// Exposes the same position to the owner's navigation and scroll listeners.
  @override
  void attach(ScrollPosition position) {
    super.attach(position);
    _delegate.attach(position);
  }

  /// Releases the position from both controllers when a viewport is replaced.
  @override
  void detach(ScrollPosition position) {
    _delegate.detach(position);
    super.detach(position);
  }

  /// Starts a new layout origin without restoring offsets from another origin.
  @override
  ScrollPosition createScrollPosition(
    ScrollPhysics physics,
    ScrollContext context,
    ScrollPosition? oldPosition,
  ) {
    return _ChatLayoutScrollPosition(
      physics: physics,
      context: context,
      initialPixels: _delegate.initialScrollOffset,
      oldPosition: oldPosition,
      shouldAlignBottom: shouldAlignBottom,
      shouldInitiallyAlignBottom: shouldInitiallyAlignBottom,
    );
  }
}

/// Resolves static bottom ownership in layout instead of moving a painted frame.
class _ChatLayoutScrollPosition extends ScrollPositionWithSingleContext {
  /// Creates a position scoped to the current chat and locator layout origin.
  _ChatLayoutScrollPosition({
    required super.physics,
    required super.context,
    required super.initialPixels,
    required super.oldPosition,
    required this.shouldAlignBottom,
    required this.shouldInitiallyAlignBottom,
  }) : super(keepScrollOffset: false);

  final ValueGetter<bool> shouldAlignBottom;
  final ValueGetter<bool> shouldInitiallyAlignBottom;

  /// Requests a new layout pass with the measured bottom before any painting.
  @override
  bool applyContentDimensions(double minScrollExtent, double maxScrollExtent) {
    final alignsBottom =
        shouldAlignBottom() ||
        (!haveDimensions && shouldInitiallyAlignBottom());
    if (alignsBottom && pixels != maxScrollExtent) {
      correctPixels(maxScrollExtent);
      return false;
    }
    return super.applyContentDimensions(minScrollExtent, maxScrollExtent);
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
    required this.identityAvatarUri,
    this.onIdentityTap,
    required this.themePreferenceSnapshot,
    required this.messageThemeColors,
    required this.chatId,
    required this.widget,
  });

  final int index;
  final ChatUiMessage message;
  final bool selected;
  final bool selectionMode;
  final bool isStreaming;
  final String? identityAvatarUri;
  final VoidCallback? onIdentityTap;
  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final ChatMessageThemeColors messageThemeColors;
  final String? chatId;
  final Widget widget;
}

/// Reports whether one cached message row can be reused unchanged.
bool _sameMessageForRender(ChatUiMessage left, ChatUiMessage right) {
  final contentStreamSame = identical(left.contentStream, right.contentStream);
  return left.sender == right.sender &&
      (_sameMessagePartsForRender(left, right) ||
          _sameLiveAiStreamForRender(left, right, contentStreamSame)) &&
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
      contentStreamSame;
}

/// Reports whether one live AI stream should keep its mounted row.
bool _sameLiveAiStreamForRender(
  ChatUiMessage left,
  ChatUiMessage right,
  bool contentStreamSame,
) {
  return left.sender == 'ai' &&
      right.sender == 'ai' &&
      contentStreamSame &&
      left.contentStream != null;
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

/// Observes live bottom row growth after its first layout.
class _LiveBottomStreamSizeObserver extends SingleChildRenderObjectWidget {
  const _LiveBottomStreamSizeObserver({
    required this.observesGrowth,
    required this.onSizeGrown,
    required super.child,
  });

  final bool observesGrowth;
  final ValueChanged<double> onSizeGrown;

  /// Creates the render object that records row dimensions.
  @override
  RenderObject createRenderObject(BuildContext context) {
    return _LiveBottomStreamSizeRenderObject(
      observesGrowth: observesGrowth,
      onSizeGrown: onSizeGrown,
    );
  }

  /// Updates the callback used by the retained render object.
  @override
  void updateRenderObject(
    BuildContext context,
    _LiveBottomStreamSizeRenderObject renderObject,
  ) {
    renderObject
      ..observesGrowth = observesGrowth
      ..onSizeGrown = onSizeGrown;
  }
}

/// Reports height increases for the live bottom row.
class _LiveBottomStreamSizeRenderObject extends RenderProxyBox {
  _LiveBottomStreamSizeRenderObject({
    required bool observesGrowth,
    required ValueChanged<double> onSizeGrown,
  }) : _observesGrowth = observesGrowth,
       _onSizeGrown = onSizeGrown;

  bool _observesGrowth;
  ValueChanged<double> _onSizeGrown;
  Size? _lastSize;

  set observesGrowth(bool value) {
    if (_observesGrowth == value) {
      return;
    }
    _observesGrowth = value;
    if (!value) {
      _lastSize = null;
    }
  }

  set onSizeGrown(ValueChanged<double> value) {
    _onSizeGrown = value;
  }

  /// Reports the first layout and every later live-row height increase.
  @override
  void performLayout() {
    final previousSize = _lastSize;
    super.performLayout();
    final currentSize = size;
    if (!_observesGrowth) {
      _lastSize = null;
      return;
    }
    _lastSize = currentSize;
    if (previousSize == null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _onSizeGrown(0);
      });
      return;
    }
    final heightDelta = currentSize.height - previousSize.height;
    if (heightDelta <= _bottomFollowPositionTolerance) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _onSizeGrown(heightDelta);
    });
  }
}

/// Stores one measured live-row height increase in monotonic time.
class _BottomGrowthSample {
  /// Creates one timestamped height-growth sample.
  const _BottomGrowthSample({
    required this.timestampMicroseconds,
    required this.heightDelta,
  });

  final int timestampMicroseconds;
  final double heightDelta;
}

class _EmptyChatArea extends StatelessWidget {
  const _EmptyChatArea({this.showMark = true});

  /// False while the new-chat intro owns the stage, so the static wordmark
  /// and the intro animation never draw on top of each other. Swapped
  /// without a fade: a fade-out still overlaps the incoming particles.
  final bool showMark;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (!showMark) {
      return const SizedBox.shrink();
    }
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
