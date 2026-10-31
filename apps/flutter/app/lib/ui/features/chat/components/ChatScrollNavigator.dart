// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart'
    show ChatMessageDisplayMode;
import '../../../common/ChatMessageIdentity.dart';
import '../viewmodel/ChatViewModel.dart';
import 'ChatScrollRailGeometry.dart';
import 'ChatMessageLocatorContent.dart';

export 'ChatMessageLocatorContent.dart';

typedef LoadMessageLocatorEntries =
    Future<List<ChatMessageLocatorPreview>> Function(
      String chatId,
      String query,
    );
typedef ToggleFavoriteMessage =
    Future<void> Function(int timestamp, bool isFavorite);
typedef RevealMessageForLocator = Future<bool> Function(int timestamp);

/// Routes lookup selections and keyboard requests for the mounted transcript.
class ChatMessageNavigationController extends ChangeNotifier {
  final openRequests = ValueNotifier<int>(0);
  String? chatId;
  int? timestamp;

  void open() => openRequests.value++;

  void jumpTo(String selectedChatId, int selectedTimestamp) {
    chatId = selectedChatId;
    timestamp = selectedTimestamp;
    notifyListeners();
  }

  @override
  void dispose() {
    openRequests.dispose();
    super.dispose();
  }
}

class ChatScrollMessageAnchor {
  const ChatScrollMessageAnchor({
    required this.timestamp,
    required this.index,
    required this.key,
    required this.absoluteTopPx,
    required this.heightPx,
  });

  final int timestamp;
  final int index;
  final GlobalKey key;
  final double absoluteTopPx;
  final double heightPx;
}

class ChatScrollNavigator extends StatefulWidget {
  const ChatScrollNavigator({
    super.key,
    this.messageNavigation,
    required this.messages,
    required this.currentChatId,
    required this.scrollController,
    required this.messageAnchors,
    required this.viewportHeight,
    required this.autoScrollToBottom,
    required this.hasNewerDisplayHistory,
    required this.loadLocatorEntries,
    required this.onRequestLatestMessages,
    required this.onAutoScrollToBottomChanged,
    required this.onJumpToMessageTimestamp,
    required this.onJumpToMessage,
    required this.onToggleFavoriteMessage,
    required this.onRequestScrollToBottom,
    required this.showNavigatorChip,
    required this.onNavigatorChipHidden,
  });

  final ChatMessageNavigationController? messageNavigation;
  final List<ChatUiMessage> messages;
  final String? currentChatId;
  final ScrollController scrollController;
  final Map<int, ChatScrollMessageAnchor> messageAnchors;
  final double viewportHeight;
  final bool autoScrollToBottom;
  final bool hasNewerDisplayHistory;
  final LoadMessageLocatorEntries loadLocatorEntries;
  final Future<void> Function() onRequestLatestMessages;
  final ValueChanged<bool> onAutoScrollToBottomChanged;
  final ValueChanged<int> onJumpToMessageTimestamp;
  final ValueChanged<int> onJumpToMessage;
  final ToggleFavoriteMessage onToggleFavoriteMessage;
  final VoidCallback onRequestScrollToBottom;
  final bool showNavigatorChip;
  final VoidCallback onNavigatorChipHidden;

  @override
  State<ChatScrollNavigator> createState() => _ChatScrollNavigatorState();
}

class _ChatScrollNavigatorState extends State<ChatScrollNavigator> {
  List<ChatMessageLocatorPreview> _locatorEntries =
      const <ChatMessageLocatorPreview>[];

  late List<ChatMessageLocatorPreview> _visibleEntries;
  int? _hoveredEntryIndex;
  int _locatorLoadGeneration = 0;

  /// Loads history previews once instead of generating them on pointer motion.
  @override
  void initState() {
    super.initState();
    widget.messageNavigation?.addListener(_onMessageNavigation);
    _visibleEntries = _fallbackEntriesFromMessages();
    _loadLocatorEntries();
  }

  /// Refreshes history geometry without retaining a preview from another chat.
  @override
  void didUpdateWidget(ChatScrollNavigator oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.messageNavigation != widget.messageNavigation) {
      oldWidget.messageNavigation?.removeListener(_onMessageNavigation);
      widget.messageNavigation?.addListener(_onMessageNavigation);
    }
    if (oldWidget.viewportHeight != widget.viewportHeight) {
      _hoveredEntryIndex = null;
    }
    final favoritesChanged =
        !identical(oldWidget.messages, widget.messages) &&
        _favoritesChanged(oldWidget.messages, widget.messages);
    if (!identical(oldWidget.messages, widget.messages)) {
      _visibleEntries = _fallbackEntriesFromMessages();
    }
    final messagesChanged =
        oldWidget.messages.length != widget.messages.length ||
        oldWidget.messages.firstOrNull?.timestamp !=
            widget.messages.firstOrNull?.timestamp ||
        oldWidget.messages.lastOrNull?.timestamp !=
            widget.messages.lastOrNull?.timestamp;
    if (messagesChanged ||
        favoritesChanged ||
        oldWidget.currentChatId != widget.currentChatId) {
      _hoveredEntryIndex = null;
      if (oldWidget.currentChatId != widget.currentChatId) {
        _locatorEntries = const <ChatMessageLocatorPreview>[];
      }
      _loadLocatorEntries();
    }
  }

  /// Builds an isolated paint-driven rail and a preview only for the selected row.
  @override
  Widget build(BuildContext context) {
    if (widget.messages.isEmpty ||
        widget.viewportHeight <= 60.0 ||
        !widget.viewportHeight.isFinite) {
      return const SizedBox.shrink();
    }

    final entries = _getEntries();
    final activeTimestamp = _resolveActiveTimestamp();
    final geometry = ChatScrollRailGeometry(
      entryCount: entries.length,
      viewportHeight: widget.viewportHeight,
    );
    final hoveredIndex = _hoveredEntryIndex;

    return LayoutBuilder(
      builder: (context, constraints) {
        final previewWidth = math.min(
          210.0,
          math.max(
            0.0,
            constraints.maxWidth - ChatScrollRailGeometry.interactionWidth - 24,
          ),
        );
        return Stack(
          children: <Widget>[
            Positioned(
              right: 0,
              top: 0,
              bottom: 0,
              width: ChatScrollRailGeometry.interactionWidth,
              child: _MessageTickRail(
                entries: entries,
                activeTimestamp: activeTimestamp,
                geometry: geometry,
                onHoverEntry: (index) {
                  if (_hoveredEntryIndex == index) return;
                  setState(() {
                    _hoveredEntryIndex = index;
                  });
                },
                onTapTick: _handleJumpToMessage,
              ),
            ),
            if (hoveredIndex != null &&
                hoveredIndex < entries.length &&
                previewWidth > 0)
              Positioned(
                right: ChatScrollRailGeometry.interactionWidth + 8,
                top: (geometry.entryCenter(hoveredIndex) - 32).clamp(
                  8.0,
                  math.max(8.0, widget.viewportHeight - 112),
                ),
                child: IgnorePointer(
                  child: SizedBox(
                    width: previewWidth,
                    child: _CodexHoverTooltipCard(entry: entries[hoveredIndex]),
                  ),
                ),
              ),
            if (!widget.autoScrollToBottom)
              Positioned(
                left: 0,
                right: 0,
                bottom: 16,
                child: Center(
                  child: _ScrollToBottomCircleButton(
                    onTap: widget.onRequestScrollToBottom,
                  ),
                ),
              ),
          ],
        );
      },
    );
  }

  /// Pins boundary highlights to endpoint messages and tracks the center between them.
  int? _resolveActiveTimestamp() {
    final scroll = widget.scrollController;
    if (!scroll.hasClients ||
        scroll.positions.length != 1 ||
        !scroll.position.hasPixels ||
        widget.viewportHeight <= 0 ||
        !widget.viewportHeight.isFinite) {
      return widget.messages.lastOrNull?.timestamp;
    }
    final position = scroll.position;
    if (position.hasContentDimensions) {
      // A transcript that fits entirely in the viewport stays on its latest message.
      if (position.pixels >= position.maxScrollExtent) {
        return widget.messages.lastOrNull?.timestamp;
      }
      if (position.pixels <= position.minScrollExtent) {
        return widget.messages.firstOrNull?.timestamp;
      }
    }
    final viewportCenter = scroll.offset + widget.viewportHeight / 2;
    ChatScrollMessageAnchor? centeredAnchor;
    double? centeredDistance;
    for (final anchor in widget.messageAnchors.values) {
      final distance =
          ((anchor.absoluteTopPx + anchor.heightPx / 2) - viewportCenter).abs();
      if (centeredDistance == null || distance < centeredDistance) {
        centeredAnchor = anchor;
        centeredDistance = distance;
      }
    }
    return centeredAnchor?.timestamp ?? widget.messages.lastOrNull?.timestamp;
  }

  /// Returns the current cached history preview collection.
  List<ChatMessageLocatorPreview> _getEntries() {
    return _locatorEntries.isNotEmpty ? _locatorEntries : _visibleEntries;
  }

  /// Projects the loaded transcript into locator previews once per message update.
  List<ChatMessageLocatorPreview> _fallbackEntriesFromMessages() {
    return widget.messages.indexed
        .map<ChatMessageLocatorPreview>((item) {
          final idx = item.$1;
          final msg = item.$2;
          final text = msg.displayText;
          return ChatMessageLocatorPreview(
            messageIndex: idx,
            timestamp: msg.timestamp,
            sender: msg.sender,
            displayName: readChatMessageIdentity(msg)?.title.trim() ?? '',
            previewContent: text,
            contentLength: text.length,
            displayMode:
                msg.displayMode == ChatMessageDisplayMode.hiddenPlaceholder
                ? 'HIDDEN_PLACEHOLDER'
                : 'NORMAL',
            isFavorite: msg.isFavorite,
          );
        })
        .toList(growable: false);
  }

  /// Routes a confirmed tap to its visible row or its history timestamp.
  void _handleJumpToMessage(int timestamp) {
    final targetIndex = widget.messages.indexWhere(
      (m) => m.timestamp == timestamp,
    );
    if (targetIndex >= 0) {
      widget.onJumpToMessage(targetIndex);
    } else {
      widget.onJumpToMessageTimestamp(timestamp);
    }
  }

  /// Detects metadata updates without reloading history on streamed text chunks.
  bool _favoritesChanged(
    List<ChatUiMessage> before,
    List<ChatUiMessage> after,
  ) {
    final favorites = {
      for (final message in before) message.timestamp: message.isFavorite,
    };
    return after.any(
      (message) => favorites[message.timestamp] != message.isFavorite,
    );
  }

  void _onMessageNavigation() {
    final navigation = widget.messageNavigation;
    if (navigation?.chatId == widget.currentChatId &&
        navigation?.timestamp != null) {
      _handleJumpToMessage(navigation!.timestamp!);
    }
  }

  @override
  void dispose() {
    widget.messageNavigation?.removeListener(_onMessageNavigation);
    super.dispose();
  }

  /// Rejects stale preview loads after a chat or history window changes.
  Future<void> _loadLocatorEntries() async {
    final generation = ++_locatorLoadGeneration;
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) {
      setState(() {
        _locatorEntries = const <ChatMessageLocatorPreview>[];
      });
      return;
    }
    List<ChatMessageLocatorPreview> entries;
    try {
      entries = await widget.loadLocatorEntries(chatId, '');
    } catch (_) {
      // The transcript remains navigable while history is unavailable.
      return;
    }
    if (!mounted || generation != _locatorLoadGeneration) {
      return;
    }
    setState(() {
      _locatorEntries = entries;
      _hoveredEntryIndex = null;
    });
  }
}

class _MessageTickRail extends StatefulWidget {
  /// Creates a paint-only rail with a separate, finger-sized gesture strip.
  const _MessageTickRail({
    required this.entries,
    required this.activeTimestamp,
    required this.geometry,
    required this.onHoverEntry,
    required this.onTapTick,
  });

  final List<ChatMessageLocatorPreview> entries;
  final int? activeTimestamp;
  final ChatScrollRailGeometry geometry;
  final ValueChanged<int?> onHoverEntry;
  final ValueChanged<int> onTapTick;

  /// Keeps pointer motion and animation outside the transcript widget tree.
  @override
  State<_MessageTickRail> createState() => _MessageTickRailState();
}

class _MessageTickRailState extends State<_MessageTickRail>
    with SingleTickerProviderStateMixin {
  final _pointer = ValueNotifier<Offset>(Offset.zero);
  late final AnimationController _proximity;
  late final Listenable _paintSignal;
  late List<_RailTick> _ticks;
  late Map<int, int> _entryIndices;
  int? _touchPointer;
  int? _previewIndex;
  double _targetProximity = 0;
  bool _disableAnimations = false;

  /// Allocates one animation and caches visual buckets for the whole history.
  @override
  void initState() {
    super.initState();
    _proximity = AnimationController(vsync: this);
    _paintSignal = Listenable.merge(<Listenable>[_pointer, _proximity]);
    _rebuildTicks();
  }

  /// Re-buckets only when history or viewport geometry actually changes.
  @override
  void didUpdateWidget(covariant _MessageTickRail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.entries, widget.entries) ||
        oldWidget.geometry.viewportHeight != widget.geometry.viewportHeight) {
      _rebuildTicks();
      _previewIndex = null;
    }
  }

  /// Releases the single repaint animation and pointer notifier.
  @override
  void dispose() {
    _proximity.dispose();
    _pointer.dispose();
    super.dispose();
  }

  /// Preserves active and favorite markers when many messages share one tick.
  void _rebuildTicks() {
    final geometry = widget.geometry;
    _entryIndices = <int, int>{
      for (final entry in widget.entries.indexed) entry.$2.timestamp: entry.$1,
    };
    _ticks = List<_RailTick>.generate(geometry.tickCount, (index) {
      final start = geometry.bucketStart(index);
      final end = geometry.bucketEnd(index);
      var favorite = false;
      for (var entry = start; entry < end; entry++) {
        favorite |= widget.entries[entry].isFavorite;
      }
      return _RailTick(
        start: start,
        end: end,
        isUser: widget.entries[(start + end - 1) ~/ 2].sender == 'user',
        isFavorite: favorite,
      );
    }, growable: false);
  }

  /// Animates only horizontal growth; vertical motion follows the pointer directly.
  void _animateProximity(double value) {
    if (_disableAnimations || value == 1) {
      _targetProximity = value;
      if (_proximity.value != value || _proximity.isAnimating) {
        _proximity.value = value;
      }
      return;
    }
    if ((_targetProximity - value).abs() < 0.005) return;
    _targetProximity = value;
    _proximity.animateTo(
      value,
      duration: Duration(milliseconds: value == 0 ? 160 : 70),
      curve: Curves.easeOutCubic,
    );
  }

  /// Repaints on every motion but rebuilds the preview only when its row changes.
  void _updatePointer(Offset position) {
    final geometry = widget.geometry;
    _pointer.value = position;
    final insideHeight =
        position.dy >= geometry.hitTop &&
        position.dy <= geometry.hitTop + geometry.hitHeight;
    _animateProximity(insideHeight ? geometry.proximityAt(position.dx) : 0);
    final index =
        insideHeight &&
            position.dx >=
                ChatScrollRailGeometry.interactionWidth -
                    ChatScrollRailGeometry.hitWidth &&
            position.dx <= ChatScrollRailGeometry.interactionWidth
        ? geometry.entryIndexAt(position.dy)
        : null;
    if (index != _previewIndex) {
      _previewIndex = index;
      widget.onHoverEntry(index);
    }
  }

  /// Collapses the wave after leaving the rail without changing scroll position.
  void _clearPointer() {
    _animateProximity(0);
    if (_previewIndex != null) {
      _previewIndex = null;
      widget.onHoverEntry(null);
    }
  }

  /// Shows touch previews immediately, before a tap or drag wins its gesture arena.
  void _handlePointerDown(PointerDownEvent event) {
    if (event.kind != PointerDeviceKind.mouse) {
      if (_touchPointer != null) return;
      _touchPointer = event.pointer;
    }
    _updatePointer(event.localPosition);
  }

  /// Scrubs previews without jumping through the transcript on drag updates.
  void _handlePointerMove(PointerMoveEvent event) {
    if (event.kind != PointerDeviceKind.mouse &&
        event.pointer != _touchPointer) {
      return;
    }
    _updatePointer(event.localPosition);
  }

  /// Clears touch state on both release and cancellation without hiding mouse hover.
  void _handlePointerEnd(PointerEvent event) {
    if (event.pointer != _touchPointer) return;
    _touchPointer = null;
    _clearPointer();
  }

  /// Observes proximity without consuming transcript gestures outside the hit strip.
  @override
  Widget build(BuildContext context) {
    _disableAnimations = MediaQuery.disableAnimationsOf(context);
    final geometry = widget.geometry;
    final colors = Theme.of(context).colorScheme;
    return MouseRegion(
      opaque: false,
      hitTestBehavior: HitTestBehavior.translucent,
      onHover: (event) => _updatePointer(event.localPosition),
      onExit: (_) {
        if (_touchPointer == null) _clearPointer();
      },
      child: Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: _handlePointerDown,
        onPointerMove: _handlePointerMove,
        onPointerUp: _handlePointerEnd,
        onPointerCancel: _handlePointerEnd,
        child: Stack(
          fit: StackFit.expand,
          children: <Widget>[
            Positioned.fill(
              child: IgnorePointer(
                child: RepaintBoundary(
                  child: CustomPaint(
                    key: const ValueKey<String>('chat-scroll-rail-paint'),
                    painter: _MessageTickPainter(
                      ticks: _ticks,
                      activeIndex: _entryIndices[widget.activeTimestamp],
                      geometry: geometry,
                      pointer: _pointer,
                      proximity: _proximity,
                      primary: colors.primary,
                      foreground: colors.onSurfaceVariant,
                      repaint: _paintSignal,
                    ),
                  ),
                ),
              ),
            ),
            Positioned(
              right: 0,
              top: geometry.hitTop,
              width: ChatScrollRailGeometry.hitWidth,
              height: geometry.hitHeight,
              child: MouseRegion(
                cursor: SystemMouseCursors.click,
                opaque: false,
                child: GestureDetector(
                  key: const ValueKey<String>('chat-scroll-rail-hit-target'),
                  behavior: HitTestBehavior.opaque,
                  onTapUp: (details) {
                    final index = geometry.entryIndexAt(
                      details.localPosition.dy + geometry.hitTop,
                    );
                    widget.onTapTick(widget.entries[index].timestamp);
                  },
                  // This strip owns scrubbing; raw motion only updates the preview.
                  onVerticalDragStart: (_) {},
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Describes one visual tick bucket without allocating a widget per message.
class _RailTick {
  /// Records the history interval and markers represented by a painted tick.
  const _RailTick({
    required this.start,
    required this.end,
    required this.isUser,
    required this.isFavorite,
  });

  final int start;
  final int end;
  final bool isUser;
  final bool isFavorite;

  /// Tests whether the active or previewed message belongs to this visual bucket.
  bool contains(int? index) => index != null && index >= start && index < end;
}

/// Draws the entire proximity wave in one bounded, independently repainted layer.
class _MessageTickPainter extends CustomPainter {
  /// Connects pointer and animation notifiers directly to painting, not rebuilding.
  _MessageTickPainter({
    required this.ticks,
    required this.activeIndex,
    required this.geometry,
    required this.pointer,
    required this.proximity,
    required this.primary,
    required this.foreground,
    required Listenable repaint,
  }) : super(repaint: repaint);

  final List<_RailTick> ticks;
  final int? activeIndex;
  final ChatScrollRailGeometry geometry;
  final ValueNotifier<Offset> pointer;
  final Animation<double> proximity;
  final Color primary;
  final Color foreground;

  /// Anchors tick ends while a capped bell-shaped wave extends them to the left.
  @override
  void paint(Canvas canvas, Size size) {
    final position = pointer.value;
    final strength = proximity.value;
    final hoveredIndex = strength > 0.01
        ? geometry.entryIndexAt(position.dy)
        : null;
    final right = size.width - ChatScrollRailGeometry.rightInset;
    final line = Paint();
    final favorite = Paint()..color = Colors.amber;
    for (var index = 0; index < ticks.length; index++) {
      final tick = ticks[index];
      final current = tick.contains(activeIndex);
      final hovered = tick.contains(hoveredIndex);
      final y = geometry.tickCenter(index);
      final restingWidth = current ? 15.0 : (tick.isUser ? 10.0 : 6.5);
      final width = geometry.tickWidth(
        restingWidth: restingWidth,
        tickY: y,
        pointerY: position.dy,
        proximity: strength,
      );
      line.color = current || hovered
          ? primary
          : foreground.withValues(alpha: tick.isUser ? 0.55 : 0.28);
      final thickness = current || hovered ? 2.8 : 1.8;
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          Rect.fromLTWH(right - width, y - thickness / 2, width, thickness),
          Radius.circular(thickness / 2),
        ),
        line,
      );
      if (tick.isFavorite) {
        canvas.drawCircle(Offset(right + 4, y), 1.2, favorite);
      }
    }
  }

  /// Lets the proximity observation area pass gestures through to the transcript.
  @override
  bool hitTest(Offset position) => false;

  /// Repaints static state only when history, viewport, selection or theme changes.
  @override
  bool shouldRepaint(covariant _MessageTickPainter oldDelegate) =>
      oldDelegate.ticks != ticks ||
      oldDelegate.activeIndex != activeIndex ||
      oldDelegate.geometry.viewportHeight != geometry.viewportHeight ||
      oldDelegate.primary != primary ||
      oldDelegate.foreground != foreground;
}

class _CodexHoverTooltipCard extends StatelessWidget {
  /// Shows the selected message preview immediately on hover or touch.
  const _CodexHoverTooltipCard({required this.entry});

  final ChatMessageLocatorPreview entry;

  /// Renders message identity and content without an extra navigation hint.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final isUser = entry.sender == 'user';
    final displayName = isUser ? '' : entry.displayName.trim();
    final title = displayName.isNotEmpty
        ? displayName
        : senderLabel(context, entry.sender);
    final previewText = buildMessagePreview(l10n, entry, '');

    return Material(
      color: Colors.transparent,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHigh.withValues(alpha: 0.95),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
          ),
          boxShadow: [
            BoxShadow(
              color: Colors.black.withValues(alpha: 0.16),
              blurRadius: 10,
              offset: const Offset(0, 3),
            ),
          ],
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(
                  isUser ? Icons.person_rounded : Icons.auto_awesome_rounded,
                  size: 11,
                  color: isUser
                      ? theme.colorScheme.primary
                      : theme.colorScheme.tertiary,
                ),
                const SizedBox(width: 4),
                Expanded(
                  child: Text(
                    isUser ? '$title #${(entry.messageIndex ?? 0) + 1}' : title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.labelSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                      fontSize: 10,
                      color: isUser
                          ? theme.colorScheme.primary
                          : theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                if (entry.isFavorite) ...[
                  const SizedBox(width: 4),
                  const Icon(Icons.star, size: 11, color: Colors.amber),
                ],
              ],
            ),
            const SizedBox(height: 3),
            Text(
              previewText,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                fontSize: 11,
                color: theme.colorScheme.onSurface,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ScrollToBottomCircleButton extends StatelessWidget {
  const _ScrollToBottomCircleButton({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: Colors.transparent,
      child: Tooltip(
        message: '回到底部',
        child: InkWell(
          borderRadius: BorderRadius.circular(14),
          onTap: onTap,
          child: Container(
            width: 28,
            height: 28,
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest.withValues(
                alpha: 0.75,
              ),
              shape: BoxShape.circle,
              border: Border.all(
                color: theme.colorScheme.outlineVariant.withValues(alpha: 0.22),
              ),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.12),
                  blurRadius: 6,
                  offset: const Offset(0, 2),
                ),
              ],
            ),
            child: Icon(
              Icons.keyboard_arrow_down_rounded,
              size: 16,
              color: theme.colorScheme.primary,
            ),
          ),
        ),
      ),
    );
  }
}
