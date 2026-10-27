// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../interactions/MessagePressShield.dart';
import 'MarkdownNodeGrouper.dart';

/// Definitions belong to one Markdown document, including its nested surfaces.
class MarkdownFootnoteScope extends InheritedWidget {
  const MarkdownFootnoteScope({
    super.key,
    required this.definitions,
    required this.definitionBuilder,
    required super.child,
  });

  final Map<String, MarkdownNodeStable> definitions;
  final Widget Function(MarkdownNodeStable definition) definitionBuilder;

  static MarkdownFootnoteScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<MarkdownFootnoteScope>();

  void open(BuildContext context, String label) {
    final definition = definitions[label];
    if (definition == null) return;
    showDialog<void>(
      context: context,
      builder: (context) => MarkdownFootnoteScope(
        definitions: definitions,
        definitionBuilder: definitionBuilder,
        child: AlertDialog(
          title: Text('[$label]'),
          content: SizedBox(
            width: 480,
            child: SingleChildScrollView(
              child: SelectionArea(child: definitionBuilder(definition)),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: Text(MaterialLocalizations.of(context).closeButtonLabel),
            ),
          ],
        ),
      ),
    );
  }

  @override
  bool updateShouldNotify(MarkdownFootnoteScope oldWidget) =>
      !mapEquals(definitions, oldWidget.definitions);
}

/// Resolves late-arriving definitions without rebuilding cached inline spans.
class MarkdownFootnoteReference extends StatelessWidget {
  const MarkdownFootnoteReference({
    super.key,
    required this.label,
    required this.style,
  });

  final String label;
  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    final scope = MarkdownFootnoteScope.maybeOf(context);
    final definition = scope?.definitions[label];
    if (scope == null || definition == null) {
      return Text('[^$label]', style: style);
    }
    final fontSize = style.fontSize ?? 14;
    return Transform.translate(
      offset: Offset(0, -fontSize * .3),
      child: MessagePressShieldRegion(
        child: Semantics(
          button: true,
          label: '[$label]: ${definition.content}',
          child: Tooltip(
            message: definition.content,
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => scope.open(context, label),
                child: Text(
                  '[$label]',
                  style: style.copyWith(
                    color: Theme.of(context).colorScheme.primary,
                    fontSize: fontSize * .7,
                    height: 1,
                    letterSpacing: 0,
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
