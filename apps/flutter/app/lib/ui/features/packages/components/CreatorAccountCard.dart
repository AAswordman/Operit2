// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../settings/components/SettingsControlStyles.dart';
import 'ExtensionMotion.dart';
import 'StoreDetailKit.dart';
import 'StoreVisuals.dart';

/// One headline figure of the creator's account card.
class CreatorStat {
  const CreatorStat({
    required this.label,
    required this.value,
    this.attention = false,
  });

  final String label;

  /// Null while the figure is not known yet.
  final int? value;

  /// Draws the figure in the error color while it is above zero.
  final bool attention;
}

/// Who is publishing: the GitHub profile with the numbers that matter, or the
/// invitation to sign in.
class CreatorAccountCard extends StatelessWidget {
  const CreatorAccountCard({
    super.key,
    required this.loading,
    required this.user,
    required this.stats,
    required this.onLogin,
    required this.onLogout,
  });

  /// The saved login has not been read yet.
  final bool loading;
  final core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? user;
  final List<CreatorStat> stats;
  final VoidCallback onLogin;
  final Future<void> Function() onLogout;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final user = this.user;
    if (loading) {
      return DecoratedBox(
        decoration: storeHeroDecoration(theme),
        child: const Padding(
          padding: EdgeInsets.all(18),
          child: StoreShimmer(
            child: Row(
              children: <Widget>[
                StoreSkeletonBox(width: 64, height: 64, radius: 32),
                SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      StoreSkeletonBox(width: 150, height: 16),
                      SizedBox(height: 10),
                      StoreSkeletonBox(width: 96, height: 12),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      );
    }
    if (user == null) {
      return _SignedOut(onLogin: onLogin);
    }
    return DecoratedBox(
      decoration: storeHeroDecoration(theme),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          StoreInlineConfirm(
            trigger: (context, requestConfirm) => Padding(
              padding: const EdgeInsets.fromLTRB(18, 18, 14, 14),
              child: _Identity(user: user, onLogout: requestConfirm),
            ),
            message: '退出后将无法发布或管理市场内容。已安装的扩展不受影响，随时可以重新登录。',
            confirmLabel: '退出登录',
            onConfirm: onLogout,
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(18, 0, 18, 18),
            child: Row(
              children: <Widget>[
                for (var index = 0; index < stats.length; index += 1) ...<Widget>[
                  if (index > 0) const SizedBox(width: 8),
                  Expanded(child: _StatTile(stat: stats[index])),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _Identity extends StatelessWidget {
  const _Identity({required this.user, required this.onLogout});

  final core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser user;
  final VoidCallback onLogout;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final name = user.name?.trim();
    final displayName = name == null || name.isEmpty ? user.login : name;
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 420;
        return Row(
          children: <Widget>[
            _AccountAvatar(user: user, displayName: displayName),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    displayName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: <Widget>[
                      Text(
                        '@${user.login}',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                      StoreBadge(
                        label: 'GitHub 已连接',
                        icon: Icons.verified_user_outlined,
                        color: colorScheme.primaryContainer.withValues(
                          alpha: 0.7,
                        ),
                        foreground: colorScheme.onPrimaryContainer,
                      ),
                    ],
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            if (compact)
              IconButton(
                tooltip: '退出登录',
                onPressed: onLogout,
                icon: const Icon(Icons.logout_rounded),
              )
            else
              OutlinedButton.icon(
                onPressed: onLogout,
                style: OutlinedButton.styleFrom(
                  visualDensity: VisualDensity.compact,
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  minimumSize: const Size(0, 36),
                  padding: const EdgeInsets.symmetric(horizontal: 14),
                ),
                icon: const Icon(Icons.logout_rounded, size: 18),
                label: const Text('退出'),
              ),
          ],
        );
      },
    );
  }
}

/// Profile picture inside a gradient ring, with the connected mark.
class _AccountAvatar extends StatelessWidget {
  const _AccountAvatar({required this.user, required this.displayName});

  final core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser user;
  final String displayName;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox.square(
      dimension: 64,
      child: Stack(
        clipBehavior: Clip.none,
        children: <Widget>[
          Positioned.fill(
            child: DecoratedBox(
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                gradient: SweepGradient(
                  colors: <Color>[
                    colorScheme.primary,
                    colorScheme.tertiary,
                    colorScheme.secondary,
                    colorScheme.primary,
                  ],
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.all(2.5),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: colorScheme.surface,
                  ),
                  child: Padding(
                    padding: const EdgeInsets.all(2),
                    child: StoreAvatar(
                      name: displayName,
                      url: user.avatarUrl,
                      radius: 27,
                    ),
                  ),
                ),
              ),
            ),
          ),
          Positioned(
            right: -2,
            bottom: -2,
            child: DecoratedBox(
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: colorScheme.primary,
                border: Border.all(color: colorScheme.surface, width: 2),
              ),
              child: SizedBox.square(
                dimension: 20,
                child: Icon(
                  Icons.check_rounded,
                  size: 12,
                  color: colorScheme.onPrimary,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Figure with its label; counts up when it first appears.
class _StatTile extends StatelessWidget {
  const _StatTile({required this.stat});

  final CreatorStat stat;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final dark = theme.brightness == Brightness.dark;
    final motion = ExtensionMotion.of(context);
    final value = stat.value;
    final alert = stat.attention && value != null && value > 0;
    final valueColor = alert
        ? colorScheme.error
        : value == null || value == 0
        ? colorScheme.onSurfaceVariant
        : colorScheme.onSurface;
    final valueStyle = theme.textTheme.titleLarge?.copyWith(
      fontWeight: FontWeight.w700,
      color: valueColor,
    );
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surface.withValues(alpha: dark ? 0.34 : 0.62),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: alert
              ? colorScheme.error.withValues(alpha: 0.4)
              : colorScheme.outlineVariant.withValues(alpha: 0.3),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 10),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            if (value == null)
              Text('–', style: valueStyle)
            else
              TweenAnimationBuilder<double>(
                tween: Tween<double>(begin: 0, end: value.toDouble()),
                duration: motion.emphasized,
                curve: ExtensionMotion.enter,
                builder: (context, shown, child) =>
                    Text('${shown.round()}', style: valueStyle),
              ),
            const SizedBox(height: 2),
            Text(
              stat.label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.labelSmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SignedOut extends StatelessWidget {
  const _SignedOut({required this.onLogin});

  final VoidCallback onLogin;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final glyph = DecoratedBox(
      decoration: ShapeDecoration(
        shape: RoundedSuperellipseBorder(
          borderRadius: BorderRadius.circular(18),
        ),
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: <Color>[
            colorScheme.primaryContainer,
            colorScheme.secondaryContainer,
          ],
        ),
      ),
      child: SizedBox.square(
        dimension: 56,
        child: Icon(
          Icons.account_circle_outlined,
          size: 30,
          color: colorScheme.onPrimaryContainer,
        ),
      ),
    );
    final text = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text(
          '登录 GitHub，开始创作',
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w700,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          '发布插件、技能和 MCP，跟踪审核进度，接收评论与审核通知。',
          style: theme.textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
            height: 1.45,
          ),
        ),
      ],
    );
    final button = FilledButton.icon(
      onPressed: onLogin,
      style: SettingsControlStyles.sectionFilledButton(),
      icon: const Icon(Icons.login_rounded, size: 18),
      label: const Text('登录 GitHub'),
    );
    return DecoratedBox(
      decoration: storeHeroDecoration(theme),
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: LayoutBuilder(
          builder: (context, constraints) {
            if (constraints.maxWidth >= 560) {
              return Row(
                children: <Widget>[
                  glyph,
                  const SizedBox(width: 16),
                  Expanded(child: text),
                  const SizedBox(width: 16),
                  button,
                ],
              );
            }
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    glyph,
                    const SizedBox(width: 14),
                    Expanded(child: text),
                  ],
                ),
                const SizedBox(height: 16),
                button,
              ],
            );
          },
        ),
      ),
    );
  }
}
