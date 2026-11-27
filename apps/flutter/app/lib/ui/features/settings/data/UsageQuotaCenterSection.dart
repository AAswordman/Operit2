// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../../l10n/generated/app_localizations.dart';
import '../model/ProviderLogo.dart';
import 'UsageBentoComponents.dart';
import 'UsageFormatters.dart';
import 'UsagePricing.dart';

enum QuotaCategoryFilter { all, balance, subscription, localCompute, budgets }

class UsageQuotaCenterSection extends StatefulWidget {
  const UsageQuotaCenterSection({
    super.key,
    required this.clients,
    required this.quotaState,
    required this.records,
    required this.pricing,
    required this.spaceDevices,
    required this.localDeviceId,
    required this.onStateUpdated,
  });

  final GeneratedCoreProxyClients clients;
  final core.ProviderQuotaCenterState quotaState;
  final List<core.UsageRequestRecord> records;
  final UsagePricingIndex pricing;
  final List<core.RuntimeDeviceSpaceDevice> spaceDevices;
  final String? localDeviceId;
  final Future<void> Function() onStateUpdated;

  @override
  State<UsageQuotaCenterSection> createState() =>
      _UsageQuotaCenterSectionState();
}

class _UsageQuotaCenterSectionState extends State<UsageQuotaCenterSection> {
  QuotaCategoryFilter _filter = QuotaCategoryFilter.all;
  bool _probingAll = false;
  final Set<String> _probingIds = <String>{};
  final Set<String> _expandedKeyPools = <String>{};

  Future<void> _probeAll() async {
    if (_probingAll) return;
    setState(() => _probingAll = true);
    try {
      await widget.clients.servicesProviderQuotaService.probeAllProviders();
      await widget.onStateUpdated();
    } catch (error) {
      if (mounted) _showSnack('$error');
    } finally {
      if (mounted) setState(() => _probingAll = false);
    }
  }

  Future<void> _probeOne(String providerId, {String? targetNodeId}) async {
    if (_probingIds.contains(providerId)) return;
    setState(() => _probingIds.add(providerId));
    try {
      if (targetNodeId != null && targetNodeId.isNotEmpty) {
        await widget.clients.serverRuntimeRemoteLinkService
            .probeProviderQuotaOnNode(
              providerId: providerId,
              targetNodeId: targetNodeId,
            );
      } else {
        await widget.clients.servicesProviderQuotaService.probeProviderQuota(
          providerId: providerId,
        );
      }
      await widget.onStateUpdated();
    } catch (error) {
      if (mounted) _showSnack('$error');
    } finally {
      if (mounted) setState(() => _probingIds.remove(providerId));
    }
  }

  void _showSnack(String text) {
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(text)));
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    final snapshots = widget.quotaState.snapshots;
    final budgetsByProvider = <String, core.ProviderLocalBudget>{
      for (final b in widget.quotaState.budgets) b.providerId: b,
    };
    final configsByProvider = <String, core.ProviderQuotaProbeConfig>{
      for (final c in widget.quotaState.probeConfigs) c.providerId: c,
    };

    final filtered = snapshots.where((snapshot) {
      final hasBudget = budgetsByProvider.containsKey(snapshot.providerId);
      return switch (_filter) {
        QuotaCategoryFilter.all => true,
        QuotaCategoryFilter.balance =>
          snapshot.accountKind == core.QuotaAccountKind.balance,
        QuotaCategoryFilter.subscription =>
          snapshot.accountKind == core.QuotaAccountKind.subscription,
        QuotaCategoryFilter.localCompute =>
          snapshot.accountKind == core.QuotaAccountKind.localCompute,
        QuotaCategoryFilter.budgets =>
          hasBudget ||
          snapshot.accountKind == core.QuotaAccountKind.unsupported,
      };
    }).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BentoStaggeredEntrance(
          child: _buildPortfolioBanner(
            context,
            l10n,
            colors,
            text,
            snapshots,
            budgetsByProvider,
          ),
        ),
        const SizedBox(height: 12),
        BentoSegmentedControl<QuotaCategoryFilter>(
          selected: _filter,
          onSelected: (filter) => setState(() => _filter = filter),
          options: [
            for (final filter in QuotaCategoryFilter.values)
              BentoSegmentedOption<QuotaCategoryFilter>(
                value: filter,
                itemKey: ValueKey('quota-filter-${filter.name}'),
                icon: switch (filter) {
                  QuotaCategoryFilter.all => Icons.dashboard_outlined,
                  QuotaCategoryFilter.balance =>
                    Icons.account_balance_wallet_outlined,
                  QuotaCategoryFilter.subscription =>
                    Icons.workspace_premium_outlined,
                  QuotaCategoryFilter.localCompute => Icons.memory_outlined,
                  QuotaCategoryFilter.budgets => Icons.tune_outlined,
                },
                label: _filterLabel(l10n, filter),
              ),
          ],
        ),
        const SizedBox(height: 14),
        if (filtered.isEmpty)
          BentoSurfaceCard(
            padding: const EdgeInsets.all(28),
            radius: 18,
            child: Center(
              child: Text(
                l10n.settingsQuotaEmptyProviders,
                style: text.bodyMedium?.copyWith(color: colors.onSurfaceVariant),
              ),
            ),
          )
        else
          LayoutBuilder(
            builder: (context, constraints) {
              final columns = constraints.maxWidth >= 1120
                  ? 3
                  : constraints.maxWidth >= 660
                  ? 2
                  : 1;
              const spacing = 12.0;
              final cardWidth =
                  (constraints.maxWidth - spacing * (columns - 1)) / columns;
              return Wrap(
                spacing: spacing,
                runSpacing: spacing,
                children: [
                  for (var i = 0; i < filtered.length; i++)
                    SizedBox(
                      width: cardWidth,
                      child: BentoStaggeredEntrance(
                        delay: Duration(milliseconds: (i * 40).clamp(0, 200)),
                        child: _buildProviderCard(
                          context,
                          l10n,
                          colors,
                          text,
                          filtered[i],
                          budgetsByProvider[filtered[i].providerId],
                          configsByProvider[filtered[i].providerId],
                        ),
                      ),
                    ),
                ],
              );
            },
          ),
      ],
    );
  }

  Widget _buildPortfolioBanner(
    BuildContext context,
    AppLocalizations l10n,
    ColorScheme colors,
    TextTheme text,
    List<core.ProviderQuotaSnapshot> snapshots,
    Map<String, core.ProviderLocalBudget> budgetsByProvider,
  ) {
    final totalsByCurrency = <String, double>{};
    var subscriptionPlans = 0;
    var localComputeNodes = 0;
    var readyCount = 0;
    var alertCount = 0;
    for (final s in snapshots) {
      if (s.status == core.QuotaProbeStatus.ok) readyCount++;
      if (s.accountKind == core.QuotaAccountKind.balance &&
          s.status == core.QuotaProbeStatus.ok &&
          s.balance?.totalBalance != null) {
        final cur = s.balance!.currency.trim().toUpperCase();
        if (cur.isNotEmpty) {
          totalsByCurrency.update(
            cur,
            (prev) => prev + s.balance!.totalBalance!,
            ifAbsent: () => s.balance!.totalBalance!,
          );
        }
      } else if (s.accountKind == core.QuotaAccountKind.subscription) {
        subscriptionPlans++;
      } else if (s.accountKind == core.QuotaAccountKind.localCompute) {
        localComputeNodes++;
      }
      final b = budgetsByProvider[s.providerId];
      if (b != null && b.monthlyBudget > 0) {
        final pct = _computeCycleSpendForBudget(s, b) / b.monthlyBudget * 100.0;
        if (pct >= b.alertThresholdPercent) alertCount++;
      }
    }

    return BentoSurfaceCard(
      padding: const EdgeInsets.all(16),
      radius: 18,
      accent: colors.primary,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(
            spacing: 12,
            runSpacing: 10,
            alignment: WrapAlignment.spaceBetween,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Wrap(
                spacing: 8,
                runSpacing: 6,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  Text(
                    l10n.settingsQuotaSummaryProviders(
                      readyCount,
                      snapshots.length,
                    ),
                    style: tabularTextStyle(
                      text.labelLarge?.copyWith(fontWeight: FontWeight.w700),
                    ),
                  ),
                  if (alertCount > 0)
                    BentoDeltaBadge(
                      icon: Icons.warning_amber_rounded,
                      label: l10n.settingsQuotaSummaryAlertCount(alertCount),
                      color: colors.error,
                    ),
                ],
              ),
              FilledButton.icon(
                key: const ValueKey('quota-probe-all-button'),
                onPressed: _probingAll ? null : _probeAll,
                icon: _probingAll
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.refresh, size: 18),
                label: Text(l10n.settingsQuotaProbeAll),
              ),
            ],
          ),
          const SizedBox(height: 14),
          Wrap(
            spacing: 18,
            runSpacing: 10,
            children: [
              for (final entry in totalsByCurrency.entries)
                _miniSummaryStat(
                  context,
                  '${entry.key} ${l10n.settingsQuotaSummaryTotalRemaining}',
                  formatCurrencyWithCode(entry.key, entry.value),
                  accent: colors.primary,
                ),
              _miniSummaryStat(
                context,
                l10n.settingsQuotaFilterSubscription,
                '$subscriptionPlans',
              ),
              _miniSummaryStat(
                context,
                l10n.settingsQuotaFilterLocal,
                '$localComputeNodes',
              ),
              _miniSummaryStat(
                context,
                l10n.settingsQuotaSetBudget,
                '${budgetsByProvider.length}',
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _miniSummaryStat(
    BuildContext context,
    String label,
    String value, {
    Color? accent,
  }) {
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: (accent ?? colors.surfaceContainerHighest).withValues(
          alpha: accent != null ? 0.10 : 0.42,
        ),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: (accent ?? colors.outlineVariant).withValues(alpha: 0.25),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            label,
            style: text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
          ),
          const SizedBox(height: 2),
          Text(
            value,
            style: tabularTextStyle(
              text.titleMedium?.copyWith(
                fontWeight: FontWeight.w800,
                color: accent ?? colors.onSurface,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildProviderCard(
    BuildContext context,
    AppLocalizations l10n,
    ColorScheme colors,
    TextTheme text,
    core.ProviderQuotaSnapshot snapshot,
    core.ProviderLocalBudget? budget,
    core.ProviderQuotaProbeConfig? config,
  ) {
    final isProbing = _probingIds.contains(snapshot.providerId);
    final isExpanded = _expandedKeyPools.contains(snapshot.providerId);
    final onlinePeers = widget.spaceDevices
        .where((d) => d.online && d.deviceId != widget.localDeviceId)
        .toList();

    final statusAccent = switch (snapshot.status) {
      core.QuotaProbeStatus.ok => const Color(0xFF10B981),
      core.QuotaProbeStatus.unauthorized ||
      core.QuotaProbeStatus.error => colors.error,
      core.QuotaProbeStatus.stale => colors.onSurfaceVariant,
      core.QuotaProbeStatus.unsupported => colors.secondary,
    };

    return BentoSurfaceCard(
      key: ValueKey('quota-card-${snapshot.providerId}'),
      padding: const EdgeInsets.all(16),
      radius: 18,
      accent: statusAccent,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              ProviderLogo(
                providerTypeId: snapshot.providerTypeId,
                fallbackName: snapshot.providerName,
                size: 38,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      snapshot.providerName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: text.titleSmall?.copyWith(fontWeight: FontWeight.w700),
                    ),
                    Text(
                      snapshot.plan ?? snapshot.protocol.value,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: text.labelSmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              _statusChip(context, l10n, colors, snapshot.status),
            ],
          ),
          const SizedBox(height: 12),
          if (snapshot.accountKind == core.QuotaAccountKind.balance &&
              snapshot.balance != null)
            _buildBalanceBody(context, l10n, colors, text, snapshot.balance!),
          if (snapshot.windows.isNotEmpty) ...[
            if (snapshot.balance != null) const SizedBox(height: 10),
            for (final window in snapshot.windows) ...[
              _buildQuotaWindowBar(context, colors, text, window),
              const SizedBox(height: 8),
            ],
          ],
          if (snapshot.accountKind == core.QuotaAccountKind.localCompute)
            _buildLocalComputeBody(context, l10n, colors, text, snapshot),
          if (snapshot.error != null && snapshot.error!.trim().isNotEmpty)
            Container(
              width: double.infinity,
              margin: const EdgeInsets.only(top: 8),
              padding: const EdgeInsets.all(9),
              decoration: BoxDecoration(
                color: colors.errorContainer.withValues(alpha: 0.32),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                  color: colors.error.withValues(alpha: 0.25),
                ),
              ),
              child: Text(
                snapshot.error!,
                style: text.labelSmall?.copyWith(color: colors.onErrorContainer),
              ),
            ),
          if (budget != null)
            _buildBudgetProgress(context, l10n, colors, text, snapshot, budget),
          if (snapshot.keySnapshots.length > 1) ...[
            const SizedBox(height: 8),
            InkWell(
              onTap: () {
                setState(() {
                  if (isExpanded) {
                    _expandedKeyPools.remove(snapshot.providerId);
                  } else {
                    _expandedKeyPools.add(snapshot.providerId);
                  }
                });
              },
              borderRadius: BorderRadius.circular(8),
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Row(
                  children: [
                    AnimatedRotation(
                      turns: isExpanded ? 0.5 : 0.0,
                      duration: const Duration(milliseconds: 180),
                      curve: Curves.easeOutCubic,
                      child: Icon(
                        Icons.expand_more,
                        size: 18,
                        color: colors.primary,
                      ),
                    ),
                    const SizedBox(width: 4),
                    Text(
                      l10n.settingsQuotaKeyPoolCount(snapshot.keySnapshots.length),
                      style: tabularTextStyle(
                        text.labelMedium?.copyWith(
                          color: colors.primary,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            AnimatedSize(
              duration: const Duration(milliseconds: 220),
              curve: Curves.easeOutCubic,
              alignment: Alignment.topCenter,
              child: isExpanded
                  ? Column(
                      children: [
                        for (final keySnap in snapshot.keySnapshots)
                          _buildPoolKeyRow(context, l10n, colors, text, keySnap),
                      ],
                    )
                  : const SizedBox.shrink(),
            ),
          ],
          const SizedBox(height: 10),
          if (snapshot.asOfMs > 0)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Text(
                l10n.settingsQuotaProbedBy(
                  snapshot.probedByDeviceName.isEmpty
                      ? l10n.settingsUsageDeviceScopeLocal
                      : snapshot.probedByDeviceName,
                  formatShortTimestamp(snapshot.asOfMs),
                ),
                style: tabularTextStyle(
                  text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
                ),
              ),
            ),
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: [
              if (snapshot.protocol != core.ProviderBalanceProtocol.localCompute)
                BentoInteractivePill(
                  icon: isProbing ? Icons.hourglass_top_rounded : Icons.refresh,
                  label: l10n.settingsQuotaProbeSingle,
                  accent: colors.primary,
                  selected: !isProbing,
                  onTap: isProbing
                      ? null
                      : () => _probeOne(
                          snapshot.providerId,
                          targetNodeId: config?.preferredProbeNodeId,
                        ),
                ),
              if (onlinePeers.isNotEmpty &&
                  snapshot.protocol != core.ProviderBalanceProtocol.localCompute)
                PopupMenuButton<String>(
                  tooltip: l10n.settingsQuotaProbeViaNode,
                  onSelected: (nodeId) =>
                      _probeOne(snapshot.providerId, targetNodeId: nodeId),
                  itemBuilder: (context) => [
                    for (final peer in onlinePeers)
                      PopupMenuItem(
                        value: peer.deviceId,
                        child: Text(peer.deviceName),
                      ),
                  ],
                  child: BentoInteractivePill(
                    icon: Icons.hub_outlined,
                    trailingIcon: Icons.expand_more,
                    label: l10n.settingsQuotaProbeViaNode,
                  ),
                ),
              BentoInteractivePill(
                icon: Icons.tune,
                label: l10n.settingsQuotaProtocolConfig,
                onTap: () => _openProtocolDialog(snapshot, config, onlinePeers),
              ),
              BentoInteractivePill(
                icon: Icons.account_balance_wallet_outlined,
                label: budget == null
                    ? l10n.settingsQuotaSetBudget
                    : l10n.settingsQuotaEditBudget,
                onTap: () => _openBudgetDialog(snapshot, budget),
              ),
              if (snapshot.consoleUrl != null &&
                  snapshot.consoleUrl!.trim().isNotEmpty)
                BentoInteractivePill(
                  icon: Icons.open_in_new,
                  label: l10n.settingsQuotaOpenConsole,
                  onTap: () async {
                    final uri = Uri.tryParse(snapshot.consoleUrl!);
                    if (uri != null) {
                      await launchUrl(uri, mode: LaunchMode.externalApplication);
                    }
                  },
                ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildBalanceBody(
    BuildContext context,
    AppLocalizations l10n,
    ColorScheme colors,
    TextTheme text,
    core.QuotaBalanceBreakdown bal,
  ) {
    final cash = (bal.cashBalance ?? 0).clamp(0.0, double.infinity);
    final granted = (bal.grantedBalance ?? 0).clamp(0.0, double.infinity);
    final used = (bal.usedAmount ?? 0).clamp(0.0, double.infinity);
    final hasMixBar = (cash > 0 ? 1 : 0) + (granted > 0 ? 1 : 0) + (used > 0 ? 1 : 0) >= 2;

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: colors.surfaceContainerHighest.withValues(alpha: 0.34),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colors.outlineVariant.withValues(alpha: 0.25),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            bal.totalBalance != null
                ? formatCurrencyWithCode(bal.currency, bal.totalBalance!)
                : '—',
            style: tabularTextStyle(
              text.headlineSmall?.copyWith(
                fontWeight: FontWeight.w800,
                letterSpacing: -0.4,
              ),
            ),
          ),
          if (hasMixBar) ...[
            const SizedBox(height: 8),
            BentoStackedBar(
              height: 6,
              segments: [
                BentoBarSegment(value: cash, color: colors.primary),
                BentoBarSegment(value: granted, color: const Color(0xFF10B981)),
                BentoBarSegment(
                  value: used,
                  color: colors.outline.withValues(alpha: 0.55),
                ),
              ],
            ),
          ],
          const SizedBox(height: 6),
          Wrap(
            spacing: 12,
            runSpacing: 4,
            children: [
              if (bal.cashBalance != null)
                Text(
                  '${l10n.settingsQuotaCashLabel}: ${formatCurrencyWithCode(bal.currency, bal.cashBalance!)}',
                  style: tabularTextStyle(
                    text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
                  ),
                ),
              if (bal.grantedBalance != null)
                Text(
                  '${l10n.settingsQuotaGrantedLabel}: ${formatCurrencyWithCode(bal.currency, bal.grantedBalance!)}',
                  style: tabularTextStyle(
                    text.labelSmall?.copyWith(color: const Color(0xFF10B981)),
                  ),
                ),
              if (bal.usedAmount != null)
                Text(
                  '${l10n.settingsQuotaUsedLabel}: ${formatCurrencyWithCode(bal.currency, bal.usedAmount!)}',
                  style: tabularTextStyle(
                    text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
                  ),
                ),
              if (bal.dailyUsed != null)
                Text(
                  '${l10n.settingsQuotaDailyUsedLabel}: ${formatCurrencyWithCode(bal.currency, bal.dailyUsed!, decimals: 3)}',
                  style: tabularTextStyle(
                    text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
                  ),
                ),
              if (bal.monthlyUsed != null)
                Text(
                  '${l10n.settingsQuotaMonthlyUsedLabel}: ${formatCurrencyWithCode(bal.currency, bal.monthlyUsed!)}',
                  style: tabularTextStyle(
                    text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildQuotaWindowBar(
    BuildContext context,
    ColorScheme colors,
    TextTheme text,
    core.QuotaWindow window,
  ) {
    final pctUsed = (window.usedPercent ?? 0.0).clamp(0.0, 100.0);
    final barColor = pctUsed >= 85
        ? colors.error
        : pctUsed >= 60
        ? Colors.amber.shade600
        : const Color(0xFF10B981);
    final resetText = formatCountdownFromWindow(window);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                window.name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: text.bodySmall?.copyWith(fontWeight: FontWeight.w600),
              ),
            ),
            if (resetText.isNotEmpty) ...[
              Text(
                resetText,
                style: tabularTextStyle(
                  text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
                ),
              ),
              const SizedBox(width: 8),
            ],
            Text(
              window.display,
              style: tabularTextStyle(
                text.labelSmall?.copyWith(
                  color: barColor,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ],
        ),
        if (!window.unlimited && window.usedPercent != null) ...[
          const SizedBox(height: 5),
          BentoAnimatedProgressBar(
            value: (pctUsed / 100.0).clamp(0.0, 1.0),
            color: barColor,
            height: 7,
          ),
        ],
      ],
    );
  }

  Widget _buildLocalComputeBody(
    BuildContext context,
    AppLocalizations l10n,
    ColorScheme colors,
    TextTheme text,
    core.ProviderQuotaSnapshot snapshot,
  ) {
    final matching = widget.records.where(
      (r) =>
          r.providerId == snapshot.providerId ||
          (r.providerId.isEmpty && r.provider == snapshot.providerTypeId),
    );
    final speed = weightedAverageTokensPerSecond(matching);
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(11),
      decoration: BoxDecoration(
        color: colors.surfaceContainerHighest.withValues(alpha: 0.34),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: const Color(0xFF10B981).withValues(alpha: 0.25),
        ),
      ),
      child: Wrap(
        spacing: 12,
        runSpacing: 4,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          Text(
            l10n.settingsQuotaLocalZeroCost,
            style: text.labelSmall?.copyWith(
              color: const Color(0xFF10B981),
              fontWeight: FontWeight.w700,
            ),
          ),
          if (speed != null)
            Text(
              '${speed.toStringAsFixed(1)} tok/s',
              style: tabularTextStyle(
                text.labelSmall?.copyWith(fontWeight: FontWeight.w700),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildBudgetProgress(
    BuildContext context,
    AppLocalizations l10n,
    ColorScheme colors,
    TextTheme text,
    core.ProviderQuotaSnapshot snapshot,
    core.ProviderLocalBudget budget,
  ) {
    final spent = _computeCycleSpendForBudget(snapshot, budget);
    final pct = budget.monthlyBudget > 0
        ? (spent / budget.monthlyBudget * 100.0).clamp(0.0, 999.0)
        : 0.0;
    final alert = pct >= budget.alertThresholdPercent;
    final barColor = alert ? colors.error : colors.primary;

    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(top: 10),
      padding: const EdgeInsets.all(11),
      decoration: BoxDecoration(
        color: colors.surfaceContainerHighest.withValues(alpha: 0.3),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: barColor.withValues(alpha: 0.32)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  l10n.settingsQuotaBudgetProgress(
                    formatCurrencyWithCode(budget.currency, spent),
                    formatCurrencyWithCode(budget.currency, budget.monthlyBudget),
                  ),
                  style: tabularTextStyle(
                    text.labelSmall?.copyWith(fontWeight: FontWeight.w600),
                  ),
                ),
              ),
              Text(
                '${pct.toStringAsFixed(0)}%',
                style: tabularTextStyle(
                  text.labelSmall?.copyWith(
                    color: barColor,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 5),
          BentoAnimatedProgressBar(
            value: (pct / 100.0).clamp(0.0, 1.0),
            color: barColor,
            height: 7,
          ),
          if (alert) ...[
            const SizedBox(height: 4),
            Text(
              l10n.settingsQuotaBudgetExceeded,
              style: text.labelSmall?.copyWith(color: colors.error),
            ),
          ],
        ],
      ),
    );
  }

  double _computeCycleSpendForBudget(
    core.ProviderQuotaSnapshot snapshot,
    core.ProviderLocalBudget budget,
  ) {
    final now = DateTime.now();
    final day = budget.resetDayOfMonth.clamp(1, 28);
    final cycleStart = now.day >= day
        ? DateTime(now.year, now.month, day)
        : DateTime(now.year, now.month - 1, day);
    final cycleStartMs = cycleStart.millisecondsSinceEpoch;
    var sum = 0.0;
    for (final r in widget.records) {
      if (r.createdAtMs < cycleStartMs) continue;
      final matchesProvider = r.providerId == snapshot.providerId ||
          (r.providerId.isEmpty && r.provider == snapshot.providerTypeId);
      if (!matchesProvider) continue;
      final cost = widget.pricing.costForRecord(r);
      if (cost != null &&
          cost.currency.toUpperCase() == budget.currency.trim().toUpperCase()) {
        sum += cost.cost;
      }
    }
    return sum;
  }

  Widget _buildPoolKeyRow(
    BuildContext context,
    AppLocalizations l10n,
    ColorScheme colors,
    TextTheme text,
    core.KeyQuotaSnapshot keySnap,
  ) {
    final balText = keySnap.balance?.totalBalance != null
        ? formatCurrencyWithCode(
            keySnap.balance!.currency,
            keySnap.balance!.totalBalance!,
          )
        : (keySnap.error ?? _statusText(l10n, keySnap.status));
    return Container(
      margin: const EdgeInsets.only(top: 4),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: colors.surface,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Expanded(
            child: Text(
              keySnap.keyName != null && keySnap.keyName!.isNotEmpty
                  ? '${keySnap.keyName} (${keySnap.keyFingerprint})'
                  : keySnap.keyFingerprint,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: text.labelSmall,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            balText,
            style: text.labelSmall?.copyWith(fontWeight: FontWeight.w600),
          ),
        ],
      ),
    );
  }

  Widget _statusChip(
    BuildContext context,
    AppLocalizations l10n,
    ColorScheme colors,
    core.QuotaProbeStatus status,
  ) {
    final color = switch (status) {
      core.QuotaProbeStatus.ok => const Color(0xFF10B981),
      core.QuotaProbeStatus.unauthorized => colors.error,
      core.QuotaProbeStatus.error => colors.error,
      core.QuotaProbeStatus.stale => colors.onSurfaceVariant,
      core.QuotaProbeStatus.unsupported => colors.secondary,
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.13),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: color.withValues(alpha: 0.28)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          BentoStatusDot(
            color: color,
            glow: status == core.QuotaProbeStatus.ok,
            size: 6,
          ),
          const SizedBox(width: 5),
          Text(
            _statusText(l10n, status),
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: color,
              fontWeight: FontWeight.w700,
            ),
          ),
        ],
      ),
    );
  }

  String _statusText(AppLocalizations l10n, core.QuotaProbeStatus status) =>
      switch (status) {
        core.QuotaProbeStatus.ok => l10n.settingsQuotaStatusOk,
        core.QuotaProbeStatus.unauthorized => l10n.settingsQuotaStatusUnauthorized,
        core.QuotaProbeStatus.error => l10n.settingsQuotaStatusError,
        core.QuotaProbeStatus.stale => l10n.settingsQuotaStatusStale,
        core.QuotaProbeStatus.unsupported => l10n.settingsQuotaStatusUnsupported,
      };

  String _filterLabel(AppLocalizations l10n, QuotaCategoryFilter filter) =>
      switch (filter) {
        QuotaCategoryFilter.all => l10n.settingsQuotaFilterAll,
        QuotaCategoryFilter.balance => l10n.settingsQuotaFilterBalance,
        QuotaCategoryFilter.subscription => l10n.settingsQuotaFilterSubscription,
        QuotaCategoryFilter.localCompute => l10n.settingsQuotaFilterLocal,
        QuotaCategoryFilter.budgets => l10n.settingsQuotaFilterBudget,
      };

  Future<void> _openBudgetDialog(
    core.ProviderQuotaSnapshot snapshot,
    core.ProviderLocalBudget? existing,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final limitCtrl = TextEditingController(
      text: existing != null ? existing.monthlyBudget.toStringAsFixed(2) : '20.00',
    );
    final resetDayCtrl = TextEditingController(
      text: (existing?.resetDayOfMonth ?? 1).toString(),
    );
    final alertCtrl = TextEditingController(
      text: (existing?.alertThresholdPercent ?? 80.0).toStringAsFixed(0),
    );
    var currency = existing?.currency.isNotEmpty == true
        ? existing!.currency
        : 'USD';

    try {
      final action = await showDialog<String>(
        context: context,
        builder: (context) => StatefulBuilder(
          builder: (context, setDialogState) => AlertDialog(
            title: Text('${snapshot.providerName} · ${l10n.settingsQuotaSetBudget}'),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  TextField(
                    controller: limitCtrl,
                    keyboardType: const TextInputType.numberWithOptions(decimal: true),
                    decoration: InputDecoration(
                      labelText: l10n.settingsQuotaDialogBudgetLimit,
                    ),
                  ),
                  const SizedBox(height: 10),
                  DropdownButtonFormField<String>(
                    initialValue: currency,
                    items: const [
                      DropdownMenuItem(value: 'USD', child: Text('USD')),
                      DropdownMenuItem(value: 'CNY', child: Text('CNY')),
                    ],
                    onChanged: (v) {
                      if (v != null) setDialogState(() => currency = v);
                    },
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: resetDayCtrl,
                    keyboardType: TextInputType.number,
                    decoration: InputDecoration(
                      labelText: l10n.settingsQuotaDialogResetDay,
                    ),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: alertCtrl,
                    keyboardType: TextInputType.number,
                    decoration: InputDecoration(
                      labelText: l10n.settingsQuotaDialogAlertPct,
                    ),
                  ),
                ],
              ),
            ),
            actions: [
              if (existing != null)
                TextButton(
                  onPressed: () => Navigator.of(context).pop('delete'),
                  child: Text(l10n.settingsQuotaDialogClear),
                ),
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () => Navigator.of(context).pop('save'),
                child: Text(l10n.save),
              ),
            ],
          ),
        ),
      );

      if (action == 'delete') {
        await widget.clients.servicesProviderQuotaService
            .deleteProviderLocalBudget(providerId: snapshot.providerId);
        await widget.onStateUpdated();
      } else if (action == 'save') {
        final limit = double.tryParse(limitCtrl.text.trim()) ?? 0.0;
        final resetDay = int.tryParse(resetDayCtrl.text.trim()) ?? 1;
        final alertPct = double.tryParse(alertCtrl.text.trim()) ?? 80.0;
        await widget.clients.servicesProviderQuotaService.saveProviderLocalBudget(
          budget: core.ProviderLocalBudget(
            providerId: snapshot.providerId,
            monthlyBudget: limit.clamp(0.0, 1000000.0),
            totalBudget: existing?.totalBudget,
            currency: currency,
            resetDayOfMonth: resetDay.clamp(1, 28),
            alertThresholdPercent: alertPct.clamp(1.0, 100.0),
            updatedAtMs: DateTime.now().millisecondsSinceEpoch,
          ),
        );
        await widget.onStateUpdated();
      }
    } finally {
      limitCtrl.dispose();
      resetDayCtrl.dispose();
      alertCtrl.dispose();
    }
  }

  Future<void> _openProtocolDialog(
    core.ProviderQuotaSnapshot snapshot,
    core.ProviderQuotaProbeConfig? existing,
    List<core.RuntimeDeviceSpaceDevice> peers,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    var protocol = existing?.protocol ?? core.ProviderBalanceProtocol.auto;
    String? preferredNodeId = existing?.preferredProbeNodeId;
    final pathCtrl = TextEditingController(text: existing?.customPath ?? '');
    final amountPathCtrl = TextEditingController(
      text: existing?.customAmountJsonPath ?? 'data.balance',
    );
    final currencyPathCtrl = TextEditingController(
      text: existing?.customCurrencyJsonPath ?? '',
    );

    try {
      final saved = await showDialog<bool>(
        context: context,
        builder: (context) => StatefulBuilder(
          builder: (context, setDialogState) => AlertDialog(
            title: Text('${snapshot.providerName} · ${l10n.settingsQuotaProtocolConfig}'),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  DropdownButtonFormField<core.ProviderBalanceProtocol>(
                    initialValue: protocol,
                    isExpanded: true,
                    decoration: InputDecoration(
                      labelText: l10n.settingsQuotaDialogProtocolLabel,
                    ),
                    items: [
                      for (final item in core.ProviderBalanceProtocol.values)
                        DropdownMenuItem(
                          value: item,
                          child: Text(item.value),
                        ),
                    ],
                    onChanged: (v) {
                      if (v != null) setDialogState(() => protocol = v);
                    },
                  ),
                  if (peers.isNotEmpty) ...[
                    const SizedBox(height: 12),
                    DropdownButtonFormField<String?>(
                      initialValue: preferredNodeId,
                      isExpanded: true,
                      decoration: InputDecoration(
                        labelText: l10n.settingsQuotaProbeViaNode,
                      ),
                      items: [
                        DropdownMenuItem<String?>(
                          value: null,
                          child: Text(l10n.settingsUsageDeviceScopeLocal),
                        ),
                        for (final peer in peers)
                          DropdownMenuItem<String?>(
                            value: peer.deviceId,
                            child: Text(peer.deviceName),
                          ),
                      ],
                      onChanged: (v) => setDialogState(() => preferredNodeId = v),
                    ),
                  ],
                  if (protocol == core.ProviderBalanceProtocol.customHttpJson) ...[
                    const SizedBox(height: 12),
                    TextField(
                      controller: pathCtrl,
                      decoration: InputDecoration(
                        labelText: l10n.settingsQuotaDialogCustomPath,
                      ),
                    ),
                    const SizedBox(height: 10),
                    TextField(
                      controller: amountPathCtrl,
                      decoration: InputDecoration(
                        labelText: l10n.settingsQuotaDialogAmountJsonPath,
                      ),
                    ),
                    const SizedBox(height: 10),
                    TextField(
                      controller: currencyPathCtrl,
                      decoration: InputDecoration(
                        labelText: l10n.settingsQuotaDialogCurrencyJsonPath,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(l10n.save),
              ),
            ],
          ),
        ),
      );

      if (saved == true) {
        await widget.clients.servicesProviderQuotaService.saveProviderProbeConfig(
          config: core.ProviderQuotaProbeConfig(
            providerId: snapshot.providerId,
            protocol: protocol,
            customPath: pathCtrl.text.trim().isEmpty ? null : pathCtrl.text.trim(),
            customAmountJsonPath: amountPathCtrl.text.trim().isEmpty
                ? null
                : amountPathCtrl.text.trim(),
            customCurrencyJsonPath: currencyPathCtrl.text.trim().isEmpty
                ? null
                : currencyPathCtrl.text.trim(),
            customCurrency: existing?.customCurrency,
            preferredProbeNodeId: preferredNodeId,
          ),
        );
        await _probeOne(snapshot.providerId, targetNodeId: preferredNodeId);
      }
    } finally {
      pathCtrl.dispose();
      amountPathCtrl.dispose();
      currencyPathCtrl.dispose();
    }
  }
}
