// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../common/interactions/MessagePressShield.dart';

class CanvasExpandableHeaderRow extends StatelessWidget {
  const CanvasExpandableHeaderRow({
    super.key,
    required this.title,
    required this.semanticDescription,
    required this.expanded,
    required this.titleColor,
    required this.rotationTurns,
    required this.onClick,
  });

  final String title;
  final String semanticDescription;
  final bool expanded;
  final Color titleColor;
  final double rotationTurns;
  final VoidCallback onClick;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      label: semanticDescription,
      child: MessagePressShieldRegion(
        child: InkWell(
          onTap: onClick,
          borderRadius: BorderRadius.circular(6),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Row(
              children: <Widget>[
                AnimatedRotation(
                  turns: rotationTurns,
                  duration: const Duration(milliseconds: 300),
                  child: Icon(
                    Icons.keyboard_arrow_right,
                    size: 18,
                    color: titleColor,
                  ),
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.labelMedium?.copyWith(
                      color: titleColor,
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

class CanvasIndentedGuide extends StatelessWidget {
  const CanvasIndentedGuide({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 24, top: 4, bottom: 8),
      child: child,
    );
  }
}

class CanvasFontTextBlock extends StatelessWidget {
  const CanvasFontTextBlock({
    super.key,
    required this.text,
    required this.style,
    required this.textColor,
    required this.backgroundColor,
  });

  final String text;
  final TextStyle style;
  final Color textColor;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    final content = SelectableText(
      text,
      style: style.copyWith(color: textColor),
    );
    if (backgroundColor == Colors.transparent) {
      return content;
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 2),
      color: backgroundColor,
      child: content,
    );
  }
}

class CanvasPillLabel extends StatelessWidget {
  const CanvasPillLabel({super.key, required this.text, required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        text,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(color: color),
      ),
    );
  }
}
