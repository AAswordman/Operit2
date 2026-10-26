import 'package:flutter/material.dart';

class ThemeInspectableTargetId {
  const ThemeInspectableTargetId._();

  static const String background = 'background';
  static const String sidebar = 'navigation.sidebar';
  static const String topBar = 'chat.topBar';
  static const String userMessage = 'chat.userMessage';
  static const String aiMessage = 'chat.aiMessage';
  static const String composer = 'chat.composer';
}

class ThemeInspectTarget extends StatefulWidget {
  const ThemeInspectTarget({
    super.key,
    required this.targetId,
    required this.label,
    required this.enabled,
    required this.selected,
    required this.onSelected,
    required this.child,
    this.fillHighlight = true,
  });

  final String targetId;
  final String label;
  final bool enabled;
  final bool selected;
  final ValueChanged<String> onSelected;
  final Widget child;
  final bool fillHighlight;

  @override
  State<ThemeInspectTarget> createState() => _ThemeInspectTargetState();
}

class _ThemeInspectTargetState extends State<ThemeInspectTarget> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    if (!widget.enabled) return widget.child;
    final highlighted = widget.selected || _hovered;
    return Semantics(
      container: true,
      label: widget.label,
      button: true,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        onEnter: (_) => setState(() => _hovered = true),
        onExit: (_) => setState(() => _hovered = false),
        child: GestureDetector(
          behavior: HitTestBehavior.translucent,
          onTap: () => widget.onSelected(widget.targetId),
          child: DecoratedBox(
            position: DecorationPosition.foreground,
            decoration: BoxDecoration(
              color: !widget.fillHighlight
                  ? null
                  : widget.selected
                  ? const Color(0x182E72E8)
                  : highlighted
                  ? const Color(0x0D2E72E8)
                  : null,
              border: highlighted
                  ? Border.all(
                      color: widget.selected
                          ? const Color(0xFF2E72E8)
                          : const Color(0xAA2E72E8),
                      width: widget.selected ? 2 : 1,
                    )
                  : null,
            ),
            child: widget.child,
          ),
        ),
      ),
    );
  }
}
