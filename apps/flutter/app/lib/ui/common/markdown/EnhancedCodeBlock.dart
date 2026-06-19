// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:webview_all/webview_all.dart';

import '../interactions/MessagePressShield.dart';
import 'CanvasMonospaceCodeBlockBody.dart';

enum CodeBlockPreviewType { mermaid, html }

class EnhancedCodeBlock extends StatefulWidget {
  const EnhancedCodeBlock({super.key, required this.code, this.language = ''});

  final String code;
  final String language;

  @override
  State<EnhancedCodeBlock> createState() => _EnhancedCodeBlockState();
}

class _EnhancedCodeBlockState extends State<EnhancedCodeBlock> {
  bool autoWrapEnabled = true;
  bool showCopiedToast = false;
  bool showRenderedMermaid = false;
  bool showRenderedHtml = false;
  bool showFullscreenPreview = false;
  CodeBlockPreviewType? fullscreenPreviewType;

  @override
  Widget build(BuildContext context) {
    final lines = widget.code.split('\n');
    final highlightedLines = <InlineSpan>[
      for (final line in lines) _highlightSyntaxLine(line, widget.language),
    ];
    final isMermaid = widget.language.toLowerCase() == 'mermaid';
    final isHtml =
        widget.language.toLowerCase() == 'html' ||
        widget.language.toLowerCase() == 'htm';
    final isPreviewMode =
        (isMermaid && showRenderedMermaid) || (isHtml && showRenderedHtml);
    const codeBlockBackground = Color(0xFF1E1E1E);
    const toolbarBackground = Color(0xFF252526);
    final block = MessagePressShieldRegion(
      child: Semantics(
        label: widget.language.isEmpty
            ? 'Code block'
            : '${widget.language} Code block',
        child: Container(
          width: double.infinity,
          margin: const EdgeInsets.symmetric(vertical: 2),
          decoration: BoxDecoration(
            color: codeBlockBackground,
            borderRadius: BorderRadius.circular(4),
          ),
          clipBehavior: Clip.antiAlias,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Container(
                width: double.infinity,
                color: toolbarBackground,
                padding: const EdgeInsets.all(4),
                child: Row(
                  children: <Widget>[
                    if (widget.language.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.only(left: 8),
                        child: Text(
                          widget.language,
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(color: const Color(0xFFAAAAAA)),
                        ),
                      ),
                    const Spacer(),
                    if (isMermaid)
                      MessagePressShieldRegion(
                        child: IconButton(
                          onPressed: () {
                            setState(() {
                              showRenderedMermaid = !showRenderedMermaid;
                            });
                          },
                          iconSize: 16,
                          constraints: const BoxConstraints.tightFor(
                            width: 28,
                            height: 28,
                          ),
                          padding: EdgeInsets.zero,
                          color: showRenderedMermaid
                              ? Theme.of(context).colorScheme.primary
                              : const Color(0xFFAAAAAA),
                          icon: const Icon(Icons.play_arrow),
                          tooltip: showRenderedMermaid ? 'Code' : 'Mermaid',
                        ),
                      ),
                    if (isHtml)
                      MessagePressShieldRegion(
                        child: IconButton(
                          onPressed: () {
                            setState(() {
                              showRenderedHtml = !showRenderedHtml;
                            });
                          },
                          iconSize: 16,
                          constraints: const BoxConstraints.tightFor(
                            width: 28,
                            height: 28,
                          ),
                          padding: EdgeInsets.zero,
                          color: showRenderedHtml
                              ? Theme.of(context).colorScheme.primary
                              : const Color(0xFFAAAAAA),
                          icon: const Icon(Icons.play_arrow),
                          tooltip: showRenderedHtml ? 'Code' : 'HTML',
                        ),
                      ),
                    if (isPreviewMode)
                      MessagePressShieldRegion(
                        child: IconButton(
                          onPressed: () {
                            setState(() {
                              fullscreenPreviewType = showRenderedMermaid
                                  ? CodeBlockPreviewType.mermaid
                                  : CodeBlockPreviewType.html;
                              showFullscreenPreview = true;
                            });
                          },
                          iconSize: 16,
                          constraints: const BoxConstraints.tightFor(
                            width: 28,
                            height: 28,
                          ),
                          padding: EdgeInsets.zero,
                          color: const Color(0xFFAAAAAA),
                          icon: const Icon(Icons.fullscreen),
                          tooltip: 'Fullscreen',
                        ),
                      ),
                    MessagePressShieldRegion(
                      child: IconButton(
                        onPressed: () {
                          setState(() {
                            autoWrapEnabled = !autoWrapEnabled;
                          });
                        },
                        disabledColor: const Color(0xFF666666),
                        enableFeedback: !isPreviewMode,
                        iconSize: 16,
                        constraints: const BoxConstraints.tightFor(
                          width: 28,
                          height: 28,
                        ),
                        padding: EdgeInsets.zero,
                        color: const Color(0xFFAAAAAA),
                        icon: const Icon(Icons.swap_horiz),
                        tooltip: 'Wrap',
                      ),
                    ),
                    MessagePressShieldRegion(
                      child: IconButton(
                        onPressed: () {
                          Clipboard.setData(ClipboardData(text: widget.code));
                          setState(() {
                            showCopiedToast = true;
                          });
                        },
                        iconSize: 16,
                        constraints: const BoxConstraints.tightFor(
                          width: 28,
                          height: 28,
                        ),
                        padding: EdgeInsets.zero,
                        color: showCopiedToast
                            ? Theme.of(context).colorScheme.primary
                            : const Color(0xFFAAAAAA),
                        icon: const Icon(Icons.content_copy),
                        tooltip: 'Copy',
                      ),
                    ),
                  ],
                ),
              ),
              if (isMermaid && showRenderedMermaid)
                MermaidRenderer(code: widget.code, height: 300)
              else if (isHtml && showRenderedHtml)
                HtmlPreviewRenderer(code: widget.code, height: 360)
              else
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 420),
                  child: SingleChildScrollView(
                    scrollDirection: Axis.vertical,
                    child: SingleChildScrollView(
                      scrollDirection: autoWrapEnabled
                          ? Axis.vertical
                          : Axis.horizontal,
                      child: Padding(
                        padding: const EdgeInsets.symmetric(vertical: 8),
                        child: CanvasMonospaceCodeBlockBody(
                          lines: lines,
                          highlightedLines: highlightedLines,
                          autoWrapEnabled: autoWrapEnabled,
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
    if (!showFullscreenPreview) {
      return block;
    }
    return Stack(
      children: <Widget>[
        block,
        _FullscreenPreviewDialog(
          type: fullscreenPreviewType,
          code: widget.code,
          onClose: () {
            setState(() {
              showFullscreenPreview = false;
            });
          },
        ),
      ],
    );
  }
}

class _FullscreenPreviewDialog extends StatelessWidget {
  const _FullscreenPreviewDialog({
    required this.type,
    required this.code,
    required this.onClose,
  });

  final CodeBlockPreviewType? type;
  final String code;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return Dialog.fullscreen(
      backgroundColor: Colors.black,
      child: Stack(
        children: <Widget>[
          Positioned.fill(
            child: switch (type) {
              CodeBlockPreviewType.mermaid => MermaidRenderer(code: code),
              CodeBlockPreviewType.html => HtmlPreviewRenderer(code: code),
              null => const SizedBox.shrink(),
            },
          ),
          Positioned(
            top: 12,
            right: 12,
            child: MessagePressShieldRegion(
              child: IconButton(
                onPressed: onClose,
                color: Colors.white,
                icon: const Icon(Icons.close),
                tooltip: 'Close',
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class MermaidRenderer extends StatelessWidget {
  const MermaidRenderer({super.key, required this.code, this.height});

  final String code;
  final double? height;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: height,
      child: _HtmlWebView(
        html:
            '''
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <script src="https://cdn.jsdelivr.net/npm/mermaid@10.6.1/dist/mermaid.min.js"></script>
  <style>
    body { background:#1E1E1E; margin:0; padding:16px; overflow:auto; }
    .mermaid { font-family: monospace; font-size:14px; }
  </style>
</head>
<body>
  <pre class="mermaid">${_escapeHtml(code.trim())}</pre>
  <script>mermaid.initialize({startOnLoad:true,theme:'dark',securityLevel:'loose',flowchart:{htmlLabels:true}});</script>
</body>
</html>
''',
        javaScriptMode: JavaScriptMode.unrestricted,
        backgroundColor: const Color(0xFF1E1E1E),
      ),
    );
  }
}

class HtmlPreviewRenderer extends StatelessWidget {
  const HtmlPreviewRenderer({super.key, required this.code, this.height});

  final String code;
  final double? height;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: height,
      child: _HtmlWebView(
        html: code.trim(),
        javaScriptMode: JavaScriptMode.disabled,
        backgroundColor: Colors.white,
      ),
    );
  }
}

class _HtmlWebView extends StatefulWidget {
  const _HtmlWebView({
    required this.html,
    required this.javaScriptMode,
    required this.backgroundColor,
  });

  final String html;
  final JavaScriptMode javaScriptMode;
  final Color backgroundColor;

  @override
  State<_HtmlWebView> createState() => _HtmlWebViewState();
}

class _HtmlWebViewState extends State<_HtmlWebView> {
  late final WebViewController _controller;

  @override
  void initState() {
    super.initState();
    _controller = WebViewController()
      ..setJavaScriptMode(widget.javaScriptMode)
      ..setBackgroundColor(widget.backgroundColor);
    _controller.loadHtmlString(widget.html);
  }

  @override
  void didUpdateWidget(covariant _HtmlWebView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.html != widget.html ||
        oldWidget.javaScriptMode != widget.javaScriptMode ||
        oldWidget.backgroundColor != widget.backgroundColor) {
      _controller
        ..setJavaScriptMode(widget.javaScriptMode)
        ..setBackgroundColor(widget.backgroundColor)
        ..loadHtmlString(widget.html);
    }
  }

  @override
  Widget build(BuildContext context) {
    return WebViewWidget(controller: _controller);
  }
}

InlineSpan _highlightSyntaxLine(String line, String language) {
  final textColor = const Color(0xFFD4D4D4);
  final keywordColor = const Color(0xFF569CD6);
  final stringColor = const Color(0xFFCE9178);
  final commentColor = const Color(0xFF6A9955);
  final numberColor = const Color(0xFFB5CEA8);
  final typeColor = const Color(0xFF4EC9B0);
  final functionColor = const Color(0xFFDCDCAA);
  final lower = language.toLowerCase();
  final keywords = lower == 'mermaid'
      ? <String>{
          'graph',
          'flowchart',
          'sequenceDiagram',
          'classDiagram',
          'stateDiagram',
          'subgraph',
          'end',
          'participant',
          'actor',
          'note',
          'loop',
          'alt',
          'else',
        }
      : <String>{
          'fun',
          'val',
          'var',
          'class',
          'interface',
          'object',
          'return',
          'if',
          'else',
          'when',
          'for',
          'while',
          'import',
          'const',
          'final',
          'static',
          'async',
          'await',
          'void',
          'true',
          'false',
          'null',
        };
  final types = <String>{
    'String',
    'Int',
    'Double',
    'Float',
    'Boolean',
    'List',
    'Map',
    'Set',
    'Array',
    'Object',
    'Promise',
  };
  if (line.trimLeft().startsWith('//') || line.trimLeft().startsWith('%')) {
    return TextSpan(
      text: line,
      style: TextStyle(color: commentColor),
    );
  }
  final spans = <TextSpan>[];
  final tokenPattern = RegExp(
    r'''("[^"]*"|'[^']*'|\d+(?:\.\d+)?|[A-Za-z_][A-Za-z0-9_]*|\S|\s+)''',
  );
  for (final match in tokenPattern.allMatches(line)) {
    final token = match.group(0)!;
    final color = token.startsWith('"') || token.startsWith("'")
        ? stringColor
        : keywords.contains(token)
        ? keywordColor
        : types.contains(token)
        ? typeColor
        : RegExp(r'^\d+(?:\.\d+)?$').hasMatch(token)
        ? numberColor
        : _looksLikeFunction(line, match.end)
        ? functionColor
        : textColor;
    spans.add(
      TextSpan(
        text: token,
        style: TextStyle(color: color),
      ),
    );
  }
  return TextSpan(children: spans);
}

bool _looksLikeFunction(String line, int tokenEnd) {
  var cursor = tokenEnd;
  while (cursor < line.length && line[cursor].trim().isEmpty) {
    cursor++;
  }
  return cursor < line.length && line[cursor] == '(';
}

String _escapeHtml(String value) {
  return value
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
}
