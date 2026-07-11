// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../../theme/OperitGlassSurface.dart';
import '../WorkspaceBrowserSessionStore.dart';
import '../tabs/WorkspaceBrowserTabModels.dart';

class WorkspaceBrowserSurfaceAttachment extends ChangeNotifier {
  /// Creates the singleton attachment state.
  WorkspaceBrowserSurfaceAttachment._();

  static final WorkspaceBrowserSurfaceAttachment instance =
      WorkspaceBrowserSurfaceAttachment._();

  final LayerLink layerLink = LayerLink();

  Object? _owner;
  Size _size = Size.zero;
  bool _notifyScheduled = false;

  bool get isAttached => _owner != null;
  Size get size => _size;

  /// Attaches the persistent browser surface to a workspace target.
  void attach({required Object owner, required Size size}) {
    final changed = !identical(_owner, owner) || _size != size;
    _owner = owner;
    _size = size;
    if (changed) {
      _scheduleNotifyListeners();
    }
  }

  /// Detaches the workspace target without disposing browser sessions.
  void detach(Object owner) {
    if (!identical(_owner, owner)) {
      return;
    }
    _owner = null;
    _scheduleNotifyListeners();
  }

  /// Schedules listener delivery after widget tree mutations finish.
  void _scheduleNotifyListeners() {
    if (_notifyScheduled) {
      return;
    }
    _notifyScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _notifyScheduled = false;
      notifyListeners();
    });
    WidgetsBinding.instance.ensureVisualUpdate();
  }
}

class WorkspaceBrowserSurfaceTarget extends StatefulWidget {
  /// Creates a workspace target for the persistent browser surface.
  const WorkspaceBrowserSurfaceTarget({super.key});

  /// Creates the workspace attachment target state.
  @override
  State<WorkspaceBrowserSurfaceTarget> createState() =>
      _WorkspaceBrowserSurfaceTargetState();
}

class _WorkspaceBrowserSurfaceTargetState
    extends State<WorkspaceBrowserSurfaceTarget> {
  final WorkspaceBrowserSurfaceAttachment _attachment =
      WorkspaceBrowserSurfaceAttachment.instance;

  /// Detaches the surface when the workspace view is removed.
  @override
  void dispose() {
    _attachment.detach(this);
    super.dispose();
  }

  /// Reports the workspace viewport and exposes its compositing anchor.
  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final size = constraints.biggest;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted) {
            return;
          }
          _attachment.attach(owner: this, size: size);
        });
        return CompositedTransformTarget(
          link: _attachment.layerLink,
          child: const SizedBox.expand(),
        );
      },
    );
  }
}

class WorkspaceBrowserSurfaceHost extends StatelessWidget {
  /// Creates the application-level host for persistent browser surfaces.
  const WorkspaceBrowserSurfaceHost({super.key, required this.child});

  final Widget child;

  /// Keeps browser platform views mounted independently from workspace views.
  @override
  Widget build(BuildContext context) {
    final sessionStore = WorkspaceBrowserSessionStore.instance;
    final attachment = WorkspaceBrowserSurfaceAttachment.instance;
    return Stack(
      fit: StackFit.expand,
      children: <Widget>[
        child,
        AnimatedBuilder(
          animation: sessionStore,
          builder: (context, child) {
            final tabs = sessionStore.tabs;
            return AnimatedBuilder(
              animation: Listenable.merge(<Listenable>[attachment, ...tabs]),
              builder: (context, child) {
                return _WorkspaceBrowserPersistentSurface(
                  tabs: tabs,
                  currentTab: sessionStore.currentTab,
                  attachment: attachment,
                );
              },
            );
          },
        ),
      ],
    );
  }
}

class _WorkspaceBrowserPersistentSurface extends StatelessWidget {
  /// Creates the persistent surface renderer.
  const _WorkspaceBrowserPersistentSurface({
    required this.tabs,
    required this.currentTab,
    required this.attachment,
  });

  final List<WorkspaceBrowserTabState> tabs;
  final WorkspaceBrowserTabState? currentTab;
  final WorkspaceBrowserSurfaceAttachment attachment;

  /// Builds every browser surface while painting only the attached session.
  @override
  Widget build(BuildContext context) {
    final currentTab = this.currentTab;
    final size = attachment.size;
    final width = size.width > 0 ? size.width : 1.0;
    final height = size.height > 0 ? size.height : 1.0;
    return Positioned(
      left: 0,
      top: 0,
      width: width,
      height: height,
      child: CompositedTransformFollower(
        link: attachment.layerLink,
        showWhenUnlinked: false,
        targetAnchor: Alignment.topLeft,
        followerAnchor: Alignment.topLeft,
        child: Stack(
          fit: StackFit.expand,
          children: <Widget>[
            for (final tab in tabs)
              Offstage(
                offstage: !attachment.isAttached || currentTab?.id != tab.id,
                child: WebViewWidget(
                  key: ValueKey<String>('workspace-browser-surface-${tab.id}'),
                  controller: tab.controller,
                ),
              ),
            if (attachment.isAttached &&
                currentTab != null &&
                currentTab.errorText != null)
              _WorkspaceBrowserErrorOverlay(
                message: currentTab.errorText!,
                onRetry: currentTab.controller.reload,
              ),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceBrowserErrorOverlay extends StatelessWidget {
  /// Creates an error overlay for the active browser surface.
  const _WorkspaceBrowserErrorOverlay({
    required this.message,
    required this.onRetry,
  });

  final String message;
  final VoidCallback onRetry;

  /// Displays a retry action above the attached browser surface.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: OperitGlassSurface(
          color: theme.colorScheme.surfaceContainerHighest.withValues(
            alpha: 0.42,
          ),
          layer: OperitGlassSurfaceLayer.card,
          borderRadius: BorderRadius.circular(18),
          border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.2),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 20),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Icon(
                  Icons.error_outline,
                  size: 42,
                  color: theme.colorScheme.error,
                ),
                const SizedBox(height: 12),
                Text(
                  l10n.pageLoadFailed,
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  message,
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodySmall,
                ),
                const SizedBox(height: 16),
                FilledButton.icon(
                  onPressed: onRetry,
                  icon: const Icon(Icons.refresh),
                  label: Text(l10n.retry),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
