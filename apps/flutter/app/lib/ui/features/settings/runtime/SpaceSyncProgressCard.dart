// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/PageActivityMixin.dart';
import '../../../theme/OperitGlassSurface.dart';

/// The worker publishes only large syncs. This view only retains their final
/// result briefly; it does not estimate work or observe small exchanges.
class SpaceSyncProgressCard extends StatefulWidget {
  const SpaceSyncProgressCard({
    super.key,
    required this.progress,
    required this.spaceId,
    required this.deviceNames,
  });

  final List<generated.SpaceSyncProgress> progress;
  final String? spaceId;
  final Map<String, String> deviceNames;

  @override
  State<SpaceSyncProgressCard> createState() => _SpaceSyncProgressCardState();
}

class _SpaceSyncProgressCardState extends State<SpaceSyncProgressCard>
    with PageActivityMixin<SpaceSyncProgressCard> {
  static const _completionHoldMs = 3000;
  Timer? _timer;
  int _now = DateTime.now().millisecondsSinceEpoch;

  @override
  void onPageActivityChanged(bool active) {
    _timer?.cancel();
    if (active) _schedule();
  }

  @override
  void didUpdateWidget(SpaceSyncProgressCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (isPageActive) _schedule();
  }

  bool _visible(generated.SpaceSyncProgress item) {
    if (item.spaceId != widget.spaceId) return false;
    return item.status != generated.SpaceSyncStatus.completed ||
        _now < item.updatedAt + _completionHoldMs;
  }

  /// Schedule completion dismissal only; progress arrives from the worker.
  void _schedule({int? now}) {
    _timer?.cancel();
    _now = now ?? DateTime.now().millisecondsSinceEpoch;
    int? next;
    for (final item in widget.progress) {
      if (item.spaceId != widget.spaceId) continue;
      final deadline = item.status == generated.SpaceSyncStatus.completed
          ? item.updatedAt + _completionHoldMs
          : null;
      if (deadline != null && deadline > _now) {
        if (next == null || deadline < next) next = deadline;
      }
    }
    if (next != null) {
      final deadline = next;
      _timer = Timer(Duration(milliseconds: deadline - _now), () {
        if (!mounted || !isPageActive) return;
        setState(() => _schedule(now: deadline));
      });
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    final items = widget.progress.where(_visible);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final item in items)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: OperitGlassSurface(
              material: true,
              color: colors.surfaceContainerHighest.withValues(alpha: 0.36),
              borderRadius: BorderRadius.circular(12),
              child: Padding(
                padding: const EdgeInsets.all(14),
                child: _progressContent(context, l10n, item),
              ),
            ),
          ),
      ],
    );
  }

  Widget _progressContent(
    BuildContext context,
    AppLocalizations l10n,
    generated.SpaceSyncProgress item,
  ) {
    final completed = item.status == generated.SpaceSyncStatus.completed;
    final waiting = item.status == generated.SpaceSyncStatus.waiting;
    final name =
        widget.deviceNames[item.peerNodeId] ?? l10n.deviceSpaceSyncPeer;
    final title = completed
        ? l10n.deviceSpaceSyncCompleted(name)
        : l10n.deviceSpaceSyncActive(name);
    final stage = waiting
        ? (item.retryAt != null
              ? l10n.deviceSpaceSyncRetrying
              : l10n.deviceSpaceSyncPending)
        : switch (item.stage) {
            generated.SpaceSyncStage.checking => l10n.deviceSpaceSyncChecking,
            generated.SpaceSyncStage.records => l10n.deviceSpaceSyncRecords,
            generated.SpaceSyncStage.files => l10n.deviceSpaceSyncFiles,
          };
    final hasFile =
        !waiting &&
        !completed &&
        item.stage == generated.SpaceSyncStage.files &&
        item.fileSize > 0;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(
              completed
                  ? Icons.check_circle_outline
                  : waiting
                  ? Icons.pause_circle_outline
                  : Icons.sync,
              size: 20,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(title, style: Theme.of(context).textTheme.titleSmall),
            ),
          ],
        ),
        if (!completed) ...[
          const SizedBox(height: 8),
          if (item.error case final error?)
            Tooltip(message: error, child: Text(stage))
          else
            Text(stage),
          if (!waiting) ...[
            const SizedBox(height: 8),
            LinearProgressIndicator(
              value: hasFile
                  ? (item.fileBytes / item.fileSize).clamp(0.0, 1.0)
                  : null,
            ),
          ],
          if (hasFile) ...[
            const SizedBox(height: 6),
            Text(
              l10n.deviceSpaceSyncCurrentFile(
                _bytes(item.fileBytes),
                _bytes(item.fileSize),
              ),
            ),
          ],
        ],
        const SizedBox(height: 6),
        Text(
          l10n.deviceSpaceSyncTransferred(
            item.exchangedRecords,
            _bytes(item.transferredBytes),
          ),
          style: Theme.of(context).textTheme.bodySmall,
        ),
        if (!completed) ...[
          const SizedBox(height: 4),
          Text(
            l10n.deviceSpaceSyncBackground,
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ],
    );
  }
}

String _bytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
}
