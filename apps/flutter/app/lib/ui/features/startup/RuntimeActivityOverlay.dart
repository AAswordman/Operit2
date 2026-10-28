// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/logging/ClientLogger.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../l10n/generated/app_localizations.dart';

/// One global surface for package loading and the worker's visible large syncs.
class RuntimeActivityOverlay extends StatefulWidget {
  const RuntimeActivityOverlay({
    super.key,
    required this.enabled,
    this.clients = const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge()),
  });

  final bool enabled;
  final GeneratedCoreProxyClients clients;

  @override
  State<RuntimeActivityOverlay> createState() => _RuntimeActivityOverlayState();
}

class _RuntimeActivityOverlayState extends State<RuntimeActivityOverlay> {
  StreamSubscription<core.PluginLoadingProgress>? _packageSubscription;
  StreamSubscription<List<core.SpaceSyncProgress>>? _syncSubscription;
  StreamSubscription<core.RuntimeDeviceSpaceSnapshot>? _spaceSubscription;
  core.PluginLoadingProgress? _packages;
  core.RuntimeDeviceSpaceSnapshot? _space;
  List<core.SpaceSyncProgress> _sync = const [];
  Timer? _completionTimer;
  Timer? _spaceRetryTimer;
  int _spaceRetryAttempt = 0;
  int _subscriptionGeneration = 0;
  int _now = DateTime.now().millisecondsSinceEpoch;
  Offset _dockTopRight = const Offset(16, 16);
  bool _expanded = false;

  bool get _loadingPackages =>
      _packages?.visible == true && _packages?.phase == 'loading';
  List<core.SpaceSyncProgress> get _visibleSync => _sync
      .where(
        (item) =>
            (_space == null || item.spaceId == _space!.space.spaceId) &&
            (item.status != core.SpaceSyncStatus.completed ||
                _now < item.updatedAt + 3000),
      )
      .toList();

  @override
  void initState() {
    super.initState();
    if (widget.enabled) _subscribe();
  }

  @override
  void didUpdateWidget(RuntimeActivityOverlay oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.enabled != widget.enabled ||
        oldWidget.clients != widget.clients) {
      _cancel();
      _packages = null;
      _space = null;
      _sync = const [];
      _expanded = false;
      if (widget.enabled) _subscribe();
    }
  }

  void _cancel() {
    _subscriptionGeneration++;
    _spaceRetryTimer?.cancel();
    _spaceRetryTimer = null;
    _spaceRetryAttempt = 0;
    unawaited(_packageSubscription?.cancel());
    unawaited(_syncSubscription?.cancel());
    unawaited(_spaceSubscription?.cancel());
    _packageSubscription = null;
    _syncSubscription = null;
    _spaceSubscription = null;
    _completionTimer?.cancel();
  }

  @override
  void dispose() {
    _cancel();
    super.dispose();
  }

  bool _isCurrentSubscription(int generation) =>
      mounted && widget.enabled && generation == _subscriptionGeneration;

  void _reportSubscriptionError(Object error, StackTrace stackTrace) {
    ClientLogger.e(
      'Runtime activity subscription failed',
      tag: 'RuntimeActivity',
      error: error,
      stackTrace: stackTrace,
    );
  }

  void _subscribe() {
    final generation = _subscriptionGeneration;
    _packageSubscription = widget.clients.application
        .pluginLoadingProgressFlow()
        .listen((progress) {
          if (!_isCurrentSubscription(generation)) return;
          setState(() {
            _packages = progress;
            if (_loadingPackages && progress.forceExpanded) _expanded = true;
            if (!_loadingPackages && _visibleSync.isEmpty) _expanded = false;
          });
        }, onError: _reportSubscriptionError);
    final service = widget.clients.server.runtimeRemoteLinkService;
    _syncSubscription = service.spaceSyncProgressFlow().listen((progress) {
      if (!_isCurrentSubscription(generation)) return;
      setState(() {
        _sync = progress;
        _scheduleCompletion();
      });
    }, onError: _reportSubscriptionError);
    _subscribeSpace(generation);
  }

  /// A failed watch opening closes its stream; reopen it after a bounded delay.
  void _subscribeSpace(int generation) {
    if (!_isCurrentSubscription(generation)) return;
    try {
      _spaceSubscription = widget.clients.server.runtimeRemoteLinkService
          .deviceSpaceSnapshotFlow()
          .listen(
            (space) {
              if (!_isCurrentSubscription(generation)) return;
              _spaceRetryAttempt = 0;
              setState(() => _space = space);
            },
            onError: (Object error, StackTrace stackTrace) {
              if (!_isCurrentSubscription(generation)) return;
              _reportSubscriptionError(error, stackTrace);
              _retrySpaceSubscription(generation);
            },
            onDone: () => _retrySpaceSubscription(generation),
            cancelOnError: true,
          );
    } catch (error, stackTrace) {
      _reportSubscriptionError(error, stackTrace);
      _retrySpaceSubscription(generation);
    }
  }

  void _retrySpaceSubscription(int generation) {
    if (!_isCurrentSubscription(generation) || _spaceRetryTimer != null) return;
    final seconds = math.min(30, 1 << math.min(_spaceRetryAttempt, 5));
    _spaceRetryAttempt++;
    _spaceRetryTimer = Timer(Duration(seconds: seconds), () {
      _spaceRetryTimer = null;
      _subscribeSpace(generation);
    });
  }

  /// Only completion dismissal needs a timer. Small syncs never reach this UI.
  void _scheduleCompletion({int? now}) {
    _completionTimer?.cancel();
    _now = now ?? DateTime.now().millisecondsSinceEpoch;
    final deadlines = _visibleSync
        .where((item) => item.status == core.SpaceSyncStatus.completed)
        .map((item) => item.updatedAt + 3000);
    if (deadlines.isEmpty) {
      if (!_loadingPackages && _visibleSync.isEmpty) _expanded = false;
      return;
    }
    final deadline = deadlines.reduce(math.min);
    _completionTimer = Timer(Duration(milliseconds: deadline - _now), () {
      if (mounted) setState(() => _scheduleCompletion(now: deadline));
    });
  }

  String _peerName(core.SpaceSyncProgress item, AppLocalizations l10n) {
    for (final device
        in _space?.topology.devices ?? <core.RuntimeDeviceSpaceDevice>[]) {
      if (device.deviceId == item.peerNodeId) return device.deviceName;
    }
    return l10n.deviceSpaceSyncPeer;
  }

  _Activity _syncActivity(core.SpaceSyncProgress item, AppLocalizations l10n) {
    final completed = item.status == core.SpaceSyncStatus.completed;
    final waiting = item.status == core.SpaceSyncStatus.waiting;
    final file = item.stage == core.SpaceSyncStage.files && item.fileSize > 0;
    return _Activity(
      icon: completed
          ? Icons.check_rounded
          : waiting
          ? Icons.pause_rounded
          : Icons.sync_rounded,
      title: completed
          ? l10n.runtimeActivitySyncCompleted
          : waiting
          ? l10n.runtimeActivityWaiting
          : l10n.runtimeActivitySyncing,
      compactDetail: file && !completed
          ? '${_bytes(item.fileBytes)} / ${_bytes(item.fileSize)}'
          : null,
      detail: file && !completed
          ? l10n.deviceSpaceSyncCurrentFile(
              _bytes(item.fileBytes),
              _bytes(item.fileSize),
            )
          : l10n.deviceSpaceSyncTransferred(
              item.exchangedRecords,
              _bytes(item.transferredBytes),
            ),
      value: completed
          ? 1
          : file && !waiting
          ? (item.fileBytes / item.fileSize).clamp(0.0, 1.0)
          : null,
      paused: waiting || completed,
    );
  }

  String _stage(core.SpaceSyncProgress item, AppLocalizations l10n) {
    if (item.status == core.SpaceSyncStatus.waiting) {
      return item.retryAt == null
          ? l10n.deviceSpaceSyncPending
          : l10n.deviceSpaceSyncRetrying;
    }
    return switch (item.stage) {
      core.SpaceSyncStage.checking => l10n.deviceSpaceSyncChecking,
      core.SpaceSyncStage.records => l10n.deviceSpaceSyncRecords,
      core.SpaceSyncStage.files => l10n.deviceSpaceSyncFiles,
    };
  }

  @override
  Widget build(BuildContext context) {
    final sync = _visibleSync;
    if (!widget.enabled || (!_loadingPackages && sync.isEmpty)) {
      return const SizedBox.shrink();
    }
    final l10n = AppLocalizations.of(context)!;
    final packages = _loadingPackages ? _packages : null;
    final packageActivity = packages == null
        ? null
        : _Activity(
            icon: Icons.inventory_2_outlined,
            title: l10n.runtimeActivityLoadingPackages,
            detail:
                '${packages.pluginsStarted}/${packages.pluginsTotal} · ${packages.currentTask.isEmpty ? l10n.pluginLoadingWaiting : packages.currentTask}',
            value: packages.progress.clamp(0.0, 1.0),
          );
    final primary = sync.isNotEmpty
        ? _syncActivity(sync.first, l10n)
        : packageActivity!;
    final count = sync.length + (packages == null ? 0 : 1);
    return Positioned.fill(
      child: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final width = math.min(
              _expanded ? 360.0 : 232.0,
              math.max(0.0, constraints.maxWidth - 24),
            );
            final height = _expanded
                ? math.min(440.0, constraints.maxHeight)
                : 64.0;
            final maxRight = math.max(0.0, constraints.maxWidth - width);
            final maxTop = math.max(0.0, constraints.maxHeight - height);
            void drag(DragUpdateDetails details) => setState(() {
              _dockTopRight = Offset(
                (_dockTopRight.dx - details.delta.dx).clamp(0.0, maxRight),
                (_dockTopRight.dy + details.delta.dy).clamp(0.0, maxTop),
              );
            });
            return Stack(
              children: [
                Positioned(
                  right: _dockTopRight.dx.clamp(0.0, maxRight),
                  top: _dockTopRight.dy.clamp(0.0, maxTop),
                  child: _ActivitySurface(
                    child: SizedBox(
                      width: width,
                      child: _expanded
                          ? ConstrainedBox(
                              constraints: BoxConstraints(maxHeight: height),
                              child: Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  GestureDetector(
                                    onPanUpdate: drag,
                                    behavior: HitTestBehavior.opaque,
                                    child: Padding(
                                      padding: const EdgeInsets.only(
                                        left: 16,
                                        right: 4,
                                        top: 4,
                                      ),
                                      child: Row(
                                        children: [
                                          Expanded(
                                            child: Text(
                                              l10n.runtimeActivityTitle,
                                              style: Theme.of(
                                                context,
                                              ).textTheme.titleSmall,
                                            ),
                                          ),
                                          IconButton(
                                            tooltip: l10n.pluginLoadingCollapse,
                                            onPressed: () => setState(
                                              () => _expanded = false,
                                            ),
                                            icon: const Icon(
                                              Icons.keyboard_arrow_up_rounded,
                                              size: 20,
                                            ),
                                          ),
                                        ],
                                      ),
                                    ),
                                  ),
                                  Flexible(
                                    child: SingleChildScrollView(
                                      padding: const EdgeInsets.fromLTRB(
                                        16,
                                        0,
                                        16,
                                        12,
                                      ),
                                      child: Column(
                                        crossAxisAlignment:
                                            CrossAxisAlignment.stretch,
                                        children: [
                                          for (final item in sync) ...[
                                            _ActivityProgress(
                                              activity: _syncActivity(
                                                item,
                                                l10n,
                                              ),
                                              title:
                                                  item.status ==
                                                      core
                                                          .SpaceSyncStatus
                                                          .completed
                                                  ? l10n.deviceSpaceSyncCompleted(
                                                      _peerName(item, l10n),
                                                    )
                                                  : l10n.deviceSpaceSyncActive(
                                                      _peerName(item, l10n),
                                                    ),
                                            ),
                                            if (item.status !=
                                                core.SpaceSyncStatus.completed)
                                              Tooltip(
                                                message: item.error ?? '',
                                                child: Text(
                                                  _stage(item, l10n),
                                                  style: Theme.of(
                                                    context,
                                                  ).textTheme.bodySmall,
                                                ),
                                              ),
                                            if (item.fileSize > 0)
                                              Text(
                                                l10n.deviceSpaceSyncTransferred(
                                                  item.exchangedRecords,
                                                  _bytes(item.transferredBytes),
                                                ),
                                                style: Theme.of(
                                                  context,
                                                ).textTheme.bodySmall,
                                              ),
                                            const SizedBox(height: 12),
                                          ],
                                          if (packages != null) ...[
                                            if (sync.isNotEmpty)
                                              const Divider(height: 16),
                                            _ActivityProgress(
                                              activity: packageActivity!,
                                              trailing: TextButton(
                                                onPressed: () => unawaited(
                                                  widget.clients.application
                                                      .skipPluginLoading(),
                                                ),
                                                child: Text(
                                                  l10n.pluginLoadingSkip,
                                                ),
                                              ),
                                            ),
                                            for (final item in packages.plugins)
                                              _PluginLoadingRow(item: item),
                                          ],
                                        ],
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                            )
                          : Tooltip(
                              message: l10n.runtimeActivityExpand,
                              child: GestureDetector(
                                onPanUpdate: drag,
                                onTap: () => setState(() => _expanded = true),
                                child: Semantics(
                                  button: true,
                                  label: l10n.runtimeActivityExpand,
                                  child: SizedBox(
                                    height: 64,
                                    child: Column(
                                      children: [
                                        Expanded(
                                          child: Padding(
                                            padding: const EdgeInsets.symmetric(
                                              horizontal: 12,
                                            ),
                                            child: Row(
                                              children: [
                                                Container(
                                                  width: 32,
                                                  height: 32,
                                                  decoration: BoxDecoration(
                                                    color: Theme.of(context)
                                                        .colorScheme
                                                        .primary
                                                        .withValues(
                                                          alpha: 0.10,
                                                        ),
                                                    borderRadius:
                                                        BorderRadius.circular(
                                                          10,
                                                        ),
                                                  ),
                                                  child: Icon(
                                                    primary.icon,
                                                    size: 19,
                                                    color: Theme.of(
                                                      context,
                                                    ).colorScheme.primary,
                                                  ),
                                                ),
                                                const SizedBox(width: 10),
                                                Expanded(
                                                  child: Column(
                                                    mainAxisAlignment:
                                                        MainAxisAlignment
                                                            .center,
                                                    crossAxisAlignment:
                                                        CrossAxisAlignment
                                                            .start,
                                                    children: [
                                                      Text(
                                                        primary.title,
                                                        maxLines: 1,
                                                        overflow: TextOverflow
                                                            .ellipsis,
                                                        style: Theme.of(context)
                                                            .textTheme
                                                            .labelLarge
                                                            ?.copyWith(
                                                              fontWeight:
                                                                  FontWeight
                                                                      .w600,
                                                            ),
                                                      ),
                                                      const SizedBox(height: 2),
                                                      Text(
                                                        primary.compactDetail ??
                                                            primary.detail,
                                                        maxLines: 1,
                                                        overflow: TextOverflow
                                                            .ellipsis,
                                                        style: Theme.of(context)
                                                            .textTheme
                                                            .labelSmall
                                                            ?.copyWith(
                                                              color: Theme.of(context)
                                                                  .colorScheme
                                                                  .onSurfaceVariant,
                                                            ),
                                                      ),
                                                    ],
                                                  ),
                                                ),
                                                if (count > 1)
                                                  Text(
                                                    '+${count - 1}',
                                                    style: Theme.of(
                                                      context,
                                                    ).textTheme.labelSmall,
                                                  ),
                                                const SizedBox(width: 4),
                                                Icon(
                                                  Icons
                                                      .keyboard_arrow_down_rounded,
                                                  size: 18,
                                                  color: Theme.of(context)
                                                      .colorScheme
                                                      .onSurfaceVariant,
                                                ),
                                              ],
                                            ),
                                          ),
                                        ),
                                        if (!primary.paused)
                                          LinearProgressIndicator(
                                            value: primary.value,
                                            minHeight: 2.5,
                                          ),
                                      ],
                                    ),
                                  ),
                                ),
                              ),
                            ),
                    ),
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _Activity {
  const _Activity({
    required this.icon,
    required this.title,
    required this.detail,
    this.compactDetail,
    this.value,
    this.paused = false,
  });
  final IconData icon;
  final String title;
  final String detail;
  final String? compactDetail;
  final double? value;
  final bool paused;
}

class _ActivitySurface extends StatelessWidget {
  const _ActivitySurface({required this.child});
  final Widget child;
  @override
  Widget build(BuildContext context) => Material(
    elevation: 6,
    shadowColor: Theme.of(context).colorScheme.shadow.withValues(alpha: 0.18),
    color: Theme.of(context).colorScheme.surfaceContainerHigh,
    shape: RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(18),
      side: BorderSide(
        color: Theme.of(
          context,
        ).colorScheme.outlineVariant.withValues(alpha: 0.6),
      ),
    ),
    clipBehavior: Clip.antiAlias,
    child: child,
  );
}

class _ActivityProgress extends StatelessWidget {
  const _ActivityProgress({required this.activity, this.title, this.trailing});
  final _Activity activity;
  final String? title;
  final Widget? trailing;
  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 6),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                title ?? activity.title,
                style: Theme.of(context).textTheme.labelLarge,
              ),
            ),
            ?trailing,
          ],
        ),
        const SizedBox(height: 6),
        Text(activity.detail, style: Theme.of(context).textTheme.bodySmall),
        if (!activity.paused) ...[
          const SizedBox(height: 8),
          LinearProgressIndicator(value: activity.value, minHeight: 3),
        ],
      ],
    ),
  );
}

String _bytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
}

class _PluginLoadingRow extends StatelessWidget {
  const _PluginLoadingRow({required this.item});

  final core.PluginLoadingItem item;

  /// Builds one package or plugin row in the expanded task list.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final failed = item.status == 'failed';
    return InkWell(
      onTap: failed ? () => _showLogs(context) : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
        child: Row(
          children: <Widget>[
            _statusIcon(context),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    item.displayName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Text(
                    _subtitle(l10n),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: failed
                          ? colorScheme.error
                          : colorScheme.onSurfaceVariant,
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

  Widget _statusIcon(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return switch (item.status) {
      'loading' => SizedBox(
        width: 22,
        height: 22,
        child: CircularProgressIndicator(
          strokeWidth: 2,
          color: colorScheme.primary,
        ),
      ),
      'success' => Icon(
        Icons.check_circle,
        size: 22,
        color: colorScheme.primary,
      ),
      'failed' => Icon(Icons.error, size: 22, color: colorScheme.error),
      _ => Container(
        width: 10,
        height: 10,
        margin: const EdgeInsets.all(6),
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerHighest,
          shape: BoxShape.circle,
        ),
      ),
    };
  }

  String _subtitle(AppLocalizations l10n) {
    final kind = switch (item.kind) {
      'mcp' => l10n.pluginLoadingKindMcp,
      'plugin' => l10n.pluginLoadingKindPlugin,
      _ => l10n.pluginLoadingKindPackage,
    };
    final status = switch (item.status) {
      'loading' => l10n.pluginLoadingTitle,
      'success' => l10n.pluginLoadingSuccess,
      'failed' =>
        item.message.isEmpty ? l10n.pluginLoadingFailed : item.message,
      _ => l10n.pluginLoadingWaiting,
    };
    return '$kind · $status';
  }

  void _showLogs(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    unawaited(
      showDialog<void>(
        context: context,
        builder: (dialogContext) {
          return AlertDialog(
            title: Text(item.displayName),
            content: SingleChildScrollView(
              child: SelectableText(
                item.logText.trim().isEmpty
                    ? l10n.pluginLoadingNoLogs
                    : item.logText,
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: Text(l10n.cancel),
              ),
            ],
          );
        },
      ),
    );
  }
}
