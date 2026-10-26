// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';

class PackageListItem extends StatefulWidget {
  /// Creates an expandable package card with a separate details action.
  const PackageListItem({
    super.key,
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.metadata,
    required this.enabled,
    required this.onEnabledChanged,
    this.onDetails,
    this.primaryAction,
    this.showEnabledSwitch = true,
    this.trailingActions = const <Widget>[],
    this.hasError = false,
    this.errorMessage,
    this.statusLabel,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final List<String> metadata;
  final bool enabled;
  final ValueChanged<bool> onEnabledChanged;
  final VoidCallback? onDetails;
  final Widget? primaryAction;
  final bool showEnabledSwitch;
  final List<Widget> trailingActions;
  final bool hasError;
  final String? errorMessage;
  final String? statusLabel;

  /// Creates the state for an expandable package card.
  @override
  State<PackageListItem> createState() => _PackageListItemState();
}

class _PackageListItemState extends State<PackageListItem> {
  bool _expanded = false;

  /// Toggles the expanded state of the package card.
  void _toggleExpanded() {
    setState(() {
      _expanded = !_expanded;
    });
  }

  /// Builds a compact or expanded package card.
  @override
  Widget build(BuildContext context) {
    final themeController = OperitTheme.of(context);
    final themeScale = themeController.themePreferenceSnapshot.fontScale;
    final textScale = MediaQuery.textScalerOf(context).scale(1);
    final scale = themeScale * textScale;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final nonEmptyMetadata = widget.metadata
        .where((item) => item.trim().isNotEmpty)
        .toList(growable: false);
    final borderRadius = BorderRadius.circular(12 * scale);
    final hasError = widget.hasError;
    final errorColor = colorScheme.error;

    return Padding(
      padding: EdgeInsets.symmetric(vertical: 4 * scale),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.42),
        layer: OperitGlassSurfaceLayer.card,
        borderRadius: borderRadius,
        border: Border.all(
          color: hasError
              ? errorColor.withValues(alpha: 0.78)
              : colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        shadows: hasError
            ? <BoxShadow>[
                BoxShadow(
                  color: errorColor.withValues(alpha: 0.30),
                  blurRadius: 18 * scale,
                  spreadRadius: 1 * scale,
                ),
              ]
            : const <BoxShadow>[],
        material: true,
        // One blur per card rasterizes the whole list on tab switches and scrolls.
        enableBackdropFilter: false,
        child: InkWell(
          borderRadius: borderRadius,
          onTap: widget.onDetails,
          child: Padding(
            padding: EdgeInsets.all(12 * scale),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: <Widget>[
                    CircleAvatar(
                      radius: 18 * scale,
                      backgroundColor: colorScheme.primary.withValues(
                        alpha: 0.12,
                      ),
                      child: Icon(
                        widget.icon,
                        color: colorScheme.primary,
                        size: 19 * scale,
                      ),
                    ),
                    SizedBox(width: 10 * scale),
                    Expanded(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          Row(
                            children: <Widget>[
                              Flexible(
                                child: Text(
                                  widget.title,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: textTheme.titleSmall?.copyWith(
                                    fontWeight: FontWeight.w700,
                                  ),
                                ),
                              ),
                              if (widget.statusLabel
                                  case final String label) ...<Widget>[
                                SizedBox(width: 6 * scale),
                                _MetadataChip(text: label),
                              ],
                            ],
                          ),
                          if (widget.subtitle.isNotEmpty)
                            Padding(
                              padding: EdgeInsets.only(top: 2 * scale),
                              child: Text(
                                widget.subtitle,
                                maxLines: _expanded ? 4 : 1,
                                overflow: TextOverflow.ellipsis,
                                style: textTheme.bodySmall?.copyWith(
                                  color: colorScheme.onSurfaceVariant,
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                    if (hasError)
                      Tooltip(
                        message: widget.errorMessage ?? '前置插件不可用',
                        child: Padding(
                          padding: EdgeInsets.only(left: 6 * scale),
                          child: Icon(
                            Icons.error_outline,
                            color: errorColor,
                            size: 20 * scale,
                          ),
                        ),
                      ),
                    if (widget.primaryAction
                        case final Widget action) ...<Widget>[
                      SizedBox(width: 6 * scale),
                      action,
                    ],
                    if (widget.showEnabledSwitch) ...<Widget>[
                      SizedBox(width: 6 * scale),
                      Switch(
                        value: widget.enabled,
                        onChanged: widget.onEnabledChanged,
                      ),
                    ],
                    SizedBox(width: 2 * scale),
                    IconButton(
                      tooltip: _expanded ? '收起' : '展开',
                      visualDensity: VisualDensity.compact,
                      onPressed: _toggleExpanded,
                      icon: Icon(
                        _expanded
                            ? Icons.keyboard_arrow_up
                            : Icons.keyboard_arrow_down,
                      ),
                    ),
                  ],
                ),
                AnimatedSize(
                  duration: const Duration(milliseconds: 180),
                  curve: Curves.easeOutCubic,
                  child: _expanded
                      ? Padding(
                          padding: EdgeInsets.only(top: 8 * scale),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            children: <Widget>[
                              if (nonEmptyMetadata.isNotEmpty)
                                Wrap(
                                  spacing: 6 * scale,
                                  runSpacing: 6 * scale,
                                  children: <Widget>[
                                    for (final item in nonEmptyMetadata)
                                      _MetadataChip(text: item),
                                  ],
                                ),
                              if (widget.trailingActions.isNotEmpty)
                                Padding(
                                  padding: EdgeInsets.only(top: 6 * scale),
                                  child: Wrap(
                                    alignment: WrapAlignment.end,
                                    spacing: 8 * scale,
                                    runSpacing: 4 * scale,
                                    children: widget.trailingActions,
                                  ),
                                ),
                            ],
                          ),
                        )
                      : const SizedBox.shrink(),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _MetadataChip extends StatelessWidget {
  /// Creates a metadata chip.
  const _MetadataChip({required this.text});

  final String text;

  /// Builds a metadata chip using the current text and UI scale.
  @override
  Widget build(BuildContext context) {
    final themeController = OperitTheme.of(context);
    final themeScale = themeController.themePreferenceSnapshot.fontScale;
    final textScale = MediaQuery.textScalerOf(context).scale(1);
    final scale = themeScale * textScale;
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: EdgeInsets.symmetric(
          horizontal: 8 * scale,
          vertical: 3 * scale,
        ),
        child: Text(
          text,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(
            context,
          ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ),
    );
  }
}
