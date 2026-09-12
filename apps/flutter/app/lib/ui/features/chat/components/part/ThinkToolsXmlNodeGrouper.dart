// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

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
  });

  final bool showThinkingProcess;
  final bool forceExpandGroups;

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        other is ThinkToolsXmlNodeGrouper &&
            other.showThinkingProcess == showThinkingProcess &&
            other.forceExpandGroups == forceExpandGroups;
  }

  @override
  int get hashCode {
    return Object.hash(showThinkingProcess, forceExpandGroups);
  }

  @override
  List<MarkdownGroupedItem> group(
    List<MarkdownNodeStable> nodes,
    String rendererId,
  ) {
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
      textColor: textColor,
      mergeRender: mergeRender,
      xmlStreamResolver: xmlStreamResolver,
      forceExpandGroups: forceExpandGroups,
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
    required this.textColor,
    required this.mergeRender,
    required this.xmlStreamResolver,
    required this.forceExpandGroups,
  });

  final MarkdownGroupItem group;
  final List<MarkdownNodeStable> nodes;
  final String rendererId;
  final bool isVisible;
  final Color textColor;
  final MarkdownNodeRangeMergeRender mergeRender;
  final Stream<String>? Function(int index) xmlStreamResolver;
  final bool forceExpandGroups;

  @override
  State<_ThinkToolsXmlGroup> createState() => _ThinkToolsXmlGroupState();
}

class _ThinkToolsXmlGroupState extends State<_ThinkToolsXmlGroup> {
  String? _stateKey;
  bool _expanded = false;
  bool? _userOverride;

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
    final l10n = AppLocalizations.of(context)!;
    final titleText = switch ((
      widget.group.stableKey.startsWith('tools-only-'),
      widget.group.stableKey.startsWith('search-only-'),
      searchCount > 0,
      toolCount > 0,
    )) {
      (true, _, _, _) => l10n.toolsGroupTitleWithCount(toolCount),
      (_, true, _, _) => l10n.searchGroupTitle,
      (_, _, true, true) => l10n.thinkingSearchToolsGroupTitleWithCount(
        toolCount,
      ),
      (_, _, true, false) => l10n.thinkingSearchGroupTitle,
      _ => l10n.thinkingToolsGroupTitleWithCount(toolCount),
    };

    final hasLiveXmlStream = List<int>.generate(slice.length, (idx) => idx).any(
      (idx) => widget.xmlStreamResolver(widget.group.startIndex + idx) != null,
    );
    final tailStartIndex = (widget.group.endIndexInclusive + 1).clamp(
      0,
      widget.nodes.length,
    );
    final hasNonConformingAfterGroup = tailStartIndex >= widget.nodes.length
        ? false
        : widget.nodes
              .sublist(tailStartIndex)
              .any((node) => !_isConformingTailNode(node));
    final shouldAutoExpand = hasLiveXmlStream && !hasNonConformingAfterGroup;
    final nextStateKey =
        '${widget.rendererId}-${widget.group.stableKey}-${widget.forceExpandGroups}';
    if (_stateKey != nextStateKey) {
      _stateKey = nextStateKey;
      _userOverride = null;
      _expanded = widget.forceExpandGroups || shouldAutoExpand;
    }
    if (widget.forceExpandGroups) {
      _expanded = true;
    } else if (_userOverride == null) {
      _expanded = shouldAutoExpand;
    }

    return _ThinkToolsGroupAlpha(
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
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: widget.textColor.withValues(alpha: 0.7),
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
                padding: const EdgeInsets.only(top: 2, bottom: 4, left: 24),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: widget.mergeRender(
                    startIndex: widget.group.startIndex,
                    endIndexInclusive: widget.group.endIndexInclusive,
                    renderInstanceKeyPrefix: 'group-${widget.group.stableKey}',
                    shouldRenderNode: _renderToolGroupXmlNode,
                  ),
                ),
              ),
            ),
          ],
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
  return toolCount >= 2 && xmlToolRelatedCount >= 2;
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
