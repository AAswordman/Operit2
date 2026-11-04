// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../settings/components/SettingsControlStyles.dart';
import '../market/ArtifactMarketSupport.dart';
import 'ExtensionAvatar.dart';
import 'ExtensionMotion.dart';
import 'StoreDetailKit.dart';
import 'StoreVisuals.dart';

/// Where a submission stands in the market's review.
enum CreatorReviewState { pending, approved, changesRequested, rejected, withdrawn }

/// Resolves the market's state code, including the codes older servers sent.
CreatorReviewState creatorReviewState(String stateCode) =>
    switch (stateCode.trim().toLowerCase()) {
      'pending' => CreatorReviewState.pending,
      'approved' || 'open' => CreatorReviewState.approved,
      'changes_requested' => CreatorReviewState.changesRequested,
      'rejected' || 'security_blocked' => CreatorReviewState.rejected,
      'withdrawn' || 'closed' || 'removed' => CreatorReviewState.withdrawn,
      _ => throw StateError('Unknown market review state: $stateCode'),
    };

extension CreatorReviewStateStyle on CreatorReviewState {
  String get label => switch (this) {
    CreatorReviewState.pending => '待审核',
    CreatorReviewState.approved => '已通过',
    CreatorReviewState.changesRequested => '已打回',
    CreatorReviewState.rejected => '已拒绝',
    CreatorReviewState.withdrawn => '已移除',
  };

  Color color(ColorScheme colorScheme) => switch (this) {
    CreatorReviewState.pending => colorScheme.tertiary,
    CreatorReviewState.approved => colorScheme.primary,
    CreatorReviewState.changesRequested => colorScheme.secondary,
    CreatorReviewState.rejected => colorScheme.error,
    CreatorReviewState.withdrawn => colorScheme.outline,
  };
}

/// Reason the reviewers gave, in words.
String creatorReasonLabel(String reasonCode) =>
    switch (reasonCode.trim().toLowerCase()) {
      'metadata-incomplete' => '元数据不完整',
      'install-config-invalid' => '安装配置无效',
      'repository-unreachable' => '仓库不可访问',
      'repository-content-invalid' => '仓库内容不合规',
      'entry-unusable' => '条目不可用',
      'quality-too-low' => '质量过低',
      'ai-hallucination' => '存在 AI 幻觉问题',
      'security-risk' => '存在安全风险',
      'duplicate-submission' => '重复投稿',
      'policy-violation' => '违反平台规范',
      _ => reasonCode,
    };

/// What a publisher may do with one of their entries.
class CreatorEntryAccess {
  CreatorEntryAccess(core_proxy.MarketPublisherEntrySummary entry)
    : state = creatorReviewState(entry.stateCode),
      owner = entry.relation == 'owner',
      pendingListing = entry.listingState == 'pending_listing';

  final CreatorReviewState state;
  final bool owner;

  /// Approved, but the public listing has not been generated yet.
  final bool pendingListing;

  bool get open => state == CreatorReviewState.approved;

  /// The public detail page exists.
  bool get publiclyListed => open && !pendingListing;

  /// A withdrawn entry can come back through a new version, but only for its owner.
  bool get canPublishVersion =>
      publiclyListed || (owner && state == CreatorReviewState.withdrawn);

  bool get canSubmitRevision => state == CreatorReviewState.changesRequested;

  bool get canWithdraw => owner && open;
}

/// Time left before a rejected-for-changes entry accepts its revised version.
Duration? creatorRevisionCooldown(String? availableAt) {
  final raw = availableAt?.trim();
  if (raw == null || raw.isEmpty) {
    return null;
  }
  final available = DateTime.tryParse(raw)?.toUtc();
  if (available == null) {
    return null;
  }
  final remaining = available.difference(DateTime.now().toUtc());
  return remaining.isNegative ? null : remaining;
}

String formatCreatorCooldown(Duration remaining) {
  final totalMinutes = remaining.inSeconds <= 0
      ? 1
      : ((remaining.inSeconds + 59) ~/ 60).clamp(1, 24 * 60).toInt();
  final hours = totalMinutes ~/ 60;
  final minutes = totalMinutes % 60;
  if (hours > 0 && minutes > 0) {
    return '$hours小时$minutes分钟';
  }
  if (hours > 0) {
    return '$hours小时';
  }
  return '$minutes分钟';
}

/// Submitted → reviewed → listed, drawn as a timeline that fills in once.
class CreatorReviewFlow extends StatelessWidget {
  const CreatorReviewFlow({
    super.key,
    required this.state,
    required this.pendingListing,
  });

  final CreatorReviewState state;
  final bool pendingListing;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final motion = ExtensionMotion.of(context);
    final reviewColor = state.color(colorScheme);
    final approved = state == CreatorReviewState.approved;
    final listed = approved && !pendingListing;
    final listingLabel = !approved
        ? '未上架'
        : pendingListing
        ? '待上架'
        : '已发布';
    final listingColor = listed ? colorScheme.primary : colorScheme.outline;
    return TweenAnimationBuilder<double>(
      tween: Tween<double>(begin: 0, end: 1),
      duration: motion.emphasized * 2,
      builder: (context, progress, child) {
        double part(double begin, double end) =>
            Interval(begin, end, curve: ExtensionMotion.enter).transform(progress);
        return Row(
          children: <Widget>[
            _FlowStep(
              label: '已提交',
              color: colorScheme.primary,
              reached: true,
              appear: part(0, 0.25),
            ),
            Expanded(
              child: _FlowConnector(color: reviewColor, fill: part(0.15, 0.5)),
            ),
            _FlowStep(
              label: state.label,
              color: reviewColor,
              reached: true,
              appear: part(0.4, 0.65),
            ),
            Expanded(
              child: _FlowConnector(
                color: colorScheme.primary,
                fill: listed ? part(0.55, 0.9) : 0,
              ),
            ),
            _FlowStep(
              label: listingLabel,
              color: listingColor,
              reached: listed,
              appear: part(0.8, 1),
            ),
          ],
        );
      },
    );
  }
}

class _FlowStep extends StatelessWidget {
  const _FlowStep({
    required this.label,
    required this.color,
    required this.reached,
    required this.appear,
  });

  final String label;
  final Color color;
  final bool reached;
  final double appear;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Opacity(
      opacity: appear,
      child: Transform.scale(
        scale: 0.7 + 0.3 * appear,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            DecoratedBox(
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: reached ? color : Colors.transparent,
                border: reached ? null : Border.all(color: color, width: 1.5),
              ),
              child: const SizedBox.square(dimension: 8),
            ),
            const SizedBox(width: 5),
            Text(
              label,
              softWrap: false,
              style: theme.textTheme.labelSmall?.copyWith(
                color: color,
                fontWeight: reached ? FontWeight.w700 : FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _FlowConnector extends StatelessWidget {
  const _FlowConnector({required this.color, required this.fill});

  final Color color;
  final double fill;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      child: SizedBox(
        height: 2,
        child: Stack(
          children: <Widget>[
            DecoratedBox(
              decoration: BoxDecoration(
                color: colorScheme.outlineVariant.withValues(alpha: 0.55),
                borderRadius: BorderRadius.circular(1),
              ),
              child: const SizedBox.expand(),
            ),
            FractionallySizedBox(
              widthFactor: fill,
              alignment: Alignment.centerLeft,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.85),
                  borderRadius: BorderRadius.circular(1),
                ),
                child: const SizedBox.expand(),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// One entry the signed-in account published or contributed to: review
/// progress, what the reviewers said and the actions that fit its state.
class CreatorEntryCard extends StatefulWidget {
  const CreatorEntryCard({
    super.key,
    required this.entry,
    required this.busy,
    required this.onOpen,
    required this.onPublishVersion,
    required this.onSubmitRevision,
    required this.onWithdraw,
  });

  final core_proxy.MarketPublisherEntrySummary entry;

  /// Another action on this entry is still running.
  final bool busy;
  final VoidCallback onOpen;
  final VoidCallback onPublishVersion;
  final VoidCallback onSubmitRevision;
  final Future<void> Function() onWithdraw;

  @override
  State<CreatorEntryCard> createState() => _CreatorEntryCardState();
}

class _CreatorEntryCardState extends State<CreatorEntryCard> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final motion = ExtensionMotion.of(context);
    final entry = widget.entry;
    final busy = widget.busy;
    final access = CreatorEntryAccess(entry);
    final kind = StoreKind.of(entry.type);
    final reasons = entry.reasonCodes
        .map(creatorReasonLabel)
        .where((reason) => reason.trim().isNotEmpty)
        .toList(growable: false);
    final reviewDetail = entry.reviewDetail?.trim() ?? '';
    final cooldown = access.canSubmitRevision
        ? creatorRevisionCooldown(entry.revisionAvailableAt)
        : null;
    final hasDetails =
        reasons.isNotEmpty || reviewDetail.isNotEmpty || cooldown != null;
    final hasActions =
        hasDetails ||
        access.canWithdraw ||
        access.canPublishVersion ||
        access.canSubmitRevision;
    final shownReasons = _expanded ? reasons : reasons.take(2).toList();

    Widget actions(VoidCallback? requestWithdraw) => Padding(
      padding: const EdgeInsets.fromLTRB(14, 8, 14, 12),
      child: Row(
        children: <Widget>[
          if (hasDetails)
            TextButton.icon(
              onPressed: () => setState(() => _expanded = !_expanded),
              style: SettingsControlStyles.sectionTextButton(),
              icon: Icon(
                _expanded
                    ? Icons.expand_less_rounded
                    : Icons.expand_more_rounded,
                size: 18,
              ),
              label: Text(_expanded ? '收起' : '审核详情'),
            ),
          Expanded(
            child: Align(
              alignment: Alignment.centerRight,
              child: Wrap(
                spacing: 8,
                runSpacing: 8,
                alignment: WrapAlignment.end,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: <Widget>[
                  if (requestWithdraw != null)
                    TextButton.icon(
                      onPressed: busy ? null : requestWithdraw,
                      style: SettingsControlStyles.sectionTextButton().copyWith(
                        foregroundColor: WidgetStatePropertyAll<Color>(
                          colorScheme.error,
                        ),
                      ),
                      icon: const Icon(Icons.delete_outline_rounded, size: 18),
                      label: const Text('移除'),
                    ),
                  if (access.canPublishVersion)
                    OutlinedButton.icon(
                      onPressed: busy ? null : widget.onPublishVersion,
                      style: OutlinedButton.styleFrom(
                        visualDensity: VisualDensity.compact,
                        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                        minimumSize: const Size(0, 36),
                        padding: const EdgeInsets.symmetric(horizontal: 14),
                      ),
                      icon: const Icon(Icons.new_releases_outlined, size: 18),
                      label: const Text('发布新版本'),
                    ),
                  if (access.canSubmitRevision)
                    FilledButton.icon(
                      onPressed: busy ? null : widget.onSubmitRevision,
                      style: SettingsControlStyles.sectionFilledButton(),
                      icon: const Icon(Icons.edit_note_rounded, size: 18),
                      label: const Text('修改后提交新版本'),
                    ),
                ],
              ),
            ),
          ),
        ],
      ),
    );

    final canTap = access.publiclyListed || hasDetails;
    return StorePressable(
      onTap: !canTap
          ? null
          : access.publiclyListed
          ? widget.onOpen
          : () => setState(() => _expanded = !_expanded),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 14, 14, 0),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                ExtensionAvatar(
                  seed: '${kind.id}:${entry.id}',
                  label: entry.title,
                  kind: kind,
                  size: 44,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        entry.title,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '${kind.label} · 更新于 ${formatMarketDate(entry.updatedAt)}',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                _StatusBadge(access: access),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 14, 14, 0),
            child: CreatorReviewFlow(
              state: access.state,
              pendingListing: access.pendingListing,
            ),
          ),
          AnimatedSize(
            duration: motion.standard,
            curve: ExtensionMotion.enter,
            alignment: Alignment.topCenter,
            child: Padding(
              padding: EdgeInsets.fromLTRB(
                14,
                shownReasons.isEmpty && reviewDetail.isEmpty && !_expanded
                    ? 0
                    : 10,
                14,
                0,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  if (shownReasons.isNotEmpty)
                    Wrap(
                      spacing: 6,
                      runSpacing: 6,
                      children: <Widget>[
                        for (final reason in shownReasons)
                          StoreBadge(
                            label: reason,
                            icon: Icons.error_outline_rounded,
                            color: colorScheme.errorContainer.withValues(
                              alpha: 0.5,
                            ),
                            foreground: colorScheme.onErrorContainer,
                          ),
                      ],
                    ),
                  if (reviewDetail.isNotEmpty) ...<Widget>[
                    SizedBox(height: shownReasons.isEmpty ? 0 : 8),
                    Text(
                      reviewDetail,
                      maxLines: _expanded ? null : 3,
                      overflow: _expanded
                          ? TextOverflow.visible
                          : TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                        height: 1.45,
                      ),
                    ),
                  ],
                  if (_expanded && cooldown != null) ...<Widget>[
                    const SizedBox(height: 8),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Icon(
                          Icons.schedule_rounded,
                          size: 16,
                          color: colorScheme.tertiary,
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(
                            '修改版提交仍在冷却中，还需等待 ${formatCreatorCooldown(cooldown)}。',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: colorScheme.tertiary,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
          ),
          if (!hasActions)
            const SizedBox(height: 14)
          else if (access.canWithdraw)
            StoreInlineConfirm(
              trigger: (context, requestConfirm) => actions(requestConfirm),
              message:
                  '「${entry.title}」会从市场下架，其他人无法再找到和安装它。'
                  '你的发布仓库不会被删除，之后仍可以发布新版本重新上架。',
              confirmLabel: '确认移除',
              destructive: true,
              onConfirm: widget.onWithdraw,
            )
          else
            actions(null),
          if (busy)
            ClipRRect(
              borderRadius: const BorderRadius.vertical(
                bottom: Radius.circular(12),
              ),
              child: const LinearProgressIndicator(minHeight: 2),
            ),
        ],
      ),
    );
  }
}

/// Overall standing of an entry: what a reader of the market would call it.
class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.access});

  final CreatorEntryAccess access;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final state = access.state;
    final (background, foreground) = switch (state) {
      CreatorReviewState.pending => (
        colorScheme.tertiaryContainer,
        colorScheme.onTertiaryContainer,
      ),
      CreatorReviewState.approved => (
        colorScheme.primaryContainer,
        colorScheme.onPrimaryContainer,
      ),
      CreatorReviewState.changesRequested => (
        colorScheme.secondaryContainer,
        colorScheme.onSecondaryContainer,
      ),
      CreatorReviewState.rejected => (
        colorScheme.errorContainer,
        colorScheme.onErrorContainer,
      ),
      CreatorReviewState.withdrawn => (
        colorScheme.surfaceContainerHighest,
        colorScheme.onSurfaceVariant,
      ),
    };
    final (label, icon) = switch (state) {
      CreatorReviewState.pending => ('待审核', Icons.schedule_rounded),
      CreatorReviewState.approved when access.pendingListing => (
        '待上架',
        Icons.hourglass_top_rounded,
      ),
      CreatorReviewState.approved => ('已发布', Icons.check_circle_outline_rounded),
      CreatorReviewState.changesRequested => ('已打回', Icons.edit_note_rounded),
      CreatorReviewState.rejected => ('已拒绝', Icons.block_rounded),
      CreatorReviewState.withdrawn => (
        '已移除',
        Icons.remove_circle_outline_rounded,
      ),
    };
    return StoreBadge(
      label: label,
      icon: icon,
      color: background,
      foreground: foreground,
    );
  }
}
