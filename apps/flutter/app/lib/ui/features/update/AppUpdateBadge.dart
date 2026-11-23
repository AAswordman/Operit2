// ignore_for_file: file_names
import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/update/AppUpdateController.dart';
import '../../../l10n/generated/app_localizations.dart';
import '../../../core/update/AppUpdateModels.dart';
import 'AppUpdateDialog.dart';

/// Allows one controller to follow all sidebar presentations, including previews.
class AppUpdateScope extends InheritedWidget {
  const AppUpdateScope({
    super.key,
    required this.controller,
    required super.child,
  });
  final AppUpdateController controller;
  static AppUpdateController? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<AppUpdateScope>()?.controller;
  @override
  bool updateShouldNotify(AppUpdateScope oldWidget) =>
      controller != oldWidget.controller;
}

/// Keeps the version entry attached to the brand in both sidebar layouts.
class AppUpdateBadge extends StatefulWidget {
  const AppUpdateBadge({
    super.key,
    required this.child,
    this.controller,
    this.showIndicator = true,
  });
  final Widget child;
  final AppUpdateController? controller;
  final bool showIndicator;

  @override
  State<AppUpdateBadge> createState() => _AppUpdateBadgeState();
}

class _AppUpdateBadgeState extends State<AppUpdateBadge> {
  late final AppLifecycleListener _lifecycle;
  AppUpdateController? _scopedController;
  AppUpdateController? _listenedController;
  bool _promptScheduled = false;
  AppUpdateController get _controller =>
      widget.controller ?? _scopedController ?? AppUpdateController.instance;

  @override
  void initState() {
    super.initState();
    _lifecycle = AppLifecycleListener(
      onResume: () {
        unawaited(_controller.check());
        _scheduleUpdatePrompt();
      },
    );
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _scopedController = AppUpdateScope.maybeOf(context);
    _bindController();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) unawaited(_controller.check());
    });
  }

  @override
  void didUpdateWidget(AppUpdateBadge oldWidget) {
    super.didUpdateWidget(oldWidget);
    _bindController();
  }

  void _bindController() {
    final controller = _controller;
    if (_listenedController != controller) {
      _listenedController?.removeListener(_scheduleUpdatePrompt);
      _listenedController = controller;
      controller.addListener(_scheduleUpdatePrompt);
    }
    _scheduleUpdatePrompt();
  }

  /// Waits for a mounted navigator and coalesces progress and layout notifications.
  void _scheduleUpdatePrompt() {
    if (_promptScheduled || !_controller.hasUnpromptedUpdate) return;
    _promptScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _promptScheduled = false;
      if (!mounted || Navigator.maybeOf(context) == null) return;
      unawaited(
        showAppUpdateDialog(context, controller: _controller, automatic: true),
      );
    });
    WidgetsBinding.instance.ensureVisualUpdate();
  }

  @override
  void dispose() {
    _listenedController?.removeListener(_scheduleUpdatePrompt);
    _lifecycle.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: _controller,
    child: widget.child,
    builder: (context, child) {
      if (!_controller.visible) return child!;
      final updateEntry = InkWell(
        key: const ValueKey('app-update-logo'),
        borderRadius: BorderRadius.circular(16),
        onTap: () =>
            unawaited(showAppUpdateDialog(context, controller: _controller)),
        child: child,
      );
      if (!widget.showIndicator) return updateEntry;
      final l10n = AppLocalizations.of(context)!;
      final colors = Theme.of(context).colorScheme;
      final busy = _controller.busy;
      final failed = _controller.phase == AppUpdatePhase.failed;
      return Stack(
        clipBehavior: Clip.none,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2, right: 22),
            child: updateEntry,
          ),
          Positioned(
            top: 0,
            right: 0,
            child: SizedBox(
              width: 24,
              height: 24,
              child: IconButton(
                key: const ValueKey('app-update-badge'),
                tooltip: busy
                    ? l10n.appUpdateViewProgress
                    : failed
                    ? l10n.appUpdateFailedRetry
                    : l10n.appUpdateBadgeAvailable(_controller.release!.version),
                padding: const EdgeInsets.all(4),
                style: IconButton.styleFrom(
                  foregroundColor: failed ? colors.error : colors.primary,
                  minimumSize: const Size(24, 24),
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
                onPressed: () => unawaited(
                  showAppUpdateDialog(context, controller: _controller),
                ),
                icon: busy
                    ? SizedBox.square(
                        dimension: 14,
                        child: CircularProgressIndicator(
                          strokeWidth: 1.5,
                          value: _controller.progress,
                          color: colors.primary,
                        ),
                      )
                    : Icon(
                        failed
                            ? Icons.error_outline_rounded
                            : Icons.arrow_circle_up_outlined,
                        size: 16,
                      ),
              ),
            ),
          ),
        ],
      );
    },
  );
}
