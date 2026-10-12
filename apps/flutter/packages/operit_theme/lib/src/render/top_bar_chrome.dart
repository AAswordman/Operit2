import 'package:flutter/material.dart';

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
        if (secondaryText.isNotEmpty)
          Flexible(
            child: Text(
              '- $secondaryText',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: contentColor.withValues(alpha: 0.8),
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
    required this.navigationIcon,
    required this.navigationTooltip,
    required this.onNavigationPressed,
    required this.contentColor,
    required this.contentForeground,
    this.topPadding = 0,
    this.flipNavigationIcon = false,
    this.actions = const <Widget>[],
  });

  final Widget title;
  final IconData navigationIcon;
  final String navigationTooltip;
  final VoidCallback onNavigationPressed;
  final Color contentColor;
  final Color contentForeground;
  final double topPadding;
  final bool flipNavigationIcon;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) {
    final navigationIconWidget = Icon(navigationIcon, color: contentForeground);
    return ColoredBox(
      color: contentColor,
      child: SizedBox(
        height: topPadding + 64,
        child: Padding(
          padding: EdgeInsets.only(top: topPadding),
          child: Row(
            children: <Widget>[
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

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsetsDirectional.only(end: 4),
      child: SizedBox(
        width: 48,
        height: 48,
        child: IconButton(
          onPressed: onPressed,
          icon: Icon(
            hasBoundWorkspace ? Icons.code : Icons.code_off,
            color: open ? scheme.primary : scheme.onSurface,
          ),
          tooltip: open ? 'Close workspace' : 'Workspace',
        ),
      ),
    );
  }
}
