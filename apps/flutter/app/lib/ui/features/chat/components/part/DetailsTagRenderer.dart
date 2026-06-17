// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import 'XmlCanvasBlockComponents.dart';

class DetailsTagRenderer extends StatefulWidget {
  const DetailsTagRenderer({
    super.key,
    required this.xmlContent,
    required this.textColor,
    required this.isStreaming,
  });

  final String xmlContent;
  final Color textColor;
  final bool isStreaming;

  @override
  State<DetailsTagRenderer> createState() => _DetailsTagRendererState();
}

class _DetailsTagRendererState extends State<DetailsTagRenderer> {
  late bool expanded = _hasOpenAttribute(widget.xmlContent);

  @override
  Widget build(BuildContext context) {
    final tagName = _extractTagName(widget.xmlContent) ?? 'details';
    final inner = _extractContentFromXml(widget.xmlContent, tagName);
    final summary = _extractSummary(inner);
    final body = _removeSummary(inner).trim();

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          CanvasExpandableHeaderRow(
            title: summary.isNotEmpty ? summary : 'Details',
            semanticDescription: expanded ? 'Collapse' : 'Expand',
            expanded: expanded,
            titleColor: widget.textColor.withValues(alpha: 0.85),
            rotationTurns: expanded ? 0.25 : 0,
            onClick: () {
              setState(() {
                expanded = !expanded;
              });
            },
          ),
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 200),
            switchInCurve: Curves.linear,
            switchOutCurve: Curves.linear,
            transitionBuilder: (child, animation) {
              return FadeTransition(opacity: animation, child: child);
            },
            child: expanded && body.isNotEmpty
                ? CanvasIndentedGuide(
                    key: const ValueKey<String>('details-expanded'),
                    child: StreamMarkdownRenderer(
                      content: body,
                      isStreaming: widget.isStreaming,
                      textColor: widget.textColor.withValues(alpha: 0.85),
                      backgroundColor: Theme.of(context).colorScheme.surface,
                      selectionRoot: false,
                    ),
                  )
                : const SizedBox.shrink(
                    key: ValueKey<String>('details-collapsed'),
                  ),
          ),
        ],
      ),
    );
  }
}

String _extractContentFromXml(String content, String tagName) {
  final open = RegExp(
    '<${RegExp.escape(tagName)}\\b[^>]*>',
    caseSensitive: false,
    dotAll: true,
  ).firstMatch(content);
  if (open == null) {
    return content;
  }
  final endTag = '</$tagName>';
  final endIndex = content.toLowerCase().lastIndexOf(endTag.toLowerCase());
  final bodyEnd = endIndex > open.end ? endIndex : content.length;
  return content.substring(open.end, bodyEnd);
}

String? _extractTagName(String content) {
  return RegExp(r'<\s*([a-zA-Z_][a-zA-Z0-9_]*)').firstMatch(content)?.group(1);
}

String _extractSummary(String detailsInner) {
  return RegExp(
        r'<summary>([\s\S]*?)<\/summary>',
        caseSensitive: false,
      ).firstMatch(detailsInner)?.group(1)?.trim() ??
      '';
}

String _removeSummary(String detailsInner) {
  return detailsInner.replaceFirst(
    RegExp(r'<summary>[\s\S]*?<\/summary>', caseSensitive: false),
    '',
  );
}

bool _hasOpenAttribute(String detailsXml) {
  return RegExp(
    r'<\s*[a-zA-Z_][\w:-]*\b[^>]*\bopen\b',
    caseSensitive: false,
  ).hasMatch(detailsXml);
}
