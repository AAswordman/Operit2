// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';
import '../../settings/components/SettingsControlStyles.dart';
import 'ExtensionMotion.dart';

/// Label and icon shared by catalog kinds and marketplace entry types.
class StoreKind {
  const StoreKind._({
    required this.id,
    required this.label,
    required this.icon,
  });

  final String id;
  final String label;
  final IconData icon;

  static const StoreKind toolpkg = StoreKind._(
    id: 'toolpkg',
    label: '插件',
    icon: Icons.extension_rounded,
  );
  static const StoreKind script = StoreKind._(
    id: 'script',
    label: '脚本包',
    icon: Icons.code_rounded,
  );
  static const StoreKind skill = StoreKind._(
    id: 'skill',
    label: '技能',
    icon: Icons.auto_awesome_rounded,
  );
  static const StoreKind mcp = StoreKind._(
    id: 'mcp',
    label: 'MCP',
    icon: Icons.cable_rounded,
  );

  static const StoreKind a2a = StoreKind._(
    id: 'a2a',
    label: 'A2A',
    icon: Icons.hub_outlined,
  );

  static const List<StoreKind> values = <StoreKind>[
    toolpkg,
    script,
    skill,
    mcp,
    a2a,
  ];

  /// Resolves catalog kinds and marketplace types to one identity.
  static StoreKind of(String kind) => switch (kind) {
    'toolpkg' || 'package' => toolpkg,
    'script' => script,
    'skill' => skill,
    'mcp' => mcp,
    'a2a' => a2a,
    _ => throw StateError('Unsupported extension kind: $kind'),
  };

  /// Marketplace type id that the market API uses for this kind.
  String get marketType => identical(this, toolpkg) ? 'package' : id;
}

/// How much larger store text renders than its design size, never below 1.
///
/// Fixed-height shelves and grid rows multiply by this so the theme font scale
/// and the system text scale cannot push their text out of bounds.
double storeTextScale(BuildContext context) {
  final fontScale = OperitTheme.of(context).themePreferenceSnapshot.fontScale;
  return math.max(1.0, fontScale * MediaQuery.textScalerOf(context).scale(1));
}

/// Corner radius of cards, matching the settings cards.
const BorderRadius storeCardRadius = BorderRadius.all(Radius.circular(12));

/// Card fill used by the settings cards.
Color storeCardColor(ColorScheme colorScheme) =>
    colorScheme.surfaceContainerHighest.withValues(alpha: 0.36);

/// Hairline around cards.
Color storeCardBorderColor(ColorScheme colorScheme) =>
    colorScheme.outlineVariant.withValues(alpha: 0.18);

/// Gradient card that opens a page: the device space and the creator account.
BoxDecoration storeHeroDecoration(ThemeData theme) {
  final colorScheme = theme.colorScheme;
  final dark = theme.brightness == Brightness.dark;
  return BoxDecoration(
    borderRadius: BorderRadius.circular(24),
    border: Border.all(
      color: colorScheme.primary.withValues(alpha: dark ? 0.16 : 0.22),
    ),
    gradient: LinearGradient(
      begin: Alignment.topLeft,
      end: Alignment.bottomRight,
      colors: <Color>[
        Color.alphaBlend(
          colorScheme.primary.withValues(alpha: dark ? 0.06 : 0.10),
          dark
              ? colorScheme.surfaceContainerLow
              : colorScheme.surfaceContainerLowest,
        ),
        colorScheme.surface.withValues(alpha: dark ? 0.6 : 0.8),
      ],
    ),
  );
}

String storeScopeLabel(String scope) => switch (scope) {
  'device' => '仅本机',
  'space' => '设备空间',
  'localOnly' => '本机运行',
  _ => throw StateError('Unknown extension scope: $scope'),
};

IconData storeScopeIcon(String scope) => switch (scope) {
  'device' => Icons.my_location_rounded,
  'space' => Icons.hub_outlined,
  'localOnly' => Icons.terminal_rounded,
  _ => throw StateError('Unknown extension scope: $scope'),
};

String storeSourceLabel(String source) => switch (source) {
  'builtin' => 'Operit 内置',
  'bundled' => 'Operit 官方',
  'market' => '来自市场',
  'manual' => '手动导入',
  _ => throw StateError('Unknown extension source: $source'),
};

/// Formats counters the way store listings show them.
String storeCompactCount(int value) {
  if (value >= 100000000) {
    return '${(value / 100000000).toStringAsFixed(1)}亿';
  }
  if (value >= 10000) {
    return '${(value / 10000).toStringAsFixed(1)}万';
  }
  return '$value';
}

int marketEntryDownloads(core_proxy.MarketEntrySummary entry) {
  return math.max(entry.downloads, entry.downloadCount);
}

int marketEntryLikes(core_proxy.MarketEntrySummary entry) {
  return entry.reactions
      .where((reaction) => reaction.reaction == '+1')
      .fold<int>(0, (sum, reaction) => sum + reaction.total);
}

String marketEntryPublisher(core_proxy.MarketEntrySummary entry) {
  final publisher = entry.publisher?.login.trim() ?? '';
  if (publisher.isNotEmpty) {
    return publisher;
  }
  return entry.author?.login.trim() ?? '';
}

/// Settings-style card that lifts on hover, sinks on press and can flash
/// after a live change.
///
/// The glass surface itself only moves and scales; it never fades, because
/// fading a backdrop blur makes the blur drop out.
class StorePressable extends StatefulWidget {
  const StorePressable({
    super.key,
    required this.child,
    this.onTap,
    this.onLongPress,
    this.onSecondaryTapUp,
    this.padding = EdgeInsets.zero,
    this.borderRadius = storeCardRadius,
    this.color,
    this.highlight = false,
  });

  final Widget child;
  final VoidCallback? onTap;
  final VoidCallback? onLongPress;
  final GestureTapUpCallback? onSecondaryTapUp;
  final EdgeInsetsGeometry padding;
  final BorderRadius borderRadius;
  final Color? color;

  /// Tints the card while it shows a change that just arrived.
  final bool highlight;

  @override
  State<StorePressable> createState() => _StorePressableState();
}

class _StorePressableState extends State<StorePressable> {
  bool _hovered = false;
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final motion = ExtensionMotion.of(context);
    final dark = theme.brightness == Brightness.dark;
    final interactive = widget.onTap != null;
    final fill = widget.color ?? storeCardColor(colorScheme);
    return AnimatedScale(
      scale: _pressed ? 0.98 : 1,
      duration: motion.fast,
      curve: ExtensionMotion.enter,
      child: TweenAnimationBuilder<double>(
        tween: Tween<double>(end: _hovered && interactive ? 1 : 0),
        duration: motion.fast,
        curve: ExtensionMotion.enter,
        builder: (context, lift, child) => TweenAnimationBuilder<double>(
          tween: Tween<double>(end: widget.highlight ? 1 : 0),
          duration: widget.highlight ? motion.standard : motion.emphasized * 3,
          curve: ExtensionMotion.enter,
          child: child,
          builder: (context, glow, child) => Transform.translate(
            offset: Offset(0, -2 * lift),
            child: OperitGlassSurface(
              color: Color.alphaBlend(
                colorScheme.primary.withValues(alpha: 0.10 * glow),
                fill,
              ),
              layer: OperitGlassSurfaceLayer.card,
              borderRadius: widget.borderRadius,
              border: Border.all(
                color: Color.lerp(
                  Color.lerp(
                    storeCardBorderColor(colorScheme),
                    colorScheme.outline.withValues(alpha: 0.42),
                    lift,
                  ),
                  colorScheme.primary.withValues(alpha: 0.6),
                  glow,
                )!,
              ),
              shadows: <BoxShadow>[
                if (lift > 0)
                  BoxShadow(
                    color: Colors.black.withValues(
                      alpha: (dark ? 0.32 : 0.08) * lift,
                    ),
                    blurRadius: 16,
                    offset: Offset(0, 6 * lift),
                  ),
              ],
              material: true,
              enableBackdropFilter: false,
              child: child!,
            ),
          ),
        ),
        child: InkWell(
          onTap: widget.onTap,
          onLongPress: widget.onLongPress,
          onSecondaryTapUp: widget.onSecondaryTapUp,
          onHover: interactive
              ? (value) => setState(() => _hovered = value)
              : null,
          onHighlightChanged: interactive
              ? (value) => setState(() => _pressed = value)
              : null,
          borderRadius: widget.borderRadius,
          child: Padding(padding: widget.padding, child: widget.child),
        ),
      ),
    );
  }
}

/// Action pill that morphs between its label, progress and done state.
class StoreGetButton extends StatelessWidget {
  const StoreGetButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.icon,
    this.busy = false,
    this.done = false,
    this.doneLabel = '已安装',
    this.prominent = false,
    this.large = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final bool busy;
  final bool done;
  final String doneLabel;
  final bool prominent;
  final bool large;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final motion = ExtensionMotion.of(context);
    final (background, foreground) = done
        ? (colorScheme.surfaceContainerHighest, colorScheme.onSurfaceVariant)
        : prominent
        ? (colorScheme.primary, colorScheme.onPrimary)
        : (colorScheme.secondaryContainer, colorScheme.onSecondaryContainer);
    final glyphSize = large ? 18.0 : 16.0;
    final Widget content;
    if (busy) {
      content = SizedBox.square(
        key: const ValueKey<String>('busy'),
        dimension: glyphSize,
        child: CircularProgressIndicator(strokeWidth: 2, color: foreground),
      );
    } else if (done) {
      content = Row(
        key: ValueKey<String>('done:$doneLabel'),
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(Icons.check_rounded, size: glyphSize, color: foreground),
          const SizedBox(width: 6),
          Text(doneLabel),
        ],
      );
    } else {
      content = Row(
        key: ValueKey<String>('label:$label'),
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          if (icon != null) ...<Widget>[
            Icon(icon, size: glyphSize, color: foreground),
            const SizedBox(width: 6),
          ],
          Text(label),
        ],
      );
    }
    final enabled = onPressed != null && !busy && !done;
    return AnimatedContainer(
      duration: motion.standard,
      curve: ExtensionMotion.enter,
      height: large ? 38 : 32,
      constraints: BoxConstraints(minWidth: large ? 104 : 68),
      decoration: ShapeDecoration(
        color: onPressed == null && !busy && !done
            ? background.withValues(alpha: 0.5)
            : background,
        shape: const StadiumBorder(),
      ),
      child: Material(
        type: MaterialType.transparency,
        child: InkWell(
          customBorder: const StadiumBorder(),
          onTap: enabled ? onPressed : null,
          child: Padding(
            padding: EdgeInsets.symmetric(horizontal: large ? 18 : 14),
            child: DefaultTextStyle.merge(
              style: textTheme.labelLarge?.copyWith(
                color: foreground,
                fontWeight: FontWeight.w600,
              ),
              child: Center(
                widthFactor: 1,
                child: AnimatedSize(
                  duration: motion.standard,
                  curve: ExtensionMotion.enter,
                  child: AnimatedSwitcher(
                    duration: motion.standard,
                    transitionBuilder: (child, animation) => FadeTransition(
                      opacity: animation,
                      child: ScaleTransition(
                        scale: Tween<double>(
                          begin: 0.8,
                          end: 1,
                        ).animate(animation),
                        child: child,
                      ),
                    ),
                    child: content,
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

/// Section title in the settings style, with an optional count and action.
class StoreSectionHeader extends StatelessWidget {
  const StoreSectionHeader({
    super.key,
    required this.title,
    this.subtitle,
    this.icon,
    this.count,
    this.trailing,
  });

  final String title;
  final String? subtitle;
  final IconData? icon;
  final int? count;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final subtitle = this.subtitle;
    final icon = this.icon;
    final count = this.count;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          if (icon != null) ...<Widget>[
            Icon(icon, size: 20, color: colorScheme.onSurfaceVariant),
            const SizedBox(width: 8),
          ],
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text.rich(
                  TextSpan(
                    text: title,
                    children: <InlineSpan>[
                      if (count != null)
                        TextSpan(
                          text: '  $count',
                          style: TextStyle(
                            color: colorScheme.onSurfaceVariant,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                    ],
                  ),
                  style: SettingsControlStyles.sectionTitleTextStyle(context),
                ),
                if (subtitle != null && subtitle.isNotEmpty) ...<Widget>[
                  const SizedBox(height: 2),
                  Text(
                    subtitle,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ],
            ),
          ),
          ?trailing,
        ],
      ),
    );
  }
}

/// "查看全部" link used in section headers.
class StoreSeeAllButton extends StatelessWidget {
  const StoreSeeAllButton({super.key, required this.onPressed});

  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return TextButton.icon(
      onPressed: onPressed,
      style: SettingsControlStyles.sectionTextButton(),
      iconAlignment: IconAlignment.end,
      icon: const Icon(Icons.chevron_right_rounded, size: 18),
      label: const Text('查看全部'),
    );
  }
}

/// Compact filter chip with an animated selected state and optional count.
class StoreFilterChip extends StatelessWidget {
  const StoreFilterChip({
    super.key,
    required this.label,
    required this.selected,
    required this.onSelected,
    this.count,
    this.icon,
  });

  final String label;
  final bool selected;
  final VoidCallback onSelected;
  final int? count;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final motion = ExtensionMotion.of(context);
    final foreground = selected
        ? colorScheme.onSecondaryContainer
        : colorScheme.onSurfaceVariant;
    final count = this.count;
    final radius = BorderRadius.circular(8);
    return Semantics(
      selected: selected,
      button: true,
      child: AnimatedContainer(
        duration: motion.standard,
        curve: ExtensionMotion.enter,
        height: 32,
        decoration: BoxDecoration(
          color: selected
              ? colorScheme.secondaryContainer
              : colorScheme.secondaryContainer.withValues(alpha: 0),
          borderRadius: radius,
          border: Border.all(
            color: selected
                ? colorScheme.secondaryContainer
                : colorScheme.outlineVariant.withValues(alpha: 0.72),
          ),
        ),
        child: Material(
          type: MaterialType.transparency,
          child: InkWell(
            borderRadius: radius,
            onTap: onSelected,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  if (icon != null) ...<Widget>[
                    Icon(icon, size: 16, color: foreground),
                    const SizedBox(width: 6),
                  ],
                  Text(
                    label,
                    style: theme.textTheme.labelLarge?.copyWith(
                      color: foreground,
                      fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
                    ),
                  ),
                  if (count != null) ...<Widget>[
                    const SizedBox(width: 6),
                    Text(
                      '$count',
                      style: theme.textTheme.labelMedium?.copyWith(
                        color: foreground.withValues(alpha: 0.72),
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

/// Segmented switch whose selection block slides on a spring.
///
/// It sizes each segment to its widest label, so it can sit in a row or in
/// the top bar title without a bounded width.
class StoreSegmentedTabs extends StatelessWidget {
  const StoreSegmentedTabs({
    super.key,
    required this.labels,
    required this.selectedIndex,
    required this.onSelected,
    this.segmentWidth = 64,
    this.height = 36,
  });

  final List<Widget> labels;
  final int selectedIndex;
  final ValueChanged<int> onSelected;
  final double segmentWidth;
  final double height;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final motion = ExtensionMotion.of(context);
    final dark = theme.brightness == Brightness.dark;
    final count = labels.length;
    final width = segmentWidth * storeTextScale(context);
    // The same tones as SettingsSegmentedSelector, with a sliding selection.
    final selectedBackground = dark
        ? colorScheme.primary.withValues(alpha: 0.22)
        : colorScheme.primaryContainer.withValues(alpha: 0.70);
    final selectedForeground = dark
        ? colorScheme.primary
        : colorScheme.onPrimaryContainer;
    return Container(
      height: height,
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        color: dark
            ? colorScheme.surfaceContainerHighest.withValues(alpha: 0.40)
            : colorScheme.surfaceContainerHigh.withValues(alpha: 0.50),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(
            alpha: dark ? 0.30 : 0.45,
          ),
          width: 0.8,
        ),
      ),
      child: SizedBox(
        width: width * count,
        child: Stack(
          children: <Widget>[
            AnimatedPositioned(
              duration: motion.reduced
                  ? Duration.zero
                  : const Duration(milliseconds: 420),
              curve: ExtensionMotion.spring,
              left: width * selectedIndex,
              top: 0,
              bottom: 0,
              width: width,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: selectedBackground,
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
            ),
            Row(
              children: <Widget>[
                for (var index = 0; index < count; index += 1)
                  SizedBox(
                    width: width,
                    child: Semantics(
                      selected: index == selectedIndex,
                      button: true,
                      child: Material(
                        type: MaterialType.transparency,
                        child: InkWell(
                          borderRadius: BorderRadius.circular(8),
                          onTap: () => onSelected(index),
                          child: Center(
                            child: AnimatedDefaultTextStyle(
                              duration: motion.standard,
                              style:
                                  (theme.textTheme.labelLarge ??
                                          const TextStyle())
                                      .copyWith(
                                        color: index == selectedIndex
                                            ? selectedForeground
                                            : colorScheme.onSurfaceVariant,
                                        fontWeight: FontWeight.w600,
                                      ),
                              child: IconTheme.merge(
                                data: IconThemeData(
                                  size: 16,
                                  color: index == selectedIndex
                                      ? selectedForeground
                                      : colorScheme.onSurfaceVariant,
                                ),
                                child: labels[index],
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// Rise-in entrance played once; [animate] false shows the child at rest.
///
/// It slides and scales but never fades, so glass cards keep their blur.
class StoreEntrance extends StatefulWidget {
  const StoreEntrance({
    super.key,
    required this.child,
    this.index = 0,
    this.animate = true,
  });

  final Widget child;
  final int index;
  final bool animate;

  @override
  State<StoreEntrance> createState() => _StoreEntranceState();
}

class _StoreEntranceState extends State<StoreEntrance>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 360),
    value: widget.animate ? 0 : 1,
  );
  late final CurvedAnimation _curve = CurvedAnimation(
    parent: _controller,
    curve: ExtensionMotion.enter,
  );
  Timer? _delay;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!widget.animate || _controller.value > 0 || _delay != null) {
      return;
    }
    final motion = ExtensionMotion.of(context);
    if (motion.reduced) {
      _controller.value = 1;
      return;
    }
    _delay = Timer(motion.stagger(widget.index), () {
      if (mounted) {
        _controller.forward();
      }
    });
  }

  @override
  void dispose() {
    _delay?.cancel();
    _curve.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _curve,
      child: widget.child,
      builder: (context, child) {
        final value = _curve.value;
        if (value >= 1) {
          return child!;
        }
        return Transform.translate(
          offset: Offset(0, (1 - value) * 10),
          child: Transform.scale(scale: 0.98 + 0.02 * value, child: child),
        );
      },
    );
  }
}

/// Sweeping highlight painted over skeleton placeholders.
class StoreShimmer extends StatefulWidget {
  const StoreShimmer({super.key, required this.child});

  final Widget child;

  @override
  State<StoreShimmer> createState() => _StoreShimmerState();
}

class _StoreShimmerState extends State<StoreShimmer>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1500),
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (ExtensionMotion.of(context).ambient) {
      if (!_controller.isAnimating) {
        _controller.repeat();
      }
    } else {
      _controller.stop();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final base = colorScheme.surfaceContainerHighest;
    final highlight = Color.lerp(base, colorScheme.onSurface, 0.08)!;
    return AnimatedBuilder(
      animation: _controller,
      child: widget.child,
      builder: (context, child) => ShaderMask(
        blendMode: BlendMode.srcATop,
        shaderCallback: (bounds) => LinearGradient(
          colors: <Color>[base, highlight, base],
          stops: const <double>[0.35, 0.5, 0.65],
          transform: _ShimmerSlide(_controller.value * 2 - 1),
        ).createShader(bounds),
        child: child,
      ),
    );
  }
}

class _ShimmerSlide extends GradientTransform {
  const _ShimmerSlide(this.percent);

  final double percent;

  @override
  Matrix4? transform(Rect bounds, {TextDirection? textDirection}) {
    return Matrix4.translationValues(bounds.width * percent, 0, 0);
  }
}

/// Placeholder block shown inside [StoreShimmer].
class StoreSkeletonBox extends StatelessWidget {
  const StoreSkeletonBox({
    super.key,
    this.width,
    required this.height,
    this.radius = 6,
  });

  final double? width;
  final double height;
  final double radius;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: width,
      height: height,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(radius),
      ),
    );
  }
}

/// Horizontal shelf that scrolls with mouse drag and shows paging arrows on hover.
class StoreShelf extends StatefulWidget {
  const StoreShelf({
    super.key,
    required this.itemCount,
    required this.itemBuilder,
    required this.itemWidth,
    required this.height,
    this.spacing = 12,
    this.padding = const EdgeInsets.only(top: 4, bottom: 12),
  });

  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;
  final double itemWidth;

  /// Total shelf height, including [padding] that leaves room for card shadows.
  final double height;
  final double spacing;
  final EdgeInsets padding;

  @override
  State<StoreShelf> createState() => _StoreShelfState();
}

class _StoreShelfState extends State<StoreShelf> {
  final ScrollController _controller = ScrollController();
  bool _hovered = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _page(int direction) {
    final position = _controller.position;
    final target =
        (position.pixels + direction * position.viewportDimension * 0.85).clamp(
          position.minScrollExtent,
          position.maxScrollExtent,
        );
    _controller.animateTo(
      target,
      duration: ExtensionMotion.of(context).emphasized,
      curve: ExtensionMotion.enter,
    );
  }

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: SizedBox(
        height: widget.height,
        child: Stack(
          children: <Widget>[
            ScrollConfiguration(
              behavior: ScrollConfiguration.of(context).copyWith(
                scrollbars: false,
                dragDevices: <PointerDeviceKind>{
                  PointerDeviceKind.touch,
                  PointerDeviceKind.mouse,
                  PointerDeviceKind.trackpad,
                  PointerDeviceKind.stylus,
                },
              ),
              child: ListView.separated(
                controller: _controller,
                scrollDirection: Axis.horizontal,
                padding: widget.padding,
                itemCount: widget.itemCount,
                separatorBuilder: (context, index) =>
                    SizedBox(width: widget.spacing),
                itemBuilder: (context, index) => SizedBox(
                  width: widget.itemWidth,
                  child: widget.itemBuilder(context, index),
                ),
              ),
            ),
            ListenableBuilder(
              listenable: _controller,
              builder: (context, child) {
                // The position attaches before its first layout; extents exist only after it.
                final position = _controller.hasClients
                    ? _controller.position
                    : null;
                final ready =
                    position != null &&
                    position.hasPixels &&
                    position.hasContentDimensions;
                final canBack =
                    ready && position.pixels > position.minScrollExtent + 4;
                final canForward =
                    ready && position.pixels < position.maxScrollExtent - 4;
                return Stack(
                  children: <Widget>[
                    _ShelfArrow(
                      alignment: Alignment.centerLeft,
                      icon: Icons.chevron_left_rounded,
                      visible: _hovered && canBack,
                      onPressed: () => _page(-1),
                    ),
                    _ShelfArrow(
                      alignment: Alignment.centerRight,
                      icon: Icons.chevron_right_rounded,
                      visible: _hovered && canForward,
                      onPressed: () => _page(1),
                    ),
                  ],
                );
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _ShelfArrow extends StatelessWidget {
  const _ShelfArrow({
    required this.alignment,
    required this.icon,
    required this.visible,
    required this.onPressed,
  });

  final Alignment alignment;
  final IconData icon;
  final bool visible;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final motion = ExtensionMotion.of(context);
    return Positioned.fill(
      child: Align(
        alignment: alignment,
        child: IgnorePointer(
          ignoring: !visible,
          child: AnimatedOpacity(
            opacity: visible ? 1 : 0,
            duration: motion.fast,
            child: AnimatedScale(
              scale: visible ? 1 : 0.8,
              duration: motion.fast,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 6),
                child: IconButton.filledTonal(
                  onPressed: onPressed,
                  icon: Icon(icon),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Small label used for kind, version and status badges.
class StoreBadge extends StatelessWidget {
  const StoreBadge({
    super.key,
    required this.label,
    this.icon,
    this.color,
    this.foreground,
  });

  final String label;
  final IconData? icon;
  final Color? color;
  final Color? foreground;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final fg = foreground ?? colorScheme.onSurfaceVariant;
    return DecoratedBox(
      decoration: BoxDecoration(
        color:
            color ?? colorScheme.surfaceContainerHighest.withValues(alpha: 0.8),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            if (icon != null) ...<Widget>[
              Icon(icon, size: 13, color: fg),
              const SizedBox(width: 4),
            ],
            Text(
              label,
              style: theme.textTheme.labelSmall?.copyWith(
                color: fg,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Centers page content and caps its width on wide windows.
class StoreContentWidth extends StatelessWidget {
  const StoreContentWidth({
    super.key,
    required this.child,
    this.maxWidth = 1120,
  });

  final Widget child;
  final double maxWidth;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: maxWidth),
        child: child,
      ),
    );
  }
}

/// Horizontal padding that keeps sliver content inside [maxWidth] and centered.
EdgeInsets storeSliverPadding(
  double crossAxisExtent, {
  double maxWidth = 1120,
  double gutter = 24,
  double top = 0,
  double bottom = 0,
}) {
  final side = math.max(gutter, (crossAxisExtent - maxWidth) / 2);
  return EdgeInsets.fromLTRB(side, top, side, bottom);
}
