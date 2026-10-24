// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../l10n/generated/app_localizations.dart';

class PluginLoadingOverlay extends StatefulWidget {
  /// Creates the process-wide package and plugin loading overlay.
  const PluginLoadingOverlay({super.key, required this.enabled});

  final bool enabled;

  /// Creates the overlay state.
  @override
  State<PluginLoadingOverlay> createState() => _PluginLoadingOverlayState();
}

class _PluginLoadingOverlayState extends State<PluginLoadingOverlay> {
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );
  static const double _collapsedWidth = 96;
  static const double _collapsedHeight = 44;
  static const double _expandedWidth = 360;
  static const double _dragSlop = 8;

  StreamSubscription<core_proxy.PluginLoadingProgress>? _subscription;
  core_proxy.PluginLoadingProgress? _progress;
  Offset _dockTopRight = const Offset(16, 16);
  bool _expanded = false;
  bool _dragMoved = false;
  double _dragDistance = 0;

  /// Subscribes to native loading progress when the runtime is ready.
  @override
  void initState() {
    super.initState();
    if (widget.enabled) {
      _subscribe();
    }
  }

  /// Starts or stops the progress subscription when runtime readiness changes.
  @override
  void didUpdateWidget(covariant PluginLoadingOverlay oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.enabled == widget.enabled) {
      return;
    }
    if (widget.enabled) {
      _subscribe();
    } else {
      unawaited(_subscription?.cancel());
      _subscription = null;
    }
  }

  /// Cancels the native loading progress subscription.
  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    super.dispose();
  }

  /// Subscribes to the runtime package and plugin loading progress flow.
  void _subscribe() {
    if (_subscription != null) {
      return;
    }
    _subscription = _clients.application.pluginLoadingProgressFlow().listen((
      progress,
    ) {
      if (!mounted) {
        return;
      }
      setState(() {
        _progress = progress;
        if (progress.forceExpanded) {
          _expanded = true;
        }
        if (!progress.visible) {
          _expanded = false;
        }
      });
    });
  }

  /// Hides the overlay without stopping the in-flight load session.
  void _skip() {
    unawaited(_clients.application.skipPluginLoading());
  }

  /// Moves the overlay by one pointer delta, clamped to the visible screen.
  void _applyDragDelta(Offset delta, double maxRight, double maxTop) {
    setState(() {
      _dockTopRight = Offset(
        (_dockTopRight.dx - delta.dx).clamp(0.0, maxRight).toDouble(),
        (_dockTopRight.dy + delta.dy).clamp(0.0, maxTop).toDouble(),
      );
    });
  }

  /// Builds the draggable Gradle-style loading overlay.
  @override
  Widget build(BuildContext context) {
    final progress = _progress;
    if (!widget.enabled ||
        progress == null ||
        !progress.visible ||
        progress.phase != 'loading') {
      return const SizedBox.shrink();
    }
    return Positioned.fill(
      child: LayoutBuilder(
        builder: (context, constraints) {
          final expanded = _expanded;
          final panelWidth = expanded
              ? constraints.maxWidth.clamp(0.0, _expandedWidth).toDouble()
              : _collapsedWidth;
          final panelHeight = expanded
              ? constraints.maxHeight.clamp(0.0, 420.0).toDouble()
              : _collapsedHeight;
          final maxRight = (constraints.maxWidth - panelWidth)
              .clamp(0.0, double.infinity)
              .toDouble();
          final maxTop = (constraints.maxHeight - panelHeight)
              .clamp(0.0, double.infinity)
              .toDouble();
          final right = _dockTopRight.dx.clamp(0.0, maxRight).toDouble();
          final top = _dockTopRight.dy.clamp(0.0, maxTop).toDouble();
          return Stack(
            children: <Widget>[
              Positioned(
                right: right,
                top: top,
                child: expanded
                    ? _ExpandedLoadingPanel(
                        progress: progress,
                        width: panelWidth,
                        maxHeight: panelHeight,
                        onCollapse: () => setState(() => _expanded = false),
                        onSkip: _skip,
                        onHeaderDragDelta: (delta) {
                          _applyDragDelta(delta, maxRight, maxTop);
                        },
                      )
                    : Listener(
                        behavior: HitTestBehavior.opaque,
                        onPointerDown: (_) {
                          _dragMoved = false;
                          _dragDistance = 0;
                        },
                        onPointerMove: (event) {
                          _dragDistance += event.delta.distance;
                          if (_dragDistance < _dragSlop) {
                            return;
                          }
                          _dragMoved = true;
                          _applyDragDelta(event.delta, maxRight, maxTop);
                        },
                        onPointerUp: (_) {
                          if (!_dragMoved) {
                            setState(() {
                              _expanded = true;
                            });
                          }
                          _dragMoved = false;
                          _dragDistance = 0;
                        },
                        child: _CollapsedLoadingChip(progress: progress),
                      ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _CollapsedLoadingChip extends StatelessWidget {
  const _CollapsedLoadingChip({required this.progress});

  final core_proxy.PluginLoadingProgress progress;

  /// Builds the compact determinate Gradle-style progress chip.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final value = progress.progress.clamp(0.0, 1.0);
    return Material(
      elevation: 8,
      color: colorScheme.surfaceContainerHigh,
      borderRadius: BorderRadius.circular(22),
      shadowColor: colorScheme.shadow,
      child: SizedBox(
        width: 96,
        height: 44,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10),
          child: Row(
            children: <Widget>[
              SizedBox(
                width: 22,
                height: 22,
                child: CircularProgressIndicator(
                  value: value,
                  strokeWidth: 2.4,
                  color: colorScheme.primary,
                  backgroundColor: colorScheme.surfaceContainerHighest,
                ),
              ),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  '${progress.pluginsStarted}/${progress.pluginsTotal}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.w800,
                    color: colorScheme.onSurface,
                  ),
                ),
              ),
              Icon(
                Icons.drag_indicator,
                size: 16,
                color: colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ExpandedLoadingPanel extends StatelessWidget {
  const _ExpandedLoadingPanel({
    required this.progress,
    required this.width,
    required this.maxHeight,
    required this.onCollapse,
    required this.onSkip,
    required this.onHeaderDragDelta,
  });

  final core_proxy.PluginLoadingProgress progress;
  final double width;
  final double maxHeight;
  final VoidCallback onCollapse;
  final VoidCallback onSkip;
  final ValueChanged<Offset> onHeaderDragDelta;

  /// Builds the expanded Gradle-style task list panel.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final value = progress.progress.clamp(0.0, 1.0);
    return Material(
      elevation: 10,
      color: colorScheme.surfaceContainerHigh,
      borderRadius: BorderRadius.circular(16),
      shadowColor: colorScheme.shadow,
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: width, maxHeight: maxHeight),
        child: SizedBox(
          width: width,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(12, 4, 8, 12),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Row(
                  children: <Widget>[
                    IconButton(
                      tooltip: l10n.pluginLoadingCollapse,
                      visualDensity: VisualDensity.compact,
                      onPressed: onCollapse,
                      icon: const Icon(Icons.keyboard_arrow_down),
                    ),
                    Expanded(
                      child: Listener(
                        behavior: HitTestBehavior.opaque,
                        onPointerMove: (event) {
                          if (event.delta == Offset.zero) {
                            return;
                          }
                          onHeaderDragDelta(event.delta);
                        },
                        child: Padding(
                          padding: const EdgeInsets.symmetric(vertical: 12),
                          child: Text(
                            l10n.pluginLoadingTitle,
                            style: textTheme.titleSmall?.copyWith(
                              fontWeight: FontWeight.w800,
                            ),
                          ),
                        ),
                      ),
                    ),
                    TextButton(
                      onPressed: onSkip,
                      child: Text(l10n.pluginLoadingSkip),
                    ),
                  ],
                ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(999),
                    child: LinearProgressIndicator(
                      value: value,
                      minHeight: 8,
                      backgroundColor: colorScheme.surfaceContainerHighest,
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  child: Text(
                    _statusMessage(l10n, progress),
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurface,
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(8, 4, 8, 8),
                  child: Text(
                    l10n.pluginLoadingStatus(
                      progress.pluginsStarted,
                      progress.pluginsTotal,
                    ),
                    style: textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                if (progress.plugins.isNotEmpty)
                  Flexible(
                    child: ListView.builder(
                      shrinkWrap: true,
                      padding: const EdgeInsets.symmetric(horizontal: 4),
                      itemCount: progress.plugins.length,
                      itemBuilder: (context, index) {
                        final item = progress.plugins[index];
                        return _PluginLoadingRow(item: item);
                      },
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  String _statusMessage(
    AppLocalizations l10n,
    core_proxy.PluginLoadingProgress progress,
  ) {
    if (progress.phase == 'complete_success') {
      return l10n.pluginLoadingCompleteSuccess;
    }
    if (progress.phase == 'complete_with_failures') {
      return l10n.pluginLoadingCompleteWithFailures;
    }
    if (progress.currentTask.isNotEmpty) {
      return l10n.pluginLoadingCurrentTask(progress.currentTask);
    }
    return l10n.pluginLoadingTitle;
  }
}

class _PluginLoadingRow extends StatelessWidget {
  const _PluginLoadingRow({required this.item});

  final core_proxy.PluginLoadingItem item;

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
      'failed' => item.message.isEmpty
          ? l10n.pluginLoadingFailed
          : item.message,
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
