// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';

List<_MarketGridGroup<T>> _marketGridGroups<T>({
  required List<T> items,
  required bool groupByUpdatedDate,
  required String Function(T item) updatedAt,
}) {
  if (!groupByUpdatedDate) {
    return <_MarketGridGroup<T>>[
      _MarketGridGroup<T>(label: null, items: items),
    ];
  }
  final groups = <_MarketGridGroup<T>>[];
  String? currentLabel;
  var currentItems = <T>[];
  for (final item in items) {
    final label = _marketUpdatedDateLabel(updatedAt(item));
    if (label != currentLabel) {
      if (currentLabel != null) {
        groups.add(
          _MarketGridGroup<T>(label: currentLabel, items: currentItems),
        );
      }
      currentLabel = label;
      currentItems = <T>[];
    }
    currentItems.add(item);
  }
  if (currentLabel != null) {
    groups.add(_MarketGridGroup<T>(label: currentLabel, items: currentItems));
  }
  return groups;
}

class _MarketGridGroup<T> {
  const _MarketGridGroup({required this.label, required this.items});

  final String? label;
  final List<T> items;
}

String _marketUpdatedDateLabel(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return '更早';
  }
  return trimmed.length >= 10 ? trimmed.substring(0, 10) : trimmed;
}

class MarketBrowseList<T> extends StatelessWidget {
  const MarketBrowseList({
    super.key,
    required this.isLoading,
    required this.isLoadingMore,
    required this.hasMore,
    required this.isEmpty,
    required this.emptyTitle,
    required this.onRefresh,
    required this.onLoadMore,
    required this.items,
    required this.groupByUpdatedDate,
    required this.updatedAt,
    required this.itemBuilder,
  });

  final bool isLoading;
  final bool isLoadingMore;
  final bool hasMore;
  final bool isEmpty;
  final String emptyTitle;
  final AsyncCallback onRefresh;
  final VoidCallback onLoadMore;
  final List<T> items;
  final bool groupByUpdatedDate;
  final String Function(T item) updatedAt;
  final Widget Function(T item) itemBuilder;

  @override
  Widget build(BuildContext context) {
    final groups = _marketGridGroups<T>(
      items: items,
      groupByUpdatedDate: groupByUpdatedDate,
      updatedAt: updatedAt,
    );
    return NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        if (notification.metrics.extentAfter < 360 &&
            hasMore &&
            !isLoadingMore) {
          onLoadMore();
        }
        return false;
      },
      child: Stack(
        children: <Widget>[
          RefreshIndicator(
            onRefresh: onRefresh,
            child: CustomScrollView(
              physics: const AlwaysScrollableScrollPhysics(),
              slivers: <Widget>[
                if (isEmpty)
                  SliverPadding(
                    padding: const EdgeInsets.fromLTRB(12, 4, 12, 120),
                    sliver: SliverFillRemaining(
                      hasScrollBody: false,
                      child: EmptyState(
                        icon: Icons.store_outlined,
                        title: emptyTitle,
                        message: '刷新或调整关键词后重试。',
                        scrollable: false,
                      ),
                    ),
                  )
                else
                  for (var index = 0; index < groups.length; index += 1) ...[
                    if (groups[index].label != null)
                      SliverPadding(
                        padding: const EdgeInsets.symmetric(horizontal: 12),
                        sliver: SliverToBoxAdapter(
                          child: _MarketDateHeader(text: groups[index].label!),
                        ),
                      ),
                    SliverPadding(
                      padding: EdgeInsets.fromLTRB(
                        12,
                        groups[index].label == null && index == 0 ? 4 : 0,
                        12,
                        0,
                      ),
                      sliver: _MarketGridSliver<T>(
                        items: groups[index].items,
                        itemBuilder: itemBuilder,
                      ),
                    ),
                  ],
                if (isLoadingMore)
                  const SliverToBoxAdapter(
                    child: Padding(
                      padding: EdgeInsets.symmetric(vertical: 18),
                      child: M3LoadingFooter(),
                    ),
                  ),
                const SliverToBoxAdapter(child: SizedBox(height: 120)),
              ],
            ),
          ),
          if (isLoading && !isEmpty)
            const Positioned.fill(child: M3LoadingOverlay()),
        ],
      ),
    );
  }
}

class _MarketGridSliver<T> extends StatelessWidget {
  const _MarketGridSliver({required this.items, required this.itemBuilder});

  final List<T> items;
  final Widget Function(T item) itemBuilder;

  @override
  Widget build(BuildContext context) {
    return SliverLayoutBuilder(
      builder: (context, constraints) {
        final columnCount = constraints.crossAxisExtent >= 1280
            ? 3
            : constraints.crossAxisExtent >= 760
            ? 2
            : 1;
        return SliverGrid.builder(
          itemCount: items.length,
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: columnCount,
            crossAxisSpacing: 10,
            mainAxisSpacing: 8,
            mainAxisExtent: 104,
          ),
          itemBuilder: (context, index) => itemBuilder(items[index]),
        );
      },
    );
  }
}

class MarketGridCard extends StatelessWidget {
  const MarketGridCard({
    super.key,
    required this.title,
    required this.description,
    required this.author,
    required this.downloads,
    required this.likes,
    required this.hearts,
    required this.actionLabel,
    required this.actionIcon,
    required this.actionBusy,
    required this.onAction,
    required this.onTap,
  });

  final String title;
  final String description;
  final String author;
  final int downloads;
  final int likes;
  final int hearts;
  final String actionLabel;
  final IconData actionIcon;
  final bool actionBusy;
  final VoidCallback onAction;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final borderRadius = BorderRadius.circular(16);
    return OperitGlassSurface(
      color: colorScheme.surface,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: borderRadius,
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: InkWell(
        borderRadius: borderRadius,
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: <Widget>[
              _MarketCardLeadingIcon(title: title),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    if (description.trim().isNotEmpty) ...<Widget>[
                      const SizedBox(height: 4),
                      Text(
                        _truncateMarketBrowseDescription(description),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                    const SizedBox(height: 4),
                    _MarketCardMetaRow(
                      author: author,
                      downloads: downloads,
                      likes: likes,
                      hearts: hearts,
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 10),
              Tooltip(
                message: actionLabel,
                child: Material(
                  color: actionBusy
                      ? colorScheme.primaryContainer
                      : colorScheme.primary,
                  shape: const CircleBorder(),
                  child: InkWell(
                    customBorder: const CircleBorder(),
                    onTap: actionBusy ? null : onAction,
                    child: SizedBox.square(
                      dimension: 30,
                      child: Center(
                        child: actionBusy
                            ? M3LoadingIndicator(
                                size: 16,
                                color: colorScheme.onPrimaryContainer,
                              )
                            : Icon(
                                actionIcon,
                                size: 17,
                                color: colorScheme.onPrimary,
                              ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MarketCardLeadingIcon extends StatelessWidget {
  const _MarketCardLeadingIcon({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final initial = title.trim().isEmpty ? '?' : title.trim()[0].toUpperCase();
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(14),
      ),
      child: SizedBox.square(
        dimension: 42,
        child: Center(
          child: Text(
            initial,
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.w700,
              color: colorScheme.onPrimaryContainer,
            ),
          ),
        ),
      ),
    );
  }
}

class _MarketCardMetaRow extends StatelessWidget {
  const _MarketCardMetaRow({
    required this.author,
    required this.downloads,
    required this.likes,
    required this.hearts,
  });

  final String author;
  final int downloads;
  final int likes;
  final int hearts;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Row(
      children: <Widget>[
        if (author.trim().isNotEmpty)
          Expanded(
            child: Text(
              author,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.labelMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          )
        else
          const Spacer(),
        _MarketCardMetaCount(
          icon: Icons.download,
          text: downloads.toString(),
          color: colorScheme.primary,
        ),
        if (likes > 0) ...<Widget>[
          const SizedBox(width: 8),
          _MarketCardMetaCount(
            icon: Icons.thumb_up,
            text: likes.toString(),
            color: colorScheme.primary,
          ),
        ],
        if (hearts > 0) ...<Widget>[
          const SizedBox(width: 8),
          _MarketCardMetaCount(
            icon: Icons.favorite,
            text: hearts.toString(),
            color: const Color(0xFFE91E63),
          ),
        ],
      ],
    );
  }
}

class _MarketCardMetaCount extends StatelessWidget {
  const _MarketCardMetaCount({
    required this.icon,
    required this.text,
    required this.color,
  });

  final IconData icon;
  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(icon, size: 12, color: color),
        const SizedBox(width: 4),
        Text(
          text,
          style: Theme.of(context).textTheme.labelSmall?.copyWith(color: color),
        ),
      ],
    );
  }
}

String _truncateMarketBrowseDescription(String description) {
  if (description.length > 100) {
    return '${description.substring(0, 100)}...';
  }
  return description;
}

class _MarketDateHeader extends StatelessWidget {
  const _MarketDateHeader({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 10, 4, 2),
      child: Text(
        text,
        style: Theme.of(context).textTheme.labelLarge?.copyWith(
          fontWeight: FontWeight.w600,
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}
