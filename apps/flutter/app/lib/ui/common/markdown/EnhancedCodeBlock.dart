// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:webview_all/webview_all.dart';

import '../interactions/MessagePressShield.dart';
import 'CanvasMonospaceCodeBlockBody.dart';
import 'MarkdownCodeTypeface.dart';
import 'Utf16Text.dart';

// Lucide SVG Icons (MIT License)
const String _kSvgWrapText =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><line x1="3" x2="21" y1="6" y2="6"/><path d="M3 12h15a3 3 0 1 1 0 6h-4"/><polyline points="16 16 14 18 16 20"/><line x1="3" x2="10" y1="18" y2="18"/></svg>';
const String _kSvgCopy =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect width="13" height="13" x="8" y="8" rx="1.2" ry="1.2"/><path d="M4 16c-.6 0-1-.4-1-1V4c0-.6.4-1 1-1h11c.6 0 1 .4 1 1"/></svg>';
const String _kSvgCheck =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>';
const String _kSvgPlay =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="6 3 20 12 6 21 6 3"/></svg>';
const String _kSvgCode =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>';
const String _kSvgMaximize =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3"/><path d="M21 8V5a2 2 0 0 0-2-2h-3"/><path d="M3 16v3a2 2 0 0 0 2 2h3"/><path d="M16 21h3a2 2 0 0 0 2-2v-3"/></svg>';
const String _kSvgClose =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>';

class _CodeBlockSvgIcon extends StatelessWidget {
  const _CodeBlockSvgIcon(this.svg, {this.size = 13.5, this.color});

  final String svg;
  final double size;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final effectiveColor = color ?? const Color(0xFFAAAAAA);
    return SvgPicture.string(
      svg,
      width: size,
      height: size,
      colorFilter: ColorFilter.mode(effectiveColor, BlendMode.srcIn),
    );
  }
}

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
  Timer? _copiedResetTimer;

  @override
  void dispose() {
    _copiedResetTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final code = sanitizeUtf16(widget.code);
    final lines = code.split('\n');
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
    const borderColor = Color(0xFF33363B);
    final toolbarButtonStyle = IconButton.styleFrom(
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      padding: EdgeInsets.zero,
      minimumSize: const Size(22, 22),
      fixedSize: const Size(22, 22),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(4),
      ),
    );
    final codeBody = Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: CanvasMonospaceCodeBlockBody(
        lines: lines,
        highlightedLines: highlightedLines,
        autoWrapEnabled: autoWrapEnabled,
      ),
    );
    final block = MessagePressShieldRegion(
      child: Semantics(
        label: widget.language.isEmpty
            ? 'Code block'
            : '${widget.language} Code block',
        child: Container(
          width: double.infinity,
          margin: const EdgeInsets.symmetric(vertical: 4),
          decoration: BoxDecoration(
            color: codeBlockBackground,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: borderColor, width: 1),
          ),
          clipBehavior: Clip.antiAlias,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              MediaQuery.withNoTextScaling(
                child: Container(
                  height: 24,
                  width: double.infinity,
                  decoration: const BoxDecoration(
                    color: toolbarBackground,
                    border: Border(
                      bottom: BorderSide(color: borderColor, width: 0.5),
                    ),
                  ),
                  padding: const EdgeInsets.symmetric(horizontal: 6),
                  child: Row(
                    children: <Widget>[
                      Expanded(
                        child: widget.language.isNotEmpty
                            ? Padding(
                                padding: const EdgeInsets.only(left: 4),
                                child: Text(
                                  widget.language,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: Theme.of(context).textTheme.bodySmall
                                      ?.copyWith(
                                        color: const Color(0xFFAAAAAA),
                                        fontSize: 11,
                                        height: 1.0,
                                        fontFamily: markdownCodeFontFamily,
                                        fontFamilyFallback:
                                            markdownCodeFontFamilyFallback,
                                      ),
                                ),
                              )
                            : const SizedBox.shrink(),
                      ),
                      if (isMermaid) ...<Widget>[
                        MessagePressShieldRegion(
                          child: IconButton(
                            onPressed: () {
                              setState(() {
                                showRenderedMermaid = !showRenderedMermaid;
                              });
                            },
                            style: toolbarButtonStyle,
                            iconSize: 13.5,
                            constraints: const BoxConstraints.tightFor(
                              width: 22,
                              height: 22,
                            ),
                            padding: EdgeInsets.zero,
                            icon: _CodeBlockSvgIcon(
                              showRenderedMermaid ? _kSvgCode : _kSvgPlay,
                              color: showRenderedMermaid
                                  ? Theme.of(context).colorScheme.primary
                                  : const Color(0xFFAAAAAA),
                            ),
                            tooltip: showRenderedMermaid ? 'Code' : 'Mermaid',
                          ),
                        ),
                        const SizedBox(width: 2),
                      ],
                      if (isHtml) ...<Widget>[
                        MessagePressShieldRegion(
                          child: IconButton(
                            onPressed: () {
                              setState(() {
                                showRenderedHtml = !showRenderedHtml;
                              });
                            },
                            style: toolbarButtonStyle,
                            iconSize: 13.5,
                            constraints: const BoxConstraints.tightFor(
                              width: 22,
                              height: 22,
                            ),
                            padding: EdgeInsets.zero,
                            icon: _CodeBlockSvgIcon(
                              showRenderedHtml ? _kSvgCode : _kSvgPlay,
                              color: showRenderedHtml
                                  ? Theme.of(context).colorScheme.primary
                                  : const Color(0xFFAAAAAA),
                            ),
                            tooltip: showRenderedHtml ? 'Code' : 'HTML',
                          ),
                        ),
                        const SizedBox(width: 2),
                      ],
                      if (isPreviewMode) ...<Widget>[
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
                            style: toolbarButtonStyle,
                            iconSize: 13.5,
                            constraints: const BoxConstraints.tightFor(
                              width: 22,
                              height: 22,
                            ),
                            padding: EdgeInsets.zero,
                            icon: const _CodeBlockSvgIcon(
                              _kSvgMaximize,
                              color: Color(0xFFAAAAAA),
                            ),
                            tooltip: 'Fullscreen',
                          ),
                        ),
                        const SizedBox(width: 2),
                      ],
                      MessagePressShieldRegion(
                        child: IconButton(
                          onPressed: isPreviewMode
                              ? null
                              : () {
                                  setState(() {
                                    autoWrapEnabled = !autoWrapEnabled;
                                  });
                                },
                          style: toolbarButtonStyle,
                          disabledColor: const Color(0xFF666666),
                          enableFeedback: !isPreviewMode,
                          iconSize: 13.5,
                          constraints: const BoxConstraints.tightFor(
                            width: 22,
                            height: 22,
                          ),
                          padding: EdgeInsets.zero,
                          icon: _CodeBlockSvgIcon(
                            _kSvgWrapText,
                            color: autoWrapEnabled
                                ? const Color(0xFFAAAAAA)
                                : Theme.of(context).colorScheme.primary,
                          ),
                          tooltip: autoWrapEnabled ? 'Disable word wrap' : 'Enable word wrap',
                        ),
                      ),
                      const SizedBox(width: 2),
                      MessagePressShieldRegion(
                        child: IconButton(
                          onPressed: () async {
                            try {
                              await Clipboard.setData(
                                ClipboardData(text: widget.code),
                              );
                            } on PlatformException {
                              return;
                            }
                            if (!mounted) {
                              return;
                            }
                            _copiedResetTimer?.cancel();
                            setState(() {
                              showCopiedToast = true;
                            });
                            _copiedResetTimer = Timer(
                              const Duration(milliseconds: 1500),
                              () {
                                if (mounted) {
                                  setState(() {
                                    showCopiedToast = false;
                                  });
                                }
                              },
                            );
                          },
                          style: toolbarButtonStyle,
                          iconSize: 13.5,
                          constraints: const BoxConstraints.tightFor(
                            width: 22,
                            height: 22,
                          ),
                          padding: EdgeInsets.zero,
                          icon: _CodeBlockSvgIcon(
                            showCopiedToast ? _kSvgCheck : _kSvgCopy,
                            color: showCopiedToast
                                ? Theme.of(context).colorScheme.primary
                                : const Color(0xFFAAAAAA),
                          ),
                          tooltip: 'Copy',
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              if (isMermaid && showRenderedMermaid)
                MermaidRenderer(code: code, height: 300)
              else if (isHtml && showRenderedHtml)
                HtmlPreviewRenderer(code: code, height: 360)
              else
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 420),
                  child: SingleChildScrollView(
                    scrollDirection: Axis.vertical,
                    child: autoWrapEnabled
                        ? codeBody
                        : SingleChildScrollView(
                            scrollDirection: Axis.horizontal,
                            child: codeBody,
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
          code: code,
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
                icon: const _CodeBlockSvgIcon(
                  _kSvgClose,
                  color: Colors.white,
                  size: 20,
                ),
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

const Color _syntaxTextColor = Color(0xFFD4D4D4);
const Color _syntaxKeywordColor = Color(0xFF569CD6);
const Color _syntaxStringColor = Color(0xFFCE9178);
const Color _syntaxCommentColor = Color(0xFF6A9955);
const Color _syntaxNumberColor = Color(0xFFB5CEA8);
const Color _syntaxTypeColor = Color(0xFF4EC9B0);
const Color _syntaxFunctionColor = Color(0xFFDCDCAA);

final RegExp _identifierPattern = RegExp(r'^[A-Za-z_$][A-Za-z0-9_$]*$');
final RegExp _numberPattern = RegExp(
  r'^(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|\d[\d_]*(?:\.[\d_]+)?(?:[eE][+-]?\d+)?)$',
);
final RegExp _tokenPattern = RegExp(
  r'''("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|//.*$|0[xX][0-9a-fA-F_]+|0[bB][01_]+|\d[\d_]*(?:\.[\d_]+)?(?:[eE][+-]?\d+)?|[A-Za-z_$][A-Za-z0-9_$]*|\S|\s+)''',
  unicode: true,
);
final RegExp _hashCommentTokenPattern = RegExp(
  r'''("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|#.*$|//.*$|0[xX][0-9a-fA-F_]+|0[bB][01_]+|\d[\d_]*(?:\.[\d_]+)?(?:[eE][+-]?\d+)?|[A-Za-z_$][A-Za-z0-9_$]*|\S|\s+)''',
  unicode: true,
);

const Set<String> _mermaidKeywords = <String>{
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
};

const Set<String> _generalKeywords = <String>{
  'fun',
  'val',
  'var',
  'let',
  'function',
  'def',
  'fn',
  'pub',
  'mut',
  'impl',
  'trait',
  'struct',
  'enum',
  'match',
  'use',
  'mod',
  'crate',
  'where',
  'class',
  'interface',
  'type',
  'extends',
  'implements',
  'object',
  'return',
  'yield',
  'if',
  'elif',
  'else',
  'when',
  'switch',
  'case',
  'default',
  'for',
  'while',
  'do',
  'break',
  'continue',
  'in',
  'of',
  'is',
  'as',
  'import',
  'export',
  'from',
  'package',
  'const',
  'final',
  'static',
  'async',
  'await',
  'try',
  'catch',
  'finally',
  'throw',
  'raise',
  'new',
  'this',
  'super',
  'self',
  'typeof',
  'instanceof',
  'void',
  'true',
  'false',
  'null',
  'undefined',
  'None',
  'True',
  'False',
};

const Set<String> _generalTypes = <String>{
  'String',
  'str',
  'Int',
  'int',
  'Double',
  'double',
  'Float',
  'float',
  'Boolean',
  'bool',
  'List',
  'Map',
  'Set',
  'Array',
  'Object',
  'Promise',
  'Future',
  'Stream',
  'Option',
  'Result',
  'Vec',
  'Box',
  'Arc',
  'Rc',
  'Date',
  'RegExp',
  'Error',
  'Widget',
  'BuildContext',
  'State',
  'Color',
};

const Set<String> _hashCommentLanguages = <String>{
  'python',
  'py',
  'sh',
  'bash',
  'zsh',
  'shell',
  'powershell',
  'ps1',
  'yaml',
  'yml',
  'toml',
  'ruby',
  'rb',
  'r',
  'dockerfile',
  'makefile',
  'conf',
  'ini',
};

InlineSpan _highlightSyntaxLine(String line, String language) {
  final lower = language.toLowerCase();
  final keywords = lower == 'mermaid' ? _mermaidKeywords : _generalKeywords;
  final trimmedLeft = line.trimLeft();
  if (trimmedLeft.startsWith('//') ||
      trimmedLeft.startsWith('%') ||
      (_hashCommentLanguages.contains(lower) && trimmedLeft.startsWith('#'))) {
    return TextSpan(
      text: line,
      style: const TextStyle(color: _syntaxCommentColor),
    );
  }
  final pattern = _hashCommentLanguages.contains(lower)
      ? _hashCommentTokenPattern
      : _tokenPattern;
  final spans = <InlineSpan>[];
  for (final match in pattern.allMatches(line)) {
    final token = match.group(0)!;
    if (token.startsWith('`') && token.endsWith('`') && token.length >= 2) {
      _appendTemplateStringSpans(spans, token, language);
      continue;
    }
    final isComment =
        token.startsWith('//') ||
        (_hashCommentLanguages.contains(lower) && token.startsWith('#'));
    final isString = token.startsWith('"') || token.startsWith("'");
    final isIdentifier = _identifierPattern.hasMatch(token);
    final Color color;
    if (isComment) {
      color = _syntaxCommentColor;
    } else if (isString) {
      color = _syntaxStringColor;
    } else if (keywords.contains(token)) {
      color = _syntaxKeywordColor;
    } else if (_generalTypes.contains(token)) {
      color = _syntaxTypeColor;
    } else if (_numberPattern.hasMatch(token)) {
      color = _syntaxNumberColor;
    } else if (isIdentifier && _looksLikeFunction(line, match.end)) {
      color = _syntaxFunctionColor;
    } else {
      color = _syntaxTextColor;
    }
    spans.add(
      TextSpan(
        text: token,
        style: TextStyle(color: color),
      ),
    );
  }
  return TextSpan(children: spans);
}

void _appendTemplateStringSpans(
  List<InlineSpan> spans,
  String token,
  String language,
) {
  var cursor = 0;
  while (cursor < token.length) {
    final exprStart = token.indexOf(r'${', cursor);
    if (exprStart < 0) {
      spans.add(
        TextSpan(
          text: token.substring(cursor),
          style: const TextStyle(color: _syntaxStringColor),
        ),
      );
      break;
    }
    final exprEnd = token.indexOf('}', exprStart + 2);
    if (exprEnd < 0) {
      spans.add(
        TextSpan(
          text: token.substring(cursor),
          style: const TextStyle(color: _syntaxStringColor),
        ),
      );
      break;
    }
    if (exprStart > cursor) {
      spans.add(
        TextSpan(
          text: token.substring(cursor, exprStart),
          style: const TextStyle(color: _syntaxStringColor),
        ),
      );
    }
    spans.add(
      const TextSpan(
        text: r'${',
        style: TextStyle(color: _syntaxKeywordColor),
      ),
    );
    final expr = token.substring(exprStart + 2, exprEnd);
    if (expr.isNotEmpty) {
      spans.add(_highlightSyntaxLine(expr, language));
    }
    spans.add(
      const TextSpan(
        text: '}',
        style: TextStyle(color: _syntaxKeywordColor),
      ),
    );
    cursor = exprEnd + 1;
  }
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
