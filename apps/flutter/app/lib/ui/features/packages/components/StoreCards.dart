// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../market/MarketInstallStateStore.dart';
import 'ExtensionAvatar.dart';
import 'ExtensionNavigation.dart';
import 'StoreVisuals.dart';

/// "获取" pill bound to the shared marketplace installation state.
class StoreMarketGetButton extends StatelessWidget {
  const StoreMarketGetButton({
    super.key,
    required this.entry,
    required this.installState,
    this.prominent = false,
  });

  final core_proxy.MarketEntrySummary entry;
  final MarketInstallStateStore installState;
  final bool prominent;

  Future<void> _install(BuildContext context) async {
    final messenger = ScaffoldMessenger.of(context);
    try {
      await installState.install(entry);
    } catch (error, stackTrace) {
      debugPrint('Failed to install market entry: $error\n$stackTrace');
      messenger.showSnackBar(
        SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = installState.stateFor(entry);
    final update = state == MarketLocalInstallState.updateAvailable;
    final installed = state == MarketLocalInstallState.installed;
    return StoreGetButton(
      label: update ? '更新' : '获取',
      busy: installState.isInstalling(entry.id),
      done: installed,
      prominent: prominent || update,
      onPressed: installed ? null : () => _install(context),
    );
  }
}

class _MarketStats extends StatelessWidget {
  const _MarketStats({required this.entry, this.color});

  final core_proxy.MarketEntrySummary entry;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final foreground = color ?? theme.colorScheme.onSurfaceVariant;
    final style = theme.textTheme.labelSmall?.copyWith(color: foreground);
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(Icons.download_rounded, size: 14, color: foreground),
        const SizedBox(width: 3),
        Text(storeCompactCount(marketEntryDownloads(entry)), style: style),
        const SizedBox(width: 10),
        Icon(Icons.favorite_rounded, size: 13, color: foreground),
        const SizedBox(width: 3),
        Text(storeCompactCount(marketEntryLikes(entry)), style: style),
      ],
    );
  }
}

/// Card for horizontal shelves: avatar, name, publisher, summary and stats.
class StoreMarketCard extends StatelessWidget {
  const StoreMarketCard({
    super.key,
    required this.entry,
    required this.installState,
  });

  final core_proxy.MarketEntrySummary entry;
  final MarketInstallStateStore installState;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final kind = StoreKind.of(entry.type);
    final publisher = marketEntryPublisher(entry);
    return StorePressable(
      onTap: () => openMarketEntryDetail(entry),
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              ExtensionAvatar(
                seed: marketEntryAvatarSeed(entry),
                label: entry.title,
                size: 44,
              ),
              const Spacer(),
              StoreBadge(label: kind.label, icon: kind.icon),
            ],
          ),
          const SizedBox(height: 12),
          Text(
            entry.title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            publisher.isEmpty ? '社区市场' : publisher,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 6),
          Expanded(
            child: Text(
              entry.description,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
                height: 1.45,
              ),
            ),
          ),
          Row(
            children: <Widget>[
              Expanded(child: _MarketStats(entry: entry)),
              StoreMarketGetButton(entry: entry, installState: installState),
            ],
          ),
        ],
      ),
    );
  }
}

/// Compact row used by ranked lists, search results and full listings.
class StoreAppRow extends StatelessWidget {
  const StoreAppRow({
    super.key,
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.action,
    required this.onTap,
    this.rank,
    this.footer,
  });

  final Widget icon;
  final String title;
  final String subtitle;
  final Widget action;
  final VoidCallback onTap;
  final int? rank;
  final Widget? footer;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final rank = this.rank;
    final footer = this.footer;
    return Material(
      type: MaterialType.transparency,
      child: InkWell(
        onTap: onTap,
        borderRadius: storeCardRadius,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
          child: Row(
            children: <Widget>[
              if (rank != null) ...<Widget>[
                SizedBox(
                  width: 24,
                  child: Text(
                    '$rank',
                    textAlign: TextAlign.center,
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                      color: rank <= 3
                          ? theme.colorScheme.primary
                          : theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                const SizedBox(width: 6),
              ],
              icon,
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      subtitle,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                    if (footer != null) ...<Widget>[
                      const SizedBox(height: 4),
                      footer,
                    ],
                  ],
                ),
              ),
              const SizedBox(width: 12),
              action,
            ],
          ),
        ),
      ),
    );
  }
}

/// [StoreAppRow] for a marketplace entry.
class StoreMarketRow extends StatelessWidget {
  const StoreMarketRow({
    super.key,
    required this.entry,
    required this.installState,
    this.rank,
  });

  final core_proxy.MarketEntrySummary entry;
  final MarketInstallStateStore installState;
  final int? rank;

  @override
  Widget build(BuildContext context) {
    final kind = StoreKind.of(entry.type);
    final publisher = marketEntryPublisher(entry);
    return StoreAppRow(
      rank: rank,
      icon: ExtensionAvatar(
        seed: marketEntryAvatarSeed(entry),
        label: entry.title,
        kind: kind,
        size: 44,
      ),
      title: entry.title,
      subtitle: <String>[
        kind.label,
        if (publisher.isNotEmpty) publisher,
      ].join(' · '),
      footer: _MarketStats(entry: entry),
      action: StoreMarketGetButton(entry: entry, installState: installState),
      onTap: () => openMarketEntryDetail(entry),
    );
  }
}

/// Featured entry banner in the entry's own tonal colors, its monogram as art.
class StoreFeaturedBanner extends StatelessWidget {
  const StoreFeaturedBanner({
    super.key,
    required this.entry,
    required this.installState,
  });

  final core_proxy.MarketEntrySummary entry;
  final MarketInstallStateStore installState;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dark = theme.brightness == Brightness.dark;
    final kind = StoreKind.of(entry.type);
    final seed = marketEntryAvatarSeed(entry);
    final palette = ExtensionAvatarPalette.of(context, seed);
    final publisher = marketEntryPublisher(entry);
    final radius = BorderRadius.circular(16);
    final ink = palette.onContainer;
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: radius,
        border: Border.all(color: ink.withValues(alpha: dark ? 0.12 : 0.08)),
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: <Color>[
            Color.lerp(palette.container, Colors.white, dark ? 0.05 : 0.3)!,
            palette.container,
          ],
        ),
      ),
      child: ClipRRect(
        borderRadius: radius,
        child: Material(
          type: MaterialType.transparency,
          child: InkWell(
            onTap: () => openMarketEntryDetail(entry),
            child: Stack(
              children: <Widget>[
                Positioned(
                  right: -12,
                  bottom: -56,
                  child: Text(
                    extensionMonogram(entry.title),
                    maxLines: 1,
                    softWrap: false,
                    textScaler: TextScaler.noScaling,
                    style: TextStyle(
                      fontSize: 200,
                      height: 1,
                      fontWeight: FontWeight.w800,
                      color: ink.withValues(alpha: dark ? 0.08 : 0.07),
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(22, 20, 22, 18),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        '${entry.featured ? '精选' : '推荐'} · ${kind.label}',
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: ink.withValues(alpha: 0.72),
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 6),
                      Text(
                        entry.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.titleLarge?.copyWith(
                          color: ink,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 6),
                      Text(
                        entry.description,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: ink.withValues(alpha: 0.8),
                          height: 1.45,
                        ),
                      ),
                      const Spacer(),
                      Row(
                        children: <Widget>[
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              mainAxisSize: MainAxisSize.min,
                              children: <Widget>[
                                Text(
                                  publisher.isEmpty ? '社区市场' : publisher,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: theme.textTheme.labelLarge?.copyWith(
                                    color: ink,
                                    fontWeight: FontWeight.w600,
                                  ),
                                ),
                                const SizedBox(height: 2),
                                _MarketStats(
                                  entry: entry,
                                  color: ink.withValues(alpha: 0.7),
                                ),
                              ],
                            ),
                          ),
                          StoreMarketGetButton(
                            entry: entry,
                            installState: installState,
                            prominent: true,
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
