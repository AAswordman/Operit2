// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../l10n/generated/app_localizations.dart';
import '../../style/MessageHeaderMetadata.dart';
import 'TopBarCapsuleModels.dart';

/// Displays context window usage using either Scheme A's segmented capsule bar
/// or Scheme B's radial telemetry arc, with a manual compaction trigger.
class ContextUsagePanel extends StatefulWidget {
  const ContextUsagePanel({
    super.key,
    required this.summary,
    required this.loading,
    required this.loadFailed,
    required this.visualMode,
    required this.compacting,
    required this.canCompact,
    required this.compactError,
    required this.onVisualModeChanged,
    required this.onCompactPressed,
    required this.onRefreshPressed,
    required this.pollingSummary,
  });

  final core_proxy.ChatInputMenuSummary? summary;
  final bool loading;
  final bool loadFailed;
  final ContextVisualMode visualMode;
  final bool compacting;
  final bool canCompact;
  final String? compactError;
  final ValueChanged<ContextVisualMode> onVisualModeChanged;
  final VoidCallback onCompactPressed;
  final VoidCallback onRefreshPressed;
  final bool pollingSummary;

  @override
  State<ContextUsagePanel> createState() => _ContextUsagePanelState();
}

class _ContextUsagePanelState extends State<ContextUsagePanel> {
  String? _hoveredSegmentId;
  bool _hoveringCompact = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _buildHeader(context, theme, colors, l10n),
        const SizedBox(height: 5),
        if (widget.loading)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Text(l10n.loading, style: theme.textTheme.bodySmall),
          )
        else ...<Widget>[
          if (widget.loadFailed || widget.summary == null)
            Text(
              l10n.contextStatsSummaryLoadFailed,
              style: theme.textTheme.bodySmall?.copyWith(color: colors.error),
            ),
          if (widget.summary != null)
            if (widget.visualMode == ContextVisualMode.segmented)
              _buildSegmentedView(theme, colors, widget.summary!)
            else
              _buildTelemetryView(theme, colors, widget.summary!),
        ],
        if (widget.compactError case final error?) ...<Widget>[
          const SizedBox(height: 6),
          Text(
            error,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodySmall?.copyWith(color: colors.error),
          ),
        ],
      ],
    );
  }

  Widget _buildHeader(
    BuildContext context,
    ThemeData theme,
    ColorScheme colors,
    AppLocalizations l10n,
  ) {
    return Wrap(
      alignment: WrapAlignment.spaceBetween,
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: 4,
      runSpacing: 4,
      children: <Widget>[
        Text(
          '上下文占用',
          style: theme.textTheme.labelLarge?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            _ModeSegmentedPill(
              mode: widget.visualMode,
              onChanged: widget.onVisualModeChanged,
            ),
            IconButton(
              tooltip: l10n.refresh,
              onPressed: widget.pollingSummary ? null : widget.onRefreshPressed,
              icon: const Icon(Icons.refresh_rounded, size: 14),
              visualDensity: VisualDensity.compact,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
            ),
            const SizedBox(width: 2),
            MouseRegion(
              onEnter: (_) => setState(() => _hoveringCompact = true),
              onExit: (_) => setState(() => _hoveringCompact = false),
              child: Tooltip(
                message: widget.canCompact
                    ? '手动压缩当前对话历史为总结检查点'
                    : '当前没有可压缩的上下文窗口',
                child: Material(
                  color: Colors.transparent,
                  child: InkWell(
                    onTap: widget.canCompact && !widget.compacting
                        ? widget.onCompactPressed
                        : null,
                    borderRadius: BorderRadius.circular(999),
                    child: AnimatedContainer(
                      duration: const Duration(milliseconds: 180),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 2.5,
                      ),
                      decoration: BoxDecoration(
                        color: widget.compacting
                            ? colors.primary
                            : widget.canCompact
                            ? colors.onSurface
                            : colors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(999),
                      ),
                      child: Text(
                        widget.compacting ? '压缩中' : '压缩',
                        style: theme.textTheme.labelSmall?.copyWith(
                          fontSize: 11,
                          color: widget.compacting
                              ? colors.onPrimary
                              : widget.canCompact
                              ? colors.surface
                              : colors.onSurfaceVariant,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildSegmentedView(
    ThemeData theme,
    ColorScheme colors,
    core_proxy.ChatInputMenuSummary summary,
  ) {
    final budget = resolveContextBudget(summary);
    final limit = budget.effectiveLimit;
    final used = summary.currentWindowSize;
    final hasLimit = budget.hasLimit;
    final usageRatio = hasLimit ? (used / limit).clamp(0.0, 1.0) : 0.0;
    final remaining = hasLimit ? math.max(0, limit - used) : 0;

    final accent = usageRatio >= 0.9
        ? colors.error
        : usageRatio >= 0.8
        ? (colors.brightness == Brightness.dark
              ? Colors.amber.shade300
              : Colors.orange.shade800)
        : colors.primary;

    final colorsPalette = _ContextBreakdownPalette.fromBrightness(colors.brightness);
    final breakdown = summary.breakdown;
    final segments = <ContextSegmentEntry>[
      if (breakdown != null) ...<ContextSegmentEntry>[
        ContextSegmentEntry(
          id: 'system',
          label: '系统提示词',
          tokens: breakdown.systemPromptTokens,
          color: colorsPalette.systemPrompt,
        ),
        ContextSegmentEntry(
          id: 'tools',
          label: '工具',
          tokens: breakdown.toolTokens,
          color: colorsPalette.tools,
        ),
        ContextSegmentEntry(
          id: 'files',
          label: '文件',
          tokens: breakdown.fileTokens,
          color: colorsPalette.files,
        ),
        ContextSegmentEntry(
          id: 'tool_results',
          label: '工具结果',
          tokens: breakdown.toolResultTokens,
          color: colorsPalette.toolResults,
        ),
        ContextSegmentEntry(
          id: 'conversation',
          label: '会话内容',
          tokens: breakdown.conversationTokens,
          color: colorsPalette.conversation,
        ),
      ] else
        ContextSegmentEntry(
          id: 'window',
          label: '上下文窗口',
          tokens: used,
          color: accent,
        ),
    ];

    final legendItems = <({String id, String label, String value, Color color})>[
      if (breakdown != null) ...<({String id, String label, String value, Color color})>[
        for (final seg in segments)
          if (seg.tokens > 0)
            (
              id: seg.id,
              label: seg.label,
              value: formatCompactTokenCount(seg.tokens),
              color: seg.color,
            ),
        if (used > 0 && segments.every((seg) => seg.tokens <= 0))
          (
            id: 'window',
            label: '窗口',
            value: formatCompactTokenCount(used),
            color: accent,
          ),
      ] else ...<({String id, String label, String value, Color color})>[
        (
          id: 'window',
          label: '窗口',
          value: formatCompactTokenCount(used),
          color: accent,
        ),
        (
          id: 'input',
          label: '输入',
          value: formatCompactTokenCount(summary.inputTokenCount),
          color: colors.secondary,
        ),
        (
          id: 'output',
          label: '输出',
          value: formatCompactTokenCount(summary.outputTokenCount),
          color: colors.tertiary,
        ),
      ],
      if (hasLimit)
        (
          id: 'remain',
          label: '剩余',
          value: formatCompactTokenCount(remaining),
          color: colors.outline,
        ),
    ];

    ({String id, String label, String value, Color color})? hoveredItem;
    if (_hoveredSegmentId != null) {
      for (final item in legendItems) {
        if (item.id == _hoveredSegmentId) {
          hoveredItem = item;
          break;
        }
      }
    }
    final cacheRate = summary.cacheHitRate;
    final cachedTokens = summary.cachedInputTokenCount;
    final ioReadout =
        '入 ${formatCompactTokenCount(summary.inputTokenCount)} · 出 ${formatCompactTokenCount(summary.outputTokenCount)}';
    final trailingReadout = _hoveringCompact && widget.canCompact
        ? '压缩后保留关键上下文'
        : hoveredItem != null
        ? '${hoveredItem.label} ${hoveredItem.value}'
        : ioReadout;
    final isHoverHighlighted =
        (_hoveringCompact && widget.canCompact) || hoveredItem != null;
    final cacheColor = cacheRate != null && cacheRate > 0
        ? (colors.brightness == Brightness.dark
              ? const Color(0xFF5EEAD4)
              : const Color(0xFF0F766E))
        : colors.onSurfaceVariant;

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            Flexible(
              flex: 3,
              child: Tooltip(
                message: budget.hasCompressionSupplement
                    ? '$used / $limit（压缩阈值 · 最大 ${budget.rawMaxTokens}）'
                    : '$used / ${hasLimit ? limit : "未设置上限"}',
                child: Text.rich(
                  TextSpan(
                    children: <InlineSpan>[
                      TextSpan(
                        text: formatCompactTokenCount(used),
                        style: theme.textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                          fontFeatures: const <FontFeature>[
                            FontFeature.tabularFigures(),
                          ],
                        ),
                      ),
                      TextSpan(
                        text: hasLimit
                            ? ' / ${formatCompactTokenCount(limit)}'
                            : ' / 未设置上限',
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: colors.onSurfaceVariant,
                          fontFeatures: const <FontFeature>[
                            FontFeature.tabularFigures(),
                          ],
                        ),
                      ),
                      if (budget.hasCompressionSupplement)
                        TextSpan(
                          text:
                              ' (最大 ${formatCompactTokenCount(budget.rawMaxTokens)})',
                          style: theme.textTheme.labelSmall?.copyWith(
                            fontSize: 10.5,
                            color: colors.onSurfaceVariant.withValues(
                              alpha: 0.72,
                            ),
                            fontFeatures: const <FontFeature>[
                              FontFeature.tabularFigures(),
                            ],
                          ),
                        ),
                    ],
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),
            const SizedBox(width: 6),
            Flexible(
              flex: 2,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.end,
                children: <Widget>[
                  Text(
                    trailingReadout,
                    textAlign: TextAlign.end,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.labelSmall?.copyWith(
                      fontSize: 11,
                      color: isHoverHighlighted
                          ? colors.primary
                          : colors.onSurfaceVariant,
                      fontFeatures: const <FontFeature>[
                        FontFeature.tabularFigures(),
                      ],
                    ),
                  ),
                  if (!isHoverHighlighted && cacheRate != null)
                    Tooltip(
                      message: cachedTokens != null
                          ? '累计输入缓存命中 $cachedTokens tokens'
                          : '累计输入缓存命中率',
                      child: Text(
                        '缓存命中 ${formatCacheHitPercentage(cacheRate)}',
                        textAlign: TextAlign.end,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.labelSmall?.copyWith(
                          fontSize: 10,
                          color: cacheColor,
                          fontWeight: cacheRate > 0
                              ? FontWeight.w600
                              : FontWeight.w400,
                          fontFeatures: const <FontFeature>[
                            FontFeature.tabularFigures(),
                          ],
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 5),
        _SegmentedCapacityBar(
          segments: segments,
          maxTokens: limit,
          hoveredSegmentId: _hoveredSegmentId,
          highlightCompressionTarget: _hoveringCompact && widget.canCompact,
          trackColor: colors.onSurface.withValues(alpha: 0.09),
          onHoverChanged: (id) => setState(() => _hoveredSegmentId = id),
        ),
        const SizedBox(height: 6),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: <Widget>[
            for (final item in legendItems)
              MouseRegion(
                onEnter: (_) => setState(() => _hoveredSegmentId = item.id),
                onExit: (_) => setState(() => _hoveredSegmentId = null),
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 160),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 2,
                  ),
                  decoration: BoxDecoration(
                    color: colors.onSurface.withValues(alpha: 0.06),
                    borderRadius: BorderRadius.circular(999),
                    border: Border.all(
                      color: _hoveredSegmentId == item.id
                          ? colors.primary
                          : Colors.transparent,
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Container(
                        width: 6,
                        height: 6,
                        decoration: BoxDecoration(
                          color: item.color,
                          borderRadius: BorderRadius.circular(2),
                        ),
                      ),
                      const SizedBox(width: 4),
                      Text(
                        '${item.label} ${item.value}',
                        style: theme.textTheme.labelSmall?.copyWith(
                          fontSize: 11,
                          color: colors.onSurface,
                          fontFeatures: const <FontFeature>[
                            FontFeature.tabularFigures(),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
          ],
        ),
      ],
    );
  }

  Widget _buildTelemetryView(
    ThemeData theme,
    ColorScheme colors,
    core_proxy.ChatInputMenuSummary summary,
  ) {
    final budget = resolveContextBudget(summary);
    final limit = budget.effectiveLimit;
    final used = summary.currentWindowSize;
    final hasLimit = budget.hasLimit;
    final fraction = hasLimit ? (used / limit).clamp(0.0, 1.0) : 0.0;
    final remaining = hasLimit ? math.max(0, limit - used) : 0;
    final arcColor = fraction >= 0.9
        ? colors.error
        : fraction >= 0.8
        ? (colors.brightness == Brightness.dark
              ? Colors.amber.shade300
              : Colors.orange.shade800)
        : colors.primary;

    final breakdown = summary.breakdown;
    final palette = _ContextBreakdownPalette.fromBrightness(colors.brightness);
    final breakdownItems = <({String label, int tokens, Color color})>[
      if (breakdown != null) ...<({String label, int tokens, Color color})>[
        if (breakdown.systemPromptTokens > 0)
          (label: '系统提示词', tokens: breakdown.systemPromptTokens, color: palette.systemPrompt),
        if (breakdown.toolTokens > 0)
          (label: '工具', tokens: breakdown.toolTokens, color: palette.tools),
        if (breakdown.fileTokens > 0)
          (label: '文件', tokens: breakdown.fileTokens, color: palette.files),
        if (breakdown.toolResultTokens > 0)
          (label: '工具结果', tokens: breakdown.toolResultTokens, color: palette.toolResults),
        if (breakdown.conversationTokens > 0)
          (label: '会话内容', tokens: breakdown.conversationTokens, color: palette.conversation),
      ],
    ];

    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: colors.onSurface.withValues(alpha: 0.045),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.5)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              SizedBox(
                width: 44,
                height: 44,
                child: TweenAnimationBuilder<double>(
                  tween: Tween<double>(end: fraction),
                  duration: const Duration(milliseconds: 420),
                  curve: Curves.easeOutCubic,
                  builder: (context, value, _) {
                    return CustomPaint(
                      painter: _TelemetryArcPainter(
                        fraction: hasLimit ? value : 0,
                        accent: arcColor,
                        trackColor: colors.onSurface.withValues(alpha: 0.12),
                      ),
                      child: Center(
                        child: Text(
                          hasLimit ? '${(value * 100).toStringAsFixed(0)}%' : '--',
                          style: theme.textTheme.labelSmall?.copyWith(
                            fontWeight: FontWeight.w700,
                            fontFeatures: const <FontFeature>[
                              FontFeature.tabularFigures(),
                            ],
                          ),
                        ),
                      ),
                    );
                  },
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    _TelemetryRow(
                      label: '上下文窗口',
                      value: hasLimit
                          ? budget.hasCompressionSupplement
                                ? '${formatCompactTokenCount(used)} / ${formatCompactTokenCount(limit)} (最大 ${formatCompactTokenCount(budget.rawMaxTokens)})'
                                : '${formatCompactTokenCount(used)} / ${formatCompactTokenCount(limit)}'
                          : '${formatCompactTokenCount(used)} / 未设置上限',
                      emphasized: true,
                    ),
                    if (hasLimit)
                      _TelemetryRow(
                        label: '可用余量',
                        value: formatCompactTokenCount(remaining),
                      ),
                    _TelemetryRow(
                      label: '输入 / 输出',
                      value:
                          '${formatCompactTokenCount(summary.inputTokenCount)} / ${formatCompactTokenCount(summary.outputTokenCount)}',
                    ),
                    if (summary.cacheHitRate case final cacheRate?)
                      _TelemetryRow(
                        label: '缓存命中',
                        value: summary.cachedInputTokenCount != null
                            ? '${formatCacheHitPercentage(cacheRate)} (${formatCompactTokenCount(summary.cachedInputTokenCount!)})'
                            : formatCacheHitPercentage(cacheRate),
                      ),
                  ],
                ),
              ),
            ],
          ),
          if (breakdownItems.isNotEmpty) ...<Widget>[
            const SizedBox(height: 6),
            Divider(height: 1, color: colors.outlineVariant.withValues(alpha: 0.35)),
            const SizedBox(height: 6),
            Wrap(
              spacing: 8,
              runSpacing: 3,
              children: <Widget>[
                for (final item in breakdownItems)
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Container(
                        width: 5,
                        height: 5,
                        decoration: BoxDecoration(
                          color: item.color,
                          borderRadius: BorderRadius.circular(1.5),
                        ),
                      ),
                      const SizedBox(width: 3.5),
                      Text(
                        '${item.label} ${formatCompactTokenCount(item.tokens)}',
                        style: theme.textTheme.labelSmall?.copyWith(
                          fontSize: 10.5,
                          color: colors.onSurfaceVariant,
                          fontFeatures: const <FontFeature>[
                            FontFeature.tabularFigures(),
                          ],
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

class _ModeSegmentedPill extends StatelessWidget {
  const _ModeSegmentedPill({required this.mode, required this.onChanged});

  final ContextVisualMode mode;
  final ValueChanged<ContextVisualMode> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    return Container(
      padding: const EdgeInsets.all(2),
      decoration: BoxDecoration(
        color: colors.onSurface.withValues(alpha: 0.06),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          _ModeOption(
            label: '分段',
            selected: mode == ContextVisualMode.segmented,
            onTap: () => onChanged(ContextVisualMode.segmented),
          ),
          _ModeOption(
            label: '仪表',
            selected: mode == ContextVisualMode.telemetry,
            onTap: () => onChanged(ContextVisualMode.telemetry),
          ),
        ],
      ),
    );
  }
}

class _ModeOption extends StatelessWidget {
  const _ModeOption({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(999),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 160),
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
        decoration: BoxDecoration(
          color: selected ? colors.surfaceContainer : Colors.transparent,
          borderRadius: BorderRadius.circular(999),
        ),
        child: Text(
          label,
          style: theme.textTheme.labelSmall?.copyWith(
            fontSize: 11,
            color: selected ? colors.onSurface : colors.onSurfaceVariant,
            fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
          ),
        ),
      ),
    );
  }
}

class _SegmentedCapacityBar extends StatelessWidget {
  const _SegmentedCapacityBar({
    required this.segments,
    required this.maxTokens,
    required this.hoveredSegmentId,
    required this.highlightCompressionTarget,
    required this.trackColor,
    required this.onHoverChanged,
  });

  final List<ContextSegmentEntry> segments;
  final int maxTokens;
  final String? hoveredSegmentId;
  final bool highlightCompressionTarget;
  final Color trackColor;
  final ValueChanged<String?> onHoverChanged;

  @override
  Widget build(BuildContext context) {
    final activeSegments = maxTokens <= 0
        ? const <ContextSegmentEntry>[]
        : segments.where((segment) => segment.tokens > 0).toList();
    final usedTokens = activeSegments.fold<int>(
      0,
      (sum, item) => sum + item.tokens,
    );
    final remainTokens = math.max(0, maxTokens - usedTokens);

    return SizedBox(
      height: 11,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final totalWidth = constraints.maxWidth;
          if (maxTokens <= 0 || activeSegments.isEmpty) {
            return Container(
              width: totalWidth,
              decoration: BoxDecoration(
                color: trackColor,
                borderRadius: BorderRadius.circular(4),
              ),
            );
          }

          final gap = activeSegments.length > 2 ? 2.0 : 3.0;
          final partCount = activeSegments.length + (remainTokens > 0 ? 1 : 0);
          final usableWidth = math.max(0.0, totalWidth - gap * (partCount - 1));
          final desiredMin = usableWidth >= 24 ? 4.0 : 2.0;
          final minSegmentWidth = activeSegments.isEmpty
              ? 0.0
              : math.min(desiredMin, usableWidth / activeSegments.length);

          final widths = <double>[];
          double allocated = 0;
          for (var i = 0; i < activeSegments.length; i++) {
            final segment = activeSegments[i];
            final reservedAfter =
                minSegmentWidth * (activeSegments.length - 1 - i);
            final maxAllowed = math.max(
              0.0,
              usableWidth - allocated - reservedAfter,
            );
            final raw = usableWidth * (segment.tokens / maxTokens);
            final lowerBound = math.min(minSegmentWidth, maxAllowed);
            final clamped = raw.clamp(lowerBound, maxAllowed);
            widths.add(clamped);
            allocated += clamped;
          }
          final remainingWidth = math.max(0.0, usableWidth - allocated);

          bool isCompressibleSegment(String id) =>
              id == 'window' ||
              id == 'files' ||
              id == 'tool_results' ||
              id == 'conversation';

          return Row(
            children: <Widget>[
              for (var i = 0; i < activeSegments.length; i++) ...<Widget>[
                if (i > 0) SizedBox(width: gap),
                MouseRegion(
                  onEnter: (_) => onHoverChanged(activeSegments[i].id),
                  onExit: (_) => onHoverChanged(null),
                  child: AnimatedContainer(
                    duration: const Duration(milliseconds: 360),
                    curve: Curves.easeOutCubic,
                    width: widths[i],
                    height: hoveredSegmentId == activeSegments[i].id ? 11 : 9,
                    decoration: BoxDecoration(
                      color: activeSegments[i].color.withValues(
                        alpha:
                            hoveredSegmentId != null &&
                                hoveredSegmentId != activeSegments[i].id
                            ? 0.4
                            : 1.0,
                      ),
                      borderRadius: BorderRadius.circular(4),
                      border:
                          highlightCompressionTarget &&
                              isCompressibleSegment(activeSegments[i].id)
                          ? Border.all(
                              color: Theme.of(context).colorScheme.onPrimary,
                              width: 1,
                            )
                          : null,
                    ),
                  ),
                ),
              ],
              if (remainTokens > 0) ...<Widget>[
                SizedBox(width: gap),
                MouseRegion(
                  onEnter: (_) => onHoverChanged('remain'),
                  onExit: (_) => onHoverChanged(null),
                  child: AnimatedContainer(
                    duration: const Duration(milliseconds: 360),
                    curve: Curves.easeOutCubic,
                    width: remainingWidth,
                    height: 9,
                    decoration: BoxDecoration(
                      color: trackColor,
                      borderRadius: BorderRadius.circular(4),
                    ),
                  ),
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}

class _TelemetryRow extends StatelessWidget {
  const _TelemetryRow({
    required this.label,
    required this.value,
    this.emphasized = false,
  });

  final String label;
  final String value;
  final bool emphasized;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 1.5),
      child: Row(
        children: <Widget>[
          Text(
            label,
            maxLines: 1,
            style: theme.textTheme.labelSmall?.copyWith(
              color: colors.onSurfaceVariant,
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              value,
              textAlign: TextAlign.end,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.labelSmall?.copyWith(
                color: emphasized ? colors.onSurface : colors.onSurfaceVariant,
                fontWeight: emphasized ? FontWeight.w600 : FontWeight.w400,
                fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _TelemetryArcPainter extends CustomPainter {
  const _TelemetryArcPainter({
    required this.fraction,
    required this.accent,
    required this.trackColor,
  });

  final double fraction;
  final Color accent;
  final Color trackColor;

  @override
  void paint(Canvas canvas, Size size) {
    final center = size.center(Offset.zero);
    final radius = size.shortestSide / 2 - 3;
    if (radius <= 0) return;
    const startAngle = math.pi * 0.75;
    const sweepTotal = math.pi * 1.5;

    final trackPaint = Paint()
      ..color = trackColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = 4
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      startAngle,
      sweepTotal,
      false,
      trackPaint,
    );

    if (fraction > 0) {
      final activePaint = Paint()
        ..color = accent
        ..style = PaintingStyle.stroke
        ..strokeWidth = 4
        ..strokeCap = StrokeCap.round;
      canvas.drawArc(
        Rect.fromCircle(center: center, radius: radius),
        startAngle,
        sweepTotal * fraction.clamp(0.02, 1.0),
        false,
        activePaint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _TelemetryArcPainter oldDelegate) =>
      oldDelegate.fraction != fraction ||
      oldDelegate.accent != accent ||
      oldDelegate.trackColor != trackColor;
}

class _ContextBreakdownPalette {
  const _ContextBreakdownPalette({
    required this.systemPrompt,
    required this.tools,
    required this.files,
    required this.toolResults,
    required this.conversation,
  });

  final Color systemPrompt;
  final Color tools;
  final Color files;
  final Color toolResults;
  final Color conversation;

  factory _ContextBreakdownPalette.fromBrightness(Brightness brightness) {
    if (brightness == Brightness.dark) {
      return const _ContextBreakdownPalette(
        systemPrompt: Color(0xFFB39DDB),
        tools: Color(0xFF5AC8FA),
        files: Color(0xFF4ADE80),
        toolResults: Color(0xFFFFB74D),
        conversation: Color(0xFF8C9EFF),
      );
    }
    return const _ContextBreakdownPalette(
      systemPrompt: Color(0xFF673AB7),
      tools: Color(0xFF0277BD),
      files: Color(0xFF1B8E52),
      toolResults: Color(0xFFD97706),
      conversation: Color(0xFF3F51B5),
    );
  }
}
