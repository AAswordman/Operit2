// ignore_for_file: file_names

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../theme/OperitGlassSurface.dart';

class WorkspacePathBar extends StatelessWidget {
  const WorkspacePathBar.readOnly({
    super.key,
    required this.path,
    this.onRefresh,
    this.leading,
  }) : isEditable = false,
       isEditing = null,
       onSubmitted = null,
       onEditToggle = null,
       controller = null;

  const WorkspacePathBar.editable({
    super.key,
    required this.path,
    required this.onRefresh,
    required this.onSubmitted,
    required this.isEditing,
    required this.onEditToggle,
    this.controller,
    this.leading,
  }) : isEditable = true;

  final String path;
  final VoidCallback? onRefresh;
  final bool isEditable;
  final bool? isEditing;
  final VoidCallback? onEditToggle;
  final ValueChanged<String>? onSubmitted;
  final TextEditingController? controller;
  final Widget? leading;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    if (!isEditable) {
      return _WorkspacePathStrip(
        onRefresh: onRefresh,
        leading: leading,
        scrollable: true,
        child: Text(
          path,
          maxLines: 1,
          softWrap: false,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      );
    }
    final editing = isEditing ?? false;
    return _WorkspacePathStrip(
      onRefresh: onRefresh,
      leading: leading,
      scrollable: !editing,
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          _WorkspacePathIconButton(
            tooltip: editing ? l10n.ok : l10n.edit,
            onPressed: editing
                ? () => onSubmitted?.call(controller?.text ?? path)
                : onEditToggle,
            icon: editing ? Icons.check : Icons.edit_outlined,
          ),
        ],
      ),
      child: editing
          ? TextField(
              controller: controller,
              autofocus: true,
              maxLines: 1,
              textInputAction: TextInputAction.done,
              onSubmitted: onSubmitted,
              decoration: const InputDecoration(
                isDense: true,
                border: InputBorder.none,
                contentPadding: EdgeInsets.zero,
              ),
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            )
          : GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: onEditToggle,
              child: Text(
                path,
                maxLines: 1,
                softWrap: false,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
    );
  }
}

class _WorkspacePathStrip extends StatelessWidget {
  const _WorkspacePathStrip({
    required this.child,
    required this.scrollable,
    this.onRefresh,
    this.leading,
    this.trailing,
  });

  final Widget child;
  final bool scrollable;
  final VoidCallback? onRefresh;
  final Widget? leading;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return OperitGlassSurface(
      color: theme.colorScheme.surfaceContainerLow.withValues(alpha: 0.5),
      layer: OperitGlassSurfaceLayer.control,
      border: Border(
        bottom: BorderSide(
          color: theme.colorScheme.outlineVariant.withValues(alpha: 0.36),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
        child: Row(
          children: <Widget>[
            ?leading,
            if (onRefresh != null)
              _WorkspacePathIconButton(
                tooltip: l10n.refresh,
                onPressed: onRefresh,
                icon: Icons.refresh,
              ),
            Expanded(
              child: scrollable
                  ? _WorkspaceScrollablePathContent(child: child)
                  : Padding(
                      padding: const EdgeInsets.only(right: 8),
                      child: child,
                    ),
            ),
            ?trailing,
          ],
        ),
      ),
    );
  }
}

class WorkspacePathIconButton extends StatelessWidget {
  const WorkspacePathIconButton({
    super.key,
    required this.tooltip,
    required this.icon,
    this.onPressed,
  });

  final String tooltip;
  final IconData icon;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return _WorkspacePathIconButton(
      tooltip: tooltip,
      icon: icon,
      onPressed: onPressed,
    );
  }
}

class _WorkspacePathIconButton extends StatelessWidget {
  const _WorkspacePathIconButton({
    required this.tooltip,
    required this.icon,
    this.onPressed,
  });

  final String tooltip;
  final IconData icon;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      tooltip: tooltip,
      onPressed: onPressed,
      icon: Icon(icon, size: 20),
      visualDensity: VisualDensity.compact,
      padding: EdgeInsets.zero,
      constraints: const BoxConstraints.tightFor(width: 36, height: 36),
    );
  }
}

class _WorkspaceScrollablePathContent extends StatefulWidget {
  const _WorkspaceScrollablePathContent({required this.child});

  final Widget child;

  @override
  State<_WorkspaceScrollablePathContent> createState() =>
      _WorkspaceScrollablePathContentState();
}

class _WorkspaceScrollablePathContentState
    extends State<_WorkspaceScrollablePathContent> {
  final ScrollController _controller = ScrollController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ScrollConfiguration(
      behavior: ScrollConfiguration.of(context).copyWith(
        dragDevices: const <PointerDeviceKind>{
          PointerDeviceKind.touch,
          PointerDeviceKind.mouse,
          PointerDeviceKind.trackpad,
          PointerDeviceKind.stylus,
        },
      ),
      child: SizedBox(
        height: 36,
        child: Scrollbar(
          controller: _controller,
          thumbVisibility: false,
          thickness: 2,
          radius: const Radius.circular(999),
          child: SingleChildScrollView(
            controller: _controller,
            scrollDirection: Axis.horizontal,
            child: Align(
              alignment: Alignment.centerLeft,
              child: Padding(
                padding: const EdgeInsets.only(right: 8),
                child: widget.child,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
