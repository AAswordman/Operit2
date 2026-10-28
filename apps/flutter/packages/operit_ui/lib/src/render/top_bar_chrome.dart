import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

/// The title treatment used by the main app bar and the Studio fixture.
class ThemeTopBarTitle extends StatelessWidget {
  const ThemeTopBarTitle({
    super.key,
    required this.primaryText,
    required this.contentColor,
    this.secondaryText = '',
  });

  final String primaryText;
  final String secondaryText;
  final Color contentColor;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: <Widget>[
        if (primaryText.isNotEmpty)
          Flexible(
            fit: FlexFit.loose,
            child: Text(
              primaryText,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.titleSmall?.copyWith(
                color: contentColor,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        if (primaryText.isNotEmpty && secondaryText.isNotEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 6),
            child: Text(
              '-',
              style: theme.textTheme.bodySmall?.copyWith(
                color: contentColor.withValues(alpha: 0.6),
                fontWeight: FontWeight.w400,
              ),
            ),
          ),
        if (secondaryText.isNotEmpty)
          Flexible(
            child: Text(
              secondaryText,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: contentColor.withValues(alpha: 0.85),
              ),
            ),
          ),
      ],
    );
  }
}

/// Shared visual frame for the main client bar. Routing and action creation
/// remain with the host; this widget owns only the fixed paint geometry.
class ThemeTopBarChrome extends StatelessWidget {
  const ThemeTopBarChrome({
    super.key,
    required this.title,
    this.navigationIcon,
    this.navigationTooltip,
    this.onNavigationPressed,
    required this.contentColor,
    required this.contentForeground,
    this.topPadding = 0,
    this.flipNavigationIcon = false,
    this.actions = const <Widget>[],
  });

  final Widget title;
  final IconData? navigationIcon;
  final String? navigationTooltip;
  final VoidCallback? onNavigationPressed;
  final Color contentColor;
  final Color contentForeground;
  final double topPadding;
  final bool flipNavigationIcon;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) {
    final navIcon = navigationIcon;
    final navigationIconWidget =
        navIcon != null ? Icon(navIcon, color: contentForeground) : null;
    return ColoredBox(
      color: contentColor,
      child: SizedBox(
        height: topPadding + 64,
        child: Padding(
          padding: EdgeInsets.only(top: topPadding),
          child: Row(
            children: <Widget>[
              if (navIcon != null && navigationIconWidget != null) ...<Widget>[
                const SizedBox(width: 4),
                SizedBox(
                  width: 48,
                  height: 48,
                  child: IconButton(
                    onPressed: onNavigationPressed,
                    icon: flipNavigationIcon
                        ? Transform(
                            alignment: Alignment.center,
                            transform: Matrix4.identity()
                              ..scaleByDouble(-1.0, 1.0, 1.0, 1.0),
                            child: navigationIconWidget,
                          )
                        : navigationIconWidget,
                    tooltip: navigationTooltip,
                  ),
                ),
              ] else ...<Widget>[
                const SizedBox(width: 16),
              ],
              Expanded(child: title),
              ...actions,
            ],
          ),
        ),
      ),
    );
  }
}

/// Shared workspace toggle used as a top-bar action.
class ThemeWorkspaceTopBarButton extends StatelessWidget {
  const ThemeWorkspaceTopBarButton({
    super.key,
    required this.open,
    required this.hasBoundWorkspace,
    required this.onPressed,
  });

  final bool open;
  final bool hasBoundWorkspace;
  final VoidCallback onPressed;

  static const String _workspaceClosedSvg = '''
<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
  <rect width="18" height="18" x="3" y="3" rx="3"/>
  <path d="M15 3v18"/>
</svg>''';

  static const String _workspaceOpenSvg = '''
<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
  <rect width="18" height="18" x="3" y="3" rx="3"/>
  <path d="M15 3v18"/>
  <path d="M15 3h3a3 3 0 0 1 3 3v12a3 3 0 0 1-3 3h-3V3z" fill="currentColor"/>
</svg>''';

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final iconColor = open ? scheme.primary : scheme.onSurface;
    return Padding(
      padding: const EdgeInsetsDirectional.only(end: 4),
      child: SizedBox(
        width: 48,
        height: 48,
        child: IconButton(
          onPressed: onPressed,
          icon: SvgPicture.string(
            open ? _workspaceOpenSvg : _workspaceClosedSvg,
            width: 20,
            height: 20,
            colorFilter: ColorFilter.mode(iconColor, BlendMode.srcIn),
          ),
          tooltip: open ? 'Close workspace' : 'Workspace',
        ),
      ),
    );
  }
}
