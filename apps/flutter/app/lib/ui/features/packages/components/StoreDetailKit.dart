// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../theme/OperitGlassSurface.dart';
import '../../settings/components/SettingsControlStyles.dart';
import '../market/ArtifactMarketSupport.dart';
import 'ExtensionMotion.dart';
import 'StoreVisuals.dart';

const double storeDetailMaxWidth = 880;

/// Scrolling body of a detail route; the shell top bar owns title and back.
class StoreDetailScaffold extends StatelessWidget {
  const StoreDetailScaffold({
    super.key,
    required this.header,
    required this.children,
  });

  final Widget header;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Material(
      type: MaterialType.transparency,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final gutter = constraints.maxWidth < 600 ? 16.0 : 24.0;
          return CustomScrollView(
            slivers: <Widget>[
              SliverPadding(
                padding: storeSliverPadding(
                  constraints.maxWidth,
                  maxWidth: storeDetailMaxWidth,
                  gutter: gutter,
                  top: 12,
                ),
                sliver: SliverToBoxAdapter(child: header),
              ),
              SliverPadding(
                padding: storeSliverPadding(
                  constraints.maxWidth,
                  maxWidth: storeDetailMaxWidth,
                  gutter: gutter,
                  bottom: 48,
                ),
                sliver: SliverList.list(children: children),
              ),
            ],
          );
        },
      ),
    );
  }
}

/// Avatar, title, developer line, badges and actions; settles in once.
class StoreDetailHeader extends StatelessWidget {
  const StoreDetailHeader({
    super.key,
    required this.icon,
    required this.title,
    required this.subtitle,
    this.badges = const <Widget>[],
    this.actions = const <Widget>[],
    this.footer,
  });

  final Widget icon;
  final String title;
  final String subtitle;
  final List<Widget> badges;
  final List<Widget> actions;

  /// Line under the actions, e.g. how a switch behaves across devices.
  final Widget? footer;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final motion = ExtensionMotion.of(context);
    final footer = this.footer;
    final identity = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Text(
          title,
          style: theme.textTheme.titleLarge?.copyWith(
            fontWeight: FontWeight.w700,
          ),
        ),
        if (subtitle.trim().isNotEmpty) ...<Widget>[
          const SizedBox(height: 4),
          Text(
            subtitle,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
        if (badges.isNotEmpty) ...<Widget>[
          const SizedBox(height: 10),
          Wrap(spacing: 6, runSpacing: 6, children: badges),
        ],
      ],
    );
    final actionBlock = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        if (actions.isNotEmpty)
          Wrap(
            spacing: 10,
            runSpacing: 10,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: actions,
          ),
        if (footer != null) ...<Widget>[
          const SizedBox(height: 10),
          DefaultTextStyle.merge(
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
            child: footer,
          ),
        ],
      ],
    );
    final hasActions = actions.isNotEmpty || footer != null;
    return TweenAnimationBuilder<double>(
      tween: Tween<double>(begin: 0, end: 1),
      duration: motion.emphasized,
      curve: ExtensionMotion.enter,
      builder: (context, settle, child) => Transform.translate(
        offset: Offset(0, 8 * (1 - settle)),
        child: child,
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final wide = constraints.maxWidth >= 560;
          final avatar = TweenAnimationBuilder<double>(
            tween: Tween<double>(begin: 0.94, end: 1),
            duration: motion.emphasized,
            curve: ExtensionMotion.enter,
            builder: (context, scale, child) =>
                Transform.scale(scale: scale, child: child),
            child: icon,
          );
          if (wide) {
            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                avatar,
                const SizedBox(width: 20),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      identity,
                      if (hasActions) ...<Widget>[
                        const SizedBox(height: 16),
                        actionBlock,
                      ],
                    ],
                  ),
                ),
              ],
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  avatar,
                  const SizedBox(width: 16),
                  Expanded(child: identity),
                ],
              ),
              if (hasActions) ...<Widget>[
                const SizedBox(height: 16),
                actionBlock,
              ],
            ],
          );
        },
      ),
    );
  }
}

class StoreStat {
  const StoreStat({required this.label, required this.value, this.icon});

  final String label;
  final String value;
  final IconData? icon;
}

/// Row of headline facts separated by hairlines.
class StoreStatsStrip extends StatelessWidget {
  const StoreStatsStrip({super.key, required this.stats});

  final List<StoreStat> stats;

  static const double _cellWidth = 108;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final divider = colorScheme.outlineVariant.withValues(alpha: 0.5);
    return Padding(
      padding: const EdgeInsets.only(top: 20),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.symmetric(horizontal: BorderSide(color: divider)),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 12),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final fits = stats.length * _cellWidth <= constraints.maxWidth;
              final cells = <Widget>[
                for (var index = 0; index < stats.length; index += 1) ...<Widget>[
                  if (index > 0)
                    SizedBox(
                      height: 36,
                      child: VerticalDivider(
                        width: 1,
                        thickness: 1,
                        color: divider,
                      ),
                    ),
                  if (fits)
                    Expanded(child: _StatCell(stat: stats[index]))
                  else
                    SizedBox(
                      width: _cellWidth,
                      child: _StatCell(stat: stats[index]),
                    ),
                ],
              ];
              if (fits) {
                return Row(children: cells);
              }
              return SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(children: cells),
              );
            },
          ),
        ),
      ),
    );
  }
}

class _StatCell extends StatelessWidget {
  const _StatCell({required this.stat});

  final StoreStat stat;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final icon = stat.icon;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Text(
          stat.label,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: theme.textTheme.labelSmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 4),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            if (icon != null) ...<Widget>[
              Icon(icon, size: 15, color: theme.colorScheme.onSurfaceVariant),
              const SizedBox(width: 4),
            ],
            Flexible(
              child: Text(
                stat.value,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.titleSmall?.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

/// Titled block on the detail page.
class StoreDetailSection extends StatelessWidget {
  const StoreDetailSection({
    super.key,
    required this.title,
    required this.child,
    this.count,
    this.trailing,
  });

  final String title;
  final Widget child;
  final int? count;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final count = this.count;
    return Padding(
      padding: const EdgeInsets.only(top: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              if (count != null) ...<Widget>[
                const SizedBox(width: 8),
                Text(
                  '$count',
                  style: theme.textTheme.titleSmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
              const Spacer(),
              ?trailing,
            ],
          ),
          const SizedBox(height: 10),
          child,
        ],
      ),
    );
  }
}

/// Long description that folds to a few lines with a smooth expand.
class StoreExpandableText extends StatefulWidget {
  const StoreExpandableText({super.key, required this.text, this.maxLines = 4});

  final String text;
  final int maxLines;

  @override
  State<StoreExpandableText> createState() => _StoreExpandableTextState();
}

class _StoreExpandableTextState extends State<StoreExpandableText> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final motion = ExtensionMotion.of(context);
    final style = theme.textTheme.bodyMedium?.copyWith(
      height: 1.6,
      color: theme.colorScheme.onSurface.withValues(alpha: 0.88),
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final painter = TextPainter(
          text: TextSpan(text: widget.text, style: style),
          maxLines: widget.maxLines,
          textDirection: Directionality.of(context),
          textScaler: MediaQuery.textScalerOf(context),
        )..layout(maxWidth: constraints.maxWidth);
        final overflows = painter.didExceedMaxLines;
        painter.dispose();
        final text = AnimatedSize(
          duration: motion.emphasized,
          curve: ExtensionMotion.enter,
          alignment: Alignment.topCenter,
          child: SelectableText(
            widget.text,
            style: style,
            maxLines: _expanded || !overflows ? null : widget.maxLines,
          ),
        );
        if (!overflows) {
          return text;
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            text,
            TextButton(
              onPressed: () => setState(() => _expanded = !_expanded),
              style: SettingsControlStyles.sectionTextButton(),
              child: Text(_expanded ? '收起' : '展开全文'),
            ),
          ],
        );
      },
    );
  }
}

/// Settings-style card that stacks rows with hairline separators.
///
/// It keeps the translucent tint but no per-card blur: section switches fade
/// whole panes, and a faded backdrop blur drops out mid-transition.
class StoreGroupedList extends StatelessWidget {
  const StoreGroupedList({super.key, required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: storeCardColor(colorScheme),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: storeCardRadius,
      border: Border.all(color: storeCardBorderColor(colorScheme)),
      material: true,
      enableBackdropFilter: false,
      child: Column(
        children: <Widget>[
          for (var index = 0; index < children.length; index += 1) ...<Widget>[
            if (index > 0)
              Divider(
                height: 1,
                thickness: 1,
                indent: 14,
                endIndent: 14,
                color: colorScheme.outlineVariant.withValues(alpha: 0.4),
              ),
            children[index],
          ],
        ],
      ),
    );
  }
}

/// Tinted square glyph that leads list rows.
class StoreRowGlyph extends StatelessWidget {
  const StoreRowGlyph({super.key, required this.icon, this.color});

  final IconData icon;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final tint = color ?? colorScheme.secondary;
    return Container(
      width: 32,
      height: 32,
      decoration: BoxDecoration(
        color: tint.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Icon(icon, size: 18, color: tint),
    );
  }
}

/// One row inside [StoreGroupedList].
class StoreListRow extends StatelessWidget {
  const StoreListRow({
    super.key,
    required this.title,
    this.subtitle,
    this.leading,
    this.trailing,
    this.footer,
    this.onTap,
    this.monospaceTitle = false,
  });

  final String title;
  final String? subtitle;
  final Widget? leading;
  final Widget? trailing;
  final Widget? footer;
  final VoidCallback? onTap;
  final bool monospaceTitle;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final subtitle = this.subtitle;
    final footer = this.footer;
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 11),
        child: Row(
          children: <Widget>[
            if (leading != null) ...<Widget>[
              leading!,
              const SizedBox(width: 12),
            ],
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    title,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                      fontFamily: monospaceTitle ? 'monospace' : null,
                    ),
                  ),
                  if (subtitle != null && subtitle.trim().isNotEmpty) ...<Widget>[
                    const SizedBox(height: 2),
                    Text(
                      subtitle,
                      maxLines: 3,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                  if (footer != null) ...<Widget>[
                    const SizedBox(height: 8),
                    footer,
                  ],
                ],
              ),
            ),
            if (trailing != null) ...<Widget>[
              const SizedBox(width: 12),
              trailing!,
            ],
          ],
        ),
      ),
    );
  }
}

/// Signature of the control that asks [StoreInlineConfirm] to open.
typedef StoreConfirmTrigger =
    Widget Function(BuildContext context, VoidCallback requestConfirm);

/// Explains a consequential action in place and runs it on the second click.
///
/// Replaces confirmation dialogs: the panel unfolds under its trigger, so the
/// user keeps their place and reads exactly what will happen.
class StoreInlineConfirm extends StatefulWidget {
  const StoreInlineConfirm({
    super.key,
    required this.trigger,
    required this.message,
    required this.confirmLabel,
    required this.onConfirm,
    this.destructive = false,
  });

  final StoreConfirmTrigger trigger;
  final String message;
  final String confirmLabel;
  final Future<void> Function() onConfirm;
  final bool destructive;

  @override
  State<StoreInlineConfirm> createState() => _StoreInlineConfirmState();
}

class _StoreInlineConfirmState extends State<StoreInlineConfirm> {
  bool _open = false;
  bool _running = false;

  Future<void> _confirm() async {
    setState(() => _running = true);
    try {
      await widget.onConfirm();
    } finally {
      if (mounted) {
        setState(() {
          _running = false;
          _open = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final motion = ExtensionMotion.of(context);
    final (tint, onTint) = widget.destructive
        ? (colorScheme.errorContainer, colorScheme.onErrorContainer)
        : (colorScheme.secondaryContainer, colorScheme.onSecondaryContainer);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        widget.trigger(context, () {
          if (!_running) {
            setState(() => _open = true);
          }
        }),
        AnimatedSize(
          duration: motion.standard,
          curve: ExtensionMotion.enter,
          alignment: Alignment.topCenter,
          child: !_open
              ? const SizedBox(width: double.infinity)
              : Padding(
                  padding: const EdgeInsets.fromLTRB(14, 0, 14, 12),
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: tint.withValues(alpha: 0.7),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Padding(
                      padding: const EdgeInsets.fromLTRB(12, 10, 8, 8),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: <Widget>[
                          Text(
                            widget.message,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: onTint,
                              height: 1.45,
                            ),
                          ),
                          const SizedBox(height: 6),
                          Row(
                            mainAxisAlignment: MainAxisAlignment.end,
                            children: <Widget>[
                              TextButton(
                                onPressed: _running
                                    ? null
                                    : () => setState(() => _open = false),
                                style: SettingsControlStyles.sectionTextButton(),
                                child: const Text('取消'),
                              ),
                              const SizedBox(width: 6),
                              FilledButton(
                                onPressed: _running ? null : _confirm,
                                style: SettingsControlStyles.sectionFilledButton()
                                    .copyWith(
                                      backgroundColor: widget.destructive
                                          ? WidgetStatePropertyAll<Color>(
                                              colorScheme.error,
                                            )
                                          : null,
                                      foregroundColor: widget.destructive
                                          ? WidgetStatePropertyAll<Color>(
                                              colorScheme.onError,
                                            )
                                          : null,
                                    ),
                                child: _running
                                    ? SizedBox.square(
                                        dimension: 16,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                          color: widget.destructive
                                              ? colorScheme.onError
                                              : colorScheme.onPrimary,
                                        ),
                                      )
                                    : Text(widget.confirmLabel),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
        ),
      ],
    );
  }
}

class StoreInfoItem {
  const StoreInfoItem(this.label, this.value);

  final String label;
  final String value;
}

/// Label/value facts; rows without a value are left out.
class StoreInfoTable extends StatelessWidget {
  const StoreInfoTable({super.key, required this.items});

  final List<StoreInfoItem> items;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final rows = items.where((item) => item.value.trim().isNotEmpty);
    return StoreGroupedList(
      children: <Widget>[
        for (final item in rows)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                SizedBox(
                  width: 92,
                  child: Text(
                    item.label,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                Expanded(
                  child: SelectableText(
                    item.value,
                    style: theme.textTheme.bodySmall,
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

enum StoreNoticeTone { info, warning, error }

/// Tinted callout for problems that need the user's attention.
class StoreNoticeCard extends StatelessWidget {
  const StoreNoticeCard({
    super.key,
    required this.icon,
    required this.title,
    this.message,
    this.tone = StoreNoticeTone.warning,
    this.action,
  });

  final IconData icon;
  final String title;
  final String? message;
  final StoreNoticeTone tone;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final (background, foreground) = switch (tone) {
      StoreNoticeTone.info => (
        colorScheme.secondaryContainer,
        colorScheme.onSecondaryContainer,
      ),
      StoreNoticeTone.warning => (
        colorScheme.tertiaryContainer,
        colorScheme.onTertiaryContainer,
      ),
      StoreNoticeTone.error => (
        colorScheme.errorContainer,
        colorScheme.onErrorContainer,
      ),
    };
    final message = this.message;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: background.withValues(alpha: 0.72),
        borderRadius: storeCardRadius,
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(14, 12, 10, 12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(icon, color: foreground, size: 20),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    title,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: foreground,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  if (message != null && message.trim().isNotEmpty) ...<Widget>[
                    const SizedBox(height: 4),
                    SelectableText(
                      message,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: foreground.withValues(alpha: 0.86),
                      ),
                    ),
                  ],
                ],
              ),
            ),
            if (action != null) ...<Widget>[
              const SizedBox(width: 8),
              action!,
            ],
          ],
        ),
      ),
    );
  }
}

/// Circular avatar from a URL, or the person's initial when they have none.
class StoreAvatar extends StatelessWidget {
  const StoreAvatar({super.key, required this.name, this.url, this.radius = 14});

  final String name;
  final String? url;
  final double radius;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final url = this.url;
    if (url != null && url.trim().isNotEmpty) {
      return CircleAvatar(radius: radius, backgroundImage: NetworkImage(url));
    }
    final trimmed = name.trim();
    return CircleAvatar(
      radius: radius,
      backgroundColor: colorScheme.primaryContainer,
      foregroundColor: colorScheme.onPrimaryContainer,
      child: Text(
        trimmed.isEmpty ? '?' : trimmed.characters.first.toUpperCase(),
        style: TextStyle(fontSize: radius * 0.9, fontWeight: FontWeight.w700),
      ),
    );
  }
}

/// One market comment with author, date and the owner's edit/delete menu.
class StoreCommentTile extends StatelessWidget {
  const StoreCommentTile({
    super.key,
    required this.comment,
    this.onEdit,
    this.onDelete,
  });

  final core_proxy.MarketComment comment;
  final VoidCallback? onEdit;
  final VoidCallback? onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final onEdit = this.onEdit;
    final onDelete = this.onDelete;
    return Padding(
      padding: const EdgeInsets.fromLTRB(14, 12, 6, 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          StoreAvatar(
            name: comment.author.login,
            url: comment.author.avatar,
            radius: 15,
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Row(
                  children: <Widget>[
                    Flexible(
                      child: Text(
                        comment.author.login,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text(
                      formatMarketDate(comment.createdAt),
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                SelectableText(
                  comment.body,
                  style: theme.textTheme.bodyMedium?.copyWith(height: 1.5),
                ),
              ],
            ),
          ),
          if (onEdit != null || onDelete != null)
            PopupMenuButton<String>(
              tooltip: '评论操作',
              icon: const Icon(Icons.more_horiz_rounded),
              onSelected: (value) {
                if (value == 'edit') {
                  onEdit?.call();
                } else if (value == 'delete') {
                  onDelete?.call();
                }
              },
              itemBuilder: (context) => <PopupMenuEntry<String>>[
                if (onEdit != null)
                  const PopupMenuItem<String>(
                    value: 'edit',
                    child: Text('编辑评论'),
                  ),
                if (onDelete != null)
                  const PopupMenuItem<String>(
                    value: 'delete',
                    child: Text('删除评论'),
                  ),
              ],
            ),
        ],
      ),
    );
  }
}
