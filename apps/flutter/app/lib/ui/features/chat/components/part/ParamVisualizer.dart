// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

class ParamItem {
  const ParamItem({required this.name, required this.value});

  final String name;
  final String value;
}

class ParamVisualizer extends StatelessWidget {
  const ParamVisualizer({super.key, required this.xmlContent});

  final String xmlContent;

  @override
  Widget build(BuildContext context) {
    final params = _parseParams(xmlContent);
    if (params.isEmpty) {
      return SelectableText(
        xmlContent,
        style: Theme.of(context).textTheme.bodyMedium,
      );
    }
    return Column(
      children: <Widget>[
        for (var index = 0; index < params.length; index++) ...<Widget>[
          _ParamItemView(param: params[index]),
          if (index < params.length - 1) const SizedBox(height: 8),
        ],
      ],
    );
  }
}

class _ParamItemView extends StatelessWidget {
  const _ParamItemView({required this.param});

  final ParamItem param;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  param.name,
                  style: theme.textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: theme.colorScheme.primary,
                  ),
                ),
              ),
              IconButton(
                constraints: const BoxConstraints.tightFor(
                  width: 24,
                  height: 24,
                ),
                padding: EdgeInsets.zero,
                onPressed: () {
                  Clipboard.setData(ClipboardData(text: param.value));
                },
                icon: Icon(
                  Icons.content_copy,
                  size: 16,
                  color: theme.colorScheme.primary.withValues(alpha: 0.6),
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          SelectableText(
            param.value,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurface,
            ),
          ),
        ],
      ),
    );
  }
}

List<ParamItem> _parseParams(String xmlContent) {
  return RegExp(
        r'''<param\s+name=["']([^"']+)["'][^>]*>([\s\S]*?)<\/param>''',
        caseSensitive: false,
      )
      .allMatches(xmlContent)
      .map(
        (match) => ParamItem(
          name: match.group(1)!,
          value: _unescapeXml(match.group(2)!.trim()),
        ),
      )
      .toList();
}

String _unescapeXml(String input) {
  var result = input;
  if (result.startsWith('<![CDATA[')) {
    result = result.substring(9);
  }
  if (result.endsWith(']]>')) {
    result = result.substring(0, result.length - 3);
  }
  return result
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .replaceAll('&amp;', '&')
      .replaceAll('&quot;', '"')
      .replaceAll('&apos;', "'");
}
