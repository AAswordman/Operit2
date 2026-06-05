// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../main/layout/NavigationLayoutMetrics.dart';
import '../../../theme/OperitGlassSurface.dart';
import 'ArtifactMarketSupport.dart';

const double _desktopDetailMaxWidth = 860;

class UnifiedMarketDetailHeader {
  const UnifiedMarketDetailHeader({
    required this.title,
    required this.fallbackAvatarText,
    this.participants = const <UnifiedMarketDetailParticipant>[],
    this.badges = const <String>[],
    this.metrics = const <UnifiedMarketDetailMetric>[],
  });

  final String title;
  final String fallbackAvatarText;
  final List<UnifiedMarketDetailParticipant> participants;
  final List<String> badges;
  final List<UnifiedMarketDetailMetric> metrics;
}

class UnifiedMarketDetailParticipant {
  const UnifiedMarketDetailParticipant({
    required this.roleLabel,
    required this.name,
    this.avatarUrl,
    required this.fallbackAvatarText,
  });

  final String roleLabel;
  final String name;
  final String? avatarUrl;
  final String fallbackAvatarText;
}

class UnifiedMarketDetailMetric {
  const UnifiedMarketDetailMetric({required this.value, required this.label});

  final String value;
  final String label;
}

class UnifiedMarketDetailAction {
  const UnifiedMarketDetailAction({
    required this.label,
    required this.onPressed,
    this.enabled = true,
    this.isLoading = false,
    this.icon,
  });

  final String label;
  final VoidCallback onPressed;
  final bool enabled;
  final bool isLoading;
  final IconData? icon;
}

class UnifiedMarketDetailCommentsState {
  const UnifiedMarketDetailCommentsState({
    required this.title,
    required this.commentCount,
    required this.isLoading,
    required this.errorMessage,
    required this.onRetry,
    required this.reactions,
    required this.comments,
    required this.canPost,
    required this.isPosting,
    required this.onRequestPost,
    this.postHint,
  });

  final String title;
  final int commentCount;
  final bool isLoading;
  final String? errorMessage;
  final VoidCallback onRetry;
  final List<Widget> reactions;
  final List<core_proxy.GitHubComment> comments;
  final bool canPost;
  final bool isPosting;
  final VoidCallback onRequestPost;
  final String? postHint;
}

class UnifiedMarketDetailScreen extends StatefulWidget {
  const UnifiedMarketDetailScreen({
    super.key,
    required this.title,
    required this.header,
    required this.overviewChildren,
    required this.comments,
    required this.primaryAction,
    this.secondaryAction,
  });

  final String title;
  final UnifiedMarketDetailHeader header;
  final List<Widget> overviewChildren;
  final UnifiedMarketDetailCommentsState comments;
  final UnifiedMarketDetailAction primaryAction;
  final UnifiedMarketDetailAction? secondaryAction;

  @override
  State<UnifiedMarketDetailScreen> createState() =>
      _UnifiedMarketDetailScreenState();
}

class _UnifiedMarketDetailScreenState extends State<UnifiedMarketDetailScreen> {
  int _selectedTabIndex = 0;

  @override
  Widget build(BuildContext context) {
    final useWideLayout = useTabletLayoutForContext(context);
    return Scaffold(
      backgroundColor: Colors.transparent,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: Text(widget.title, maxLines: 1, overflow: TextOverflow.ellipsis),
      ),
      body: Column(
        children: <Widget>[
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.fromLTRB(20, 16, 20, 20),
              child: _MarketDetailWidthLimiter(
                enabled: useWideLayout,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    UnifiedMarketDetailHeaderCard(header: widget.header),
                    const SizedBox(height: 18),
                    UnifiedMarketDetailTabs(
                      selectedIndex: _selectedTabIndex,
                      commentCount: widget.comments.commentCount,
                      onSelected: (index) {
                        setState(() {
                          _selectedTabIndex = index;
                        });
                      },
                    ),
                    const SizedBox(height: 18),
                    if (_selectedTabIndex == 0)
                      ...widget.overviewChildren
                    else
                      UnifiedMarketDetailCommentsSection(
                        state: widget.comments,
                      ),
                  ],
                ),
              ),
            ),
          ),
          OperitGlassSurface(
            color: Theme.of(context).colorScheme.surface,
            layer: OperitGlassSurfaceLayer.panel,
            transparentAlpha: 0.055,
            clip: false,
            material: true,
            child: SafeArea(
              top: false,
              child: Padding(
                padding: const EdgeInsets.fromLTRB(20, 12, 20, 12),
                child: _MarketDetailWidthLimiter(
                  enabled: useWideLayout,
                  child: UnifiedMarketDetailActionRow(
                    primaryAction: widget.primaryAction,
                    secondaryAction: widget.secondaryAction,
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _MarketDetailWidthLimiter extends StatelessWidget {
  const _MarketDetailWidthLimiter({required this.enabled, required this.child});

  final bool enabled;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!enabled) {
      return child;
    }
    return Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: _desktopDetailMaxWidth),
        child: SizedBox(width: double.infinity, child: child),
      ),
    );
  }
}

class UnifiedMarketDetailHeaderCard extends StatelessWidget {
  const UnifiedMarketDetailHeaderCard({super.key, required this.header});

  final UnifiedMarketDetailHeader header;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final participants = header.participants.take(2).toList(growable: false);
    final metrics = header.metrics.take(4).toList(growable: false);
    return OperitGlassSurface(
      color: colorScheme.surface,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(20),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                _ArtifactDetailLargeIcon(title: header.fallbackAvatarText),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        header.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.titleLarge?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 8),
                      Wrap(
                        spacing: 8,
                        runSpacing: 6,
                        children: <Widget>[
                          for (final badge in header.badges.where(
                            (item) => item.trim().isNotEmpty,
                          ))
                            _ArtifactDetailBadge(text: badge),
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            ),
            if (participants.isNotEmpty) ...<Widget>[
              const SizedBox(height: 14),
              Row(
                children: <Widget>[
                  for (
                    var index = 0;
                    index < participants.length;
                    index += 1
                  ) ...[
                    if (index > 0) const SizedBox(width: 16),
                    Expanded(
                      child: _ArtifactDetailPerson(
                        label: participants[index].roleLabel,
                        name: participants[index].name,
                        avatarUrl: participants[index].avatarUrl,
                        fallbackAvatarText:
                            participants[index].fallbackAvatarText,
                      ),
                    ),
                  ],
                ],
              ),
            ],
            if (metrics.isNotEmpty) ...<Widget>[
              const SizedBox(height: 14),
              Row(
                children: <Widget>[
                  for (var index = 0; index < metrics.length; index += 1) ...[
                    if (index > 0)
                      _ArtifactDetailMetricDivider(
                        color: colorScheme.outlineVariant,
                      ),
                    Expanded(
                      child: _ArtifactDetailMetric(
                        value: metrics[index].value,
                        label: metrics[index].label,
                      ),
                    ),
                  ],
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }
}

String marketDetailInitial(String value) {
  final trimmed = value.trim();
  return trimmed.isEmpty ? '?' : trimmed[0].toUpperCase();
}

class _ArtifactDetailLargeIcon extends StatelessWidget {
  const _ArtifactDetailLargeIcon({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final initial = marketDetailInitial(title);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.primaryContainer.withValues(alpha: 0.85),
        borderRadius: BorderRadius.circular(20),
      ),
      child: SizedBox.square(
        dimension: 76,
        child: Center(
          child: Text(
            initial,
            style: Theme.of(context).textTheme.titleLarge?.copyWith(
              fontWeight: FontWeight.w700,
              color: colorScheme.onPrimaryContainer,
            ),
          ),
        ),
      ),
    );
  }
}

class _ArtifactDetailBadge extends StatelessWidget {
  const _ArtifactDetailBadge({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.35),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 3),
        child: Text(
          text,
          style: Theme.of(
            context,
          ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ),
    );
  }
}

class _ArtifactDetailPerson extends StatelessWidget {
  const _ArtifactDetailPerson({
    required this.label,
    required this.name,
    required this.avatarUrl,
    required this.fallbackAvatarText,
  });

  final String label;
  final String name;
  final String? avatarUrl;
  final String fallbackAvatarText;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final initial = marketDetailInitial(fallbackAvatarText);
    return Row(
      children: <Widget>[
        if (avatarUrl != null && avatarUrl!.trim().isNotEmpty)
          CircleAvatar(radius: 14, backgroundImage: NetworkImage(avatarUrl!))
        else
          CircleAvatar(
            radius: 14,
            backgroundColor: colorScheme.primaryContainer,
            foregroundColor: colorScheme.onPrimaryContainer,
            child: Text(initial, style: Theme.of(context).textTheme.labelLarge),
          ),
        const SizedBox(width: 8),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              Text(
                name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _ArtifactDetailMetric extends StatelessWidget {
  const _ArtifactDetailMetric({required this.value, required this.label});

  final String value;
  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      children: <Widget>[
        Text(
          value,
          maxLines: 2,
          overflow: TextOverflow.clip,
          textAlign: TextAlign.center,
          style: Theme.of(
            context,
          ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
        ),
        const SizedBox(height: 2),
        Text(
          label,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(
            context,
          ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ],
    );
  }
}

class _ArtifactDetailMetricDivider extends StatelessWidget {
  const _ArtifactDetailMetricDivider({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 32,
      child: VerticalDivider(width: 1, thickness: 1, color: color),
    );
  }
}

class UnifiedMarketDetailTabs extends StatelessWidget {
  const UnifiedMarketDetailTabs({
    super.key,
    required this.selectedIndex,
    required this.commentCount,
    required this.onSelected,
  });

  final int selectedIndex;
  final int commentCount;
  final ValueChanged<int> onSelected;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: SizedBox(
        width: 240,
        child: Row(
          children: <Widget>[
            _UnifiedMarketDetailTabItem(
              selected: selectedIndex == 0,
              text: '关于',
              onTap: () => onSelected(0),
            ),
            _UnifiedMarketDetailTabItem(
              selected: selectedIndex == 1,
              text: '评论 $commentCount',
              onTap: () => onSelected(1),
            ),
          ],
        ),
      ),
    );
  }
}

class _UnifiedMarketDetailTabItem extends StatelessWidget {
  const _UnifiedMarketDetailTabItem({
    required this.selected,
    required this.text,
    required this.onTap,
  });

  final bool selected;
  final String text;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Expanded(
      child: InkWell(
        onTap: onTap,
        child: SizedBox(
          height: 44,
          child: Column(
            mainAxisAlignment: MainAxisAlignment.end,
            children: <Widget>[
              Expanded(
                child: Center(
                  child: Text(
                    text,
                    style: textTheme.labelLarge?.copyWith(
                      fontWeight: selected
                          ? FontWeight.w600
                          : FontWeight.normal,
                      color: selected
                          ? colorScheme.onSurface
                          : colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ),
              AnimatedContainer(
                duration: const Duration(milliseconds: 160),
                height: 2,
                width: selected ? 96 : 0,
                color: colorScheme.primary,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class ArtifactDetailSectionCard extends StatelessWidget {
  const ArtifactDetailSectionCard({
    super.key,
    required this.title,
    required this.child,
  });

  final String title;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surface,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              title,
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 10),
            child,
          ],
        ),
      ),
    );
  }
}

class UnifiedMarketDetailCommentsSection extends StatelessWidget {
  const UnifiedMarketDetailCommentsSection({super.key, required this.state});

  final UnifiedMarketDetailCommentsState state;

  @override
  Widget build(BuildContext context) {
    return ArtifactDetailSectionCard(
      title: '社区反馈',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (state.isLoading)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 18),
              child: Center(child: CircularProgressIndicator()),
            )
          else if (state.errorMessage != null)
            ArtifactErrorPanel(
              message: state.errorMessage!,
              onRetry: state.onRetry,
            )
          else ...<Widget>[
            Wrap(spacing: 8, runSpacing: 8, children: state.reactions),
            const SizedBox(height: 14),
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    state.title,
                    style: Theme.of(context).textTheme.labelLarge?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                TextButton.icon(
                  onPressed: state.canPost && !state.isPosting
                      ? state.onRequestPost
                      : null,
                  icon: state.isPosting
                      ? const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.add_comment_outlined),
                  label: const Text('发表评论'),
                ),
              ],
            ),
            if (state.postHint != null && state.postHint!.trim().isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 6),
                child: Text(
                  state.postHint!,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            const SizedBox(height: 12),
            if (state.comments.isEmpty)
              Text(
                '暂无评论',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              )
            else
              for (final comment in state.comments)
                ArtifactCommentTile(comment: comment),
          ],
        ],
      ),
    );
  }
}

class UnifiedMarketDetailActionRow extends StatelessWidget {
  const UnifiedMarketDetailActionRow({
    super.key,
    required this.primaryAction,
    this.secondaryAction,
  });

  final UnifiedMarketDetailAction primaryAction;
  final UnifiedMarketDetailAction? secondaryAction;

  @override
  Widget build(BuildContext context) {
    final secondary = secondaryAction;
    return Row(
      children: <Widget>[
        Expanded(child: _UnifiedMarketDetailPrimaryButton(primaryAction)),
        if (secondary != null) ...<Widget>[
          const SizedBox(width: 10),
          Expanded(child: _UnifiedMarketDetailSecondaryButton(secondary)),
        ],
      ],
    );
  }
}

class _UnifiedMarketDetailPrimaryButton extends StatelessWidget {
  const _UnifiedMarketDetailPrimaryButton(this.action);

  final UnifiedMarketDetailAction action;

  @override
  Widget build(BuildContext context) {
    return FilledButton.icon(
      onPressed: action.enabled && !action.isLoading ? action.onPressed : null,
      icon: _UnifiedMarketDetailActionIcon(action: action),
      label: Text(action.label),
    );
  }
}

class _UnifiedMarketDetailSecondaryButton extends StatelessWidget {
  const _UnifiedMarketDetailSecondaryButton(this.action);

  final UnifiedMarketDetailAction action;

  @override
  Widget build(BuildContext context) {
    return OutlinedButton.icon(
      onPressed: action.enabled && !action.isLoading ? action.onPressed : null,
      icon: _UnifiedMarketDetailActionIcon(action: action),
      label: Text(action.label),
    );
  }
}

class _UnifiedMarketDetailActionIcon extends StatelessWidget {
  const _UnifiedMarketDetailActionIcon({required this.action});

  final UnifiedMarketDetailAction action;

  @override
  Widget build(BuildContext context) {
    if (action.isLoading) {
      return const SizedBox.square(
        dimension: 18,
        child: CircularProgressIndicator(strokeWidth: 2),
      );
    }
    final icon = action.icon;
    if (icon == null) {
      return const SizedBox.shrink();
    }
    return Icon(icon);
  }
}

class ArtifactInfoRow {
  const ArtifactInfoRow({required this.label, required this.value});

  final String label;
  final String value;
}

class ArtifactInfoTable extends StatelessWidget {
  const ArtifactInfoTable({super.key, required this.rows});

  final List<ArtifactInfoRow> rows;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: colorScheme.outlineVariant),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        children: <Widget>[
          for (var index = 0; index < rows.length; index += 1)
            DecoratedBox(
              decoration: BoxDecoration(
                border: index == rows.length - 1
                    ? null
                    : Border(
                        bottom: BorderSide(color: colorScheme.outlineVariant),
                      ),
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 9,
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    SizedBox(
                      width: 88,
                      child: Text(
                        rows[index].label,
                        style: textTheme.labelMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                    Expanded(
                      child: SelectableText(
                        rows[index].value,
                        style: textTheme.bodySmall,
                      ),
                    ),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class ArtifactCommentTile extends StatelessWidget {
  const ArtifactCommentTile({super.key, required this.comment});

  final core_proxy.GitHubComment comment;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerLow,
        layer: OperitGlassSurfaceLayer.card,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.12),
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  CircleAvatar(
                    radius: 13,
                    backgroundImage: comment.user.avatarUrl.trim().isEmpty
                        ? null
                        : NetworkImage(comment.user.avatarUrl),
                    child: comment.user.avatarUrl.trim().isEmpty
                        ? Text(
                            comment.user.login.trim().isEmpty
                                ? '?'
                                : comment.user.login.trim()[0],
                          )
                        : null,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      comment.user.login,
                      style: textTheme.labelLarge?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  Text(
                    formatMarketDate(comment.createdAt),
                    style: textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              SelectableText(comment.body, style: textTheme.bodySmall),
            ],
          ),
        ),
      ),
    );
  }
}

class ArtifactCompatibilityBanner extends StatelessWidget {
  const ArtifactCompatibilityBanner({
    super.key,
    required this.project,
    required this.node,
  });

  final core_proxy.ArtifactProjectDetailResponse project;
  final core_proxy.ArtifactProjectNodeResponse node;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(
              Icons.warning_amber_outlined,
              color: colorScheme.onErrorContainer,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '当前软件版本可能不兼容',
                    style: textTheme.titleSmall?.copyWith(
                      color: colorScheme.onErrorContainer,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    unsupportedArtifactVersionMessage(project, node),
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onErrorContainer,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class ArtifactErrorPanel extends StatelessWidget {
  const ArtifactErrorPanel({
    super.key,
    required this.message,
    required this.onRetry,
  });

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(message)),
          TextButton.icon(
            onPressed: onRetry,
            icon: const Icon(Icons.refresh),
            label: const Text('重试'),
          ),
        ],
      ),
    );
  }
}
