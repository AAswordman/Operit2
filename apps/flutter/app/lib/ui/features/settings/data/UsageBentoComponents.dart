import 'package:flutter/material.dart';

/// Shared Bento dashboard styling, tabular typography, and micro-interaction
/// primitives used across the Token Usage and Provider Quota sections.

TextStyle? tabularTextStyle(TextStyle? style) =>
    style?.copyWith(fontFeatures: const [FontFeature.tabularFigures()]);

BoxDecoration bentoSurfaceDecoration(
  ColorScheme colors, {
  bool hovered = false,
  bool active = false,
  Color? accent,
  double radius = 18,
}) {
  final isDark = colors.brightness == Brightness.dark;
  final topAlpha = isDark ? (hovered ? 0.86 : 0.74) : (hovered ? 0.98 : 0.92);
  final bottomAlpha = isDark ? (hovered ? 0.62 : 0.52) : (hovered ? 0.88 : 0.80);
  final borderColor = active
      ? (accent ?? colors.primary).withValues(alpha: isDark ? 0.56 : 0.52)
      : hovered
      ? (accent ?? colors.primary).withValues(alpha: isDark ? 0.42 : 0.36)
      : colors.outlineVariant.withValues(alpha: isDark ? 0.28 : 0.55);

  return BoxDecoration(
    borderRadius: BorderRadius.circular(radius),
    gradient: LinearGradient(
      begin: Alignment.topLeft,
      end: Alignment.bottomRight,
      colors: [
        Color.alphaBlend(
          (accent ?? colors.primary).withValues(
            alpha: active ? 0.10 : (hovered ? 0.05 : 0.025),
          ),
          colors.surfaceContainerHigh.withValues(alpha: topAlpha),
        ),
        colors.surfaceContainer.withValues(alpha: bottomAlpha),
      ],
    ),
    border: Border.all(color: borderColor, width: active ? 1.2 : 1.0),
    boxShadow: [
      BoxShadow(
        color: Colors.black.withValues(alpha: isDark ? 0.24 : 0.05),
        blurRadius: hovered ? 18 : 12,
        offset: const Offset(0, 4),
      ),
      if (hovered && accent != null)
        BoxShadow(
          color: accent.withValues(alpha: isDark ? 0.12 : 0.08),
          blurRadius: 16,
          spreadRadius: -2,
        ),
    ],
  );
}

class BentoSurfaceCard extends StatefulWidget {
  const BentoSurfaceCard({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(16),
    this.radius = 18,
    this.accent,
    this.active = false,
    this.hoverable = true,
    this.onTap,
  });

  final Widget child;
  final EdgeInsetsGeometry padding;
  final double radius;
  final Color? accent;
  final bool active;
  final bool hoverable;
  final VoidCallback? onTap;

  @override
  State<BentoSurfaceCard> createState() => _BentoSurfaceCardState();
}

class _BentoSurfaceCardState extends State<BentoSurfaceCard> {
  bool _hovered = false;
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final interactive = widget.onTap != null;
    if (!interactive && !widget.hoverable) {
      return Container(
        padding: widget.padding,
        decoration: bentoSurfaceDecoration(
          colors,
          active: widget.active,
          accent: widget.accent,
          radius: widget.radius,
        ),
        child: widget.child,
      );
    }

    final card = AnimatedScale(
      scale: _pressed ? 0.985 : 1.0,
      duration: const Duration(milliseconds: 110),
      curve: Curves.easeOutCubic,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 180),
        curve: Curves.easeOutCubic,
        padding: widget.padding,
        decoration: bentoSurfaceDecoration(
          colors,
          hovered: _hovered,
          active: widget.active,
          accent: widget.accent,
          radius: widget.radius,
        ),
        child: widget.child,
      ),
    );

    if (!interactive) {
      return MouseRegion(
        onEnter: (_) => setState(() => _hovered = true),
        onExit: (_) => setState(() => _hovered = false),
        child: card,
      );
    }

    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() {
        _hovered = false;
        _pressed = false;
      }),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: (_) => setState(() => _pressed = true),
        onTapCancel: () => setState(() => _pressed = false),
        onTapUp: (_) => setState(() => _pressed = false),
        onTap: widget.onTap,
        child: card,
      ),
    );
  }
}

class BentoStaggeredEntrance extends StatefulWidget {
  const BentoStaggeredEntrance({
    super.key,
    required this.child,
    this.delay = Duration.zero,
    this.duration = const Duration(milliseconds: 340),
  });

  final Widget child;
  final Duration delay;
  final Duration duration;

  @override
  State<BentoStaggeredEntrance> createState() => _BentoStaggeredEntranceState();
}

class _BentoStaggeredEntranceState extends State<BentoStaggeredEntrance>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;
  late final Animation<Offset> _slide;

  @override
  void initState() {
    super.initState();
    final total = widget.delay + widget.duration;
    final startFraction = total.inMilliseconds == 0
        ? 0.0
        : (widget.delay.inMilliseconds / total.inMilliseconds).clamp(0.0, 0.8);
    _controller = AnimationController(vsync: this, duration: total);
    final curved = CurvedAnimation(
      parent: _controller,
      curve: Interval(startFraction, 1.0, curve: Curves.easeOutCubic),
    );
    _opacity = curved;
    _slide = Tween<Offset>(
      begin: const Offset(0, 0.03),
      end: Offset.zero,
    ).animate(curved);
    _controller.forward();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FadeTransition(
      opacity: _opacity,
      child: SlideTransition(position: _slide, child: widget.child),
    );
  }
}

class BentoSegmentedOption<T> {
  const BentoSegmentedOption({
    required this.value,
    required this.label,
    this.icon,
    this.badge,
    this.itemKey,
  });

  final T value;
  final String label;
  final IconData? icon;
  final String? badge;
  final Key? itemKey;
}

class BentoSegmentedControl<T> extends StatelessWidget {
  const BentoSegmentedControl({
    super.key,
    required this.options,
    required this.selected,
    required this.onSelected,
    this.expand = false,
    this.compact = false,
  });

  final List<BentoSegmentedOption<T>> options;
  final T selected;
  final ValueChanged<T> onSelected;
  final bool expand;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final isDark = colors.brightness == Brightness.dark;
    final trackColor = isDark
        ? colors.surfaceContainerLowest.withValues(alpha: 0.72)
        : colors.surfaceContainerHigh.withValues(alpha: 0.78);

    final track = Container(
      padding: EdgeInsets.all(compact ? 3 : 4),
      decoration: BoxDecoration(
        color: trackColor,
        borderRadius: BorderRadius.circular(compact ? 11 : 13),
        border: Border.all(
          color: colors.outlineVariant.withValues(alpha: isDark ? 0.25 : 0.45),
        ),
      ),
      child: expand
          ? Row(
              children: [
                for (final option in options)
                  Expanded(
                    child: _BentoSegmentItem<T>(
                      option: option,
                      selected: option.value == selected,
                      compact: compact,
                      multiline: true,
                      onTap: () => onSelected(option.value),
                    ),
                  ),
              ],
            )
          : SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  for (final option in options)
                    _BentoSegmentItem<T>(
                      option: option,
                      selected: option.value == selected,
                      compact: compact,
                      onTap: () => onSelected(option.value),
                    ),
                ],
              ),
            ),
    );

    return track;
  }
}

class _BentoSegmentItem<T> extends StatefulWidget {
  const _BentoSegmentItem({
    required this.option,
    required this.selected,
    required this.compact,
    this.multiline = false,
    required this.onTap,
  });

  final BentoSegmentedOption<T> option;
  final bool selected;
  final bool compact;
  final bool multiline;
  final VoidCallback onTap;

  @override
  State<_BentoSegmentItem<T>> createState() => _BentoSegmentItemState<T>();
}

class _BentoSegmentItemState<T> extends State<_BentoSegmentItem<T>> {
  bool _hovered = false;
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    final isDark = colors.brightness == Brightness.dark;
    final selected = widget.selected;
    final fg = selected
        ? colors.onSurface
        : (_hovered ? colors.onSurface.withValues(alpha: 0.88) : colors.onSurfaceVariant);

    return Semantics(
      selected: selected,
      button: true,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        onEnter: (_) => setState(() => _hovered = true),
        onExit: (_) => setState(() {
          _hovered = false;
          _pressed = false;
        }),
        child: GestureDetector(
          key: widget.option.itemKey,
          behavior: HitTestBehavior.opaque,
          onTapDown: (_) => setState(() => _pressed = true),
          onTapCancel: () => setState(() => _pressed = false),
          onTapUp: (_) => setState(() => _pressed = false),
          onTap: widget.onTap,
          child: AnimatedScale(
            scale: _pressed ? 0.96 : 1.0,
            duration: const Duration(milliseconds: 95),
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 200),
              curve: Curves.easeOutCubic,
              padding: EdgeInsets.symmetric(
                horizontal: widget.compact ? 10 : 12,
                vertical: widget.compact ? 6 : 8,
              ),
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(widget.compact ? 8 : 10),
                color: selected
                    ? (isDark
                        ? Color.alphaBlend(
                            colors.primary.withValues(alpha: 0.16),
                            colors.surfaceContainerHighest,
                          )
                        : colors.surface)
                    : (_hovered
                        ? colors.onSurface.withValues(alpha: 0.05)
                        : Colors.transparent),
                border: selected
                    ? Border.all(
                        color: colors.primary.withValues(alpha: isDark ? 0.42 : 0.28),
                      )
                    : null,
                boxShadow: selected
                    ? [
                        BoxShadow(
                          color: Colors.black.withValues(alpha: isDark ? 0.22 : 0.07),
                          blurRadius: 8,
                          offset: const Offset(0, 2),
                        ),
                      ]
                    : null,
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  if (widget.option.icon != null) ...[
                    Icon(
                      widget.option.icon,
                      size: widget.compact ? 14 : 15,
                      color: selected ? colors.primary : fg,
                    ),
                    const SizedBox(width: 6),
                  ],
                  Flexible(
                    child: AnimatedDefaultTextStyle(
                      duration: const Duration(milliseconds: 160),
                      style:
                          (widget.compact ? text.labelMedium : text.labelLarge)
                              ?.copyWith(
                                color: fg,
                                fontWeight: selected
                                    ? FontWeight.w600
                                    : FontWeight.w500,
                              ) ??
                          TextStyle(color: fg),
                      child: Text(
                        widget.option.label,
                        maxLines: widget.multiline ? 2 : 1,
                        textAlign: widget.multiline ? TextAlign.center : null,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ),
                  if (widget.option.badge != null) ...[
                    const SizedBox(width: 6),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 5,
                        vertical: 1,
                      ),
                      decoration: BoxDecoration(
                        color: colors.primary.withValues(alpha: 0.14),
                        borderRadius: BorderRadius.circular(999),
                      ),
                      child: Text(
                        widget.option.badge!,
                        style: tabularTextStyle(
                          text.labelSmall?.copyWith(
                            color: colors.primary,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class BentoInteractivePill extends StatefulWidget {
  const BentoInteractivePill({
    super.key,
    required this.label,
    this.icon,
    this.trailingIcon,
    this.accent,
    this.selected = false,
    this.onTap,
    this.tooltip,
  });

  final String label;
  final IconData? icon;
  final IconData? trailingIcon;
  final Color? accent;
  final bool selected;
  final VoidCallback? onTap;
  final String? tooltip;

  @override
  State<BentoInteractivePill> createState() => _BentoInteractivePillState();
}

class _BentoInteractivePillState extends State<BentoInteractivePill> {
  bool _hovered = false;
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    final isDark = colors.brightness == Brightness.dark;
    final accent = widget.accent ?? colors.primary;
    final fg = widget.selected
        ? accent
        : (_hovered ? colors.onSurface : colors.onSurfaceVariant);

    final pill = MouseRegion(
      cursor: widget.onTap != null
          ? SystemMouseCursors.click
          : SystemMouseCursors.basic,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() {
        _hovered = false;
        _pressed = false;
      }),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: widget.onTap == null
            ? null
            : (_) => setState(() => _pressed = true),
        onTapCancel: () => setState(() => _pressed = false),
        onTapUp: (_) => setState(() => _pressed = false),
        onTap: widget.onTap,
        child: AnimatedScale(
          scale: _pressed ? 0.96 : 1.0,
          duration: const Duration(milliseconds: 100),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 170),
            curve: Curves.easeOutCubic,
            padding: const EdgeInsets.symmetric(horizontal: 11, vertical: 7),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(11),
              color: widget.selected
                  ? accent.withValues(alpha: isDark ? 0.18 : 0.12)
                  : (_hovered
                      ? colors.surfaceContainerHighest.withValues(alpha: 0.72)
                      : colors.surfaceContainerHigh.withValues(alpha: 0.52)),
              border: Border.all(
                color: widget.selected
                    ? accent.withValues(alpha: 0.45)
                    : (_hovered
                        ? colors.outline.withValues(alpha: 0.45)
                        : colors.outlineVariant.withValues(alpha: 0.35)),
              ),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (widget.icon != null) ...[
                  Icon(widget.icon, size: 15, color: fg),
                  const SizedBox(width: 6),
                ],
                Flexible(
                  child: Text(
                    widget.label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: tabularTextStyle(
                      text.labelMedium?.copyWith(
                        color: fg,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ),
                if (widget.trailingIcon != null) ...[
                  const SizedBox(width: 4),
                  Icon(widget.trailingIcon, size: 14, color: fg),
                ],
              ],
            ),
          ),
        ),
      ),
    );

    if (widget.tooltip != null) {
      return Tooltip(message: widget.tooltip!, child: pill);
    }
    return pill;
  }
}

class BentoBarSegment {
  const BentoBarSegment({
    required this.value,
    required this.color,
    this.label,
  });

  final double value;
  final Color color;
  final String? label;
}

class BentoStackedBar extends StatelessWidget {
  const BentoStackedBar({
    super.key,
    required this.segments,
    this.height = 7,
    this.emptyColor,
    this.duration = const Duration(milliseconds: 520),
  });

  final List<BentoBarSegment> segments;
  final double height;
  final Color? emptyColor;
  final Duration duration;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final active = segments.where((s) => s.value > 0).toList(growable: false);
    final total = active.fold<double>(0, (sum, s) => sum + s.value);

    return ClipRRect(
      borderRadius: BorderRadius.circular(999),
      child: Container(
        height: height,
        color: emptyColor ?? colors.surfaceContainerHighest.withValues(alpha: 0.55),
        child: total <= 0
            ? const SizedBox.expand()
            : TweenAnimationBuilder<double>(
                tween: Tween<double>(begin: 0, end: 1),
                duration: duration,
                curve: Curves.easeOutCubic,
                builder: (context, progress, _) {
                  return FractionallySizedBox(
                    alignment: Alignment.centerLeft,
                    widthFactor: progress.clamp(0.0, 1.0),
                    child: Row(
                      children: [
                        for (var i = 0; i < active.length; i++) ...[
                          Expanded(
                            flex: (active[i].value / total * 1000)
                                .round()
                                .clamp(1, 1000),
                            child: Container(color: active[i].color),
                          ),
                          if (i + 1 < active.length)
                            Container(width: 1.5, color: colors.surface),
                        ],
                      ],
                    ),
                  );
                },
              ),
      ),
    );
  }
}

class BentoAnimatedProgressBar extends StatelessWidget {
  const BentoAnimatedProgressBar({
    super.key,
    required this.value,
    required this.color,
    this.height = 7,
    this.trackColor,
    this.duration = const Duration(milliseconds: 500),
  });

  final double value;
  final Color color;
  final double height;
  final Color? trackColor;
  final Duration duration;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final target = value.clamp(0.0, 1.0);
    return ClipRRect(
      borderRadius: BorderRadius.circular(999),
      child: Container(
        height: height,
        color: trackColor ?? colors.surfaceContainerHighest.withValues(alpha: 0.55),
        child: TweenAnimationBuilder<double>(
          tween: Tween<double>(begin: 0, end: target),
          duration: duration,
          curve: Curves.easeOutCubic,
          builder: (context, current, _) => FractionallySizedBox(
            alignment: Alignment.centerLeft,
            widthFactor: current.clamp(0.0, 1.0),
            child: DecoratedBox(
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(999),
                gradient: LinearGradient(
                  colors: [color.withValues(alpha: 0.82), color],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class BentoStatusDot extends StatelessWidget {
  const BentoStatusDot({
    super.key,
    required this.color,
    this.glow = false,
    this.size = 8,
  });

  final Color color;
  final bool glow;
  final double size;

  @override
  Widget build(BuildContext context) {
    return TweenAnimationBuilder<double>(
      tween: Tween<double>(begin: glow ? 0.35 : 1.0, end: 1.0),
      duration: const Duration(milliseconds: 650),
      curve: Curves.easeOutCubic,
      builder: (context, t, _) {
        return Container(
          width: size,
          height: size,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: color,
            boxShadow: glow
                ? [
                    BoxShadow(
                      color: color.withValues(alpha: 0.52 * t),
                      blurRadius: 7 * t,
                      spreadRadius: 1.5 * t,
                    ),
                  ]
                : null,
          ),
        );
      },
    );
  }
}

class BentoDeltaBadge extends StatelessWidget {
  const BentoDeltaBadge({
    super.key,
    required this.label,
    required this.color,
    this.icon,
  });

  final String label;
  final Color color;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.13),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: color.withValues(alpha: 0.28)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 11, color: color),
            const SizedBox(width: 4),
          ],
          Text(
            label,
            style: tabularTextStyle(
              text.labelSmall?.copyWith(
                color: color,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
