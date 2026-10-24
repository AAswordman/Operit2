import 'package:flutter/material.dart';

import 'desktop_sidebar_surface.dart';

class ThemeBottomThemeToggleButton extends StatelessWidget {
  const ThemeBottomThemeToggleButton({
    super.key,
    required this.appearance,
    required this.darkThemeActive,
    required this.onToggle,
  });

  final ThemeDrawerPalette appearance;
  final bool darkThemeActive;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(8);
    return Tooltip(
      message: darkThemeActive ? '切换白天模式' : '切换黑夜模式',
      child: SizedBox(
        width: 34,
        height: 34,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: appearance.buttonContainerColor.withValues(alpha: 0.55),
            borderRadius: shape,
            border: Border.all(
              color: appearance.dividerColor.withValues(alpha: 0.35),
              width: 1,
            ),
          ),
          child: Material(
            color: Colors.transparent,
            borderRadius: shape,
            child: InkWell(
              borderRadius: shape,
              onTap: onToggle,
              child: Center(
                child: Icon(
                  darkThemeActive
                      ? Icons.light_mode_outlined
                      : Icons.dark_mode_outlined,
                  size: 16,
                  color: appearance.itemColor.withValues(alpha: 0.85),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Declarative drawer colors; both production navigation and Studio consume
/// the same mapping from the active ColorScheme and transparency setting.
class ThemeDrawerPalette {
  const ThemeDrawerPalette({
    required this.containerColor,
    required this.titleColor,
    required this.statusAvailableColor,
    required this.itemColor,
    required this.buttonContainerColor,
    required this.selectedContainerColor,
    required this.selectedContentColor,
    required this.dividerColor,
    required this.transparentSurfaceEnabled,
  });

  /// Preserves the application surface palette for shared navigation widgets.
  factory ThemeDrawerPalette.fromScheme(
    ColorScheme scheme, {
    required bool transparentSurface,
  }) => ThemeDrawerPalette(
    containerColor: themeSidebarContainerColor(
      scheme,
      transparentSurface: transparentSurface,
    ),
    titleColor: scheme.onSurface,
    statusAvailableColor: scheme.primary,
    itemColor: scheme.onSurfaceVariant,
    buttonContainerColor: transparentSurface
        ? scheme.surfaceContainerLow.withValues(alpha: 0.18)
        : scheme.surfaceContainerLow.withValues(alpha: 0.72),
    selectedContainerColor: transparentSurface
        ? scheme.secondaryContainer.withValues(alpha: 0.34)
        : scheme.secondaryContainer.withValues(alpha: 0.78),
    selectedContentColor: scheme.onSecondaryContainer,
    dividerColor: transparentSurface
        ? scheme.outlineVariant.withValues(alpha: 0.34)
        : scheme.outlineVariant.withValues(alpha: 0.62),
    transparentSurfaceEnabled: transparentSurface,
  );

  final Color containerColor;
  final Color titleColor;
  final Color statusAvailableColor;
  final Color itemColor;
  final Color buttonContainerColor;
  final Color selectedContainerColor;
  final Color selectedContentColor;
  final Color dividerColor;
  final bool transparentSurfaceEnabled;
}

class ThemeSidebarInfoCard extends StatelessWidget {
  const ThemeSidebarInfoCard({
    super.key,
    required this.brandName,
    required this.appearance,
    this.trailing,
  });

  final String brandName;
  final ThemeDrawerPalette appearance;
  final Widget? trailing;

  /// Constrains the brand title to the space left by the native tab switch.
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsetsDirectional.fromSTEB(20, 4, 14, 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          Expanded(
            child: Text(
              brandName,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.titleLarge?.copyWith(
                letterSpacing: 0,
                color: appearance.titleColor,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
          ?trailing,
        ],
      ),
    );
  }
}

class ThemeBottomSidebarAction extends StatelessWidget {
  const ThemeBottomSidebarAction({
    super.key,
    required this.icon,
    required this.label,
    required this.appearance,
    required this.onClick,
    this.selected = false,
  });

  final IconData icon;
  final String label;
  final ThemeDrawerPalette appearance;
  final VoidCallback onClick;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(8);
    final backgroundColor = selected
        ? appearance.selectedContainerColor.withValues(alpha: 0.65)
        : appearance.buttonContainerColor.withValues(alpha: 0.55);
    final borderColor = selected
        ? appearance.selectedContentColor.withValues(alpha: 0.12)
        : appearance.dividerColor.withValues(alpha: 0.35);
    final contentColor = selected
        ? appearance.selectedContentColor
        : appearance.itemColor.withValues(alpha: 0.88);

    return SizedBox(
      height: 34,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: backgroundColor,
          borderRadius: shape,
          border: Border.all(color: borderColor, width: 1),
        ),
        child: Material(
          color: Colors.transparent,
          borderRadius: shape,
          child: InkWell(
            borderRadius: shape,
            onTap: onClick,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: <Widget>[
                  Icon(icon, size: 16, color: contentColor),
                  const SizedBox(width: 6),
                  Flexible(
                    child: Text(
                      label,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12.5,
                        fontWeight: selected
                            ? FontWeight.w600
                            : FontWeight.w500,
                        color: contentColor,
                        letterSpacing: -0.1,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Shared split action used for creating a conversation or a group.
class ThemeUnifiedCreateBar extends StatelessWidget {
  const ThemeUnifiedCreateBar({
    super.key,
    required this.onCreateConversation,
    required this.onCreateGroup,
    this.conversationLabel = '新建对话',
    this.groupTooltip = '新建分组',
  });

  final VoidCallback onCreateConversation;
  final VoidCallback onCreateGroup;
  final String conversationLabel;
  final String groupTooltip;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final contentColor = scheme.onPrimaryContainer;
    return SizedBox(
      height: 34,
      child: Material(
        color: scheme.primaryContainer,
        shape: const StadiumBorder(),
        clipBehavior: Clip.antiAlias,
        child: Row(
          children: <Widget>[
            Expanded(
              child: InkWell(
                borderRadius: const BorderRadius.horizontal(
                  left: Radius.circular(17),
                ),
                hoverColor: contentColor.withValues(alpha: 0.08),
                focusColor: contentColor.withValues(alpha: 0.10),
                splashColor: contentColor.withValues(alpha: 0.10),
                highlightColor: contentColor.withValues(alpha: 0.10),
                onTap: onCreateConversation,
                child: Center(
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Icon(Icons.add_rounded, size: 17, color: contentColor),
                      const SizedBox(width: 6),
                      Text(
                        conversationLabel,
                        style: TextStyle(
                          fontSize: 13,
                          fontWeight: FontWeight.w600,
                          color: contentColor,
                          letterSpacing: -0.1,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            Container(
              width: 1,
              height: 16,
              color: contentColor.withValues(alpha: 0.16),
            ),
            Tooltip(
              message: groupTooltip,
              child: SizedBox(
                width: 38,
                height: 34,
                child: InkWell(
                  borderRadius: const BorderRadius.horizontal(
                    right: Radius.circular(17),
                  ),
                  hoverColor: contentColor.withValues(alpha: 0.08),
                  focusColor: contentColor.withValues(alpha: 0.10),
                  splashColor: contentColor.withValues(alpha: 0.10),
                  highlightColor: contentColor.withValues(alpha: 0.10),
                  onTap: onCreateGroup,
                  child: Center(
                    child: Icon(
                      Icons.create_new_folder_outlined,
                      size: 16,
                      color: contentColor,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Shared compact toolbar icon used beside the create bar.
class ThemeDrawerToolbarIconButton extends StatelessWidget {
  const ThemeDrawerToolbarIconButton({
    super.key,
    required this.icon,
    required this.tooltip,
    required this.appearance,
    required this.onClick,
    this.active = false,
  });

  final IconData icon;
  final String tooltip;
  final ThemeDrawerPalette appearance;
  final VoidCallback onClick;
  final bool active;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: SizedBox(
        width: 34,
        height: 34,
        child: Material(
          color: active
              ? appearance.selectedContainerColor.withValues(alpha: 0.35)
              : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
          child: InkWell(
            borderRadius: BorderRadius.circular(6),
            onTap: onClick,
            child: Center(
              child: Icon(
                icon,
                size: 17,
                color: active
                    ? appearance.statusAvailableColor
                    : appearance.itemColor.withValues(alpha: 0.78),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
