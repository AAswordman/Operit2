// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../main/layout/NavigationLayoutMetrics.dart';
import '../viewmodel/ChatViewModel.dart';

typedef LoadMessageLocatorEntries =
    Future<List<ChatMessageLocatorPreview>> Function(
      String chatId,
      String query,
    );
typedef ToggleFavoriteMessage =
    Future<void> Function(int timestamp, bool isFavorite);

const double _navigatorRightInset = 16;

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
    required this.onJumpToMessage,
    required this.onToggleFavoriteMessage,
    required this.onRequestScrollToBottom,
    required this.showNavigatorChip,
    required this.onNavigatorChipHidden,
  });

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
  final ValueChanged<int> onJumpToMessage;
  final ToggleFavoriteMessage onToggleFavoriteMessage;
  final VoidCallback onRequestScrollToBottom;
  final bool showNavigatorChip;
  final VoidCallback onNavigatorChipHidden;

  @override
  State<ChatScrollNavigator> createState() => _ChatScrollNavigatorState();
}

class _ChatScrollNavigatorState extends State<ChatScrollNavigator> {
  int? _currentMessageIndex;
  List<ChatMessageLocatorPreview> _locatorEntries =
      const <ChatMessageLocatorPreview>[];
  bool _isLoadingLocatorEntries = false;

  @override
  void initState() {
    super.initState();
    _currentMessageIndex = widget.messages.isEmpty
        ? null
        : widget.messages.length - 1;
    _loadLocatorEntries();
  }

  @override
  void didUpdateWidget(ChatScrollNavigator oldWidget) {
    super.didUpdateWidget(oldWidget);
    final messagesChanged =
        oldWidget.messages.length != widget.messages.length ||
        oldWidget.messages.firstOrNull?.timestamp !=
            widget.messages.firstOrNull?.timestamp ||
        oldWidget.messages.lastOrNull?.timestamp !=
            widget.messages.lastOrNull?.timestamp;
    if (messagesChanged || oldWidget.currentChatId != widget.currentChatId) {
      _loadLocatorEntries();
    }
    _updateCenteredMessage();
  }

  @override
  Widget build(BuildContext context) {
    _updateCenteredMessage();
    final activeMessageIndex = _currentMessageIndex;
    final activeMessageTimestamp = activeMessageIndex == null
        ? null
        : widget.messages.elementAtOrNull(activeMessageIndex)?.timestamp;
    final activeGlobalMessageIndex = activeMessageTimestamp == null
        ? null
        : _locatorEntries.indexWhere(
            (entry) => entry.timestamp == activeMessageTimestamp,
          );

    return Stack(
      children: <Widget>[
        Positioned(
          right: _navigatorRightInset,
          top: (widget.viewportHeight / 2) - 57,
          child: AnimatedSwitcher(
            duration: widget.showNavigatorChip
                ? const Duration(milliseconds: 180)
                : const Duration(milliseconds: 120),
            switchInCurve: Curves.easeOutCubic,
            switchOutCurve: Curves.easeInCubic,
            transitionBuilder: (child, animation) {
              final offsetAnimation = Tween<Offset>(
                begin: const Offset(0.5, 0),
                end: Offset.zero,
              ).animate(animation);
              return FadeTransition(
                opacity: animation,
                child: SlideTransition(position: offsetAnimation, child: child),
              );
            },
            child: widget.showNavigatorChip && activeMessageIndex != null
                ? _NavigatorChip(
                    key: const ValueKey<String>('chat-scroll-navigator-chip'),
                    progress: _progress(
                      activeGlobalMessageIndex,
                      activeMessageIndex,
                    ),
                    onOpenLocator: () {
                      widget.onNavigatorChipHidden();
                      _showLocatorDialog(activeMessageTimestamp);
                    },
                    onScrollToBottom: widget.onRequestScrollToBottom,
                  )
                : const SizedBox(
                    key: ValueKey<String>('chat-scroll-navigator-empty'),
                  ),
          ),
        ),
      ],
    );
  }

  Future<void> _showLocatorDialog(int? activeMessageTimestamp) async {
    if (activeMessageTimestamp == null) {
      return;
    }
    await showDialog<void>(
      context: context,
      builder: (dialogContext) {
        return ChatMessageLocatorDialog(
          locatorEntries: _locatorEntries,
          currentMessageTimestamp: activeMessageTimestamp,
          isLoading: _isLoadingLocatorEntries,
          currentChatId: widget.currentChatId,
          loadLocatorEntries: widget.loadLocatorEntries,
          onDismiss: () {
            Navigator.of(dialogContext).pop();
          },
          onToggleFavoriteMessage: widget.onToggleFavoriteMessage,
          onJumpToMessage: (timestamp) {
            Navigator.of(dialogContext).pop();
            widget.onJumpToMessage(timestamp);
          },
        );
      },
    );
  }

  double _progress(int? activeGlobalMessageIndex, int activeMessageIndex) {
    final totalCount = _locatorEntries.isNotEmpty
        ? _locatorEntries.length
        : widget.messages.length;
    final progressIndex =
        activeGlobalMessageIndex != null && activeGlobalMessageIndex >= 0
        ? activeGlobalMessageIndex
        : activeMessageIndex;
    if (totalCount <= 1) {
      return 1;
    }
    return (progressIndex / (totalCount - 1)).clamp(0, 1).toDouble();
  }

  void _updateCenteredMessage() {
    if (widget.viewportHeight <= 0 || widget.messageAnchors.isEmpty) {
      return;
    }
    final viewportCenter =
        widget.scrollController.offset + widget.viewportHeight / 2;
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
    final index = centeredAnchor?.index;
    if (index != null && index != _currentMessageIndex) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) {
          return;
        }
        setState(() {
          _currentMessageIndex = index;
        });
      });
    }
  }

  Future<void> _loadLocatorEntries() async {
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) {
      setState(() {
        _locatorEntries = const <ChatMessageLocatorPreview>[];
        _isLoadingLocatorEntries = false;
      });
      return;
    }
    setState(() {
      _isLoadingLocatorEntries = true;
    });
    final entries = await widget.loadLocatorEntries(chatId, '');
    if (!mounted) {
      return;
    }
    setState(() {
      _locatorEntries = entries;
      _isLoadingLocatorEntries = false;
    });
  }
}

class _NavigatorChip extends StatelessWidget {
  const _NavigatorChip({
    super.key,
    required this.progress,
    required this.onOpenLocator,
    required this.onScrollToBottom,
  });

  final double progress;
  final VoidCallback onOpenLocator;
  final VoidCallback onScrollToBottom;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final bubbleColor = theme.colorScheme.surfaceContainerHighest.withValues(
      alpha: 0.58,
    );
    final borderColor = theme.colorScheme.outlineVariant.withValues(
      alpha: 0.12,
    );
    final lineColor = theme.colorScheme.outlineVariant.withValues(alpha: 0.55);
    final dotColor = theme.colorScheme.primary.withValues(alpha: 0.92);

    return SizedBox(
      width: 34,
      height: 114,
      child: Stack(
        children: <Widget>[
          Positioned(
            left: 4.5,
            top: 28,
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: onOpenLocator,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Container(
                    width: 20,
                    height: 58,
                    decoration: BoxDecoration(
                      color: bubbleColor,
                      border: Border.all(color: borderColor),
                      borderRadius: const BorderRadius.horizontal(
                        left: Radius.circular(14),
                        right: Radius.circular(10),
                      ),
                    ),
                    child: Center(
                      child: CustomPaint(
                        size: const Size(8, 34),
                        painter: _ProgressRailPainter(
                          progress: progress,
                          lineColor: lineColor,
                          dotColor: dotColor,
                        ),
                      ),
                    ),
                  ),
                  Transform.translate(
                    offset: const Offset(-1, 0),
                    child: CustomPaint(
                      size: const Size(9, 18),
                      painter: _NavigatorArrowPainter(color: bubbleColor),
                    ),
                  ),
                ],
              ),
            ),
          ),
          Positioned(
            left: 2.5,
            top: 90,
            child: GestureDetector(
              onTap: onScrollToBottom,
              child: Container(
                width: 24,
                height: 24,
                decoration: BoxDecoration(
                  color: bubbleColor,
                  shape: BoxShape.circle,
                  border: Border.all(color: borderColor),
                ),
                child: Icon(
                  Icons.keyboard_arrow_down,
                  size: 16,
                  color: theme.colorScheme.primary.withValues(alpha: 0.92),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ProgressRailPainter extends CustomPainter {
  const _ProgressRailPainter({
    required this.progress,
    required this.lineColor,
    required this.dotColor,
  });

  final double progress;
  final Color lineColor;
  final Color dotColor;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = lineColor
      ..strokeWidth = 1.5
      ..strokeCap = StrokeCap.round;
    final centerX = size.width / 2;
    const topY = 2.0;
    final bottomY = size.height - 2;
    canvas.drawLine(Offset(centerX, topY), Offset(centerX, bottomY), paint);
    canvas.drawCircle(
      Offset(centerX, topY + (bottomY - topY) * progress),
      3,
      Paint()..color = dotColor,
    );
  }

  @override
  bool shouldRepaint(_ProgressRailPainter oldDelegate) {
    return oldDelegate.progress != progress ||
        oldDelegate.lineColor != lineColor ||
        oldDelegate.dotColor != dotColor;
  }
}

class _NavigatorArrowPainter extends CustomPainter {
  const _NavigatorArrowPainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final path = Path()
      ..moveTo(0, 0)
      ..lineTo(size.width, size.height / 2)
      ..lineTo(0, size.height)
      ..close();
    canvas.drawPath(path, Paint()..color = color);
  }

  @override
  bool shouldRepaint(_NavigatorArrowPainter oldDelegate) {
    return oldDelegate.color != color;
  }
}

class ChatMessageLocatorDialog extends StatefulWidget {
  const ChatMessageLocatorDialog({
    super.key,
    required this.locatorEntries,
    required this.currentMessageTimestamp,
    required this.isLoading,
    required this.currentChatId,
    required this.loadLocatorEntries,
    required this.onDismiss,
    required this.onToggleFavoriteMessage,
    required this.onJumpToMessage,
  });

  final List<ChatMessageLocatorPreview> locatorEntries;
  final int currentMessageTimestamp;
  final bool isLoading;
  final String? currentChatId;
  final LoadMessageLocatorEntries loadLocatorEntries;
  final VoidCallback onDismiss;
  final ToggleFavoriteMessage onToggleFavoriteMessage;
  final ValueChanged<int> onJumpToMessage;

  @override
  State<ChatMessageLocatorDialog> createState() =>
      _ChatMessageLocatorDialogState();
}

class _ChatMessageLocatorDialogState extends State<ChatMessageLocatorDialog> {
  final ScrollController _scrollController = ScrollController();
  final TextEditingController _searchController = TextEditingController();
  Timer? _searchDebounce;
  String _searchQuery = '';
  List<ChatMessageLocatorPreview> _searchEntries =
      const <ChatMessageLocatorPreview>[];
  bool _isLoadingSearchEntries = false;
  bool _favoritesOnly = false;
  Map<int, bool> _favoriteOverrides = const <int, bool>{};

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _scrollToInitialRow());
  }

  @override
  void dispose() {
    _searchDebounce?.cancel();
    _scrollController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final normalizedSearchQuery = normalizeMessageSearchText(_searchQuery);
    final activeEntries = normalizedSearchQuery.isEmpty
        ? widget.locatorEntries
        : _searchEntries;
    final indexedEntries = activeEntries.indexed
        .map(
          (entry) => _ChatMessageLocatorEntry(
            index: entry.$2.messageIndex ?? entry.$1,
            preview: entry.$2,
          ),
        )
        .toList(growable: false);
    final filteredEntries = indexedEntries
        .where((entry) {
          final isFavorite =
              _favoriteOverrides[entry.preview.timestamp] ??
              entry.preview.isFavorite;
          return !_favoritesOnly || isFavorite;
        })
        .toList(growable: false);
    final currentMessageIndex = widget.locatorEntries.indexWhere(
      (entry) => entry.timestamp == widget.currentMessageTimestamp,
    );
    final maxMessageLength = activeEntries
        .map((preview) => messageContentLength(l10n, preview))
        .fold<int>(1, (max, value) => math.max(max, value));
    final dialogIsLoading = widget.isLoading || _isLoadingSearchEntries;
    final useTabletLayout = useTabletLayoutForContext(context);
    final dialogContent = Material(
      color: theme.colorScheme.surface.withValues(alpha: 0.95),
      borderRadius: useTabletLayout ? BorderRadius.circular(24) : null,
      clipBehavior: Clip.antiAlias,
      child: SafeArea(
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 18),
          child: Column(
            mainAxisSize: useTabletLayout ? MainAxisSize.min : MainAxisSize.max,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: <Widget>[
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        l10n.messageLocatorTitle,
                        style: theme.textTheme.titleMedium,
                      ),
                      Text(
                        l10n.messageLocatorCurrent(
                          (currentMessageIndex + 1).clamp(
                            0,
                            widget.locatorEntries.length,
                          ),
                          widget.locatorEntries.length,
                        ),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  TextButton(
                    onPressed: widget.onDismiss,
                    child: Text(l10n.close),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Row(
                children: <Widget>[
                  Expanded(
                    child: TextField(
                      controller: _searchController,
                      maxLines: 1,
                      decoration: InputDecoration(
                        labelText: l10n.search,
                        hintText: l10n.messageLocatorSearchHint,
                        border: const OutlineInputBorder(),
                      ),
                      onChanged: _onSearchChanged,
                    ),
                  ),
                  const SizedBox(width: 10),
                  _FavoriteFilterButton(
                    selected: _favoritesOnly,
                    onPressed: () {
                      setState(() {
                        _favoritesOnly = !_favoritesOnly;
                      });
                    },
                  ),
                ],
              ),
              const SizedBox(height: 12),
              if (normalizedSearchQuery.isEmpty && !_favoritesOnly)
                Text(
                  l10n.messageLocatorInstruction,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                )
              else if (filteredEntries.isNotEmpty)
                Text(
                  l10n.messageLocatorResultCount(filteredEntries.length),
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              const SizedBox(height: 8),
              Flexible(
                child: useTabletLayout
                    ? SizedBox(
                        height: 420,
                        child: _buildEntryList(
                          context,
                          dialogIsLoading,
                          filteredEntries,
                          currentMessageIndex,
                          maxMessageLength,
                        ),
                      )
                    : _buildEntryList(
                        context,
                        dialogIsLoading,
                        filteredEntries,
                        currentMessageIndex,
                        maxMessageLength,
                      ),
              ),
            ],
          ),
        ),
      ),
    );

    if (!useTabletLayout) {
      return Dialog.fullscreen(child: dialogContent);
    }

    return Dialog(
      insetPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720, maxHeight: 560),
        child: dialogContent,
      ),
    );
  }

  Widget _buildEntryList(
    BuildContext context,
    bool dialogIsLoading,
    List<_ChatMessageLocatorEntry> filteredEntries,
    int currentMessageIndex,
    int maxMessageLength,
  ) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    if (dialogIsLoading) {
      return Center(
        child: Text(
          l10n.loading,
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      );
    }
    if (filteredEntries.isEmpty) {
      return Center(
        child: Text(
          l10n.messageLocatorNoMatches,
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      );
    }
    return ListView.separated(
      controller: _scrollController,
      itemCount: filteredEntries.length,
      separatorBuilder: (context, index) => const SizedBox(height: 8),
      itemBuilder: (context, index) {
        final entry = filteredEntries[index];
        final isFavorite =
            _favoriteOverrides[entry.preview.timestamp] ??
            entry.preview.isFavorite;
        return ChatMessageLocatorRow(
          key: ValueKey<String>('${entry.preview.timestamp}_${entry.index}'),
          index: entry.index,
          preview: entry.preview,
          isFavorite: isFavorite,
          isCurrent: entry.index == currentMessageIndex,
          maxMessageLength: maxMessageLength,
          searchQuery: _searchQuery,
          onToggleFavorite: () => _toggleFavorite(entry.preview, isFavorite),
          onClick: () => widget.onJumpToMessage(entry.preview.timestamp),
        );
      },
    );
  }

  void _onSearchChanged(String value) {
    setState(() {
      _searchQuery = value;
    });
    _searchDebounce?.cancel();
    final normalizedSearchQuery = normalizeMessageSearchText(value);
    if (normalizedSearchQuery.isEmpty) {
      setState(() {
        _searchEntries = const <ChatMessageLocatorPreview>[];
        _isLoadingSearchEntries = false;
      });
      return;
    }
    _searchDebounce = Timer(const Duration(milliseconds: 180), () {
      _loadSearchEntries(normalizedSearchQuery);
    });
  }

  Future<void> _loadSearchEntries(String normalizedSearchQuery) async {
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) {
      setState(() {
        _searchEntries = const <ChatMessageLocatorPreview>[];
        _isLoadingSearchEntries = false;
      });
      return;
    }
    setState(() {
      _isLoadingSearchEntries = true;
    });
    try {
      final entries = await widget.loadLocatorEntries(
        chatId,
        normalizedSearchQuery,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _searchEntries = entries;
        _isLoadingSearchEntries = false;
      });
      _scrollToClosestSearchRow();
    } catch (error, stackTrace) {
      debugPrint('Failed to search chat locator entries: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _isLoadingSearchEntries = false;
      });
    }
  }

  Future<void> _toggleFavorite(
    ChatMessageLocatorPreview preview,
    bool isFavorite,
  ) async {
    final nextFavorite = !isFavorite;
    setState(() {
      _favoriteOverrides = Map<int, bool>.of(_favoriteOverrides)
        ..[preview.timestamp] = nextFavorite;
    });
    await widget.onToggleFavoriteMessage(preview.timestamp, nextFavorite);
  }

  void _scrollToInitialRow() {
    final currentMessageIndex = widget.locatorEntries.indexWhere(
      (entry) => entry.timestamp == widget.currentMessageTimestamp,
    );
    if (currentMessageIndex < 0 || !_scrollController.hasClients) {
      return;
    }
    final target = (currentMessageIndex - 2).clamp(
      0,
      widget.locatorEntries.length,
    );
    _scrollController.jumpTo(target * 56);
  }

  void _scrollToClosestSearchRow() {
    final currentMessageIndex = widget.locatorEntries.indexWhere(
      (entry) => entry.timestamp == widget.currentMessageTimestamp,
    );
    if (currentMessageIndex < 0 ||
        _searchEntries.isEmpty ||
        !_scrollController.hasClients) {
      return;
    }
    final target = _searchEntries.indexed
        .map(
          (entry) => MapEntry(
            entry.$1,
            ((entry.$2.messageIndex ?? entry.$1) - currentMessageIndex).abs(),
          ),
        )
        .reduce((left, right) => left.value <= right.value ? left : right)
        .key;
    _scrollController.jumpTo(target * 56);
  }
}

class _FavoriteFilterButton extends StatelessWidget {
  const _FavoriteFilterButton({
    required this.selected,
    required this.onPressed,
  });

  final bool selected;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: selected
          ? theme.colorScheme.primaryContainer
          : theme.colorScheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(
          color: selected
              ? theme.colorScheme.primary.withValues(alpha: 0.42)
              : theme.colorScheme.outlineVariant.withValues(alpha: 0.32),
        ),
      ),
      child: IconButton(
        onPressed: onPressed,
        icon: Icon(selected ? Icons.star : Icons.star_outline),
        color: selected
            ? theme.colorScheme.primary
            : theme.colorScheme.onSurfaceVariant,
      ),
    );
  }
}

class ChatMessageLocatorRow extends StatelessWidget {
  const ChatMessageLocatorRow({
    super.key,
    required this.index,
    required this.preview,
    required this.isFavorite,
    required this.isCurrent,
    required this.maxMessageLength,
    required this.searchQuery,
    required this.onToggleFavorite,
    required this.onClick,
  });

  final int index;
  final ChatMessageLocatorPreview preview;
  final bool isFavorite;
  final bool isCurrent;
  final int maxMessageLength;
  final String searchQuery;
  final VoidCallback onToggleFavorite;
  final VoidCallback onClick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDarkSurface = theme.colorScheme.surface.computeLuminance() < 0.5;
    final fillColor = _fillColor(theme, isDarkSurface);
    final previewTextColor = isDarkSurface
        ? theme.colorScheme.onSurface.withValues(alpha: isCurrent ? 0.96 : 0.88)
        : _previewTextColor(theme);
    final containerColor = isCurrent
        ? theme.colorScheme.secondaryContainer.withValues(alpha: 0.72)
        : theme.colorScheme.surfaceContainerLow;
    final borderColor = isCurrent
        ? theme.colorScheme.primary.withValues(alpha: 0.4)
        : theme.colorScheme.outlineVariant.withValues(alpha: 0.22);
    final l10n = AppLocalizations.of(context)!;
    final messageLength = messageContentLength(l10n, preview);
    final previewText = buildMessagePreview(l10n, preview, searchQuery);

    return Material(
      color: containerColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(18),
        side: BorderSide(color: borderColor),
      ),
      child: InkWell(
        borderRadius: BorderRadius.circular(18),
        onTap: onClick,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: Row(
            children: <Widget>[
              SizedBox(
                width: 90,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Row(
                      children: <Widget>[
                        Text('${index + 1}', style: theme.textTheme.titleSmall),
                        SizedBox(
                          width: 24,
                          height: 24,
                          child: IconButton(
                            padding: EdgeInsets.zero,
                            iconSize: 13,
                            onPressed: onToggleFavorite,
                            icon: Icon(
                              isFavorite ? Icons.star : Icons.star_outline,
                              color: isFavorite
                                  ? theme.colorScheme.primary
                                  : theme.colorScheme.onSurfaceVariant
                                        .withValues(alpha: 0.72),
                            ),
                          ),
                        ),
                      ],
                    ),
                    Text(
                      senderLabel(context, preview.sender),
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              Expanded(
                child: Container(
                  height: 38,
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerHighest.withValues(
                      alpha: 0.42,
                    ),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Stack(
                    children: <Widget>[
                      FractionallySizedBox(
                        widthFactor: messageBarFraction(
                          messageLength,
                          maxMessageLength,
                        ),
                        heightFactor: 1,
                        child: Container(
                          decoration: BoxDecoration(
                            color: fillColor.withValues(
                              alpha: isDarkSurface ? 0.9 : 0.98,
                            ),
                            borderRadius: BorderRadius.circular(12),
                          ),
                        ),
                      ),
                      Align(
                        alignment: Alignment.centerLeft,
                        child: Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 10),
                          child: Text(
                            previewText,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.labelMedium?.copyWith(
                              color: previewTextColor,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Text(
                messageLength.toString(),
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Color _fillColor(ThemeData theme, bool isDarkSurface) {
    if (isDarkSurface) {
      switch (preview.sender) {
        case 'user':
          return theme.colorScheme.primaryContainer;
        case 'summary':
          return theme.colorScheme.tertiaryContainer;
        case 'system':
          return theme.colorScheme.secondaryContainer;
        case 'think':
          return theme.colorScheme.surfaceContainerHighest;
        default:
          return theme.colorScheme.secondaryContainer;
      }
    }
    switch (preview.sender) {
      case 'user':
        return theme.colorScheme.primaryContainer;
      case 'ai':
        return theme.colorScheme.tertiaryContainer;
      case 'summary':
        return theme.colorScheme.secondaryContainer;
      case 'system':
      case 'think':
        return theme.colorScheme.surfaceContainerHighest;
      default:
        return theme.colorScheme.secondaryContainer;
    }
  }

  Color _previewTextColor(ThemeData theme) {
    switch (preview.sender) {
      case 'user':
        return theme.colorScheme.onPrimaryContainer;
      case 'ai':
        return theme.colorScheme.onTertiaryContainer;
      case 'summary':
      case 'system':
      case 'think':
      default:
        return theme.colorScheme.onSecondaryContainer;
    }
  }
}

class _ChatMessageLocatorEntry {
  const _ChatMessageLocatorEntry({required this.index, required this.preview});

  final int index;
  final ChatMessageLocatorPreview preview;
}

String senderLabel(BuildContext context, String sender) {
  final l10n = AppLocalizations.of(context)!;
  switch (sender) {
    case 'user':
      return l10n.messageSenderUser;
    case 'ai':
      return 'AI';
    case 'summary':
      return l10n.messageSenderSummary;
    case 'system':
      return l10n.messageSenderSystem;
    case 'think':
      return l10n.messageSenderThinking;
    default:
      return l10n.messageSenderOther;
  }
}

String visibleLocatorContent(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
) {
  if (preview.sender == 'user' && preview.displayMode == 'HIDDEN_PLACEHOLDER') {
    return l10n.hiddenUserMessage;
  }
  return preview.previewContent;
}

int messageContentLength(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
) {
  if (preview.contentLength > 0) {
    return preview.contentLength;
  }
  return math.max(visibleLocatorContent(l10n, preview).length, 1);
}

double messageBarFraction(int messageLength, int maxMessageLength) {
  if (maxMessageLength <= 0) {
    return 0.18;
  }
  return math.sqrt(messageLength / maxMessageLength).clamp(0.18, 1).toDouble();
}

String buildMessagePreview(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
  String searchQuery,
) {
  final content = normalizeMessageSearchText(
    visibleLocatorContent(l10n, preview),
  );
  if (content.isEmpty) {
    return preview.sender;
  }
  final normalizedSearchQuery = normalizeMessageSearchText(searchQuery);
  if (normalizedSearchQuery.isNotEmpty) {
    final matchIndex = content.toLowerCase().indexOf(
      normalizedSearchQuery.toLowerCase(),
    );
    if (matchIndex >= 0) {
      const previewLength = 72;
      final preferredStart = math.max(matchIndex - 18, 0);
      final start = math.min(
        preferredStart,
        math.max(content.length - previewLength, 0),
      );
      final end = math.min(start + previewLength, content.length);
      final prefix = start > 0 ? '...' : '';
      final suffix = end < content.length ? '...' : '';
      final snippet = content.substring(start, end).trim();
      if (snippet.isNotEmpty) {
        return '$prefix$snippet$suffix';
      }
    }
  }
  final previewText = content.length > 72 ? content.substring(0, 72) : content;
  return previewText.length < content.length
      ? '${previewText.trimRight()}...'
      : previewText;
}

String normalizeMessageSearchText(String text) {
  if (text.isEmpty) {
    return '';
  }
  final buffer = StringBuffer();
  var pendingWhitespace = false;
  for (final codePoint in text.runes) {
    final char = String.fromCharCode(codePoint);
    final normalizedChar = char == '\n' || char == '\r' || char == '\t'
        ? ' '
        : char;
    if (normalizedChar.trim().isEmpty) {
      pendingWhitespace = buffer.isNotEmpty;
      continue;
    }
    if (pendingWhitespace) {
      buffer.write(' ');
      pendingWhitespace = false;
    }
    buffer.write(normalizedChar);
  }
  return buffer.toString().trim();
}
