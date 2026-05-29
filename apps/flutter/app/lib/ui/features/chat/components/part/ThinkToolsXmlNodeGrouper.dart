// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../util/ChatMarkupRegex.dart';
import '../../../../common/markdown/MarkdownNodeGrouper.dart';

const Duration _groupFadeDuration = Duration(milliseconds: 800);
const Duration _contentFadeDuration = Duration(milliseconds: 200);
const Duration _arrowRotationDuration = Duration(milliseconds: 300);
const Duration _instantDuration = Duration.zero;

enum ToolCollapseMode { full, readOnly, all }

class ThinkToolsXmlNodeGrouper extends MarkdownNodeGrouper {
  const ThinkToolsXmlNodeGrouper({
    required this.showThinkingProcess,
    this.forceExpandGroups = false,
    this.toolCollapseMode = ToolCollapseMode.all,
  });

  final bool showThinkingProcess;
  final bool forceExpandGroups;
  final ToolCollapseMode toolCollapseMode;

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

      final tag = _extractXmlTagName(node.content);

      if (showThinkingProcess && (tag == 'think' || tag == 'thinking')) {
        var j = i + 1;
        var toolCount = 0;
        var xmlToolRelatedCount = 0;
        while (j < nodes.length) {
          final next = nodes[j];
          if (next.type == MarkdownNodeType.plainText &&
              next.content.trim().isEmpty) {
            j++;
            continue;
          }
          if (next.type != MarkdownNodeType.xmlBlock) {
            break;
          }

          final nextTag = _extractXmlTagName(next.content);
          if (_isIgnorableXmlTagForToolGrouping(nextTag)) {
            j++;
            continue;
          }
          final isThinkAgain = nextTag == 'think' || nextTag == 'thinking';
          final isToolRelated = nextTag == 'tool' || nextTag == 'tool_result';
          if (!isThinkAgain && !isToolRelated) {
            break;
          }

          if (isToolRelated) {
            final toolName = _extractToolNameFromToolOrResult(next.content);
            if (!_shouldGroupToolByName(toolName, toolCollapseMode)) {
              break;
            }
            if (nextTag == 'tool') {
              toolCount++;
            }
            xmlToolRelatedCount++;
          }

          j++;
        }

        if (_shouldCollapseToolSequence(
          toolCollapseMode,
          toolCount,
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
        final firstToolName = _extractToolNameFromToolOrResult(node.content);
        if (!_shouldGroupToolByName(firstToolName, toolCollapseMode)) {
          out.add(MarkdownSingleItem(i));
          i++;
          continue;
        }

        var j = i + 1;
        var toolCount = tag == 'tool' ? 1 : 0;
        var xmlToolRelatedCount = 1;

        while (j < nodes.length) {
          final next = nodes[j];
          if (next.type == MarkdownNodeType.plainText &&
              next.content.trim().isEmpty) {
            j++;
            continue;
          }
          if (next.type != MarkdownNodeType.xmlBlock) {
            break;
          }

          final nextTag = _extractXmlTagName(next.content);
          if (_isIgnorableXmlTagForToolGrouping(nextTag)) {
            j++;
            continue;
          }
          final isToolRelated = nextTag == 'tool' || nextTag == 'tool_result';
          if (!isToolRelated) {
            break;
          }

          final toolName = _extractToolNameFromToolOrResult(next.content);
          if (!_shouldGroupToolByName(toolName, toolCollapseMode)) {
            break;
          }

          xmlToolRelatedCount++;
          if (nextTag == 'tool') {
            toolCount++;
          }
          j++;
        }

        if (_shouldCollapseToolSequence(
          toolCollapseMode,
          toolCount,
          xmlToolRelatedCount,
        )) {
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
    required Widget Function(int index) renderNodeAt,
  }) {
    return _ThinkToolsXmlGroup(
      group: group,
      nodes: nodes,
      rendererId: rendererId,
      isVisible: isVisible,
      textColor: textColor,
      renderNodeAt: renderNodeAt,
      forceExpandGroups: forceExpandGroups,
      toolCollapseMode: toolCollapseMode,
    );
  }
}

class _ThinkToolsXmlGroup extends StatefulWidget {
  const _ThinkToolsXmlGroup({
    required this.group,
    required this.nodes,
    required this.rendererId,
    required this.isVisible,
    required this.textColor,
    required this.renderNodeAt,
    required this.forceExpandGroups,
    required this.toolCollapseMode,
  });

  final MarkdownGroupItem group;
  final List<MarkdownNodeStable> nodes;
  final String rendererId;
  final bool isVisible;
  final Color textColor;
  final Widget Function(int index) renderNodeAt;
  final bool forceExpandGroups;
  final ToolCollapseMode toolCollapseMode;

  @override
  State<_ThinkToolsXmlGroup> createState() => _ThinkToolsXmlGroupState();
}

class _ThinkToolsXmlGroupState extends State<_ThinkToolsXmlGroup> {
  bool? _userOverride;
  final Set<String> _appearedItemKeys = <String>{};
  final Set<String> _visibleItemKeys = <String>{};
  final Set<String> _scheduledItemKeys = <String>{};

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
          _extractXmlTagName(node.content) == 'tool';
    }).length;
    final titleText = widget.group.stableKey.startsWith('tools-only-')
        ? 'Tools ($toolCount)'
        : 'Thinking & tools ($toolCount)';

    final hasLiveXmlStream = slice.any(
      (node) => node.type == MarkdownNodeType.xmlBlock && node.isStreaming,
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
    final expanded =
        widget.forceExpandGroups || (_userOverride ?? shouldAutoExpand);

    return AnimatedOpacity(
      opacity: widget.forceExpandGroups || widget.isVisible ? 1 : 0,
      duration: widget.forceExpandGroups
          ? _instantDuration
          : _groupFadeDuration,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(0, 0, 0, 4),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            InkWell(
              onTap: () {
                setState(() {
                  _userOverride = !expanded;
                });
              },
              borderRadius: BorderRadius.circular(6),
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Row(
                  children: <Widget>[
                    AnimatedRotation(
                      turns: expanded ? 0.25 : 0,
                      duration: widget.forceExpandGroups
                          ? _instantDuration
                          : _arrowRotationDuration,
                      child: Icon(
                        Icons.keyboard_arrow_right,
                        size: 18,
                        color: widget.textColor.withValues(alpha: 0.7),
                      ),
                    ),
                    const SizedBox(width: 6),
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
            AnimatedSwitcher(
              duration: widget.forceExpandGroups
                  ? _instantDuration
                  : _contentFadeDuration,
              switchInCurve: Curves.linear,
              switchOutCurve: Curves.linear,
              transitionBuilder: (child, animation) {
                return FadeTransition(opacity: animation, child: child);
              },
              child: expanded
                  ? Padding(
                      key: const ValueKey<String>('expanded'),
                      padding: const EdgeInsets.only(
                        top: 4,
                        bottom: 8,
                        left: 24,
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          for (var idx = 0; idx < slice.length; idx++)
                            if (slice[idx].type == MarkdownNodeType.xmlBlock)
                              _renderXmlItem(widget.group.startIndex + idx),
                        ],
                      ),
                    )
                  : const SizedBox.shrink(key: ValueKey<String>('collapsed')),
            ),
          ],
        ),
      ),
    );
  }

  Widget _renderXmlItem(int absoluteIndex) {
    final child = widget.renderNodeAt(absoluteIndex);
    if (widget.forceExpandGroups) {
      return child;
    }

    final itemKey =
        'think-tools-${widget.rendererId}-${widget.group.stableKey}-$absoluteIndex';
    final isVisible = _isXmlItemVisible(itemKey);
    return AnimatedOpacity(
      key: ValueKey<String>(itemKey),
      opacity: isVisible ? 1 : 0,
      duration: _groupFadeDuration,
      child: child,
    );
  }

  bool _isXmlItemVisible(String itemKey) {
    if (_appearedItemKeys.contains(itemKey)) {
      return true;
    }
    if (_scheduledItemKeys.add(itemKey)) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) {
          return;
        }
        setState(() {
          _visibleItemKeys.add(itemKey);
          _appearedItemKeys.add(itemKey);
          _scheduledItemKeys.remove(itemKey);
        });
      });
    }
    return _visibleItemKeys.contains(itemKey);
  }

  bool _isConformingTailNode(MarkdownNodeStable node) {
    switch (node.type) {
      case MarkdownNodeType.plainText:
      case MarkdownNodeType.header:
      case MarkdownNodeType.orderedList:
      case MarkdownNodeType.unorderedList:
        return node.content.trim().isEmpty;
      case MarkdownNodeType.xmlBlock:
        final tag = _extractXmlTagName(node.content);
        switch (tag) {
          case 'think':
          case 'thinking':
          case 'meta':
            return true;
          case 'tool':
          case 'tool_result':
            final toolName = _extractToolNameFromToolOrResult(node.content);
            if (toolName == null && !_isXmlFullyClosed(node.content)) {
              return true;
            }
            return _shouldGroupToolByName(toolName, widget.toolCollapseMode);
          case null:
            return !_isXmlFullyClosed(node.content);
          default:
            return false;
        }
    }
  }
}

String? _extractXmlTagName(String xml) {
  return ChatMarkupRegex.normalizeToolLikeTagName(_extractRawXmlTagName(xml));
}

String? _extractRawXmlTagName(String xml) {
  return ChatMarkupRegex.extractOpeningTagName(xml);
}

String? _extractToolName(String xml) {
  final nameMatch = ChatMarkupRegex.nameAttr.firstMatch(xml);
  return nameMatch?.group(1);
}

bool _isXmlFullyClosed(String xml) {
  final tagName = _extractRawXmlTagName(xml);
  if (tagName == null) {
    return false;
  }
  final trimmed = xml.trim();
  if (trimmed.endsWith('/>') ||
      trimmed.startsWith('<$tagName') && trimmed.endsWith('/>')) {
    return true;
  }
  return trimmed.toLowerCase().contains('</${tagName.toLowerCase()}>');
}

String? _extractToolNameFromToolOrResult(String xml) {
  final tag = _extractXmlTagName(xml);
  return switch (tag) {
    'tool' || 'tool_result' => _extractToolName(xml),
    _ => null,
  };
}

bool _isIgnorableXmlTagForToolGrouping(String? tag) {
  return tag == 'meta';
}

bool _shouldGroupToolByName(
  String? toolName,
  ToolCollapseMode toolCollapseMode,
) {
  if (toolCollapseMode == ToolCollapseMode.all ||
      toolCollapseMode == ToolCollapseMode.full) {
    return true;
  }

  final n = toolName?.trim().toLowerCase();
  if (n == null) {
    return false;
  }
  if (n.contains('search')) {
    return true;
  }
  return const <String>{
    'list_files',
    'grep_code',
    'grep_context',
    'read_file',
    'read_file_part',
    'read_file_full',
    'read_file_binary',
    'use_package',
    'find_files',
    'visit_web',
  }.contains(n);
}

bool _shouldCollapseToolSequence(
  ToolCollapseMode toolCollapseMode,
  int toolCount,
  int xmlToolRelatedCount,
) {
  if (xmlToolRelatedCount <= 0) {
    return false;
  }
  return switch (toolCollapseMode) {
    ToolCollapseMode.full => true,
    ToolCollapseMode.readOnly ||
    ToolCollapseMode.all => toolCount >= 2 && xmlToolRelatedCount >= 2,
  };
}
