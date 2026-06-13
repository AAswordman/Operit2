// ignore_for_file: file_names

import 'package:flutter/material.dart';

class OperitDialogScaffold extends StatelessWidget {
  const OperitDialogScaffold({
    super.key,
    required this.title,
    required this.child,
    this.actions = const <Widget>[],
    this.icon,
    this.maxWidth = 560,
    this.maxHeight,
    this.contentPadding = const EdgeInsets.fromLTRB(24, 16, 24, 20),
    this.actionsPadding = const EdgeInsets.fromLTRB(24, 0, 24, 20),
    this.titleActions = const <Widget>[],
    this.showCloseButton = false,
    this.closeButtonEnabled = true,
    this.onClose,
  });

  final String title;
  final Widget child;
  final List<Widget> actions;
  final Widget? icon;
  final double maxWidth;
  final double? maxHeight;
  final EdgeInsetsGeometry contentPadding;
  final EdgeInsetsGeometry actionsPadding;
  final List<Widget> titleActions;
  final bool showCloseButton;
  final bool closeButtonEnabled;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final dialogTheme = DialogTheme.of(context);
    final shape =
        dialogTheme.shape ??
        RoundedRectangleBorder(borderRadius: BorderRadius.circular(28));
    return Dialog(
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: maxWidth,
          maxHeight: maxHeight ?? double.infinity,
        ),
        child: Material(
          color: Colors.transparent,
          shape: shape,
          clipBehavior: Clip.antiAlias,
          child: Column(
            mainAxisSize: maxHeight == null
                ? MainAxisSize.min
                : MainAxisSize.max,
            children: <Widget>[
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 22, 16, 12),
                child: Row(
                  children: <Widget>[
                    if (icon != null) ...<Widget>[
                      IconTheme.merge(
                        data: IconThemeData(color: colorScheme.primary),
                        child: icon!,
                      ),
                      const SizedBox(width: 12),
                    ],
                    Expanded(
                      child: Text(
                        title,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style:
                            dialogTheme.titleTextStyle ??
                            theme.textTheme.headlineSmall?.copyWith(
                              fontWeight: FontWeight.w700,
                            ),
                      ),
                    ),
                    for (final action in titleActions) action,
                    if (showCloseButton)
                      IconButton(
                        tooltip: MaterialLocalizations.of(
                          context,
                        ).closeButtonTooltip,
                        onPressed: closeButtonEnabled
                            ? onClose ?? () => Navigator.of(context).maybePop()
                            : null,
                        icon: const Icon(Icons.close),
                      ),
                  ],
                ),
              ),
              if (maxHeight == null)
                Padding(padding: contentPadding, child: child)
              else
                Expanded(
                  child: Padding(padding: contentPadding, child: child),
                ),
              if (actions.isNotEmpty) ...<Widget>[
                OperitDialogActionBar(
                  padding: actionsPadding,
                  children: actions,
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class OperitDialogActionBar extends StatelessWidget {
  const OperitDialogActionBar({
    super.key,
    required this.children,
    this.padding = const EdgeInsets.fromLTRB(24, 12, 24, 20),
  });

  final List<Widget> children;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: padding,
      child: Align(
        alignment: AlignmentDirectional.centerEnd,
        child: Wrap(
          spacing: 8,
          runSpacing: 8,
          alignment: WrapAlignment.end,
          children: children,
        ),
      ),
    );
  }
}
