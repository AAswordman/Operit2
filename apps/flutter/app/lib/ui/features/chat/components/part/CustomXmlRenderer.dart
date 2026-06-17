// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../common/interactions/MessagePressShield.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../common/markdown/XmlRenderPluginRegistry.dart';
import '../../../../../util/ChatMarkupRegex.dart';
import 'DetailsTagRenderer.dart';
import 'DialogComponents.dart';
import 'FileDiffDisplay.dart';
import 'FontTagRenderer.dart';
import 'ToolDisplayComponents.dart';
import 'ToolResultDisplay.dart';

class CustomXmlRenderer extends StatelessWidget {
  const CustomXmlRenderer({
    super.key,
    required this.xmlContent,
    required this.isStreaming,
    required this.textColor,
    this.xmlStream,
    this.xmlMarkdownEventStream,
    this.showThinkingProcess = true,
    this.initialThinkingExpanded = false,
    this.allowExpandedThinkingFullHeight = false,
  });

  final String xmlContent;
  final bool isStreaming;
  final Color textColor;
  final Stream<String>? xmlStream;
  final Stream<Object>? xmlMarkdownEventStream;
  final bool showThinkingProcess;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;

  @override
  Widget build(BuildContext context) {
    final parsed = _ParsedXml.from(xmlContent);
    if (_shouldHideGeminiThoughtSignatureMeta(xmlContent, parsed.tagName)) {
      return const SizedBox.shrink();
    }
    if ((parsed.tagName == 'think' || parsed.tagName == 'thinking') &&
        !showThinkingProcess) {
      return const SizedBox.shrink();
    }
    final pluginRender = XmlRenderPluginRegistry.renderIfMatched(
      tagName: parsed.tagName,
      xmlContent: xmlContent,
      textColor: textColor,
      isStreaming: isStreaming,
      xmlStream: xmlStream,
    );
    if (pluginRender != null) {
      return pluginRender;
    }
    if (!_isXmlFullyClosed(xmlContent) &&
        _builtInTags.contains(parsed.tagName) &&
        !const {
          'tool',
          'think',
          'thinking',
          'search',
        }.contains(parsed.tagName)) {
      return const SizedBox.shrink();
    }
    switch (parsed.tagName) {
      case 'think':
      case 'thinking':
        return _ThinkPanel(
          text: parsed.body,
          textColor: textColor,
          isStreaming: xmlStream != null && !_isXmlFullyClosed(xmlContent),
          xmlStream: xmlStream,
          markdownEventStream: xmlMarkdownEventStream,
          initiallyExpanded: initialThinkingExpanded,
          fullHeight: allowExpandedThinkingFullHeight,
        );
      case 'search':
        return _LabeledPanel(
          label: 'Search',
          text: parsed.body,
          color: Theme.of(context).colorScheme.tertiary,
          isStreaming: isStreaming,
        );
      case 'status':
        return _StatusChip(
          parsed: parsed,
          textColor: textColor,
          isStreaming: isStreaming,
        );
      case 'meta':
        return const SizedBox.shrink();
      case 'tool':
        return _ToolRequestRenderer(
          xmlContent: xmlContent,
          parsed: parsed,
          textColor: textColor,
          isStreaming: isStreaming,
          xmlStream: xmlStream,
        );
      case 'tool_result':
        final result = _extractToolResult(parsed, xmlContent);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            for (final fileDiff in result.fileDiffs)
              FileDiffDisplay(diff: fileDiff),
            if (result.fileDiffs.isEmpty)
              ToolResultDisplay(
                toolName: result.toolName,
                result: result.resultContent,
                isSuccess: result.isSuccess,
                isStreaming: isStreaming,
              ),
          ],
        );
      case 'html':
        return StreamMarkdownRenderer(
          content: parsed.body,
          isStreaming: isStreaming,
          textColor: textColor,
          backgroundColor: Theme.of(context).colorScheme.surface,
          selectionRoot: false,
        );
      case 'details':
      case 'detail':
        return DetailsTagRenderer(
          xmlContent: xmlContent,
          textColor: textColor,
          isStreaming: isStreaming,
        );
      case 'font':
        return FontTagRenderer(xmlContent: xmlContent, textColor: textColor);
      case 'mood':
        return StreamMarkdownRenderer(
          content: parsed.body,
          isStreaming: isStreaming,
          textColor: textColor,
          backgroundColor: Theme.of(context).colorScheme.surface,
          selectionRoot: false,
        );
    }
    return SelectableText(
      parsed.body.isEmpty ? xmlContent : parsed.body,
      style: Theme.of(
        context,
      ).textTheme.bodyMedium?.copyWith(color: textColor, height: 1.45),
    );
  }
}

const _toolParamTokenThreshold = 50;

const _builtInTags = <String>{
  'think',
  'thinking',
  'search',
  'tool',
  'status',
  'tool_result',
  'html',
  'mood',
  'font',
  'details',
  'detail',
  'meta',
};

class _ToolRequestRenderer extends StatelessWidget {
  const _ToolRequestRenderer({
    required this.xmlContent,
    required this.parsed,
    required this.textColor,
    required this.isStreaming,
    required this.xmlStream,
  });

  final String xmlContent;
  final _ParsedXml parsed;
  final Color textColor;
  final bool isStreaming;
  final Stream<String>? xmlStream;

  @override
  Widget build(BuildContext context) {
    final rawToolName = parsed.attr('name') ?? 'Unknown tool';
    final params = _extractParamsFromTool(xmlContent);
    final paramText = _extractContentFromXml(
      xmlContent,
      tagName: 'tool',
    ).trim();
    final displayToolName = _resolveToolDisplayNameForRender(
      rawToolName,
      params,
    );
    final isClosed = _isXmlFullyClosed(xmlContent);
    final initialTokenEstimate = _estimateTokenCount(paramText);

    Widget renderWithEstimate(int paramTokenEstimate) {
      if (displayToolName == 'apply_file' ||
          displayToolName == 'create_file' ||
          displayToolName == 'edit_file') {
        if (isClosed) {
          return CompactToolDisplay(
            toolName: rawToolName,
            params: paramText,
            textColor: textColor,
            isStreaming: isStreaming,
          );
        }
        return DetailedToolDisplay(
          toolName: rawToolName,
          params: paramText,
          textColor: textColor,
          isStreaming: isStreaming,
        );
      }

      if (!isClosed && paramTokenEstimate > _toolParamTokenThreshold) {
        return DetailedToolDisplay(
          toolName: rawToolName,
          params: paramText,
          textColor: textColor,
          isStreaming: isStreaming,
        );
      }

      return CompactToolDisplay(
        toolName: rawToolName,
        params: paramText,
        textColor: textColor,
        isStreaming: isStreaming,
      );
    }

    final stream = xmlStream;
    if (stream == null) {
      return renderWithEstimate(initialTokenEstimate);
    }
    return StreamBuilder<int>(
      stream: _toolParamTokenEstimateStream(stream, initialTokenEstimate),
      initialData: initialTokenEstimate,
      builder: (context, snapshot) => renderWithEstimate(snapshot.requireData),
    );
  }
}

class _ToolResultRenderState {
  const _ToolResultRenderState({
    required this.toolName,
    required this.isSuccess,
    required this.resultContent,
    required this.fileDiffs,
  });

  final String toolName;
  final bool isSuccess;
  final String resultContent;
  final List<FileDiff> fileDiffs;
}

class _ThinkPanel extends StatefulWidget {
  const _ThinkPanel({
    required this.text,
    required this.textColor,
    required this.isStreaming,
    required this.xmlStream,
    required this.markdownEventStream,
    required this.initiallyExpanded,
    required this.fullHeight,
  });

  final String text;
  final Color textColor;
  final bool isStreaming;
  final Stream<String>? xmlStream;
  final Stream<Object>? markdownEventStream;
  final bool initiallyExpanded;
  final bool fullHeight;

  @override
  State<_ThinkPanel> createState() => _ThinkPanelState();
}

class _ThinkPanelState extends State<_ThinkPanel> {
  late bool _expanded;
  late bool _bodyFullHeight;
  late final ScrollController _scrollController;
  bool _skipCollapseAnimationOnce = false;
  bool _autoScrollEnabled = true;
  bool _userHasInteractedWithScroll = false;
  bool _isProgrammaticScroll = false;
  int _expandSession = 0;

  @override
  void initState() {
    super.initState();
    _expanded = _targetExpandedFor(widget);
    _bodyFullHeight = widget.fullHeight && widget.initiallyExpanded;
    _expandSession = _expanded ? 1 : 0;
    _scrollController = ScrollController();
  }

  @override
  void didUpdateWidget(covariant _ThinkPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.isStreaming != widget.isStreaming ||
        oldWidget.initiallyExpanded != widget.initiallyExpanded) {
      final targetExpanded = _targetExpandedFor(widget);
      if (targetExpanded && !_expanded) {
        _expandSession += 1;
      }
      if (!targetExpanded && oldWidget.isStreaming && !widget.isStreaming) {
        _skipCollapseAnimationOnce = true;
        _bodyFullHeight = false;
      }
      _expanded = targetExpanded;
      if (_expanded) {
        _resetAutoScrollState();
      }
      if (_skipCollapseAnimationOnce) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted) {
            return;
          }
          setState(() {
            _skipCollapseAnimationOnce = false;
          });
        });
      }
    }

    if (_expanded && widget.isStreaming && _autoScrollEnabled) {
      _scrollToBottomAfterFrame();
    }
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  bool _targetExpandedFor(_ThinkPanel widget) {
    if (widget.initiallyExpanded && !widget.isStreaming) {
      return true;
    }
    if (widget.isStreaming) {
      return true;
    }
    return false;
  }

  void _handleHeaderTap() {
    setState(() {
      _skipCollapseAnimationOnce = false;
      final nextExpanded = !_expanded;
      if (nextExpanded) {
        _expandSession += 1;
        _resetAutoScrollState();
      }
      _expanded = nextExpanded;
    });
  }

  void _handleBodyTap() {
    setState(() {
      _bodyFullHeight = !_bodyFullHeight;
    });
  }

  void _resetAutoScrollState() {
    _autoScrollEnabled = true;
    _userHasInteractedWithScroll = false;
  }

  bool _handleScrollNotification(ScrollNotification notification) {
    if (!_expanded || _isProgrammaticScroll || !_scrollController.hasClients) {
      return false;
    }
    final isUserScroll =
        notification is UserScrollNotification ||
        (notification is ScrollUpdateNotification &&
            notification.dragDetails != null);
    if (!isUserScroll) {
      return false;
    }
    _userHasInteractedWithScroll = true;
    _updateAutoScrollFromPosition();
    return false;
  }

  void _updateAutoScrollFromPosition() {
    if (!_userHasInteractedWithScroll || !_scrollController.hasClients) {
      return;
    }
    final position = _scrollController.position;
    const threshold = 80.0;
    _autoScrollEnabled =
        position.pixels >= position.maxScrollExtent - threshold;
  }

  void _scrollToBottomAfterFrame() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_scrollController.hasClients || !_autoScrollEnabled) {
        return;
      }
      _isProgrammaticScroll = true;
      _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      _isProgrammaticScroll = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = theme.colorScheme.secondary;
    final contentText = widget.text.trim();
    final titleColor = widget.textColor.withValues(alpha: 0.7);
    final l10n = AppLocalizations.of(context)!;
    final thinkingTitle = l10n.thinkingProcess;
    final hasStreamingMarkdown =
        widget.isStreaming && widget.markdownEventStream != null;
    final shouldRenderBody =
        _expanded && (contentText.isNotEmpty || hasStreamingMarkdown);
    final renderFullHeight =
        (widget.fullHeight || _bodyFullHeight) && _expanded;
    final switchDuration = _skipCollapseAnimationOnce
        ? Duration.zero
        : const Duration(milliseconds: 220);
    return Semantics(
      label: contentText.isEmpty
          ? thinkingTitle
          : '$thinkingTitle\n$contentText',
      child: Padding(
        padding: const EdgeInsets.only(bottom: 2),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            MessagePressShieldRegion(
              child: InkWell(
                onTap: _handleHeaderTap,
                borderRadius: BorderRadius.circular(6),
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    children: <Widget>[
                      AnimatedRotation(
                        turns: _expanded ? 0.25 : 0,
                        duration: _skipCollapseAnimationOnce
                            ? Duration.zero
                            : const Duration(milliseconds: 300),
                        child: Icon(
                          Icons.keyboard_arrow_right,
                          size: 20,
                          color: titleColor,
                        ),
                      ),
                      const SizedBox(width: 4),
                      _ThinkingTitle(
                        text: thinkingTitle,
                        color: titleColor,
                        streaming: widget.isStreaming,
                      ),
                    ],
                  ),
                ),
              ),
            ),
            AnimatedSwitcher(
              duration: switchDuration,
              reverseDuration: switchDuration,
              switchInCurve: Curves.easeOutCubic,
              switchOutCurve: Curves.easeOutCubic,
              transitionBuilder: (child, animation) {
                return SizeTransition(
                  sizeFactor: animation,
                  alignment: Alignment.topCenter,
                  child: FadeTransition(opacity: animation, child: child),
                );
              },
              child: shouldRenderBody
                  ? Padding(
                      key: ValueKey<int>(_expandSession),
                      padding: const EdgeInsets.only(top: 2, bottom: 4),
                      child: Stack(
                        children: <Widget>[
                          PositionedDirectional(
                            start: 10,
                            top: 0,
                            bottom: 0,
                            child: Container(
                              width: 1,
                              decoration: BoxDecoration(
                                color: color.withValues(alpha: 0.2),
                                borderRadius: BorderRadius.circular(999),
                              ),
                            ),
                          ),
                          Padding(
                            padding: const EdgeInsetsDirectional.only(
                              start: 24,
                            ),
                            child: MessagePressShieldRegion(
                              child: GestureDetector(
                                behavior: HitTestBehavior.translucent,
                                onTap: _handleBodyTap,
                                child: ConstrainedBox(
                                  constraints: renderFullHeight
                                      ? const BoxConstraints()
                                      : const BoxConstraints(maxHeight: 300),
                                  child: renderFullHeight
                                      ? _ThinkMarkdownBody(
                                          contentText: contentText,
                                          contentStream: hasStreamingMarkdown
                                              ? widget.markdownEventStream
                                              : null,
                                          textColor: widget.textColor,
                                        )
                                      : NotificationListener<
                                          ScrollNotification
                                        >(
                                          onNotification:
                                              _handleScrollNotification,
                                          child: SingleChildScrollView(
                                            controller: _scrollController,
                                            child: _ThinkMarkdownBody(
                                              contentText: contentText,
                                              contentStream:
                                                  hasStreamingMarkdown
                                                  ? widget.markdownEventStream
                                                  : null,
                                              textColor: widget.textColor,
                                            ),
                                          ),
                                        ),
                                ),
                              ),
                            ),
                          ),
                        ],
                      ),
                    )
                  : const SizedBox.shrink(key: ValueKey<String>('empty')),
            ),
          ],
        ),
      ),
    );
  }
}

class _ThinkMarkdownBody extends StatelessWidget {
  const _ThinkMarkdownBody({
    required this.contentText,
    required this.contentStream,
    required this.textColor,
  });

  final String contentText;
  final Stream<Object>? contentStream;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final stream = contentStream;
    return Theme(
      data: theme.copyWith(
        textTheme: theme.textTheme.copyWith(
          bodyMedium: theme.textTheme.bodySmall,
        ),
      ),
      child: stream == null
          ? StreamMarkdownRenderer(
              content: contentText,
              isStreaming: false,
              textColor: textColor.withValues(alpha: 0.6),
              backgroundColor: Colors.transparent,
              selectionRoot: false,
            )
          : StreamMarkdownRenderer(
              content: '',
              contentStream: stream,
              isStreaming: true,
              textColor: textColor.withValues(alpha: 0.6),
              backgroundColor: Colors.transparent,
              selectionRoot: false,
            ),
    );
  }
}

class _ThinkingTitle extends StatefulWidget {
  const _ThinkingTitle({
    required this.text,
    required this.color,
    required this.streaming,
  });

  final String text;
  final Color color;
  final bool streaming;

  @override
  State<_ThinkingTitle> createState() => _ThinkingTitleState();
}

class _ThinkingTitleState extends State<_ThinkingTitle>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1400),
    );
    if (widget.streaming) {
      _controller.repeat();
    }
  }

  @override
  void didUpdateWidget(covariant _ThinkingTitle oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.streaming == widget.streaming) {
      return;
    }
    if (widget.streaming) {
      _controller.repeat();
    } else {
      _controller.stop();
      _controller.value = 0;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = theme.textTheme.bodySmall?.copyWith(
      color: widget.color,
      fontWeight: FontWeight.w500,
    );
    final text = Text(widget.text, style: style);
    if (!widget.streaming) {
      return text;
    }
    final highlightStyle = style?.copyWith(color: Colors.white);
    return AnimatedBuilder(
      animation: _controller,
      builder: (context, child) {
        final shift = _controller.value;
        return Stack(
          fit: StackFit.passthrough,
          children: <Widget>[
            child!,
            IgnorePointer(
              child: ExcludeSemantics(
                child: ShaderMask(
                  blendMode: BlendMode.srcIn,
                  shaderCallback: (bounds) {
                    return LinearGradient(
                      begin: Alignment.centerLeft,
                      end: Alignment.centerRight,
                      colors: <Color>[
                        Colors.transparent,
                        theme.colorScheme.primary.withValues(alpha: 0.95),
                        Colors.transparent,
                      ],
                      stops: <double>[
                        (shift - 0.25).clamp(0.0, 1.0),
                        shift.clamp(0.0, 1.0),
                        (shift + 0.25).clamp(0.0, 1.0),
                      ],
                    ).createShader(bounds);
                  },
                  child: Text(widget.text, style: highlightStyle),
                ),
              ),
            ),
          ],
        );
      },
      child: text,
    );
  }
}

class _LabeledPanel extends StatelessWidget {
  const _LabeledPanel({
    required this.label,
    required this.text,
    required this.color,
    required this.isStreaming,
  });

  final String label;
  final String text;
  final Color color;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 4),
      padding: const EdgeInsets.fromLTRB(10, 6, 10, 6),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(8),
        border: Border(
          left: BorderSide(color: color.withValues(alpha: 0.55), width: 3),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            label,
            style: theme.textTheme.labelSmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 4),
          SelectableText(
            text,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              height: 1.35,
            ),
          ),
          if (isStreaming)
            const Padding(
              padding: EdgeInsets.only(top: 4),
              child: StreamingCursor(),
            ),
        ],
      ),
    );
  }
}

class _StatusChip extends StatelessWidget {
  const _StatusChip({
    required this.parsed,
    required this.textColor,
    required this.isStreaming,
  });

  final _ParsedXml parsed;
  final Color textColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final statusType = (parsed.attr('type') ?? 'info').trim();
    final statusContent = _isQuietStatus(parsed) ? '' : parsed.body;
    final statusText = switch (statusType) {
      'completion' || 'complete' => '✓ Task completed',
      'wait_for_user_need' => '✓ Ready for further assistance',
      _ => statusContent,
    };

    if (statusType == 'warning') {
      final l10n = AppLocalizations.of(context)!;
      return _WarningStatusRow(
        summaryText: l10n.statusWarningAiErrorSummary,
        detailText: statusContent,
        detailTitle: l10n.statusWarningAiErrorDetailTitle,
        isStreaming: isStreaming,
      );
    }

    final backgroundColor = switch (statusType) {
      'completion' ||
      'complete' => theme.colorScheme.primaryContainer.withValues(alpha: 0.3),
      'wait_for_user_need' => theme.colorScheme.tertiaryContainer.withValues(
        alpha: 0.3,
      ),
      _ => theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.2),
    };
    final borderColor = switch (statusType) {
      'completion' ||
      'complete' => theme.colorScheme.primary.withValues(alpha: 0.3),
      'wait_for_user_need' => theme.colorScheme.tertiary.withValues(alpha: 0.3),
      _ => theme.colorScheme.outline.withValues(alpha: 0.3),
    };
    final effectiveTextColor = switch (statusType) {
      'completion' || 'complete' => theme.colorScheme.primary,
      'wait_for_user_need' => theme.colorScheme.tertiary,
      _ => textColor,
    };

    return _StatusCard(
      text: statusText,
      textColor: effectiveTextColor,
      backgroundColor: backgroundColor,
      borderColor: borderColor,
      isStreaming: isStreaming,
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({
    required this.text,
    required this.textColor,
    required this.backgroundColor,
    required this.borderColor,
    required this.isStreaming,
  });

  final String text;
  final Color textColor;
  final Color backgroundColor;
  final Color borderColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(vertical: 4),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: backgroundColor,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: borderColor),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          Expanded(
            child: Text(
              text,
              style: theme.textTheme.bodySmall?.copyWith(color: textColor),
            ),
          ),
          if (isStreaming)
            const Padding(
              padding: EdgeInsets.only(left: 6),
              child: StreamingCursor(),
            ),
        ],
      ),
    );
  }
}

class _WarningStatusRow extends StatelessWidget {
  const _WarningStatusRow({
    required this.summaryText,
    required this.detailText,
    required this.detailTitle,
    required this.isStreaming,
  });

  final String summaryText;
  final String detailText;
  final String detailTitle;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final canOpenDetail = detailText.trim().isNotEmpty;
    final row = SizedBox(
      width: double.infinity,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 6),
        child: Row(
          children: <Widget>[
            Container(
              width: 2,
              height: 16,
              decoration: BoxDecoration(
                color: theme.colorScheme.error.withValues(alpha: 0.7),
                borderRadius: BorderRadius.circular(999),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                summaryText,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.error.withValues(alpha: 0.9),
                ),
              ),
            ),
            if (isStreaming)
              const Padding(
                padding: EdgeInsets.only(left: 6),
                child: StreamingCursor(),
              ),
          ],
        ),
      ),
    );

    if (!canOpenDetail) {
      return row;
    }

    return Semantics(
      button: true,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () {
            showDialog<void>(
              context: context,
              builder: (dialogContext) {
                return ContentDetailDialog(
                  title: detailTitle,
                  content: detailText,
                  icon: Icons.error,
                  onDismiss: () => Navigator.of(dialogContext).pop(),
                );
              },
            );
          },
          child: row,
        ),
      ),
    );
  }
}

class _ParsedXml {
  const _ParsedXml({
    required this.tagName,
    required this.attributes,
    required this.body,
  });

  factory _ParsedXml.from(String xml) {
    final open = RegExp(
      r'^<([a-zA-Z_][\w:-]*)\b([^>]*)>',
      dotAll: true,
    ).firstMatch(xml.trim());
    if (open == null) {
      return _ParsedXml(tagName: '', attributes: const {}, body: xml);
    }
    final rawTagName = open.group(1)!;
    final tagName = ChatMarkupRegex.normalizeToolLikeTagName(rawTagName)!;
    final attributes = _parseAttributes(open.group(2) ?? '');
    final closeTag = '</${rawTagName.toLowerCase()}>';
    final lowerXml = xml.toLowerCase();
    final closeIndex = lowerXml.lastIndexOf(closeTag);
    final bodyEnd = closeIndex > open.end ? closeIndex : xml.length;
    return _ParsedXml(
      tagName: tagName,
      attributes: attributes,
      body: xml.substring(open.end, bodyEnd).trim(),
    );
  }

  final String tagName;
  final Map<String, String> attributes;
  final String body;

  String? attr(String name) {
    return attributes[name.toLowerCase()];
  }

  bool get success {
    final value = attr('success') ?? attr('status') ?? attr('ok');
    if (value == null) {
      return true;
    }
    return !const {
      'false',
      'failed',
      'error',
      '0',
    }.contains(value.toLowerCase());
  }
}

Map<String, String> _parseAttributes(String source) {
  final result = <String, String>{};
  final pattern = RegExp(
    r'''([\w:-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+))''',
  );
  for (final match in pattern.allMatches(source)) {
    result[match.group(1)!.toLowerCase()] =
        match.group(2) ?? match.group(3) ?? match.group(4) ?? '';
  }
  return result;
}

bool _shouldHideGeminiThoughtSignatureMeta(String content, String tagName) {
  return tagName == 'meta' &&
      RegExp(
        r'''\bprovider\s*=\s*["']gemini:thought_signature["']''',
        caseSensitive: false,
      ).hasMatch(content);
}

bool _isQuietStatus(_ParsedXml parsed) {
  final statusType = parsed.attr('type');
  return const {
    'completion',
    'complete',
    'wait_for_user_need',
  }.contains(statusType);
}

bool _isXmlFullyClosed(String xml) {
  final trimmed = xml.trim();
  final rawTagName = _extractRawXmlTagName(trimmed);
  if (rawTagName == null) {
    return false;
  }
  if (trimmed.endsWith('/>')) {
    return true;
  }
  return trimmed.toLowerCase().contains('</${rawTagName.toLowerCase()}>');
}

String? _extractRawXmlTagName(String xml) {
  return ChatMarkupRegex.extractOpeningTagName(xml);
}

String _extractContentFromXml(String content, {String? tagName}) {
  final rawTagName = _extractRawXmlTagName(content);
  if (rawTagName == null) {
    return content;
  }
  final normalizedRawTagName = ChatMarkupRegex.normalizeToolLikeTagName(
    rawTagName,
  );
  final effectiveTagName = tagName != null && normalizedRawTagName != tagName
      ? tagName
      : rawTagName;
  final openMatch = RegExp(
    '<${RegExp.escape(effectiveTagName)}\\b[^>]*>',
    caseSensitive: false,
    dotAll: true,
  ).firstMatch(content);
  if (openMatch == null) {
    return content;
  }
  final endTag = '</$effectiveTagName>';
  final lowerContent = content.toLowerCase();
  final endIndex = lowerContent.lastIndexOf(endTag.toLowerCase());
  final contentEndExclusive = endIndex > openMatch.end
      ? endIndex
      : content.length;
  return content.substring(openMatch.end, contentEndExclusive).trim();
}

Map<String, String> _extractParamsFromTool(String content) {
  final params = <String, String>{};
  final pattern = RegExp(
    r'''<param\b[^>]*name=["']([^"']+)["'][^>]*>([\s\S]*?)<\/param>''',
    caseSensitive: false,
  );
  for (final match in pattern.allMatches(content)) {
    params[match.group(1)!] = match.group(2)!.trim();
  }
  return params;
}

String _resolveToolDisplayNameForRender(
  String toolName,
  Map<String, String> params,
) {
  if (toolName != 'package_proxy' && toolName != 'proxy') {
    return toolName;
  }
  final targetToolName = params['tool_name'] == null
      ? ''
      : normalizeEscapedTextForDisplay(params['tool_name']!).trim();
  return targetToolName.isNotEmpty ? targetToolName : toolName;
}

int _estimateTokenCount(String text) {
  var chineseCharCount = 0;
  var otherCharCount = 0;
  for (final codePoint in text.runes) {
    if (codePoint >= 0x4E00 && codePoint <= 0x9FFF) {
      chineseCharCount++;
    } else {
      otherCharCount++;
    }
  }
  return (chineseCharCount * 1.5 + otherCharCount * 0.25).toInt();
}

Stream<int> _toolParamTokenEstimateStream(
  Stream<String> xmlStream,
  int initialValue,
) async* {
  final counter = _XmlInnerTokenCounter(tagName: 'tool');
  var value = initialValue;
  await for (final chunk in xmlStream) {
    final next = counter.append(chunk);
    if (next > value) {
      value = next;
    }
    yield value;
  }
}

class _IncrementalTokenEstimator {
  int _chineseCharCount = 0;
  int _otherCharCount = 0;

  void append(String text) {
    for (final codePoint in text.runes) {
      appendCodePoint(codePoint);
    }
  }

  void appendCodePoint(int codePoint) {
    if (codePoint >= 0x4E00 && codePoint <= 0x9FFF) {
      _chineseCharCount++;
    } else {
      _otherCharCount++;
    }
  }

  int estimate() {
    return (_chineseCharCount * 1.5 + _otherCharCount * 0.25).toInt();
  }
}

class _XmlInnerTokenCounter {
  _XmlInnerTokenCounter({required String tagName})
    : _closingPattern = '</$tagName>';

  static const String _openingTagEndChar = '>';
  final String _closingPattern;
  final _IncrementalTokenEstimator _estimator = _IncrementalTokenEstimator();
  final StringBuffer _closeCandidate = StringBuffer();
  bool _isInsideOuterContent = false;
  bool _isClosed = false;

  int append(String chunk) {
    if (_isClosed || chunk.isEmpty) {
      return _estimator.estimate();
    }

    for (final codePoint in chunk.runes) {
      if (_isClosed) {
        break;
      }
      final char = String.fromCharCode(codePoint);

      if (!_isInsideOuterContent) {
        if (char == _openingTagEndChar) {
          _isInsideOuterContent = true;
        }
        continue;
      }

      if (_closeCandidate.isNotEmpty || char == '<') {
        final candidate = _closeCandidate.toString() + char;
        if (_closingPattern.startsWith(candidate)) {
          _closeCandidate.write(char);
          if (candidate == _closingPattern) {
            _isClosed = true;
            _closeCandidate.clear();
          }
          continue;
        }

        _estimator.append(candidate);
        _closeCandidate.clear();
        continue;
      }

      _estimator.appendCodePoint(codePoint);
    }

    return _estimator.estimate();
  }
}

_ToolResultRenderState _extractToolResult(
  _ParsedXml parsed,
  String xmlContent,
) {
  final toolName = (parsed.attr('name') ?? '').trim();
  final status = (parsed.attr('status') ?? 'success').trim().toLowerCase();
  final contentMatch = RegExp(
    r'<content\b[^>]*>([\s\S]*?)<\/content>',
    caseSensitive: false,
  ).firstMatch(xmlContent);
  final resultContent = (contentMatch?.group(1) ?? '').trim();
  final isSuccess = status == 'success';

  if (!isSuccess) {
    final errorMatch = RegExp(
      r'<error\b[^>]*>([\s\S]*?)<\/error>',
      caseSensitive: false,
    ).firstMatch(resultContent);
    return _ToolResultRenderState(
      toolName: toolName.isEmpty ? 'Unknown tool' : toolName,
      isSuccess: false,
      resultContent: (errorMatch?.group(1) ?? resultContent).trim(),
      fileDiffs: const <FileDiff>[],
    );
  }

  final fileDiffs = _extractFileDiffs(resultContent);
  final withoutFileDiff = resultContent
      .replaceAll(
        RegExp(r'<file-diff[\s\S]*<\/file-diff>', caseSensitive: false),
        '',
      )
      .trim();
  return _ToolResultRenderState(
    toolName: toolName.isEmpty ? 'Unknown tool' : toolName,
    isSuccess: true,
    resultContent: withoutFileDiff,
    fileDiffs: _isFileDiffTool(toolName) ? fileDiffs : const <FileDiff>[],
  );
}

bool _isFileDiffTool(String toolName) {
  return toolName == 'apply_file' ||
      toolName == 'create_file' ||
      toolName == 'edit_file';
}

List<FileDiff> _extractFileDiffs(String resultContent) {
  return RegExp(
    r'<file-diff\b([^>]*)>([\s\S]*?)<\/file-diff>',
    caseSensitive: false,
  ).allMatches(resultContent).map((match) {
    final attrs = match.group(1) ?? '';
    final body = match.group(2) ?? '';
    final path =
        RegExp(
          r'\bpath="([^"]*)"',
          caseSensitive: false,
        ).firstMatch(attrs)?.group(1) ??
        '';
    final details =
        RegExp(
          r'\bdetails="([^"]*)"',
          caseSensitive: false,
        ).firstMatch(attrs)?.group(1) ??
        '';
    final cdata = RegExp(
      r'<!\[CDATA\[([\s\S]*?)\]\]>',
      caseSensitive: false,
    ).firstMatch(body)?.group(1);
    return FileDiff(
      path: path,
      details: _decodeXmlText(details),
      diffContent: _decodeXmlText((cdata ?? body).trim()),
    );
  }).toList();
}

String _decodeXmlText(String input) {
  return input
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .replaceAll('&amp;', '&')
      .replaceAll('&quot;', '"')
      .replaceAll('&apos;', "'");
}
