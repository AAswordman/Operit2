import 'package:flutter/material.dart';

import 'drawer_chrome.dart';

/// The vertical guide used by nested conversation rows in the desktop drawer.
///
/// This is intentionally a presentational primitive. Runtime code supplies the
/// drag/drop and selection state; the client and the Studio use the same paint
/// geometry for that state.
class ThemeHistoryRail extends StatelessWidget {
  const ThemeHistoryRail({
    super.key,
    required this.height,
    required this.appearance,
    this.width = 20,
    this.thickness = 1,
    this.selected = false,
    this.hovered = false,
  });

  final double height;
  final ThemeDrawerPalette appearance;
  final double width;
  final double thickness;
  final bool selected;
  final bool hovered;

  @override
  Widget build(BuildContext context) {
    final baseColor = appearance.dividerColor.withValues(alpha: 0.45);
    final activeColor = selected
        ? appearance.statusAvailableColor
        : appearance.titleColor.withValues(alpha: 0.55);
    final showIndicator = selected || hovered;
    return SizedBox(
      width: width,
      height: height,
      child: Stack(
        alignment: Alignment.center,
        children: <Widget>[
          Container(width: thickness, height: height, color: baseColor),
          AnimatedContainer(
            duration: const Duration(milliseconds: 160),
            curve: Curves.easeOutCubic,
            width: showIndicator ? 2.0 : thickness,
            height: showIndicator ? (selected ? 16.0 : 10.0) : 0.0,
            decoration: BoxDecoration(
              color: showIndicator ? activeColor : Colors.transparent,
              borderRadius: BorderRadius.circular(1.5),
            ),
          ),
        ],
      ),
    );
  }
}

/// The six-dot drag/status handle used by a conversation row.
class ThemeConversationStatusHandle extends StatelessWidget {
  const ThemeConversationStatusHandle({
    super.key,
    required this.isRunning,
    required this.selected,
    required this.hovered,
    required this.appearance,
    this.animation,
    this.compact = false,
  });

  final bool isRunning;
  final bool selected;
  final bool hovered;
  final ThemeDrawerPalette appearance;
  final Animation<double>? animation;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final width = compact ? 8.5 : 10.0;
    final height = compact ? 12.0 : 13.5;
    final restingColor = selected
        ? appearance.selectedContentColor.withValues(alpha: 0.70)
        : appearance.itemColor.withValues(alpha: hovered ? 0.65 : 0.30);
    final activeColor = appearance.statusAvailableColor;
    return Tooltip(
      message: isRunning ? '正在运行' : '拖动对话',
      child: RepaintBoundary(
        child: SizedBox(
          key: isRunning
              ? const ValueKey<String>('conversation-running-indicator')
              : null,
          width: width,
          height: height,
          child: CustomPaint(
            size: Size(width, height),
            painter: isRunning && animation != null
                ? _ThemeMarqueeDotsPainter(
                    animation: animation!,
                    activeColor: activeColor,
                    dimColor: restingColor.withValues(alpha: 0.18),
                  )
                : _ThemeStaticDotsPainter(color: restingColor),
          ),
        ),
      ),
    );
  }
}

/// Shared visual for a top-level character/workspace history section.
class ThemeHistorySectionHeader extends StatelessWidget {
  const ThemeHistorySectionHeader({
    super.key,
    required this.label,
    required this.count,
    required this.expanded,
    required this.appearance,
    required this.onToggleExpanded,
    this.workspaceStyle = false,
    this.avatar,
    this.fallbackIcon = Icons.person_outline,
  });

  final String label;
  final int count;
  final bool expanded;
  final ThemeDrawerPalette appearance;
  final VoidCallback onToggleExpanded;
  final bool workspaceStyle;
  final Widget? avatar;
  final IconData fallbackIcon;

  @override
  Widget build(BuildContext context) {
    if (workspaceStyle) {
      return Padding(
        padding: const EdgeInsetsDirectional.only(
          start: 18,
          end: 12,
          top: 8,
          bottom: 4,
        ),
        child: InkWell(
          borderRadius: BorderRadius.circular(8),
          onTap: onToggleExpanded,
          child: Padding(
            padding: const EdgeInsetsDirectional.fromSTEB(2, 3, 4, 3),
            child: Row(
              children: <Widget>[
                Container(
                  width: 3,
                  height: 17,
                  decoration: BoxDecoration(
                    color: appearance.statusAvailableColor.withValues(
                      alpha: 0.62,
                    ),
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
                const SizedBox(width: 9),
                Icon(
                  Icons.work_outline,
                  size: 15,
                  color: appearance.itemColor.withValues(alpha: 0.82),
                ),
                const SizedBox(width: 7),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.labelLarge?.copyWith(
                      color: appearance.titleColor,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                _ThemeHistoryCountBadge(count: count, appearance: appearance),
                const SizedBox(width: 6),
                Icon(
                  expanded ? Icons.expand_less : Icons.expand_more,
                  size: 18,
                  color: appearance.itemColor.withValues(alpha: 0.70),
                ),
              ],
            ),
          ),
        ),
      );
    }

    return Padding(
      padding: const EdgeInsetsDirectional.only(
        start: 20,
        end: 12,
        top: 10,
        bottom: 5,
      ),
      child: InkWell(
        borderRadius: BorderRadius.circular(18),
        onTap: onToggleExpanded,
        child: Row(
          children: <Widget>[
            Container(
              width: 22,
              height: 22,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: appearance.buttonContainerColor,
              ),
              alignment: Alignment.center,
              child: avatar == null
                  ? Icon(fallbackIcon, size: 14, color: appearance.itemColor)
                  : ClipOval(child: avatar),
            ),
            const SizedBox(width: 8),
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 170),
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.titleSmall?.copyWith(
                  color: appearance.titleColor,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            const SizedBox(width: 8),
            Text(
              count.toString(),
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: appearance.itemColor.withValues(alpha: 0.64),
                fontWeight: FontWeight.w700,
              ),
            ),
            Expanded(
              child: Container(
                height: 2,
                margin: const EdgeInsetsDirectional.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    colors: <Color>[
                      appearance.dividerColor,
                      Colors.transparent,
                    ],
                  ),
                ),
              ),
            ),
            Icon(
              expanded ? Icons.keyboard_arrow_up : Icons.keyboard_arrow_down,
              size: 23,
              color: appearance.itemColor.withValues(alpha: 0.78),
            ),
          ],
        ),
      ),
    );
  }
}

/// Shared visual for a collapsible group header. Runtime code may supply its
/// own menu widget; the Studio leaves that slot empty.
class ThemeHistoryGroupHeader extends StatelessWidget {
  const ThemeHistoryGroupHeader({
    super.key,
    required this.label,
    required this.pinned,
    required this.workspaceStyle,
    required this.expanded,
    required this.appearance,
    required this.showActions,
    required this.onToggleExpanded,
    required this.action,
    this.border,
  });

  final String label;
  final bool pinned;
  final bool workspaceStyle;
  final bool expanded;
  final ThemeDrawerPalette appearance;
  final bool showActions;
  final VoidCallback onToggleExpanded;
  final Widget action;
  final Border? border;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(workspaceStyle ? 8 : 12);
    final child = Padding(
      padding: workspaceStyle
          ? const EdgeInsetsDirectional.fromSTEB(8, 4, 6, 4)
          : const EdgeInsetsDirectional.fromSTEB(12, 6, 8, 6),
      child: Row(
        children: <Widget>[
          Icon(
            Icons.folder_outlined,
            size: workspaceStyle ? 14 : 16,
            color: workspaceStyle
                ? appearance.itemColor.withValues(alpha: 0.76)
                : appearance.itemColor,
          ),
          SizedBox(width: workspaceStyle ? 7 : 8),
          Expanded(
            child: Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style:
                  (workspaceStyle
                          ? Theme.of(context).textTheme.bodySmall
                          : Theme.of(context).textTheme.labelLarge)
                      ?.copyWith(
                        color: workspaceStyle
                            ? appearance.titleColor.withValues(alpha: 0.86)
                            : appearance.titleColor,
                        fontWeight: workspaceStyle
                            ? FontWeight.w600
                            : FontWeight.w700,
                      ),
            ),
          ),
          if (pinned) ...<Widget>[
            const SizedBox(width: 4),
            Icon(
              Icons.push_pin_rounded,
              size: 12,
              color: appearance.itemColor.withValues(
                alpha: workspaceStyle ? 0.65 : 0.68,
              ),
            ),
          ],
          const SizedBox(width: 2),
          AnimatedOpacity(
            duration: const Duration(milliseconds: 140),
            opacity: showActions ? 1.0 : 0.0,
            child: IgnorePointer(ignoring: !showActions, child: action),
          ),
          const SizedBox(width: 2),
          Icon(
            expanded
                ? (workspaceStyle ? Icons.expand_less : Icons.keyboard_arrow_up)
                : (workspaceStyle
                      ? Icons.expand_more
                      : Icons.keyboard_arrow_down),
            size: workspaceStyle ? 17 : 20,
            color: appearance.itemColor.withValues(
              alpha: workspaceStyle ? 0.62 : 0.68,
            ),
          ),
        ],
      ),
    );
    return Padding(
      padding: workspaceStyle
          ? EdgeInsetsDirectional.only(
              start: 44,
              end: 12,
              top: 2,
              bottom: expanded ? 2 : 0,
            )
          : EdgeInsetsDirectional.only(
              start: 46,
              end: 12,
              top: 4,
              bottom: expanded ? 2 : 0,
            ),
      child: DecoratedBox(
        decoration: BoxDecoration(borderRadius: shape, border: border),
        child: Material(
          color: workspaceStyle
              ? Colors.transparent
              : appearance.buttonContainerColor,
          borderRadius: shape,
          child: InkWell(
            borderRadius: shape,
            onTap: onToggleExpanded,
            child: child,
          ),
        ),
      ),
    );
  }
}

/// Shared visual surface for one conversation row. Drag/drop and swipe
/// gestures remain runtime concerns and are wrapped around this surface by the
/// client.
class ThemeConversationRow extends StatelessWidget {
  const ThemeConversationRow({
    super.key,
    required this.title,
    required this.selected,
    required this.active,
    required this.appearance,
    required this.leading,
    required this.onTap,
    required this.onLongPress,
    required this.showActions,
    required this.action,
    this.workspaceStyle = false,
    this.pinned = false,
    this.locked = false,
    this.dragHovering = false,
    this.nested = true,
    this.surfaceWrapper,
  });

  final String title;
  final bool selected;
  final bool active;
  final ThemeDrawerPalette appearance;
  final Widget leading;
  final VoidCallback onTap;
  final VoidCallback onLongPress;
  final bool showActions;
  final Widget action;
  final bool workspaceStyle;
  final bool pinned;
  final bool locked;
  final bool dragHovering;
  final bool nested;
  final Widget Function(Widget surface)? surfaceWrapper;

  @override
  Widget build(BuildContext context) {
    final itemShape = BorderRadius.circular(8);
    final titleColor = selected
        ? appearance.selectedContentColor
        : active
        ? appearance.titleColor.withValues(alpha: 0.94)
        : appearance.itemColor.withValues(alpha: 0.80);
    final containerColor = selected
        ? appearance.selectedContainerColor.withValues(
            alpha: workspaceStyle ? 0.50 : 0.62,
          )
        : active
        ? appearance.itemColor.withValues(alpha: 0.07)
        : Colors.transparent;
    final rowHeight = workspaceStyle ? 30.0 : 34.0;
    final surface = Material(
      color: containerColor,
      borderRadius: itemShape,
      child: InkWell(
        borderRadius: itemShape,
        onTap: onTap,
        onLongPress: onLongPress,
        child: Padding(
          padding: EdgeInsetsDirectional.fromSTEB(
            workspaceStyle ? 4.0 : 6.0,
            0,
            workspaceStyle ? 4.0 : 5.0,
            0,
          ),
          child: Row(
            children: <Widget>[
              leading,
              SizedBox(width: workspaceStyle ? 4 : 5),
              Expanded(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    fontSize: workspaceStyle ? 12.0 : 13.0,
                    letterSpacing: -0.1,
                    color: titleColor,
                    fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                  ),
                ),
              ),
              if (pinned) ...<Widget>[
                const SizedBox(width: 5),
                Icon(
                  Icons.push_pin_rounded,
                  size: 12,
                  color: titleColor.withValues(alpha: 0.60),
                ),
              ],
              if (locked) ...<Widget>[
                const SizedBox(width: 5),
                Icon(
                  Icons.lock_rounded,
                  size: 12,
                  color: titleColor.withValues(alpha: 0.60),
                ),
              ],
              const SizedBox(width: 2),
              AnimatedOpacity(
                duration: const Duration(milliseconds: 140),
                opacity: showActions ? 1.0 : 0.0,
                child: IgnorePointer(ignoring: !showActions, child: action),
              ),
            ],
          ),
        ),
      ),
    );
    return Padding(
      padding: EdgeInsetsDirectional.only(
        start: nested ? (workspaceStyle ? 51 : 56) : 12,
        end: 12,
      ),
      child: SizedBox(
        height: rowHeight,
        child: Row(
          children: <Widget>[
            if (nested) ...<Widget>[
              ThemeHistoryRail(
                height: rowHeight,
                appearance: appearance,
                width: workspaceStyle ? 16 : 20,
                thickness: 1,
                selected: selected,
                hovered: active,
              ),
              const SizedBox(width: 2),
            ],
            Expanded(
              child: Padding(
                padding: EdgeInsets.symmetric(
                  vertical: workspaceStyle ? 1.0 : 1.5,
                ),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    borderRadius: itemShape,
                    border: dragHovering
                        ? Border.all(
                            color: appearance.statusAvailableColor.withValues(
                              alpha: 0.55,
                            ),
                          )
                        : selected
                        ? Border.all(
                            color: appearance.selectedContentColor.withValues(
                              alpha: 0.08,
                            ),
                          )
                        : null,
                  ),
                  child: surfaceWrapper == null
                      ? surface
                      : surfaceWrapper!(surface),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Shared glyph and reserved hit-target geometry of the history menu button.
class ThemeHistoryMoreGlyph extends StatelessWidget {
  const ThemeHistoryMoreGlyph({
    super.key,
    required this.color,
    required this.side,
    this.compact = false,
  });
  final Color color;
  final double side;
  final bool compact;
  @override
  Widget build(BuildContext context) => SizedBox(
    width: side,
    height: side,
    child: Center(
      child: Icon(
        Icons.more_horiz_rounded,
        size: compact ? 14 : 16,
        color: color,
      ),
    ),
  );
}

class _ThemeHistoryCountBadge extends StatelessWidget {
  const _ThemeHistoryCountBadge({
    required this.count,
    required this.appearance,
  });

  final int count;
  final ThemeDrawerPalette appearance;

  @override
  Widget build(BuildContext context) {
    return Container(
      constraints: const BoxConstraints(minWidth: 18),
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: appearance.dividerColor),
      ),
      alignment: Alignment.center,
      child: Text(
        count.toString(),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(
          color: appearance.itemColor.withValues(alpha: 0.70),
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}

class _ThemeMarqueeDotsPainter extends CustomPainter {
  const _ThemeMarqueeDotsPainter({
    required this.animation,
    required this.activeColor,
    required this.dimColor,
  }) : super(repaint: animation);

  final Animation<double> animation;
  final Color activeColor;
  final Color dimColor;

  static const List<Offset> _positions = <Offset>[
    Offset(0.25, 0.18),
    Offset(0.25, 0.50),
    Offset(0.25, 0.82),
    Offset(0.75, 0.82),
    Offset(0.75, 0.50),
    Offset(0.75, 0.18),
  ];

  @override
  void paint(Canvas canvas, Size size) {
    final activePos = animation.value * 6.0;
    final baseDotRadius = size.width * 0.115;
    for (var i = 0; i < 6; i++) {
      var distance = (activePos - i) % 6.0;
      if (distance < 0) distance += 6.0;
      final intensity = distance < 3.2 ? 1.0 - (distance / 3.2) : 0.0;
      final color = intensity > 0
          ? Color.lerp(dimColor, activeColor, intensity)!
          : dimColor;
      final radius = baseDotRadius * (1.0 + intensity * 0.25);
      final center = Offset(
        size.width * _positions[i].dx,
        size.height * _positions[i].dy,
      );
      if (intensity > 0.82) {
        final glowPaint = Paint()
          ..color = activeColor.withValues(alpha: 0.35 * intensity)
          ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 1.4);
        canvas.drawCircle(center, radius * 1.35, glowPaint);
      }
      canvas.drawCircle(center, radius, Paint()..color = color);
    }
  }

  @override
  bool shouldRepaint(covariant _ThemeMarqueeDotsPainter oldDelegate) =>
      oldDelegate.animation != animation ||
      oldDelegate.activeColor != activeColor ||
      oldDelegate.dimColor != dimColor;

  @override
  bool shouldRebuildSemantics(covariant _ThemeMarqueeDotsPainter oldDelegate) =>
      false;
}

class _ThemeStaticDotsPainter extends CustomPainter {
  const _ThemeStaticDotsPainter({required this.color});

  final Color color;

  static const List<Offset> _positions = <Offset>[
    Offset(0.22, 0.16),
    Offset(0.22, 0.50),
    Offset(0.22, 0.84),
    Offset(0.78, 0.84),
    Offset(0.78, 0.50),
    Offset(0.78, 0.16),
  ];

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..color = color;
    final dotRadius = size.width * 0.11;
    for (final position in _positions) {
      canvas.drawCircle(
        Offset(size.width * position.dx, size.height * position.dy),
        dotRadius,
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _ThemeStaticDotsPainter oldDelegate) =>
      oldDelegate.color != color;

  @override
  bool shouldRebuildSemantics(covariant _ThemeStaticDotsPainter oldDelegate) =>
      false;
}
