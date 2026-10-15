import 'package:flutter/material.dart';

import '../assets/theme_asset_resolver.dart';

class ThemeComposerHorizontalVisibility extends StatelessWidget {
  const ThemeComposerHorizontalVisibility({
    super.key,
    required this.width,
    required this.height,
    required this.visible,
    required this.alignment,
    required this.child,
  });
  final double width;
  final double height;
  final bool visible;
  final AlignmentGeometry alignment;
  final Widget child;
  @override
  Widget build(BuildContext context) => ClipRect(
    child: AnimatedContainer(
      width: visible ? width : 0,
      height: height,
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      alignment: alignment,
      child: OverflowBox(
        alignment: alignment,
        minWidth: width,
        maxWidth: width,
        minHeight: height,
        maxHeight: height,
        child: child,
      ),
    ),
  );
}

/// Shared Classic editor, including outline, inline settings and expansion.
class ThemeClassicComposerTextEditor extends StatelessWidget {
  const ThemeClassicComposerTextEditor({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.hintText,
    required this.inputExpanded,
    required this.readOnly,
    required this.hasDraftText,
    required this.settings,
    required this.onSubmitted,
    required this.onToggleExpansion,
    required this.expandTooltip,
    required this.collapseTooltip,
    this.contentInsertionConfiguration,
  });
  final TextEditingController controller;
  final FocusNode focusNode;
  final String hintText;
  final bool inputExpanded;
  final bool readOnly;
  final bool hasDraftText;
  final Widget settings;
  final ValueChanged<String> onSubmitted;
  final VoidCallback onToggleExpansion;
  final String expandTooltip;
  final String collapseTooltip;
  final ContentInsertionConfiguration? contentInsertionConfiguration;
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final inputBorder = OutlineInputBorder(
      borderRadius: BorderRadius.circular(14),
      borderSide: BorderSide(
        color: scheme.outline.withValues(alpha: hasDraftText ? 0.68 : 0.42),
      ),
    );
    return Stack(
      children: <Widget>[
        TextField(
          key: const ValueKey<String>('chat.input'),
          controller: controller,
          focusNode: focusNode,
          minLines: inputExpanded ? 10 : 1,
          maxLines: inputExpanded ? 16 : 5,
          readOnly: readOnly,
          textInputAction: TextInputAction.newline,
          contentInsertionConfiguration: contentInsertionConfiguration,
          style: theme.textTheme.bodyMedium?.copyWith(height: 20 / 14),
          decoration: InputDecoration(
            hintText: hintText,
            hintStyle: theme.textTheme.bodyMedium?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
            filled: true,
            fillColor: scheme.surface.withValues(alpha: 0.84),
            border: inputBorder,
            enabledBorder: inputBorder,
            focusedBorder: inputBorder.copyWith(
              borderSide: BorderSide(color: scheme.primary, width: 1.2),
            ),
            contentPadding: const EdgeInsets.fromLTRB(14, 9, 44, 9),
            prefixIconConstraints: const BoxConstraints(
              minWidth: 0,
              minHeight: 34,
            ),
            prefixIcon: ThemeComposerHorizontalVisibility(
              width: 34,
              height: 34,
              visible: !inputExpanded,
              alignment: AlignmentDirectional.centerStart,
              child: settings,
            ),
          ),
          onSubmitted: onSubmitted,
        ),
        PositionedDirectional(
          top: 0,
          end: 4,
          child: ThemeComposerIconTarget(
            icon: inputExpanded ? Icons.fullscreen_exit : Icons.fullscreen,
            color: scheme.onSurfaceVariant,
            onTap: onToggleExpansion,
            size: 18,
            targetSize: 34,
            tooltip: inputExpanded ? collapseTooltip : expandTooltip,
          ),
        ),
      ],
    );
  }
}

/// Shared Classic editor/action-row geometry. Runtime wrappers stay in slots.
class ThemeClassicComposerBody extends StatelessWidget {
  const ThemeClassicComposerBody({
    super.key,
    required this.inputExpanded,
    required this.editor,
    required this.attachment,
    required this.action,
  });
  final bool inputExpanded;
  final Widget editor;
  final Widget attachment;
  final Widget action;
  @override
  Widget build(BuildContext context) => AnimatedSize(
    duration: const Duration(milliseconds: 340),
    reverseDuration: const Duration(milliseconds: 300),
    curve: Curves.easeInOutCubicEmphasized,
    alignment: Alignment.bottomCenter,
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: <Widget>[
        Expanded(child: editor),
        ThemeComposerHorizontalVisibility(
          width: 86,
          height: 36,
          visible: !inputExpanded,
          alignment: AlignmentDirectional.centerEnd,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              const SizedBox(width: 8),
              attachment,
              const SizedBox(width: 8),
              action,
            ],
          ),
        ),
      ],
    ),
  );
}

/// Shared model identity using the active font's actual x-height and cap height.
class ThemeComposerModelIdentity extends StatelessWidget {
  const ThemeComposerModelIdentity({
    super.key,
    required this.label,
    required this.style,
    required this.chevronColor,
    required this.expanded,
    this.logoBuilder,
  });
  final String label;
  final TextStyle style;
  final Color chevronColor;
  final bool expanded;
  final Widget Function(double size)? logoBuilder;

  @override
  Widget build(BuildContext context) {
    TextPainter probe(String glyph) => TextPainter(
      text: TextSpan(text: glyph, style: style),
      textDirection: Directionality.of(context),
      maxLines: 1,
      strutStyle: StrutStyle.fromTextStyle(
        style,
        forceStrutHeight: true,
        height: 1,
      ),
    )..layout();
    final line = probe('Hg');
    final xProbe = probe('x');
    final capProbe = probe('H');
    late final double lineHeight;
    late final double xCenter;
    late final double markSize;
    try {
      final xBoxes = xProbe.getBoxesForSelection(
        const TextSelection(baseOffset: 0, extentOffset: 1),
      );
      final capBoxes = capProbe.getBoxesForSelection(
        const TextSelection(baseOffset: 0, extentOffset: 1),
      );
      lineHeight = line.height;
      if (!lineHeight.isFinite ||
          lineHeight < 1 ||
          xBoxes.isEmpty ||
          capBoxes.isEmpty) {
        throw StateError('Composer font has no measurable H/x glyph metrics');
      }
      xCenter = (xBoxes.first.top + xBoxes.first.bottom) / 2;
      markSize = (capBoxes.first.bottom - capBoxes.first.top)
          .clamp(1.0, lineHeight)
          .toDouble();
      if (!xCenter.isFinite || !markSize.isFinite) {
        throw StateError('Composer font metrics are not finite');
      }
    } finally {
      line.dispose();
      xProbe.dispose();
      capProbe.dispose();
    }
    final extra = lineHeight - markSize;
    final markAlignment = extra.abs() < 0.001
        ? Alignment.center
        : Alignment(0, 2 * (xCenter - markSize / 2) / extra - 1);
    Widget mark(Widget child) => SizedBox(
      width: markSize,
      height: lineHeight,
      child: Align(
        alignment: markAlignment,
        child: SizedBox(width: markSize, height: markSize, child: child),
      ),
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final reserved =
            (logoBuilder != null ? markSize + 4 : 0) + 4 + markSize;
        if (!constraints.maxWidth.isFinite) {
          throw StateError('Composer model selector requires bounded width');
        }
        final textMax = (constraints.maxWidth - reserved)
            .clamp(24.0, 220.0)
            .toDouble();
        return Row(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            if (logoBuilder != null) ...<Widget>[
              ExcludeSemantics(child: mark(logoBuilder!(markSize))),
              const SizedBox(width: 4),
            ],
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: textMax),
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: style,
                strutStyle: StrutStyle.fromTextStyle(
                  style,
                  forceStrutHeight: true,
                  height: 1,
                ),
              ),
            ),
            const SizedBox(width: 4),
            mark(
              AnimatedRotation(
                turns: expanded ? 0.5 : 0,
                duration: const Duration(milliseconds: 220),
                curve: Curves.easeOutCubic,
                child: Icon(
                  Icons.keyboard_arrow_down,
                  size: markSize,
                  color: chevronColor,
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

/// Shared paint for a provider's explicit initial.
class ThemeComposerProviderInitial extends StatelessWidget {
  const ThemeComposerProviderInitial({
    super.key,
    required this.initial,
    required this.size,
  });
  final String initial;
  final double size;
  @override
  Widget build(BuildContext context) => Text(
    initial,
    textAlign: TextAlign.center,
    style: Theme.of(context).textTheme.titleSmall?.copyWith(
      fontWeight: FontWeight.w700,
      fontSize: size * 0.78,
      height: 1,
      color: Theme.of(context).colorScheme.onSurfaceVariant,
    ),
  );
}

/// Shared tappable model selector shell. The caller supplies provider data.
class ThemeComposerModelSelector extends StatelessWidget {
  const ThemeComposerModelSelector({
    super.key,
    required this.identity,
    required this.expanded,
    required this.onTap,
  });
  final Widget identity;
  final bool expanded;
  final VoidCallback? onTap;
  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    const radius = BorderRadius.all(Radius.circular(12));
    return Material(
      color: expanded
          ? scheme.onSurface.withValues(alpha: 0.08)
          : Colors.transparent,
      borderRadius: radius,
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        borderRadius: radius,
        splashFactory: NoSplash.splashFactory,
        overlayColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.pressed)) {
            return scheme.onSurface.withValues(alpha: 0.12);
          }
          if (states.contains(WidgetState.hovered) ||
              states.contains(WidgetState.focused)) {
            return scheme.onSurface.withValues(alpha: 0.08);
          }
          return Colors.transparent;
        }),
        onTap: onTap,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 220),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            child: identity,
          ),
        ),
      ),
    );
  }
}

/// Shared Agent composer layout with runtime behavior supplied through slots.
class ThemeAgentComposerBody extends StatelessWidget {
  const ThemeAgentComposerBody({
    super.key,
    required this.editor,
    required this.modelSelector,
    required this.settings,
    required this.attachment,
    required this.action,
    this.attachments = const <Widget>[],
  });
  final Widget editor;
  final Widget modelSelector;
  final Widget settings;
  final Widget attachment;
  final Widget action;
  final List<Widget> attachments;
  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: <Widget>[
      ...attachments,
      editor,
      const SizedBox(height: 8),
      Row(
        children: <Widget>[
          Expanded(
            child: Align(alignment: Alignment.centerLeft, child: modelSelector),
          ),
          const SizedBox(width: 6),
          settings,
          const SizedBox(width: 8),
          attachment,
          const SizedBox(width: 6),
          action,
        ],
      ),
    ],
  );
}

/// Shared text editor geometry for the Agent composer and the Studio fixture.
class ThemeComposerTextEditor extends StatelessWidget {
  const ThemeComposerTextEditor({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.hintText,
    required this.inputExpanded,
    required this.readOnly,
    required this.onSubmitted,
    required this.onToggleExpansion,
    required this.expandTooltip,
    required this.collapseTooltip,
    this.contentInsertionConfiguration,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final String hintText;
  final bool inputExpanded;
  final bool readOnly;
  final ValueChanged<String> onSubmitted;
  final VoidCallback onToggleExpansion;
  final String expandTooltip;
  final String collapseTooltip;
  final ContentInsertionConfiguration? contentInsertionConfiguration;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return AnimatedSize(
      duration: const Duration(milliseconds: 340),
      reverseDuration: const Duration(milliseconds: 300),
      curve: Curves.easeInOutCubicEmphasized,
      alignment: Alignment.bottomCenter,
      child: Stack(
        children: <Widget>[
          TextField(
            key: const ValueKey<String>('chat.input'),
            controller: controller,
            focusNode: focusNode,
            minLines: inputExpanded ? 10 : 1,
            maxLines: inputExpanded ? 16 : 6,
            enabled: true,
            readOnly: readOnly,
            textInputAction: TextInputAction.newline,
            contentInsertionConfiguration: contentInsertionConfiguration,
            style: theme.textTheme.bodyMedium?.copyWith(height: 20 / 14),
            decoration: InputDecoration(
              hintText: hintText,
              hintStyle: theme.textTheme.bodyMedium?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
              filled: false,
              fillColor: Colors.transparent,
              border: InputBorder.none,
              enabledBorder: InputBorder.none,
              focusedBorder: InputBorder.none,
              contentPadding: const EdgeInsets.fromLTRB(16, 10, 48, 8),
            ),
            onSubmitted: onSubmitted,
          ),
          PositionedDirectional(
            top: 0,
            end: 4,
            child: ThemeComposerIconTarget(
              icon: inputExpanded ? Icons.fullscreen_exit : Icons.fullscreen,
              color: scheme.onSurfaceVariant,
              onTap: onToggleExpansion,
              tooltip: inputExpanded ? collapseTooltip : expandTooltip,
            ),
          ),
        ],
      ),
    );
  }
}

/// Shared hit target for small composer controls.
class ThemeComposerIconTarget extends StatelessWidget {
  const ThemeComposerIconTarget({
    super.key,
    required this.icon,
    required this.color,
    required this.onTap,
    required this.tooltip,
    this.imageAsset,
    this.imageSize,
    this.size = 20,
    this.targetSize = 36,
  });

  final IconData icon;
  final Color color;
  final VoidCallback? onTap;
  final String tooltip;
  final String? imageAsset;
  final double? imageSize;
  final double size;
  final double targetSize;

  @override
  Widget build(BuildContext context) {
    final hasImage = imageAsset != null && imageAsset!.isNotEmpty;
    final visualSize = hasImage ? (imageSize ?? size) : size;
    final effectiveTargetSize = visualSize > targetSize
        ? visualSize
        : targetSize;
    return Tooltip(
      message: tooltip,
      child: InkResponse(
        onTap: onTap,
        radius: effectiveTargetSize / 2,
        child: SizedBox(
          width: effectiveTargetSize,
          height: effectiveTargetSize,
          child: Center(
            child: hasImage
                ? SizedBox(
                    width: visualSize,
                    height: visualSize,
                    child: ResolvedThemeImage(
                      resolver: ThemeAssetScope.of(context),
                      reference: imageAsset!,
                      fit: BoxFit.contain,
                    ),
                  )
                : Icon(icon, size: size, color: color),
          ),
        ),
      ),
    );
  }
}

/// Shared circular action control for the Agent composer.
class ThemeComposerActionButton extends StatelessWidget {
  const ThemeComposerActionButton({
    super.key,
    required this.processing,
    required this.progress,
    required this.background,
    required this.foreground,
    required this.icon,
    required this.tooltip,
    required this.onPressed,
    this.imageAsset,
    this.imageSize,
    this.buttonSize = 40,
    this.contentSize = 36,
    this.iconSize = 18,
  });

  final bool processing;
  final double? progress;
  final Color background;
  final Color foreground;
  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;
  final String? imageAsset;
  final double? imageSize;
  final double buttonSize;
  final double contentSize;
  final double iconSize;

  @override
  Widget build(BuildContext context) {
    final hasImage = imageAsset != null && imageAsset!.isNotEmpty;
    final visualSize = hasImage ? (imageSize ?? contentSize) : contentSize;
    final outerSize = hasImage
        ? (visualSize > buttonSize ? visualSize : buttonSize)
        : buttonSize;
    final button = SizedBox(
      width: outerSize,
      height: outerSize,
      child: Stack(
        alignment: Alignment.center,
        children: <Widget>[
          if (hasImage)
            Material(
              color: Colors.transparent,
              child: InkResponse(
                onTap: onPressed,
                radius: outerSize / 2,
                child: SizedBox(
                  width: outerSize,
                  height: outerSize,
                  child: Center(
                    child: SizedBox(
                      width: visualSize,
                      height: visualSize,
                      child: ResolvedThemeImage(
                        resolver: ThemeAssetScope.of(context),
                        reference: imageAsset!,
                        fit: BoxFit.contain,
                      ),
                    ),
                  ),
                ),
              ),
            )
          else
            Material(
              color: background,
              shape: const CircleBorder(),
              child: InkWell(
                customBorder: const CircleBorder(),
                onTap: onPressed,
                child: SizedBox(
                  width: contentSize,
                  height: contentSize,
                  child: Icon(icon, size: iconSize, color: foreground),
                ),
              ),
            ),
          if (processing)
            Positioned.fill(
              child: IgnorePointer(
                child: CircularProgressIndicator(
                  value: progress,
                  strokeWidth: 2.4,
                  color: foreground.withValues(alpha: 0.9),
                  backgroundColor: foreground.withValues(alpha: 0.24),
                ),
              ),
            ),
        ],
      ),
    );
    return tooltip.isEmpty ? button : Tooltip(message: tooltip, child: button);
  }
}
