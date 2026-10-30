// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart' show ScrollDirection;

import '../../../../../l10n/generated/app_localizations.dart';

import '../../../../../util/ChatMarkupRegex.dart';
import '../../../../common/interactions/MessagePressShield.dart';
import '../../../../common/markdown/MarkdownNodeGrouper.dart';

const Duration _groupFadeDuration = Duration(milliseconds: 800);
const Duration _contentFadeDuration = Duration(milliseconds: 200);
const Duration _arrowRotationDuration = Duration(milliseconds: 300);
const Duration _instantDuration = Duration.zero;

class ThinkToolsXmlNodeGrouper extends MarkdownNodeGrouper {
  const ThinkToolsXmlNodeGrouper({
    required this.showThinkingProcess,
    this.forceExpandGroups = false,
    this.defaultCompletedDuration,
  });

  final bool showThinkingProcess;
  final bool forceExpandGroups;
  final Duration? defaultCompletedDuration;

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        other is ThinkToolsXmlNodeGrouper &&
            other.showThinkingProcess == showThinkingProcess &&
            other.forceExpandGroups == forceExpandGroups &&
            other.defaultCompletedDuration == defaultCompletedDuration;
  }

  @override
  int get hashCode {
    return Object.hash(
      showThinkingProcess,
      forceExpandGroups,
      defaultCompletedDuration,
    );
  }

  @override
  List<MarkdownGroupedItem> group(
    List<MarkdownNodeStable> nodes,
    String rendererId,
  ) {
    // Group bodies render XML only, so visible Markdown is a hard boundary.
    final out = <MarkdownGroupedItem>[];
    var i = 0;
    while (i < nodes.length) {
      final node = nodes[i];

      if (node.type != MarkdownNodeType.xmlBlock) {
        out.add(MarkdownSingleItem(i));
        i++;
        continue;
      }

      final tag = _extractXmlTagName(node);

      if (showThinkingProcess && (tag == 'think' || tag == 'thinking')) {
        var j = i + 1;
        var toolCount = 0;
        var searchCount = 0;
        var xmlToolRelatedCount = 0;
        while (j < nodes.length) {
          final next = nodes[j];
          if (_isToolGroupingSeparatorNode(next)) {
            j++;
            continue;
          }
          if (next.type != MarkdownNodeType.xmlBlock) {
            break;
          }

          final nextTag = _extractXmlTagName(next);
          if (_isIgnorableXmlTagForToolGrouping(nextTag)) {
            j++;
            continue;
          }
          final isThinkAgain = nextTag == 'think' || nextTag == 'thinking';
          final isToolRelated = nextTag == 'tool' || nextTag == 'tool_result';
          final isSearchRelated = nextTag == 'search';
          if (!isThinkAgain && !isToolRelated && !isSearchRelated) {
            break;
          }

          if (isToolRelated) {
            if (nextTag == 'tool') {
              toolCount++;
            }
            xmlToolRelatedCount++;
          }
          if (isSearchRelated) {
            searchCount++;
            xmlToolRelatedCount++;
          }

          j++;
        }

        if (_shouldCollapseThinkSequence(
          toolCount,
          searchCount,
          xmlToolRelatedCount,
        )) {
          out.add(
            MarkdownGroupItem(
              startIndex: i,
              endIndexInclusive: j - 1,
              stableKey: 'think-tools-$i',
            ),
          );
          i = j;
          continue;
        }

        out.add(MarkdownSingleItem(i));
        i++;
        continue;
      }

      if (tag == 'tool' || tag == 'tool_result') {
        var j = i + 1;
        var toolCount = tag == 'tool' ? 1 : 0;
        var xmlToolRelatedCount = 1;

        while (j < nodes.length) {
          final next = nodes[j];
          if (_isToolGroupingSeparatorNode(next)) {
            j++;
            continue;
          }
          if (next.type != MarkdownNodeType.xmlBlock) {
            break;
          }

          final nextTag = _extractXmlTagName(next);
          if (_isIgnorableXmlTagForToolGrouping(nextTag)) {
            j++;
            continue;
          }
          final isToolRelated = nextTag == 'tool' || nextTag == 'tool_result';
          if (!isToolRelated) {
            break;
          }

          xmlToolRelatedCount++;
          if (nextTag == 'tool') {
            toolCount++;
          }
          j++;
        }

        if (_shouldCollapseToolSequence(toolCount, xmlToolRelatedCount)) {
          out.add(
            MarkdownGroupItem(
              startIndex: i,
              endIndexInclusive: j - 1,
              stableKey: 'tools-only-$i',
            ),
          );
          i = j;
        } else {
          out.add(MarkdownSingleItem(i));
          i++;
        }
        continue;
      }

      if (tag == 'search') {
        var j = i + 1;

        while (j < nodes.length) {
          final next = nodes[j];
          if (_isToolGroupingSeparatorNode(next)) {
            j++;
            continue;
          }
          if (next.type != MarkdownNodeType.xmlBlock) {
            break;
          }

          final nextTag = _extractXmlTagName(next);
          if (_isIgnorableXmlTagForToolGrouping(nextTag)) {
            j++;
            continue;
          }
          if (nextTag != 'search') {
            break;
          }

          j++;
        }

        out.add(
          MarkdownGroupItem(
            startIndex: i,
            endIndexInclusive: j - 1,
            stableKey: 'search-only-$i',
          ),
        );
        i = j;
        continue;
      }

      out.add(MarkdownSingleItem(i));
      i++;
    }

    return out;
  }

  @override
  Widget renderGroup({
    required MarkdownGroupItem group,
    required List<MarkdownNodeStable> nodes,
    required String rendererId,
    required bool isVisible,
    required bool isLastNode,
    required Color textColor,
    required MarkdownXmlRenderer xmlRenderer,
    required MarkdownNodeRangeMergeRender mergeRender,
    required Stream<String>? Function(int index) xmlStreamResolver,
    required Stream<Object>? Function(int index) xmlMarkdownEventStreamResolver,
    required void Function(String url)? onLinkClick,
    required bool fillMaxWidth,
    required TextStyle textStyle,
  }) {
    return _ThinkToolsXmlGroup(
      key: ValueKey<String>('group-$rendererId-${group.stableKey}'),
      group: group,
      nodes: nodes,
      rendererId: rendererId,
      isVisible: isVisible,
      isLastNode: isLastNode,
      textColor: textColor,
      mergeRender: mergeRender,
      xmlStreamResolver: xmlStreamResolver,
      forceExpandGroups: forceExpandGroups,
      defaultCompletedDuration: defaultCompletedDuration,
    );
  }
}

class _ThinkToolsXmlGroup extends StatefulWidget {
  const _ThinkToolsXmlGroup({
    super.key,
    required this.group,
    required this.nodes,
    required this.rendererId,
    required this.isVisible,
    required this.isLastNode,
    required this.textColor,
    required this.mergeRender,
    required this.xmlStreamResolver,
    required this.forceExpandGroups,
    this.defaultCompletedDuration,
  });

  final MarkdownGroupItem group;
  final List<MarkdownNodeStable> nodes;
  final String rendererId;
  final bool isVisible;
  final bool isLastNode;
  final Color textColor;
  final MarkdownNodeRangeMergeRender mergeRender;
  final Stream<String>? Function(int index) xmlStreamResolver;
  final bool forceExpandGroups;
  final Duration? defaultCompletedDuration;

  @override
  State<_ThinkToolsXmlGroup> createState() => _ThinkToolsXmlGroupState();
}

class _ThinkToolsXmlGroupState extends State<_ThinkToolsXmlGroup> {
  static const double _maxBodyHeight = 420;
  static final Map<String, Duration> _measuredGroupDurations =
      <String, Duration>{};

  String? _stateKey;
  bool _expanded = false;
  bool? _userOverride;
  bool _wasExecutionActive = false;
  bool _followLatest = true;
  bool _scrollScheduled = false;
  ScrollDirection _bodyScrollDirection = ScrollDirection.idle;
  final Set<int> _activeBodyScrollPointers = <int>{};
  final ScrollController _bodyScrollController = ScrollController();
  final Stopwatch _stopwatch = Stopwatch();
  Timer? _tickerTimer;
  Duration? _completedDuration;
  bool _wasStreaming = false;

  void _recordCompletedDuration() {
    if (_stopwatch.isRunning) {
      _stopwatch.stop();
    }
    if (_wasStreaming && _stopwatch.elapsed.inSeconds > 0) {
      _completedDuration = _stopwatch.elapsed;
      final key = '${widget.rendererId}-${widget.group.stableKey}';
      if (_measuredGroupDurations.length > 500) {
        _measuredGroupDurations.remove(_measuredGroupDurations.keys.first);
      }
      _measuredGroupDurations[key] = _stopwatch.elapsed;
    }
    _tickerTimer?.cancel();
    _tickerTimer = null;
  }

  @override
  void dispose() {
    _recordCompletedDuration();
    _bodyScrollController.dispose();
    super.dispose();
  }

  void _scheduleFollowLatest() {
    if (_scrollScheduled ||
        !_expanded ||
        !_wasExecutionActive ||
        !_followLatest ||
        _activeBodyScrollPointers.isNotEmpty) {
      return;
    }
    _scrollScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scrollScheduled = false;
      if (!mounted ||
          !_expanded ||
          !_wasExecutionActive ||
          !_followLatest ||
          _activeBodyScrollPointers.isNotEmpty ||
          !_bodyScrollController.hasClients ||
          _bodyScrollController.position.isScrollingNotifier.value) {
        return;
      }
      final position = _bodyScrollController.position;
      if (position.pixels != position.maxScrollExtent) {
        _bodyScrollController.jumpTo(position.maxScrollExtent);
      }
    });
    WidgetsBinding.instance.ensureVisualUpdate();
  }

  /// Holds off queued jumps while touch or trackpad gestures take ownership.
  void _handleBodyPointerStart(PointerEvent event) {
    _activeBodyScrollPointers.add(event.pointer);
  }

  void _handleBodyPointerEnd(PointerEvent event) {
    _activeBodyScrollPointers.remove(event.pointer);
    _scheduleFollowLatest();
  }

  bool _handleBodyScrollNotification(ScrollNotification notification) {
    // Shrink-wrapped viewports can leave nested notifications at depth zero,
    // so also verify which scrollable actually sent the notification.
    final source = notification.context;
    if (notification.depth != 0 ||
        source == null ||
        !_bodyScrollController.hasClients ||
        Scrollable.maybeOf(source)?.position !=
            _bodyScrollController.position) {
      return false;
    }
    if (notification is UserScrollNotification) {
      _bodyScrollDirection = notification.direction;
      if (_bodyScrollDirection == ScrollDirection.forward) {
        _followLatest = false;
      } else if (_bodyScrollDirection == ScrollDirection.reverse) {
        _followLatest = notification.metrics.extentAfter <= 1;
      }
    } else if (notification is ScrollUpdateNotification &&
        (_bodyScrollDirection != ScrollDirection.idle ||
            notification.dragDetails != null)) {
      // Wheel/trackpad updates can have no dragDetails. Pause on the first
      // upward movement; resume only after the user actually reaches bottom.
      final delta = notification.scrollDelta ?? 0;
      if (delta < 0) {
        _followLatest = false;
      } else if (delta > 0 && notification.metrics.extentAfter <= 1) {
        _followLatest = true;
      }
    } else if (notification is ScrollEndNotification) {
      _scheduleFollowLatest();
    }
    return false;
  }

  @override
  Widget build(BuildContext context) {
    final endExclusive = (widget.group.endIndexInclusive + 1).clamp(
      0,
      widget.nodes.length,
    );
    final slice =
        widget.group.startIndex >= 0 && widget.group.startIndex < endExclusive
        ? widget.nodes.sublist(widget.group.startIndex, endExclusive)
        : <MarkdownNodeStable>[];

    final toolCount = slice.where((node) {
      return node.type == MarkdownNodeType.xmlBlock &&
          _extractXmlTagName(node) == 'tool';
    }).length;
    final searchCount = slice.where((node) {
      return node.type == MarkdownNodeType.xmlBlock &&
          _extractXmlTagName(node) == 'search';
    }).length;
    final thinkCount = slice.where((node) {
      if (node.type != MarkdownNodeType.xmlBlock) return false;
      final tag = _extractXmlTagName(node);
      return tag == 'think' || tag == 'thinking';
    }).length;

    final tailStartIndex = (widget.group.endIndexInclusive + 1).clamp(
      0,
      widget.nodes.length,
    );
    final tailNodes = widget.nodes.sublist(tailStartIndex);
    final hasNonConformingAfterGroup = tailNodes.any(
      (node) => !_isConformingTailNode(node),
    );
    final hasLiveXmlStream = List<int>.generate(slice.length, (idx) => idx).any(
      (idx) => widget.xmlStreamResolver(widget.group.startIndex + idx) != null,
    );
    // A group can include trailing separators beyond the last visible node.
    final isLastContentGroup =
        widget.isLastNode || tailNodes.every(_isToolGroupingSeparatorNode);
    final isExecutionActive =
        hasLiveXmlStream && isLastContentGroup && !hasNonConformingAfterGroup;

    if (isExecutionActive) {
      if (!_wasExecutionActive && _userOverride == null) {
        _expanded = true;
        _followLatest = true;
      }
      if (!_stopwatch.isRunning) {
        _stopwatch.start();
        _tickerTimer ??= Timer.periodic(const Duration(seconds: 1), (_) {
          if (mounted) {
            setState(() {});
          }
        });
      }
      _wasStreaming = true;
    } else {
      // Execution finished (body text arrived, a later group took over, or
      // the stream closed): fold the phase away unless the user opened it.
      if (_wasExecutionActive &&
          !widget.forceExpandGroups &&
          _userOverride == null) {
        _expanded = false;
      }
      if (_wasStreaming && _stopwatch.isRunning) {
        _recordCompletedDuration();
      }
    }

    final l10n =
        AppLocalizations.of(context) ??
        lookupAppLocalizations(Localizations.localeOf(context));
    String buildDynamicTitle() {
      final counts = <String>[
        if (thinkCount > 0) l10n.toolGroupThinkingCount(thinkCount),
        if (toolCount > 0) l10n.toolGroupCallCount(toolCount),
        if (searchCount > 0 && toolCount == 0)
          l10n.toolGroupSearchCount(searchCount),
      ];
      final countsSummary = counts.isEmpty
          ? l10n.toolGroupCallCount(toolCount)
          : counts.join(' · ');

      if (isExecutionActive) {
        final elapsed = _stopwatch.elapsed.inSeconds;
        final timeSuffix = elapsed > 0
            ? ' (${formatExecutionDuration(_stopwatch.elapsed)})'
            : '';
        return '${l10n.toolGroupExecuting}$timeSuffix · $countsSummary';
      }

      final measuredDuration =
          _completedDuration ??
          _measuredGroupDurations['${widget.rendererId}-${widget.group.stableKey}'] ??
          widget.defaultCompletedDuration;
      final timeSuffix =
          (measuredDuration != null && measuredDuration.inSeconds > 0)
          ? ' (${formatExecutionDuration(measuredDuration)})'
          : '';
      return '${l10n.toolGroupCompleted}$timeSuffix · $countsSummary';
    }

    final titleText = buildDynamicTitle();

    final nextStateKey =
        '${widget.rendererId}-${widget.group.stableKey}-${widget.forceExpandGroups}';
    if (_stateKey != nextStateKey) {
      _stateKey = nextStateKey;
      _userOverride = null;
      _expanded = widget.forceExpandGroups || isExecutionActive;
      _followLatest = true;
    }
    if (widget.forceExpandGroups) {
      _expanded = true;
    }
    _wasExecutionActive = isExecutionActive;
    _scheduleFollowLatest();

    final bodyChildren = widget.mergeRender(
      startIndex: widget.group.startIndex,
      endIndexInclusive: widget.group.endIndexInclusive,
      renderInstanceKeyPrefix: 'group-${widget.group.stableKey}',
      shouldRenderNode: _renderToolGroupXmlNode,
    );

    return SelectionContainer.disabled(
      child: _ThinkToolsGroupAlpha(
        visible: widget.forceExpandGroups || widget.isVisible,
        duration: widget.forceExpandGroups
            ? _instantDuration
            : _groupFadeDuration,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(0, 0, 0, 4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              MessagePressShieldRegion(
                child: InkWell(
                  onTap: () {
                    setState(() {
                      final nextExpanded = !_expanded;
                      _expanded = nextExpanded;
                      _userOverride = nextExpanded;
                    });
                  },
                  borderRadius: BorderRadius.circular(6),
                  child: Padding(
                    padding: EdgeInsets.zero,
                    child: Row(
                      children: <Widget>[
                        AnimatedRotation(
                          turns: _expanded ? 0.25 : 0,
                          duration: widget.forceExpandGroups
                              ? _instantDuration
                              : _arrowRotationDuration,
                          child: Icon(
                            Icons.keyboard_arrow_right,
                            size: 20,
                            color: widget.textColor.withValues(alpha: 0.7),
                          ),
                        ),
                        const SizedBox(width: 4),
                        Text(
                          titleText,
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(
                                color: isExecutionActive
                                    ? Theme.of(context).colorScheme.primary
                                    : widget.textColor.withValues(alpha: 0.7),
                                fontWeight: FontWeight.w500,
                              ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
              _ThinkToolsContentVisibility(
                visible: _expanded,
                duration: widget.forceExpandGroups
                    ? _instantDuration
                    : _contentFadeDuration,
                child: Padding(
                  padding: const EdgeInsets.only(top: 2, bottom: 4, left: 10),
                  child: Container(
                    decoration: BoxDecoration(
                      border: Border(
                        left: BorderSide(
                          color: Theme.of(
                            context,
                          ).colorScheme.outlineVariant.withValues(alpha: 0.55),
                          width: 1.5,
                        ),
                      ),
                    ),
                    padding: const EdgeInsets.only(left: 12),
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(
                        maxHeight: _maxBodyHeight,
                      ),
                      child: NotificationListener<ScrollNotification>(
                        onNotification: _handleBodyScrollNotification,
                        child: Listener(
                          onPointerDown: _handleBodyPointerStart,
                          onPointerUp: _handleBodyPointerEnd,
                          onPointerCancel: _handleBodyPointerEnd,
                          onPointerPanZoomStart: _handleBodyPointerStart,
                          onPointerPanZoomEnd: _handleBodyPointerEnd,
                          child: Scrollbar(
                            controller: _bodyScrollController,
                            child: ListView(
                              controller: _bodyScrollController,
                              primary: false,
                              shrinkWrap: true,
                              padding: const EdgeInsets.only(right: 14),
                              children: <Widget>[
                                for (final child in bodyChildren)
                                  _KeepAliveGroupItem(child: child),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Reports whether a trailing node is only part of the active tool sequence layout.
  bool _isConformingTailNode(MarkdownNodeStable node) {
    switch (node.type) {
      case MarkdownNodeType.plainText:
        return node.content.trim().isEmpty;
      case MarkdownNodeType.htmlBreak:
        return true;
      case MarkdownNodeType.xmlBlock:
        final tag = _extractXmlTagName(node);
        switch (tag) {
          case 'think':
          case 'thinking':
          case 'search':
          case 'meta':
            return true;
          case 'tool':
          case 'tool_result':
            final toolName = _extractToolNameFromToolOrResult(node);
            if (toolName == null && !_isXmlFullyClosed(node)) {
              return true;
            }
            return true;
          case null:
            return !_isXmlFullyClosed(node);
          default:
            return false;
        }
      default:
        return false;
    }
  }
}

class _ThinkToolsContentVisibility extends StatefulWidget {
  const _ThinkToolsContentVisibility({
    required this.visible,
    required this.duration,
    required this.child,
  });

  final bool visible;
  final Duration duration;
  final Widget child;

  @override
  State<_ThinkToolsContentVisibility> createState() =>
      _ThinkToolsContentVisibilityState();
}

class _ThinkToolsContentVisibilityState
    extends State<_ThinkToolsContentVisibility>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _alpha;
  late bool _present;

  @override
  void initState() {
    super.initState();
    _present = widget.visible;
    _controller =
        AnimationController(
          vsync: this,
          duration: widget.duration,
          value: widget.visible ? 1 : 0,
        )..addStatusListener((status) {
          if (status == AnimationStatus.dismissed && _present) {
            setState(() {
              _present = false;
            });
          }
        });
    _alpha = CurvedAnimation(parent: _controller, curve: Curves.linear);
  }

  @override
  void didUpdateWidget(covariant _ThinkToolsContentVisibility oldWidget) {
    super.didUpdateWidget(oldWidget);
    _controller.duration = widget.duration;
    if (oldWidget.visible == widget.visible) {
      return;
    }
    if (widget.visible) {
      setState(() {
        _present = true;
      });
      _controller.forward();
    } else {
      _controller.reverse();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (!_present) {
      return const SizedBox.shrink();
    }
    return FadeTransition(opacity: _alpha, child: widget.child);
  }
}

class _ThinkToolsGroupAlpha extends StatefulWidget {
  const _ThinkToolsGroupAlpha({
    required this.visible,
    required this.duration,
    required this.child,
  });

  final bool visible;
  final Duration duration;
  final Widget child;

  @override
  State<_ThinkToolsGroupAlpha> createState() => _ThinkToolsGroupAlphaState();
}

class _ThinkToolsGroupAlphaState extends State<_ThinkToolsGroupAlpha>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _alpha;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: widget.duration,
      value: widget.visible ? 1 : 0,
    );
    _alpha = CurvedAnimation(parent: _controller, curve: Curves.linear);
  }

  @override
  void didUpdateWidget(covariant _ThinkToolsGroupAlpha oldWidget) {
    super.didUpdateWidget(oldWidget);
    _controller.duration = widget.duration;
    if (oldWidget.visible == widget.visible) {
      return;
    }
    if (widget.visible) {
      _controller.forward();
    } else {
      _controller.reverse();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FadeTransition(
      opacity: _alpha,
      child: RepaintBoundary(child: widget.child),
    );
  }
}

String? _extractXmlTagName(MarkdownNodeStable node) {
  return ChatMarkupRegex.normalizeToolLikeTagName(node.xmlTagName);
}

String? _extractToolName(MarkdownNodeStable node) {
  return node.xmlAttributes?['name'];
}

bool _isXmlFullyClosed(MarkdownNodeStable node) {
  return node.xmlIsClosed == true;
}

String? _extractToolNameFromToolOrResult(MarkdownNodeStable node) {
  final tag = _extractXmlTagName(node);
  return switch (tag) {
    'tool' || 'tool_result' => _extractToolName(node),
    _ => null,
  };
}

bool _isIgnorableXmlTagForToolGrouping(String? tag) {
  return tag == 'meta';
}

/// Identifies layout-only Markdown nodes inside one XML grouping sequence.
bool _isToolGroupingSeparatorNode(MarkdownNodeStable node) {
  return node.type == MarkdownNodeType.htmlBreak ||
      node.type == MarkdownNodeType.plainText && node.content.trim().isEmpty;
}

/// Includes only XML nodes in the visible body of a tool group.
bool _renderToolGroupXmlNode(MarkdownNodeStable node) {
  return node.type == MarkdownNodeType.xmlBlock;
}

/// Returns whether a tool-only sequence should render as one group.
bool _shouldCollapseToolSequence(int toolCount, int xmlToolRelatedCount) {
  if (xmlToolRelatedCount <= 0) {
    return false;
  }
  return toolCount >= 1 && xmlToolRelatedCount >= 1;
}

/// Returns whether a thinking-led sequence should render as one group.
bool _shouldCollapseThinkSequence(
  int toolCount,
  int searchCount,
  int xmlToolRelatedCount,
) {
  if (xmlToolRelatedCount <= 0) {
    return false;
  }
  return searchCount > 0 || toolCount > 0;
}

class _KeepAliveGroupItem extends StatefulWidget {
  const _KeepAliveGroupItem({required this.child});

  final Widget child;

  @override
  State<_KeepAliveGroupItem> createState() => _KeepAliveGroupItemState();
}

class _KeepAliveGroupItemState extends State<_KeepAliveGroupItem>
    with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context);
    return widget.child;
  }
}

/// Formats execution or stopwatch duration into compact human-readable text
/// with carryover: <60s -> Xs, 60s~59m59s -> Xmin [Ys], >=1h -> Xh [Ymin] [Zs].
String formatExecutionDuration(Duration duration) {
  final totalSeconds = duration.inSeconds;
  if (totalSeconds <= 0) {
    return '0s';
  }
  if (totalSeconds < 60) {
    return '${totalSeconds}s';
  }
  final hours = totalSeconds ~/ 3600;
  final minutes = (totalSeconds % 3600) ~/ 60;
  final seconds = totalSeconds % 60;

  if (hours > 0) {
    final parts = <String>['${hours}h'];
    if (minutes > 0) parts.add('${minutes}min');
    if (seconds > 0) parts.add('${seconds}s');
    return parts.join(' ');
  }

  if (seconds > 0) {
    return '${minutes}min ${seconds}s';
  }
  return '${minutes}min';
}
