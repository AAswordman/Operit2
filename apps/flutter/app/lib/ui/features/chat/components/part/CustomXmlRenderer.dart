// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:xml/xml.dart' as xml;

import '../../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../../core/logging/ClientLogger.dart';
import '../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../common/interactions/MessagePressShield.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../common/markdown/XmlRenderPluginRegistry.dart';
import '../../../../../util/ChatMarkupRegex.dart';
import '../../../packages/screens/ToolPkgUiLauncherScreen.dart';
import 'DetailsTagRenderer.dart';
import 'DialogComponents.dart';
import 'FileDiffDisplay.dart';
import 'FontTagRenderer.dart';
import 'ToolDisplayComponents.dart';
import 'ToolResultDisplay.dart';
import 'XmlCanvasBlockComponents.dart';

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
    required this.splitMarkdownContent,
  });

  final String xmlContent;
  final bool isStreaming;
  final Color textColor;
  final Stream<String>? xmlStream;
  final Stream<Object>? xmlMarkdownEventStream;
  final bool showThinkingProcess;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final MarkdownContentSplitter splitMarkdownContent;

  /// Builds the rendered XML message content.
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
    return _ToolPkgXmlRenderBridge(
      tagName: parsed.tagName,
      xmlContent: xmlContent,
      isStreaming: isStreaming,
      textColor: textColor,
      splitMarkdownContent: splitMarkdownContent,
      defaultBuilder: (context) => _buildDefaultXml(context, parsed),
    );
  }

  /// Builds the built-in XML renderer when no ToolPkg hook handles the tag.
  Widget _buildDefaultXml(BuildContext context, _ParsedXml parsed) {
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
          splitMarkdownContent: splitMarkdownContent,
          initiallyExpanded: initialThinkingExpanded,
          fullHeight: allowExpandedThinkingFullHeight,
        );
      case 'search':
        return _SearchPanel(
          xmlContent: xmlContent,
          textColor: textColor,
          isStreaming: isStreaming,
          splitMarkdownContent: splitMarkdownContent,
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
          splitMarkdownContent: splitMarkdownContent,
        );
      case 'details':
      case 'detail':
        return DetailsTagRenderer(
          xmlContent: xmlContent,
          textColor: textColor,
          isStreaming: isStreaming,
          splitMarkdownContent: splitMarkdownContent,
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
          splitMarkdownContent: splitMarkdownContent,
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

class _ToolPkgXmlRenderBridge extends StatefulWidget {
  const _ToolPkgXmlRenderBridge({
    required this.tagName,
    required this.xmlContent,
    required this.isStreaming,
    required this.textColor,
    required this.defaultBuilder,
    required this.splitMarkdownContent,
  });

  final String tagName;
  final String xmlContent;
  final bool isStreaming;
  final Color textColor;
  final WidgetBuilder defaultBuilder;
  final MarkdownContentSplitter splitMarkdownContent;

  @override
  State<_ToolPkgXmlRenderBridge> createState() =>
      _ToolPkgXmlRenderBridgeState();
}

class _ToolPkgXmlRenderBridgeState extends State<_ToolPkgXmlRenderBridge> {
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );

  late Future<Object?> _renderFuture = _loadRender();

  @override
  void didUpdateWidget(covariant _ToolPkgXmlRenderBridge oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.tagName != widget.tagName ||
        oldWidget.xmlContent != widget.xmlContent) {
      _renderFuture = _loadRender();
    }
  }

  /// Requests ToolPkg XML render output from the active Core runtime.
  Future<Object?> _loadRender() {
    return _clients.chatRuntimeHolderMain.renderToolPkgXml(
      tagName: widget.tagName,
      xmlContent: widget.xmlContent,
    );
  }

  /// Builds ToolPkg XML output when a hook handled the current tag.
  @override
  Widget build(BuildContext context) {
    return FutureBuilder<Object?>(
      future: _renderFuture,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done ||
            snapshot.hasError ||
            snapshot.data == null) {
          return widget.defaultBuilder(context);
        }
        final data = snapshot.data;
        if (data is! Map<String, Object?>) {
          return widget.defaultBuilder(context);
        }
        final kind = data['kind'];
        if (kind == 'text') {
          final text = data['text'];
          if (text is String && text.trim().isNotEmpty) {
            return StreamMarkdownRenderer(
              content: text,
              isStreaming: widget.isStreaming,
              textColor: widget.textColor,
              backgroundColor: Theme.of(context).colorScheme.surface,
              selectionRoot: false,
              splitMarkdownContent: widget.splitMarkdownContent,
            );
          }
        }
        if (kind == 'composeDsl') {
          final containerPackageName = data['containerPackageName'];
          final screen = data['screen'];
          if (containerPackageName is String && screen is String) {
            return _ToolPkgXmlComposeDslRender(
              clients: _clients,
              containerPackageName: containerPackageName,
              screen: screen,
              initialState: _requiredJsonObject(data['state'], 'state'),
              initialMemo: _requiredJsonObject(data['memo'], 'memo'),
              initialModuleSpec: data['moduleSpec'] == null
                  ? null
                  : _requiredJsonObject(data['moduleSpec'], 'moduleSpec'),
              defaultBuilder: widget.defaultBuilder,
            );
          }
        }
        return widget.defaultBuilder(context);
      },
    );
  }
}

class _ToolPkgXmlComposeDslRender extends StatefulWidget {
  const _ToolPkgXmlComposeDslRender({
    required this.clients,
    required this.containerPackageName,
    required this.screen,
    required this.initialState,
    required this.initialMemo,
    required this.initialModuleSpec,
    required this.defaultBuilder,
  });

  final GeneratedCoreProxyClients clients;
  final String containerPackageName;
  final String screen;
  final Map<String, Object?> initialState;
  final Map<String, Object?> initialMemo;
  final Map<String, Object?>? initialModuleSpec;
  final WidgetBuilder defaultBuilder;

  @override
  State<_ToolPkgXmlComposeDslRender> createState() =>
      _ToolPkgXmlComposeDslRenderState();
}

class _ToolPkgXmlComposeDslRenderState
    extends State<_ToolPkgXmlComposeDslRender> {
  late Future<core_proxy.ToolPkgContainerRuntime?> _pluginFuture =
      _loadPlugin();

  @override
  void didUpdateWidget(covariant _ToolPkgXmlComposeDslRender oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.containerPackageName != widget.containerPackageName) {
      _pluginFuture = _loadPlugin();
    }
  }

  /// Loads the ToolPkg container needed by the embedded Compose DSL renderer.
  Future<core_proxy.ToolPkgContainerRuntime?> _loadPlugin() {
    return widget.clients.application
        .packageManager()
        .getToolPkgContainerRuntime(
          containerPackageName: widget.containerPackageName,
        );
  }

  /// Builds an embedded Compose DSL surface for XML-render hook output.
  @override
  Widget build(BuildContext context) {
    return FutureBuilder<core_proxy.ToolPkgContainerRuntime?>(
      future: _pluginFuture,
      builder: (context, snapshot) {
        final plugin = snapshot.data;
        if (snapshot.connectionState != ConnectionState.done ||
            snapshot.hasError ||
            plugin == null) {
          return widget.defaultBuilder(context);
        }
        return ToolPkgUiLauncherScreen(
          clients: widget.clients,
          plugin: plugin,
          initialRouteId: widget.screen,
          showLauncherChrome: false,
          initialState: widget.initialState,
          initialMemo: widget.initialMemo,
          initialModuleSpec: widget.initialModuleSpec,
        );
      },
    );
  }
}

Map<String, Object?> _requiredJsonObject(Object? value, String fieldName) {
  if (value is! Map<Object?, Object?>) {
    throw StateError('ToolPkg XML composeDsl $fieldName must be an object');
  }
  final result = <String, Object?>{};
  for (final entry in value.entries) {
    if (entry.key is! String) {
      throw StateError(
        'ToolPkg XML composeDsl $fieldName has a non-string key',
      );
    }
    result[entry.key as String] = entry.value;
  }
  return result;
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

/// Renders thinking content with the established chat disclosure design.
class _ThinkPanel extends StatefulWidget {
  const _ThinkPanel({
    required this.text,
    required this.textColor,
    required this.isStreaming,
    required this.xmlStream,
    required this.markdownEventStream,
    required this.initiallyExpanded,
    required this.fullHeight,
    required this.splitMarkdownContent,
  });

  final String text;
  final Color textColor;
  final bool isStreaming;
  final Stream<String>? xmlStream;
  final Stream<Object>? markdownEventStream;
  final bool initiallyExpanded;
  final bool fullHeight;
  final MarkdownContentSplitter splitMarkdownContent;

  /// Creates state for the expandable thinking panel.
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

  /// Initializes expansion, height, and scroll state.
  @override
  void initState() {
    super.initState();
    _expanded = _targetExpandedFor(widget);
    _bodyFullHeight = widget.fullHeight && widget.initiallyExpanded;
    _expandSession = _expanded ? 1 : 0;
    _scrollController = ScrollController();
  }

  /// Syncs expansion state when streaming or caller defaults change.
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

  /// Releases the body scroll controller.
  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  /// Resolves the current expansion target from stream state and caller intent.
  bool _targetExpandedFor(_ThinkPanel widget) {
    if (widget.initiallyExpanded && !widget.isStreaming) {
      return true;
    }
    if (widget.isStreaming) {
      return true;
    }
    return false;
  }

  /// Toggles visibility for the thinking body.
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

  /// Toggles the thinking body between capped and complete height.
  void _handleBodyTap() {
    setState(() {
      _bodyFullHeight = !_bodyFullHeight;
    });
  }

  /// Restores automatic scrolling for a new expansion session.
  void _resetAutoScrollState() {
    _autoScrollEnabled = true;
    _userHasInteractedWithScroll = false;
  }

  /// Tracks user scroll position inside capped thinking bodies.
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

  /// Updates automatic scroll intent from the current capped scroll position.
  void _updateAutoScrollFromPosition() {
    if (!_userHasInteractedWithScroll || !_scrollController.hasClients) {
      return;
    }
    final position = _scrollController.position;
    const threshold = 80.0;
    _autoScrollEnabled =
        position.pixels >= position.maxScrollExtent - threshold;
  }

  /// Pins the capped thinking body to its latest streamed content.
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

  /// Builds the expandable thinking panel.
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
    final bodyConstraints = renderFullHeight
        ? const BoxConstraints()
        : const BoxConstraints(maxHeight: 300);
    final bodyScrollPhysics = renderFullHeight
        ? const NeverScrollableScrollPhysics()
        : null;
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
                  padding: EdgeInsets.zero,
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
                  axisAlignment: -1.0,
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
                                key: const ValueKey<String>(
                                  'thinking-body-toggle',
                                ),
                                behavior: HitTestBehavior.translucent,
                                onTap: _handleBodyTap,
                                child: SizedBox(
                                  width: double.infinity,
                                  child: AnimatedSize(
                                    alignment: Alignment.topCenter,
                                    duration: const Duration(milliseconds: 240),
                                    curve: Curves.easeOutCubic,
                                    child: ConstrainedBox(
                                      constraints: bodyConstraints,
                                      child:
                                          NotificationListener<
                                            ScrollNotification
                                          >(
                                            onNotification:
                                                _handleScrollNotification,
                                            child: SingleChildScrollView(
                                              controller: _scrollController,
                                              physics: bodyScrollPhysics,
                                              child: _ThinkMarkdownBody(
                                                contentText: contentText,
                                                contentStream:
                                                    hasStreamingMarkdown
                                                    ? widget.markdownEventStream
                                                    : null,
                                                textColor: widget.textColor,
                                                splitMarkdownContent:
                                                    widget.splitMarkdownContent,
                                              ),
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
    required this.splitMarkdownContent,
  });

  final String contentText;
  final Stream<Object>? contentStream;
  final Color textColor;
  final MarkdownContentSplitter splitMarkdownContent;

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
              splitMarkdownContent: splitMarkdownContent,
            )
          : StreamMarkdownRenderer(
              content: '',
              contentStream: stream,
              isStreaming: true,
              textColor: textColor.withValues(alpha: 0.6),
              backgroundColor: Colors.transparent,
              selectionRoot: false,
              splitMarkdownContent: splitMarkdownContent,
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

  /// Creates state that owns the title sweep animation.
  @override
  State<_ThinkingTitle> createState() => _ThinkingTitleState();
}

class _ThinkingTitleState extends State<_ThinkingTitle>
    with SingleTickerProviderStateMixin {
  static const double _highlightBandWidth = 0.9;
  static const double _highlightPathStart = -1.0 - _highlightBandWidth;
  static const double _highlightPathEnd = 1.0;

  late final AnimationController _controller;

  /// Starts the sweep animation when streamed thinking is visible.
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

  /// Syncs the sweep animation with streaming state changes.
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

  /// Disposes the sweep animation controller.
  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Builds the thinking title with a continuous off-text sweep loop.
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
        final sweepStart =
            _highlightPathStart +
            (_highlightPathEnd - _highlightPathStart) * _controller.value;
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
                      begin: Alignment(sweepStart, 0),
                      end: Alignment(sweepStart + _highlightBandWidth, 0),
                      colors: <Color>[
                        Colors.transparent,
                        theme.colorScheme.primary.withValues(alpha: 0.95),
                        Colors.transparent,
                      ],
                      stops: const <double>[0.0, 0.5, 1.0],
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

class _SearchPanel extends StatefulWidget {
  const _SearchPanel({
    required this.xmlContent,
    required this.textColor,
    required this.isStreaming,
    required this.splitMarkdownContent,
  });

  final String xmlContent;
  final Color textColor;
  final bool isStreaming;
  final MarkdownContentSplitter splitMarkdownContent;

  /// Creates state for rendering structured and text search content.
  @override
  State<_SearchPanel> createState() => _SearchPanelState();
}

class _SearchPanelState extends State<_SearchPanel> {
  bool _expanded = false;

  /// Toggles the visible source Markdown body.
  void _handleHeaderTap() {
    setState(() {
      _expanded = !_expanded;
    });
  }

  /// Builds the search body with the legacy structured and text modes.
  @override
  Widget build(BuildContext context) {
    final state = _SearchRenderState.parse(widget.xmlContent);
    if (state.isStructured) {
      if (state.sources.isEmpty && state.queries.isEmpty) {
        return const SizedBox.shrink();
      }
      return _StructuredSearchBody(
        renderState: state,
        textColor: widget.textColor,
      );
    }
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final bodyText = state.bodyText.trim();
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          CanvasExpandableHeaderRow(
            title: l10n.searchSources,
            semanticDescription: _expanded
                ? l10n.commonCollapse
                : l10n.commonExpand,
            expanded: _expanded,
            titleColor: widget.textColor.withValues(alpha: 0.7),
            rotationTurns: _expanded ? 0.25 : 0,
            onClick: _handleHeaderTap,
          ),
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 200),
            switchInCurve: Curves.linear,
            switchOutCurve: Curves.linear,
            transitionBuilder: (child, animation) {
              return FadeTransition(opacity: animation, child: child);
            },
            child: _expanded && bodyText.isNotEmpty
                ? CanvasIndentedGuide(
                    key: const ValueKey<String>('search-expanded'),
                    child: StreamMarkdownRenderer(
                      content: bodyText,
                      isStreaming: widget.isStreaming,
                      textColor: widget.textColor.withValues(alpha: 0.8),
                      backgroundColor: theme.colorScheme.surface,
                      selectionRoot: false,
                      splitMarkdownContent: widget.splitMarkdownContent,
                    ),
                  )
                : const SizedBox.shrink(
                    key: ValueKey<String>('search-collapsed'),
                  ),
          ),
        ],
      ),
    );
  }
}

class _StructuredSearchBody extends StatelessWidget {
  const _StructuredSearchBody({
    required this.renderState,
    required this.textColor,
  });

  final _SearchRenderState renderState;
  final Color textColor;

  /// Builds the structured source list or query list.
  @override
  Widget build(BuildContext context) {
    final searchRows = renderState.sources.isNotEmpty
        ? renderState.sources
        : renderState.queries.map(_SearchSource.query).toList(growable: false);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final source in searchRows)
          _SearchSourceRow(
            source: source,
            textColor: textColor,
            onClick: () {
              _showSearchSourceDetailDialog(context, source);
            },
          ),
      ],
    );
  }
}

class _SearchSourceRow extends StatelessWidget {
  const _SearchSourceRow({
    required this.source,
    required this.textColor,
    required this.onClick,
  });

  final _SearchSource source;
  final Color textColor;
  final VoidCallback onClick;

  /// Builds one compact, clickable search source row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final icon = source.url.isEmpty ? Icons.search : Icons.language;
    return MessagePressShieldRegion(
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onClick,
          borderRadius: BorderRadius.circular(6),
          child: Container(
            constraints: const BoxConstraints(minHeight: 24),
            padding: const EdgeInsets.symmetric(vertical: 2),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: <Widget>[
                SizedBox(
                  width: 18,
                  height: 18,
                  child: Center(
                    child: source.faviconUrl.isNotEmpty
                        ? _SearchFavicon(url: source.faviconUrl)
                        : Icon(
                            icon,
                            size: 14,
                            color: theme.colorScheme.primary.withValues(
                              alpha: 0.55,
                            ),
                          ),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    source.displayName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: textColor.withValues(alpha: 0.9),
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _SearchFavicon extends StatelessWidget {
  const _SearchFavicon({required this.url, this.size = 18});

  final String url;
  final double size;

  /// Builds the favicon image for one search source.
  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.primary;
    return ClipRRect(
      borderRadius: BorderRadius.circular(4),
      child: Image.network(
        url,
        width: size,
        height: size,
        fit: BoxFit.cover,
        loadingBuilder: (context, child, loadingProgress) {
          if (loadingProgress == null) {
            return child;
          }
          return Icon(
            Icons.language,
            size: size * 0.78,
            color: color.withValues(alpha: 0.45),
          );
        },
        errorBuilder: (context, error, stackTrace) {
          return Icon(
            Icons.language,
            size: size * 0.78,
            color: color.withValues(alpha: 0.55),
          );
        },
      ),
    );
  }
}

class _SearchSourceDetailDialog extends StatelessWidget {
  const _SearchSourceDetailDialog({
    required this.source,
    required this.onDismiss,
  });

  final _SearchSource source;
  final VoidCallback onDismiss;

  /// Builds the detail dialog for one search source row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final icon = source.url.isEmpty ? Icons.search : Icons.language;
    return AlertDialog(
      icon: source.faviconUrl.isNotEmpty
          ? SizedBox.square(
              dimension: 32,
              child: _SearchFavicon(url: source.faviconUrl, size: 32),
            )
          : Icon(icon),
      title: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Text(
            source.displayName,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          if (source.host.isNotEmpty)
            Text(
              source.host,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.labelMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
        ],
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (source.title.isNotEmpty)
            SelectableText(source.title, style: theme.textTheme.bodyMedium),
          if (source.url.isNotEmpty) ...<Widget>[
            if (source.title.isNotEmpty) const SizedBox(height: 8),
            SelectableText(
              source.url,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
      actions: <Widget>[
        if (source.url.isNotEmpty)
          TextButton(
            onPressed: () {
              unawaited(_openSearchSource(source.url));
            },
            child: Text(l10n.open),
          ),
        TextButton(onPressed: onDismiss, child: Text(l10n.close)),
      ],
    );
  }
}

class _SearchRenderState {
  const _SearchRenderState({
    required this.provider,
    required this.action,
    required this.status,
    required this.queries,
    required this.sources,
    required this.bodyText,
    required this.isStructured,
  });

  final String provider;
  final String action;
  final String status;
  final List<String> queries;
  final List<_SearchSource> sources;
  final String bodyText;
  final bool isStructured;

  /// Parses the search XML into the render state used by the UI.
  static _SearchRenderState parse(String content) {
    final bodyText = _extractContentFromXml(content, tagName: 'search').trim();
    final parsed = _parseStructuredSearchXml(content, bodyText);
    return parsed ??
        _SearchRenderState(
          provider: '',
          action: '',
          status: '',
          queries: const <String>[],
          sources: const <_SearchSource>[],
          bodyText: bodyText,
          isStructured: false,
        );
  }
}

class _SearchSource {
  const _SearchSource({
    required this.title,
    required this.url,
    required this.faviconUrl,
    required this.host,
    required this.siteName,
    required this.displayName,
    required this.attributes,
  });

  factory _SearchSource.query(String query) {
    return _SearchSource(
      title: query,
      url: '',
      faviconUrl: '',
      host: '',
      siteName: '',
      displayName: query,
      attributes: <String, String>{'type': 'query', 'query': query},
    );
  }

  final String title;
  final String url;
  final String faviconUrl;
  final String host;
  final String siteName;
  final String displayName;
  final Map<String, String> attributes;
}

/// Parses structured search XML produced by model search providers.
_SearchRenderState? _parseStructuredSearchXml(String content, String bodyText) {
  try {
    final document = xml.XmlDocument.parse(content);
    final root = document.rootElement;
    final hasSearchRoot = root.name.local.toLowerCase() == 'search';
    final provider = _searchAttribute(root, 'provider');
    final action = _searchAttribute(root, 'action');
    final status = _searchAttribute(root, 'status');
    final queries = root.descendants
        .whereType<xml.XmlElement>()
        .where((element) => element.name.local.toLowerCase() == 'query')
        .map((element) => element.innerText.trim())
        .where((value) => value.isNotEmpty)
        .toList(growable: false);
    final sources = root.descendants
        .whereType<xml.XmlElement>()
        .where((element) => element.name.local.toLowerCase() == 'source')
        .map(_searchSourceFromElement)
        .whereType<_SearchSource>()
        .toList(growable: false);
    final structured =
        hasSearchRoot &&
        (provider.isNotEmpty ||
            action.isNotEmpty ||
            status.isNotEmpty ||
            queries.isNotEmpty ||
            sources.isNotEmpty);
    if (!structured) {
      return null;
    }
    return _SearchRenderState(
      provider: provider,
      action: action,
      status: status,
      queries: queries,
      sources: sources,
      bodyText: bodyText,
      isStructured: true,
    );
  } on xml.XmlException {
    return null;
  }
}

/// Builds one render source from a structured source element.
_SearchSource? _searchSourceFromElement(xml.XmlElement element) {
  final url = _searchAttribute(element, 'url');
  if (url.isEmpty) {
    return null;
  }
  final title = _searchAttribute(element, 'title');
  final host = _hostFromSearchUrl(url);
  final siteName = _searchSourceSiteName(element);
  return _SearchSource(
    title: title,
    url: url,
    faviconUrl: _faviconUrlForSearchSource(url),
    host: host,
    siteName: siteName,
    displayName: _searchSourceDisplayName(siteName, host, title, url),
    attributes: _searchAttributes(element),
  );
}

/// Reads and trims a named XML attribute.
String _searchAttribute(xml.XmlElement element, String name) {
  return element.getAttribute(name)?.trim() ?? '';
}

/// Reads all non-empty XML attributes from a source element.
Map<String, String> _searchAttributes(xml.XmlElement element) {
  final attributes = <String, String>{};
  for (final attribute in element.attributes) {
    final name = attribute.name.qualified.trim();
    final value = attribute.value.trim();
    if (name.isNotEmpty && value.isNotEmpty) {
      attributes[name] = value;
    }
  }
  return attributes;
}

/// Resolves the site name attribute used by search providers.
String _searchSourceSiteName(xml.XmlElement element) {
  for (final name in const <String>[
    'site_name',
    'siteName',
    'source',
    'name',
  ]) {
    final value = _searchAttribute(element, name);
    if (value.isNotEmpty) {
      return value;
    }
  }
  return '';
}

/// Extracts a host name from a search result URL.
String _hostFromSearchUrl(String url) {
  var host = url;
  final schemeIndex = host.indexOf('://');
  if (schemeIndex >= 0) {
    host = host.substring(schemeIndex + 3);
  }
  final slashIndex = host.indexOf('/');
  if (slashIndex >= 0) {
    host = host.substring(0, slashIndex);
  }
  final questionIndex = host.indexOf('?');
  if (questionIndex >= 0) {
    host = host.substring(0, questionIndex);
  }
  final fragmentIndex = host.indexOf('#');
  if (fragmentIndex >= 0) {
    host = host.substring(0, fragmentIndex);
  }
  final atIndex = host.lastIndexOf('@');
  if (atIndex >= 0) {
    host = host.substring(atIndex + 1);
  }
  final colonIndex = host.indexOf(':');
  if (colonIndex >= 0) {
    host = host.substring(0, colonIndex);
  }
  return host.trim();
}

/// Chooses the compact row label for a search source.
String _searchSourceDisplayName(
  String siteName,
  String host,
  String title,
  String url,
) {
  if (siteName.isNotEmpty) {
    return siteName;
  }
  final domainName = host.startsWith('www.') ? host.substring(4).trim() : host;
  if (domainName.isNotEmpty) {
    return domainName;
  }
  if (title.isNotEmpty) {
    return title;
  }
  return url;
}

/// Builds the favicon URL for a search source host.
String _faviconUrlForSearchSource(String url) {
  final host = _hostFromSearchUrl(url).toLowerCase();
  if (host.isEmpty) {
    return '';
  }
  return 'https://www.google.com/s2/favicons?sz=64&domain=$host';
}

/// Opens a search source URL from the detail dialog.
Future<void> _openSearchSource(String url) async {
  try {
    await launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);
  } catch (error, stackTrace) {
    ClientLogger.w(
      'Cannot open search source: $url',
      tag: 'SearchDisplay',
      error: error,
      stackTrace: stackTrace,
    );
  }
}

/// Shows the detail dialog for a selected search source.
void _showSearchSourceDetailDialog(BuildContext context, _SearchSource source) {
  showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return _SearchSourceDetailDialog(
        source: source,
        onDismiss: () => Navigator.of(dialogContext).pop(),
      );
    },
  );
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
    xml = sanitizeUtf16(xml);
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
