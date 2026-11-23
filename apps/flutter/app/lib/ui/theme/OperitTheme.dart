// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:fvp/fvp.dart' show FvpCreateCancelledException;
import 'package:video_player/video_player.dart';

import '../../core/errors/UnhandledErrorReporter.dart';
import '../../core/host/AppToastController.dart';
import '../../core/logging/ClientLogger.dart';
import '../../core/runtime/RuntimeBootstrapManager.dart';
import '../../data/preferences/UserPreferencesManager.dart';
import '../../l10n/generated/app_localizations.dart';
import '../common/AppToastHost.dart';
import '../common/MacNoticeScaffoldMessenger.dart';
import '../common/RuntimeBootstrapScreen.dart';
import '../common/layout/ApplicationZoom.dart';
import '../common/layout/ApplicationZoomPreferences.dart';
import '../features/chat/tts/TtsFloatingPanel.dart';
import '../features/startup/RuntimeActivityOverlay.dart';
import '../../core/host/browser/RuntimeBrowserOwnerHost.dart';
import '../window/DesktopWindowChrome.dart';
import 'OperitThemeAssets.dart';
import 'ThemeAppearanceErrorScreen.dart';
import 'ThemeCircularRevealHost.dart';
import 'ThemeVideoAssetLoader.dart';

class OperitTheme extends StatefulWidget {
  /// Creates one themed Flutter application root.
  const OperitTheme({
    super.key,
    required this.child,
    required this.initialThemePreferenceSnapshot,
    this.initialThemeMode,
    required this.initialThemeIsReady,
    this.hostInteractionHostsEnabled = true,
    this.unconfiguredChildEnabled = false,
  });

  final Widget child;
  final ThemePreferenceSnapshot initialThemePreferenceSnapshot;
  final ThemeMode? initialThemeMode;
  final bool initialThemeIsReady;
  final bool hostInteractionHostsEnabled;
  final bool unconfiguredChildEnabled;

  static OperitThemeController of(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<_OperitThemeScope>();
    if (scope == null) {
      throw StateError('OperitTheme scope not found');
    }
    return scope.controller;
  }

  @override
  State<OperitTheme> createState() => _OperitThemeState();
}

class _OperitThemeState extends State<OperitTheme> {
  final RuntimeBootstrapManager _runtimeManager =
      RuntimeBootstrapManager.instance;
  final ApplicationZoomPreferences _zoomPreferences =
      const ApplicationZoomPreferences();
  final ThemeAssetStore _themeAssets = ThemeAssetStore();
  final ThemeAssetDiagnostics _assetDiagnostics = ThemeAssetDiagnostics();
  ThemePreferenceSnapshot? _assetSnapshot;
  int _assetRevision = 0;
  double _zoom = 1.0;
  Future<void> _zoomSave = Future<void>.value();
  StreamSubscription<LongPastedTextInputSettings>? _longPasteSubscription;
  StreamSubscription<bool?>? _enterSendsMessageSubscription;
  StreamSubscription<bool>? _chatScrollNavigatorOnLeftSubscription;
  late final OperitThemeController _controller = OperitThemeController(
    onChanged: () {
      if (mounted) {
        _syncAssetContext();
        setState(() {});
      }
    },
  );
  Future<void>? _runtimeStartFuture;
  Object? _runtimeStartupError;
  bool _runtimeUiReady = false;
  bool _runtimeStartCompleted = false;
  bool _preserveUnconfiguredChild = false;
  int _runtimeGeneration = 0;
  Timer? _runtimeStartupStatusTimer;
  String _runtimeStartupMessage = '正在准备本地运行时';

  static const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');

  /// Subscribes to runtime readiness and starts theme services when configured.
  @override
  void initState() {
    super.initState();
    _runtimeManager.addListener(_handleRuntimeBootstrapChanged);
    _preserveUnconfiguredChild =
        widget.unconfiguredChildEnabled && !_runtimeManager.runtimeConfigured;
    _controller.setInitialThemePreferenceSnapshot(
      widget.initialThemePreferenceSnapshot,
    );
    _syncAssetContext();
    // Each engine prepares its synchronized appearance before mounting the app.
    _runtimeUiReady = false;
    if (_runtimeManager.runtimeConfigured) {
      unawaited(_startRuntimeUi());
    }
  }

  /// Releases runtime readiness and theme subscriptions.
  @override
  void dispose() {
    _stopRuntimeStartupStatusPolling();
    _runtimeManager.removeListener(_handleRuntimeBootstrapChanged);
    _controller.dispose();
    _assetDiagnostics.dispose();
    _cancelPreferenceSubscriptions();
    super.dispose();
  }

  void _syncAssetContext() {
    final snapshot = _controller.themePreferenceSnapshot;
    if (_assetSnapshot != null && mapEquals(_assetSnapshot!.toJson(), snapshot.toJson())) return;
    _assetSnapshot = snapshot;
    _assetDiagnostics.updateContext(
      themeRevision: ++_assetRevision,
      activeReferences: _themeAssetReferences(snapshot),
    );
  }

  /// Synchronizes the theme lifecycle with runtime configuration changes.
  void _handleRuntimeBootstrapChanged() {
    if (!_runtimeManager.runtimeConfigured) {
      _runtimeGeneration++;
      _stopRuntimeStartupStatusPolling();
      _controller.dispose();
      _syncAssetContext();
      _cancelPreferenceSubscriptions();
      setState(() {
        _runtimeUiReady = false;
        _runtimeStartCompleted = false;
        _runtimeStartupError = null;
        _runtimeStartFuture = null;
        _preserveUnconfiguredChild = widget.unconfiguredChildEnabled;
        _runtimeStartupMessage = '正在准备本地运行时';
      });
      return;
    }
    unawaited(_startRuntimeUi());
  }

  /// Starts core-backed theme state after the runtime is configured.
  Future<void> _startRuntimeUi() async {
    if (_runtimeStartCompleted ||
        _runtimeStartFuture != null ||
        !_runtimeManager.runtimeConfigured) {
      return;
    }
    final generation = ++_runtimeGeneration;
    _startRuntimeStartupStatusPolling(generation);
    final startFuture = _controller.start();
    _runtimeStartFuture = startFuture;
    try {
      await startFuture;
      final zoom = await _zoomPreferences.load();
      final enterSendsMessage = await const UserPreferencesManager()
          .loadEnterSendsMessage();
      final chatScrollNavigatorOnLeft = await const UserPreferencesManager()
          .loadChatScrollNavigatorOnLeft();
      if (!mounted ||
          generation != _runtimeGeneration ||
          !_runtimeManager.runtimeConfigured) {
        return;
      }
      UserPreferencesManager.enterSendsMessage.value = enterSendsMessage;
      UserPreferencesManager.chatScrollNavigatorOnLeft.value =
          chatScrollNavigatorOnLeft;
      _chatScrollNavigatorOnLeftSubscription = const UserPreferencesManager()
          .chatScrollNavigatorOnLeftFlow()
          .listen((enabled) {
            if (!mounted || generation != _runtimeGeneration) {
              return;
            }
            UserPreferencesManager.chatScrollNavigatorOnLeft.value = enabled;
          });
      _longPasteSubscription = const UserPreferencesManager()
          .longPastedTextInputSettingsFlow()
          .listen((settings) {
            if (!mounted || generation != _runtimeGeneration) {
              return;
            }
            UserPreferencesManager.longPastedTextInputSettings.value = settings;
          });
      _enterSendsMessageSubscription = const UserPreferencesManager()
          .enterSendsMessageFlow()
          .listen((enabled) {
            if (!mounted || generation != _runtimeGeneration) {
              return;
            }
            UserPreferencesManager.enterSendsMessage.value = enabled;
          });
      setState(() {
        _zoom = zoom;
        _runtimeUiReady = true;
        _runtimeStartCompleted = true;
        _runtimeStartupError = null;
      });
    } catch (error, stackTrace) {
      if (!mounted || generation != _runtimeGeneration) {
        return;
      }
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'operit runtime bootstrap',
          context: ErrorDescription('while starting core-backed theme state'),
        ),
      );
      setState(() {
        _runtimeUiReady = false;
        _runtimeStartupError = error;
      });
    } finally {
      if (generation == _runtimeGeneration) {
        _stopRuntimeStartupStatusPolling();
        _runtimeStartFuture = null;
      }
    }
  }

  /// Detaches application-level preference caches from the active runtime.
  void _cancelPreferenceSubscriptions() {
    unawaited(_longPasteSubscription?.cancel());
    _longPasteSubscription = null;
    unawaited(_enterSendsMessageSubscription?.cancel());
    _enterSendsMessageSubscription = null;
    unawaited(_chatScrollNavigatorOnLeftSubscription?.cancel());
    _chatScrollNavigatorOnLeftSubscription = null;
  }

  /// Starts reading Android native runtime stages while Core-backed UI is starting.
  void _startRuntimeStartupStatusPolling(int generation) {
    if (kIsWeb || defaultTargetPlatform != TargetPlatform.android) {
      return;
    }
    _stopRuntimeStartupStatusPolling();
    unawaited(_refreshRuntimeStartupStatus(generation));
    _runtimeStartupStatusTimer = Timer.periodic(
      const Duration(milliseconds: 200),
      (_) => unawaited(_refreshRuntimeStartupStatus(generation)),
    );
  }

  /// Stops Android native runtime stage polling.
  void _stopRuntimeStartupStatusPolling() {
    _runtimeStartupStatusTimer?.cancel();
    _runtimeStartupStatusTimer = null;
  }

  /// Applies the current Android native runtime stage to the bootstrap screen.
  Future<void> _refreshRuntimeStartupStatus(int generation) async {
    final status = await _runtimeChannel.invokeMapMethod<String, String>(
      'localRuntimeStartupStatus',
    );
    final message = status?['message'];
    if (!mounted || generation != _runtimeGeneration || message == null) {
      return;
    }
    if (_runtimeStartupMessage == message) {
      return;
    }
    setState(() {
      _runtimeStartupMessage = message;
    });
  }

  /// Applies a keyboard-selected zoom and persists changes in input order.
  void _changeZoom(double zoom) {
    if (_zoom == zoom) {
      return;
    }
    setState(() {
      _zoom = zoom;
    });
    _zoomSave = _zoomSave.then((_) => _zoomPreferences.save(zoom));
    unawaited(_zoomSave);
  }

  /// Builds the bootstrap or fully initialized application theme.
  @override
  Widget build(BuildContext context) {
    final runtimeConfigured = _runtimeManager.runtimeConfigured;
    final runtimeReady = runtimeConfigured && _runtimeUiReady;
    final activeThemeReady =
        runtimeReady &&
        !_controller.themeIsPreparing &&
        _controller.themeLoadError == null;
    final showUnconfiguredApp =
        !runtimeConfigured &&
        (widget.unconfiguredChildEnabled || _preserveUnconfiguredChild);
    final retainApplication =
        _controller.hasLoadedTheme ||
        showUnconfiguredApp ||
        _preserveUnconfiguredChild;
    final themeMode = runtimeReady
        ? _controller.themeMode
        : widget.initialThemeMode ??
              widget.initialThemePreferenceSnapshot.themeMode;
    return _OperitThemeScope(
      controller: _controller,
      child: DesktopWindowChrome(
        themeMode: themeMode,
        lightColorScheme: operitColorScheme(
          Brightness.light,
          _controller.themePreferenceSnapshot,
        ),
        darkColorScheme: operitColorScheme(
          Brightness.dark,
          _controller.themePreferenceSnapshot,
        ),
        child: ThemeRenderStack(
          children: <Widget>[
            if (retainApplication)
              Offstage(
                key: const ValueKey('appearance-application'),
                offstage: !activeThemeReady && !showUnconfiguredApp,
                child: TickerMode(
                  enabled: activeThemeReady || showUnconfiguredApp,
                  child: ExcludeFocus(
                    excluding: !activeThemeReady && !showUnconfiguredApp,
                    child: ThemeAssetScope(
                      resolver: _themeAssets,
                      diagnostics: _assetDiagnostics,
                      child: _OperitMaterialApp(
                        themeMode: themeMode,
                        themePreferenceSnapshot:
                            _controller.themePreferenceSnapshot,
                        hostInteractionHostsEnabled:
                            activeThemeReady &&
                            widget.hostInteractionHostsEnabled,
                        zoom: _zoom,
                        zoomShortcutsEnabled:
                            runtimeReady && _runtimeStartCompleted,
                        onZoomChanged: _changeZoom,
                        assetDiagnostics: _assetDiagnostics,
                        child: widget.child,
                      ),
                    ),
                  ),
                ),
              ),
            if (!activeThemeReady && !showUnconfiguredApp)
              ThemeRenderRoot(
                key: const ValueKey('appearance-status'),
                title: 'Operit2',
                themePreferenceSnapshot: _controller.themePreferenceSnapshot,
                lightColorScheme: operitColorScheme(
                  Brightness.light,
                  _controller.themePreferenceSnapshot,
                ),
                darkColorScheme: operitColorScheme(
                  Brightness.dark,
                  _controller.themePreferenceSnapshot,
                ),
                // Startup and repair use system typography while font assets are being prepared.
                fontFamily: _defaultFontFamily,
                fontFamilyFallback: _defaultFontFamilyFallback,
                themeMode: themeMode,
                localizationsDelegates: AppLocalizations.localizationsDelegates,
                supportedLocales: AppLocalizations.supportedLocales,
                builder: (context, child) =>
                    AnnotatedRegion<SystemUiOverlayStyle>(
                      value: operitSystemUiOverlayStyle(
                        Theme.of(context).colorScheme,
                      ),
                      child: DesktopWindowFrame(
                        transparentSurface: _controller
                            .themePreferenceSnapshot
                            .transparentSurfaceEnabled,
                        child: child!,
                      ),
                    ),
                home: runtimeReady && _controller.themeLoadError != null
                    ? ThemeAppearanceErrorScreen(
                        error: _controller.themeLoadError!,
                        onRetry: _controller.reloadTheme,
                        onReset: _controller.restoreDefaultAppearance,
                      )
                    : RuntimeBootstrapScreen(
                        message: runtimeReady
                            ? '正在读取外观设置'
                            : !runtimeConfigured
                            ? '请先在主窗口配置运行时目录和工作区目录'
                            : _runtimeStartupMessage,
                        errorText: _runtimeStartupError?.toString(),
                      ),
              ),
          ],
        ),
      ),
    );
  }
}

class _OperitMaterialApp extends StatelessWidget {
  const _OperitMaterialApp({
    required this.themeMode,
    required this.themePreferenceSnapshot,
    required this.hostInteractionHostsEnabled,
    required this.zoom,
    required this.zoomShortcutsEnabled,
    required this.onZoomChanged,
    required this.assetDiagnostics,
    required this.child,
  });

  final ThemeMode themeMode;
  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final bool hostInteractionHostsEnabled;
  final double zoom;
  final bool zoomShortcutsEnabled;
  final ValueChanged<double> onZoomChanged;
  final ThemeAssetDiagnostics assetDiagnostics;
  final Widget child;

  /// Builds the themed application and process-wide host overlays.
  @override
  Widget build(BuildContext context) {
    final lightColorScheme = operitColorScheme(
      Brightness.light,
      themePreferenceSnapshot,
    );
    final darkColorScheme = operitColorScheme(
      Brightness.dark,
      themePreferenceSnapshot,
    );
    return ThemeRenderRoot(
      title: 'Operit2',
      themePreferenceSnapshot: themePreferenceSnapshot,
      lightColorScheme: lightColorScheme,
      darkColorScheme: darkColorScheme,
      fontFamily: _fontFamily(themePreferenceSnapshot),
      fontFamilyFallback: _fontFamilyFallback(themePreferenceSnapshot),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      themeMode: themeMode,
      themeAnimationDuration: OperitTheme.of(context).suppressThemeAnimation
          ? Duration.zero
          : kThemeAnimationDuration,
      builder: (context, materialChild) {
        final appContent = ThemeCircularRevealHost(
          child: AnnotatedRegion<SystemUiOverlayStyle>(
            value: operitSystemUiOverlayStyle(Theme.of(context).colorScheme),
            child: OperitThemeBackground(
              themePreferenceSnapshot: themePreferenceSnapshot,
              child: DesktopWindowFrame(
                transparentSurface:
                    themePreferenceSnapshot.transparentSurfaceEnabled,
                zoom: zoom,
                child: materialChild!,
              ),
            ),
          ),
        );
        return ApplicationZoomHost(
          zoom: zoom,
          shortcutsEnabled: zoomShortcutsEnabled,
          onZoomChanged: onZoomChanged,
          child: AppToastHost(
            child: ThemeAssetNoticeHost(
              diagnostics: assetDiagnostics,
              child: MacNoticeScaffoldMessenger(child: appContent),
            ),
          ),
        );
      },
      home: UnhandledErrorHost(
        child: Stack(
          fit: StackFit.expand,
          children: <Widget>[
            Positioned.fill(
              child: RuntimeBrowserOwnerHost(
                enabled: hostInteractionHostsEnabled,
                child: child,
              ),
            ),
            const TtsFloatingPanel(),
            RuntimeActivityOverlay(
              key: const ValueKey<String>('runtime-activity-overlay'),
              enabled: hostInteractionHostsEnabled,
            ),
          ],
        ),
      ),
    );
  }
}

/// Displays recoverable resource failures without replacing the application.
class ThemeAssetNoticeHost extends StatefulWidget {
  const ThemeAssetNoticeHost({
    super.key,
    required this.diagnostics,
    required this.child,
    this.inline = false,
  });

  final ThemeAssetDiagnostics diagnostics;
  final Widget child;
  final bool inline;

  @override
  State<ThemeAssetNoticeHost> createState() => _ThemeAssetNoticeHostState();
}

class _ThemeAssetNoticeHostState extends State<ThemeAssetNoticeHost> {
  final Set<String> _shown = {};
  final Map<String, int> _notices = {};
  bool _scheduled = false;

  @override
  void initState() {
    super.initState();
    widget.diagnostics.addListener(_schedule);
    _schedule();
  }

  @override
  void didUpdateWidget(covariant ThemeAssetNoticeHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.diagnostics, widget.diagnostics)) {
      oldWidget.diagnostics.removeListener(_schedule);
      widget.diagnostics.addListener(_schedule);
      _dismissNotices();
      _shown.clear();
    }
    if (oldWidget.inline != widget.inline) _dismissNotices();
    _schedule();
  }

  String _noticeKey(ThemeAssetFailure failure) =>
      '${failure.key}|${widget.diagnostics.retryEpoch(failure.reference)}';

  void _schedule() {
    if (!mounted || _scheduled) return;
    _scheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scheduled = false;
      if (!mounted) return;
      final l10n = AppLocalizations.of(context) ?? lookupAppLocalizations(const Locale('en'));
      final failures = widget.diagnostics.failures;
      final current = failures.map(_noticeKey).toSet();
      _shown.retainAll(current);
      for (final key in _notices.keys.toList()) {
        if (!current.contains(key)) {
          AppToastController.instance.dismiss(_notices.remove(key)!);
        }
      }
      for (final failure in failures) {
        final key = _noticeKey(failure);
        if (!_shown.add(key)) continue;
        ClientLogger.w(failure.details, tag: 'ThemeResource', error: failure);
        if (!widget.inline) {
          _notices[key] = AppToastController.instance.show(
            _themeAssetFailureMessage(l10n, failure, widget.diagnostics),
            // Keep recovery available while a copied/synchronized file arrives.
            duration: const Duration(minutes: 5),
            actionLabel: l10n.settingsAppearanceAssetRetry,
            onAction: () {
              if (mounted) widget.diagnostics.retry(failure.reference);
            },
          );
        }
      }
      if (widget.inline) setState(() {});
    });
    WidgetsBinding.instance.scheduleFrame();
  }

  void _dismissNotices() {
    for (final id in _notices.values) {
      AppToastController.instance.dismiss(id);
    }
    _notices.clear();
  }

  @override
  void dispose() {
    widget.diagnostics.removeListener(_schedule);
    _dismissNotices();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final failures = widget.diagnostics.failures;
    final failure = failures.isEmpty ? null : failures.first;
    final l10n = AppLocalizations.of(context) ?? lookupAppLocalizations(const Locale('en'));
    final colors = Theme.of(context).colorScheme;
    // This parent is stable even as notices appear, disappear, or are retried.
    return Stack(
      children: [
        widget.child,
        if (widget.inline && failure != null)
          Positioned(
            left: 8,
            right: 8,
            bottom: 8,
            child: Material(
              color: colors.errorContainer,
              borderRadius: BorderRadius.circular(8),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                child: Row(
                  children: [
                    Expanded(child: Text(
                      _themeAssetFailureMessage(l10n, failure, widget.diagnostics),
                      style: TextStyle(color: colors.onErrorContainer),
                    )),
                    TextButton(
                      onPressed: () => widget.diagnostics.retry(failure.reference),
                      child: Text(l10n.settingsAppearanceAssetRetry),
                    ),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}

String _themeAssetFailureMessage(
  AppLocalizations l10n,
  ThemeAssetFailure failure,
  ThemeAssetDiagnostics diagnostics,
) {
  final reference = ThemeAssetFailure.safeReference(failure.reference)
      .replaceAll('\\', '/').split('/').last;
  final extension = reference.split('.').last.toLowerCase();
  final video = failure.stage == ThemeAssetStage.videoOpen ||
      failure.stage == ThemeAssetStage.playback ||
      const {'mp4', 'mov', 'm4v', 'webm', 'mkv', 'avi'}.contains(extension);
  final message = failure.retainedPrevious
      ? l10n.settingsAppearanceAssetPreviousRetained(reference)
      : failure.purpose == ThemeAssetPurpose.bubble
      ? l10n.settingsAppearanceAssetBubbleFailed(reference)
      : video
      ? l10n.settingsAppearanceAssetVideoFailed(reference)
      : l10n.settingsAppearanceAssetBackgroundFailed(reference);
  return diagnostics.retryEpoch(failure.reference) == 0
      ? message
      : '$message ${l10n.settingsAppearanceAssetReimportHint}';
}

Set<String> _themeAssetReferences(ThemePreferenceSnapshot snapshot) {
  final references = <String>{};
  void add(String? reference) {
    if (reference != null && reference.isNotEmpty) references.add(reference);
  }
  if (snapshot.useBackgroundImage) add(snapshot.backgroundImageUri);
  if (snapshot.bubbleUserUseImage) add(snapshot.bubbleUserImageUri);
  if (snapshot.bubbleAiUseImage) add(snapshot.bubbleAiImageUri);
  if (snapshot.inputUseImage) add(snapshot.inputImageUri);
  for (final entry in snapshot.toJson().entries) {
    if (entry.key.endsWith('Asset') && entry.value is String) {
      add(entry.value as String);
    }
  }
  return references;
}

/// Observes and edits the ordinary synchronized appearance configuration.
class OperitThemeController {
  /// Creates the controller that owns the effective application theme.
  OperitThemeController({
    required VoidCallback onChanged,
    Future<void> Function(ThemeMode)? saveStartupThemeMode,
    UserPreferencesManager preferencesManager = const UserPreferencesManager(),
  }) : _onChanged = onChanged,
       _saveStartupThemeMode =
           saveStartupThemeMode ?? _saveStartupThemeModeDefault,
       _preferencesManager = preferencesManager;

  final VoidCallback _onChanged;
  final Future<void> Function(ThemeMode) _saveStartupThemeMode;
  final UserPreferencesManager _preferencesManager;
  StreamSubscription<ThemePreferenceSnapshot>? _appearanceSubscription;
  ThemePreferenceSnapshot _themePreferenceSnapshot =
      UserPreferencesManager.defaultThemePreferenceSnapshot;
  Object? _themeLoadError;
  bool _themeIsPreparing = false;
  bool _hasLoadedState = false;
  int _themePreferenceRevision = 0;
  int _lifecycleRevision = 0;
  bool _suppressThemeAnimation = false;

  ThemeMode get themeMode => _themePreferenceSnapshot.themeMode;
  ThemePreferenceSnapshot get themePreferenceSnapshot =>
      _themePreferenceSnapshot;
  Object? get themeLoadError => _themeLoadError;
  bool get themeIsPreparing => _themeIsPreparing;

  /// Reports whether a complete appearance has finished resource preparation.
  bool get hasLoadedTheme => _hasLoadedState;
  bool get suppressThemeAnimation => _suppressThemeAnimation;

  /// Applies the known first-frame theme state before asynchronous startup.
  void setInitialThemePreferenceSnapshot(ThemePreferenceSnapshot snapshot) {
    _themePreferenceSnapshot = snapshot;
  }

  /// Observes the existing shared appearance owner without retaining a template selection.
  Future<void> start() async {
    final lifecycle = ++_lifecycleRevision;
    await _appearanceSubscription?.cancel();
    _appearanceSubscription = null;
    if (lifecycle != _lifecycleRevision) return;
    await _reloadThemePreferenceSnapshot();
    if (lifecycle != _lifecycleRevision) return;
    _appearanceSubscription = _preferencesManager
        .themePreferenceSnapshotFlow()
        .listen(
          (snapshot) {
            if (lifecycle != _lifecycleRevision) return;
            unawaited(
              _reloadThemePreferenceSnapshot(committedSnapshot: snapshot),
            );
          },
          onError: (Object error, StackTrace stack) {
            if (lifecycle != _lifecycleRevision) return;
            ClientLogger.e(
              'Appearance preference subscription failed lifecycle=$lifecycle '
              'hasLoadedTheme=$_hasLoadedState',
              tag: 'ThemeAppearance',
              error: error,
              stackTrace: stack,
            );
            _themePreferenceRevision++;
            _themeIsPreparing = false;
            _themeLoadError = error;
            _onChanged();
          },
        );
  }

  /// Releases the ordinary appearance observer when this controller leaves its runtime.
  void dispose() {
    _lifecycleRevision++;
    _themePreferenceRevision++;
    unawaited(_appearanceSubscription?.cancel());
    _appearanceSubscription = null;
    _themePreferenceSnapshot =
        UserPreferencesManager.defaultThemePreferenceSnapshot;
    _hasLoadedState = false;
  }

  /// Reports whether the effective Material theme is dark.
  bool isDark(BuildContext context) {
    return Theme.of(context).brightness == Brightness.dark;
  }

  /// Toggles directly between explicit light and dark modes.
  void toggle(BuildContext context) {
    final nextMode = isDark(context) ? ThemeMode.light : ThemeMode.dark;
    final reveal = ThemeCircularRevealHost.maybeOf(context);
    if (reveal == null) {
      unawaited(setThemeMode(nextMode));
      return;
    }
    unawaited(
      reveal.switchTheme(
        originContext: context,
        applyTheme: () async {
          _suppressThemeAnimation = true;
          try {
            await setThemeMode(nextMode);
            // Paint the target theme before revealing it through the circle.
            await WidgetsBinding.instance.endOfFrame;
          } finally {
            _suppressThemeAnimation = false;
          }
        },
      ),
    );
  }

  /// Persists and applies this node's light/dark/system choice.
  Future<void> setThemeMode(ThemeMode themeMode) async {
    if (_themePreferenceSnapshot.themeMode == themeMode) {
      return;
    }
    await _preferencesManager.saveThemeSettings(themeMode: themeMode);
    await _reloadThemePreferenceSnapshot();
  }

  /// Applies transient slider values without persisting them.
  void previewThemeSettings({
    double? backgroundImageOpacity,
    double? backgroundBlurRadius,
    double? fontScale,
  }) {
    // Keep slider previews local until their explicit save operation.
    final snapshot = _themePreferenceSnapshotWith(
      _themePreferenceSnapshot,
      backgroundImageOpacity: backgroundImageOpacity,
      backgroundBlurRadius: backgroundBlurRadius,
      fontScale: fontScale,
    );
    _themePreferenceSnapshot = snapshot;
    _onChanged();
  }

  /// Persists a partial update through synchronized Space appearance preferences.
  Future<void> saveThemeSettings({
    String? inputStyle,
    String? userChatStyle,
    String? aiChatStyle,
    bool? bubbleShowAvatar,
    bool? bubbleWideLayoutEnabled,
    bool? bubbleUserRoundedCornersEnabled,
    bool? bubbleAiRoundedCornersEnabled,
    bool? transparentSurfaceEnabled,
    bool? chatInputFloating,
    int? cursorUserBubbleColor,
    int? bubbleUserBubbleColor,
    int? bubbleAiBubbleColor,
    int? bubbleUserTextColor,
    int? bubbleAiTextColor,
    bool? bubbleUserUseImage,
    bool? bubbleAiUseImage,
    String? bubbleUserImageUri,
    String? bubbleAiImageUri,
    String? bubbleUserImageRenderMode,
    String? bubbleAiImageRenderMode,
    double? bubbleUserImageCropLeft,
    double? bubbleUserImageCropTop,
    double? bubbleUserImageCropRight,
    double? bubbleUserImageCropBottom,
    double? bubbleUserImageRepeatStart,
    double? bubbleUserImageRepeatEnd,
    double? bubbleUserImageRepeatYStart,
    double? bubbleUserImageRepeatYEnd,
    double? bubbleUserImageScale,
    double? bubbleAiImageCropLeft,
    double? bubbleAiImageCropTop,
    double? bubbleAiImageCropRight,
    double? bubbleAiImageCropBottom,
    double? bubbleAiImageRepeatStart,
    double? bubbleAiImageRepeatEnd,
    double? bubbleAiImageRepeatYStart,
    double? bubbleAiImageRepeatYEnd,
    double? bubbleAiImageScale,
    double? bubbleUserContentPaddingLeft,
    double? bubbleUserContentPaddingRight,
    double? bubbleAiContentPaddingLeft,
    double? bubbleAiContentPaddingRight,
    String? fontType,
    String? systemFontName,
    bool? useCustomFont,
    bool? useCustomColors,
    int? customPrimaryColor,
    int? customSecondaryColor,
    bool? useBackgroundImage,
    String? backgroundImageUri,
    double? backgroundImageOpacity,
    String? backgroundMediaType,
    bool? videoBackgroundMuted,
    bool? videoBackgroundLoop,
    bool? useBackgroundBlur,
    double? backgroundBlurRadius,
    double? fontScale,
    String? customFontPath,
    bool? showThinkingProcess,
    bool? showModelProvider,
    bool? showModelName,
    bool? showRoleName,
    bool? showUserName,
    bool? showMessageTokenStats,
    bool? showMessageTimingStats,
    bool? showMessageTimestamp,
    bool? showInputProcessingStatus,
    String? avatarShape,
    bool? bubbleUserUseCustomFont,
    String? bubbleUserFontType,
    String? bubbleUserSystemFontName,
    String? bubbleUserCustomFontPath,
    bool? bubbleAiUseCustomFont,
    String? bubbleAiFontType,
    String? bubbleAiSystemFontName,
    String? bubbleAiCustomFontPath,
  }) async {
    await _preferencesManager.saveThemeSettings(
      inputStyle: inputStyle,
      userChatStyle: userChatStyle,
      aiChatStyle: aiChatStyle,
      bubbleShowAvatar: bubbleShowAvatar,
      bubbleWideLayoutEnabled: bubbleWideLayoutEnabled,
      bubbleUserRoundedCornersEnabled: bubbleUserRoundedCornersEnabled,
      bubbleAiRoundedCornersEnabled: bubbleAiRoundedCornersEnabled,
      transparentSurfaceEnabled: transparentSurfaceEnabled,
      chatInputFloating: chatInputFloating,
      cursorUserBubbleColor: cursorUserBubbleColor,
      bubbleUserBubbleColor: bubbleUserBubbleColor,
      bubbleAiBubbleColor: bubbleAiBubbleColor,
      bubbleUserTextColor: bubbleUserTextColor,
      bubbleAiTextColor: bubbleAiTextColor,
      bubbleUserUseImage: bubbleUserUseImage,
      bubbleAiUseImage: bubbleAiUseImage,
      bubbleUserImageUri: bubbleUserImageUri,
      bubbleAiImageUri: bubbleAiImageUri,
      bubbleUserImageRenderMode: bubbleUserImageRenderMode,
      bubbleAiImageRenderMode: bubbleAiImageRenderMode,
      bubbleUserImageCropLeft: bubbleUserImageCropLeft,
      bubbleUserImageCropTop: bubbleUserImageCropTop,
      bubbleUserImageCropRight: bubbleUserImageCropRight,
      bubbleUserImageCropBottom: bubbleUserImageCropBottom,
      bubbleUserImageRepeatStart: bubbleUserImageRepeatStart,
      bubbleUserImageRepeatEnd: bubbleUserImageRepeatEnd,
      bubbleUserImageRepeatYStart: bubbleUserImageRepeatYStart,
      bubbleUserImageRepeatYEnd: bubbleUserImageRepeatYEnd,
      bubbleUserImageScale: bubbleUserImageScale,
      bubbleAiImageCropLeft: bubbleAiImageCropLeft,
      bubbleAiImageCropTop: bubbleAiImageCropTop,
      bubbleAiImageCropRight: bubbleAiImageCropRight,
      bubbleAiImageCropBottom: bubbleAiImageCropBottom,
      bubbleAiImageRepeatStart: bubbleAiImageRepeatStart,
      bubbleAiImageRepeatEnd: bubbleAiImageRepeatEnd,
      bubbleAiImageRepeatYStart: bubbleAiImageRepeatYStart,
      bubbleAiImageRepeatYEnd: bubbleAiImageRepeatYEnd,
      bubbleAiImageScale: bubbleAiImageScale,
      bubbleUserContentPaddingLeft: bubbleUserContentPaddingLeft,
      bubbleUserContentPaddingRight: bubbleUserContentPaddingRight,
      bubbleAiContentPaddingLeft: bubbleAiContentPaddingLeft,
      bubbleAiContentPaddingRight: bubbleAiContentPaddingRight,
      fontType: fontType,
      systemFontName: systemFontName,
      useCustomFont: useCustomFont,
      useCustomColors: useCustomColors,
      customPrimaryColor: customPrimaryColor,
      customSecondaryColor: customSecondaryColor,
      useBackgroundImage: useBackgroundImage,
      backgroundImageUri: backgroundImageUri,
      backgroundImageOpacity: backgroundImageOpacity,
      backgroundMediaType: backgroundMediaType,
      videoBackgroundMuted: videoBackgroundMuted,
      videoBackgroundLoop: videoBackgroundLoop,
      useBackgroundBlur: useBackgroundBlur,
      backgroundBlurRadius: backgroundBlurRadius,
      fontScale: fontScale,
      customFontPath: customFontPath,
      showThinkingProcess: showThinkingProcess,
      showModelProvider: showModelProvider,
      showModelName: showModelName,
      showRoleName: showRoleName,
      showUserName: showUserName,
      showMessageTokenStats: showMessageTokenStats,
      showMessageTimingStats: showMessageTimingStats,
      showMessageTimestamp: showMessageTimestamp,
      showInputProcessingStatus: showInputProcessingStatus,
      avatarShape: avatarShape,
      bubbleUserUseCustomFont: bubbleUserUseCustomFont,
      bubbleUserFontType: bubbleUserFontType,
      bubbleUserSystemFontName: bubbleUserSystemFontName,
      bubbleUserCustomFontPath: bubbleUserCustomFontPath,
      bubbleAiUseCustomFont: bubbleAiUseCustomFont,
      bubbleAiFontType: bubbleAiFontType,
      bubbleAiSystemFontName: bubbleAiSystemFontName,
      bubbleAiCustomFontPath: bubbleAiCustomFontPath,
    );
    await _reloadThemePreferenceSnapshot();
  }

  /// Persists the global user avatar shown by the active theme.
  Future<void> saveActiveThemeUserAvatarSettings({
    required String customUserAvatarUri,
  }) async {
    await _preferencesManager.saveGlobalUserAvatarSettings(
      customUserAvatarUri: customUserAvatarUri,
    );
    await _reloadThemePreferenceSnapshot();
  }

  Future<void> resetMessageColorSettings() async {
    await _preferencesManager.saveAppearanceFields(<String, Object?>{
      'cursorUserBubbleColor': null,
      'bubbleUserBubbleColor': null,
      'bubbleAiBubbleColor': null,
      'bubbleUserTextColor': null,
      'bubbleAiTextColor': null,
    });
    await _reloadThemePreferenceSnapshot();
  }

  /// Resets appearance using the existing named configuration owner.
  Future<void> resetThemeSettings() async {
    await _preferencesManager.resetThemeSettings();
    await _reloadThemePreferenceSnapshot();
  }

  /// Replaces appearance only after an explicit reset action from the error screen.
  Future<void> restoreDefaultAppearance() async {
    await _preferencesManager.restoreDefaultAppearance();
    await _reloadThemePreferenceSnapshot();
  }

  /// Re-reads committed appearance and renews its subscription for an explicit retry.
  Future<void> reloadTheme() => start();

  /// Saves ordinary appearance through its synchronized preference owner.
  Future<void> saveAppearanceFields(Map<String, Object?> updates) async {
    await _preferencesManager.saveAppearanceFields(updates);
    await _reloadThemePreferenceSnapshot();
  }

  /// Loads an independent appearance configuration without looking up its creation source.
  Future<void> _reloadThemePreferenceSnapshot({
    ThemePreferenceSnapshot? committedSnapshot,
  }) async {
    final lifecycle = _lifecycleRevision;
    final revision = ++_themePreferenceRevision;

    /// Checks that asynchronous work still belongs to the current preference revision.
    bool current() =>
        lifecycle == _lifecycleRevision && revision == _themePreferenceRevision;
    if (!_hasLoadedState) _themeIsPreparing = true;
    var stage = 'read_preferences';
    try {
      final snapshot =
          committedSnapshot ??
          await _preferencesManager.resolveThemePreferenceSnapshot();
      if (!current()) return;
      if (_hasLoadedState &&
          _themeLoadError == null &&
          mapEquals(snapshot.toJson(), _themePreferenceSnapshot.toJson())) {
        return;
      }
      stage = 'validate_palette';
      final palette = snapshot.themePaletteJson;
      if (palette != null) {
        final colors = jsonDecode(palette) as Map<String, dynamic>;
        for (final brightness in Brightness.values) {
          ThemeVisualSpecV1.colorSchemeFromPalette(colors, brightness);
        }
      }
      stage = 'prepare_fonts';
      final families =
          !_hasLoadedState ||
              !listEquals(
                _themeFontReferences(snapshot),
                _themeFontReferences(_themePreferenceSnapshot),
              )
          ? await _prepareCustomFonts(snapshot)
          : const <String, String>{};
      if (!current()) return;
      stage = 'save_startup_theme_mode';
      if (!_hasLoadedState || snapshot.themeMode != themeMode) {
        await _saveStartupThemeMode(snapshot.themeMode);
      }
      if (!current()) return;
      _loadedCustomFontFamilies.addAll(families);
      _themePreferenceSnapshot = snapshot;
      _themeLoadError = null;
      _themeIsPreparing = false;
      _hasLoadedState = true;
      _onChanged();
    } catch (error, stack) {
      if (!current()) return;
      ClientLogger.e(
        'Appearance load failed stage=$stage lifecycle=$lifecycle '
        'revision=$revision hasLoadedTheme=$_hasLoadedState',
        tag: 'ThemeAppearance',
        error: error,
        stackTrace: stack,
      );
      _themeLoadError = error;
      _themeIsPreparing = false;
      _onChanged();
    }
  }
}

/// Persists the first-frame theme mode through the existing bootstrap host.
Future<void> _saveStartupThemeModeDefault(ThemeMode themeMode) {
  return RuntimeBootstrapManager.instance.saveStartupThemeMode(themeMode);
}

/// Renders an inert appearance in its own scope without starting runtime
/// observers or writing the application's selected theme.
class OperitThemePreview extends StatefulWidget {
  const OperitThemePreview({
    super.key,
    required this.snapshot,
    required this.child,
    this.visual,
    this.assets,
  });

  final ThemePreferenceSnapshot snapshot;
  final PreparedThemeVisual? visual;
  final ThemeAssetResolver? assets;
  final Widget child;

  @override
  State<OperitThemePreview> createState() => _OperitThemePreviewState();
}

class _OperitThemePreviewState extends State<OperitThemePreview> {
  late final OperitThemeController _controller = OperitThemeController(
    onChanged: () {},
  );
  final _assets = ThemeAssetStore();
  final _assetDiagnostics = ThemeAssetDiagnostics();
  PreparedThemeVisual? _boundVisual;
  ThemeAssetResolver? _boundRuntime;
  int _assetRevision = 0;
  Future<void>? _preparation;
  late ThemeAssetResolver _resolver;

  /// Prepares this preview's initial asset bindings.
  @override
  void initState() {
    super.initState();
    _prepare();
  }

  /// Updates presentation without rereading unchanged private font assets.
  @override
  void didUpdateWidget(covariant OperitThemePreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!mapEquals(oldWidget.snapshot.toJson(), widget.snapshot.toJson()) ||
        oldWidget.visual != widget.visual ||
        oldWidget.assets != widget.assets) {
      _prepare(
        prepareFonts:
            oldWidget.visual != widget.visual ||
            !listEquals(
              _themeFontReferences(oldWidget.snapshot),
              _themeFontReferences(widget.snapshot),
            ),
      );
    }
  }

  /// Rebinds appearance while retaining any existing font preparation future.
  void _prepare({bool prepareFonts = true}) {
    final visual = widget.visual;
    final runtime = widget.assets ?? _assets;
    if (_boundRuntime == null ||
        !identical(_boundVisual, visual) || !identical(_boundRuntime, runtime)) {
      _resolver = visual == null
          ? runtime
          : _PreviewAssetResolver(visual: visual.resources, runtime: runtime);
      _boundVisual = visual;
      _boundRuntime = runtime;
    }
    final snapshot = visual?.snapshot(widget.snapshot) ?? widget.snapshot;
    _assetDiagnostics.updateContext(
      themeRevision: ++_assetRevision,
      activeReferences: _themeAssetReferences(snapshot),
    );
    _controller.setInitialThemePreferenceSnapshot(snapshot);
    if (visual != null) {
      for (final font in [
        visual.resources.sans,
        visual.resources.user,
        visual.resources.ai,
      ]) {
        _loadedCustomFontFamilies[font.family] = font.family;
      }
      _preparation = null;
    } else if (_usesFontAsset(snapshot.fontType) ||
        (snapshot.bubbleUserUseCustomFont &&
            _usesFontAsset(snapshot.bubbleUserFontType)) ||
        (snapshot.bubbleAiUseCustomFont &&
            _usesFontAsset(snapshot.bubbleAiFontType))) {
      if (prepareFonts) {
        _preparation = _prepareCustomFonts(snapshot).then((families) {
          _loadedCustomFontFamilies.addAll(families);
        });
      }
    } else {
      _preparation = null;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    _assetDiagnostics.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<void>(
      future: _preparation,
      builder: (context, preparation) {
        if (preparation.hasError) {
          return Center(
            child: Text(
              AppLocalizations.of(
                context,
              )!.settingsAppearanceThemeConfigFailed('${preparation.error}'),
            ),
          );
        }
        if (_preparation != null &&
            preparation.connectionState != ConnectionState.done) {
          return const Center(child: CircularProgressIndicator());
        }
        final snapshot = _controller.themePreferenceSnapshot;
        final brightness = switch (snapshot.themeMode) {
          ThemeMode.light => Brightness.light,
          ThemeMode.dark => Brightness.dark,
          ThemeMode.system => MediaQuery.platformBrightnessOf(context),
        };
        return _OperitThemeScope(
          controller: _controller,
          child: ThemeAssetScope(
            resolver: _resolver,
            diagnostics: _assetDiagnostics,
            child: Theme(
              data: buildOperitThemeData(
                operitColorScheme(brightness, snapshot),
                snapshot,
                fontFamily: _fontFamily(snapshot),
                fontFamilyFallback: _fontFamilyFallback(snapshot),
              ),
              child: Builder(
                builder: (context) => ThemeAssetNoticeHost(
                  diagnostics: _assetDiagnostics,
                  inline: true,
                  child: DefaultTextStyle(
                    style: Theme.of(context).textTheme.bodyMedium!,
                    child: IgnorePointer(child: ExcludeFocus(child: widget.child)),
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

/// Template references are checked in memory; saved media uses the same runtime
/// owner as the catalog. Both are available in previews that mix these sources.
class _PreviewAssetResolver implements ThemeAssetResolver {
  const _PreviewAssetResolver({required this.visual, required this.runtime});
  final ThemeAssetResolver? visual;
  final ThemeAssetResolver runtime;

  @override
  Future<Uint8List> readBytes(String reference) =>
      (reference.startsWith('assets/') ? visual ?? runtime : runtime).readBytes(
        reference,
      );
}

class _OperitThemeScope extends InheritedWidget {
  /// Captures immutable appearance values for dependency comparisons.
  _OperitThemeScope({required this.controller, required super.child})
    : snapshot = controller.themePreferenceSnapshot,
      suppressAnimation = controller.suppressThemeAnimation;

  final OperitThemeController controller;
  final ThemePreferenceSnapshot snapshot;
  final bool suppressAnimation;

  /// Leaves theme consumers untouched when unrelated root state changes.
  @override
  bool updateShouldNotify(_OperitThemeScope oldWidget) {
    return controller != oldWidget.controller ||
        snapshot != oldWidget.snapshot ||
        suppressAnimation != oldWidget.suppressAnimation;
  }
}

ThemePreferenceSnapshot _themePreferenceSnapshotWith(
  ThemePreferenceSnapshot snapshot, {
  double? backgroundImageOpacity,
  double? backgroundBlurRadius,
  double? fontScale,
}) {
  return ThemePreferenceSnapshot(
    themeMode: snapshot.themeMode,
    themePaletteJson: snapshot.themePaletteJson,
    useCustomColors: snapshot.useCustomColors,
    customPrimaryColor: snapshot.customPrimaryColor,
    customSecondaryColor: snapshot.customSecondaryColor,
    useBackgroundImage: snapshot.useBackgroundImage,
    backgroundImageUri: snapshot.backgroundImageUri,
    backgroundMediaType: snapshot.backgroundMediaType,
    backgroundImageOpacity:
        backgroundImageOpacity ?? snapshot.backgroundImageOpacity,
    videoBackgroundMuted: snapshot.videoBackgroundMuted,
    videoBackgroundLoop: snapshot.videoBackgroundLoop,
    useBackgroundBlur: snapshot.useBackgroundBlur,
    backgroundBlurRadius: backgroundBlurRadius ?? snapshot.backgroundBlurRadius,
    transparentSurfaceEnabled: snapshot.transparentSurfaceEnabled,
    chatInputFloating: snapshot.chatInputFloating,
    inputStyle: snapshot.inputStyle,
    inputBackgroundColor: snapshot.inputBackgroundColor,
    inputRoundedCornersEnabled: snapshot.inputRoundedCornersEnabled,
    inputUseImage: snapshot.inputUseImage,
    inputImageUri: snapshot.inputImageUri,
    inputImageRenderMode: snapshot.inputImageRenderMode,
    inputImageCropLeft: snapshot.inputImageCropLeft,
    inputImageCropTop: snapshot.inputImageCropTop,
    inputImageCropRight: snapshot.inputImageCropRight,
    inputImageCropBottom: snapshot.inputImageCropBottom,
    inputImageRepeatStart: snapshot.inputImageRepeatStart,
    inputImageRepeatEnd: snapshot.inputImageRepeatEnd,
    inputImageRepeatYStart: snapshot.inputImageRepeatYStart,
    inputImageRepeatYEnd: snapshot.inputImageRepeatYEnd,
    inputImageScale: snapshot.inputImageScale,
    inputSettingsAsset: snapshot.inputSettingsAsset,
    inputSettingsAssetSize: snapshot.inputSettingsAssetSize,
    inputAttachmentAsset: snapshot.inputAttachmentAsset,
    inputAttachmentAssetSize: snapshot.inputAttachmentAssetSize,
    inputMicAsset: snapshot.inputMicAsset,
    inputMicAssetSize: snapshot.inputMicAssetSize,
    inputSendAsset: snapshot.inputSendAsset,
    inputSendAssetSize: snapshot.inputSendAssetSize,
    inputStopAsset: snapshot.inputStopAsset,
    inputStopAssetSize: snapshot.inputStopAssetSize,
    userChatStyle: snapshot.userChatStyle,
    aiChatStyle: snapshot.aiChatStyle,
    bubbleShowAvatar: snapshot.bubbleShowAvatar,
    bubbleWideLayoutEnabled: snapshot.bubbleWideLayoutEnabled,
    cursorUserBubbleColor: snapshot.cursorUserBubbleColor,
    bubbleUserBubbleColor: snapshot.bubbleUserBubbleColor,
    bubbleAiBubbleColor: snapshot.bubbleAiBubbleColor,
    bubbleUserTextColor: snapshot.bubbleUserTextColor,
    bubbleAiTextColor: snapshot.bubbleAiTextColor,
    bubbleUserUseImage: snapshot.bubbleUserUseImage,
    bubbleAiUseImage: snapshot.bubbleAiUseImage,
    bubbleUserImageUri: snapshot.bubbleUserImageUri,
    bubbleAiImageUri: snapshot.bubbleAiImageUri,
    bubbleUserImageRenderMode: snapshot.bubbleUserImageRenderMode,
    bubbleAiImageRenderMode: snapshot.bubbleAiImageRenderMode,
    bubbleUserImageCropLeft: snapshot.bubbleUserImageCropLeft,
    bubbleUserImageCropTop: snapshot.bubbleUserImageCropTop,
    bubbleUserImageCropRight: snapshot.bubbleUserImageCropRight,
    bubbleUserImageCropBottom: snapshot.bubbleUserImageCropBottom,
    bubbleUserImageRepeatStart: snapshot.bubbleUserImageRepeatStart,
    bubbleUserImageRepeatEnd: snapshot.bubbleUserImageRepeatEnd,
    bubbleUserImageRepeatYStart: snapshot.bubbleUserImageRepeatYStart,
    bubbleUserImageRepeatYEnd: snapshot.bubbleUserImageRepeatYEnd,
    bubbleUserImageScale: snapshot.bubbleUserImageScale,
    bubbleAiImageCropLeft: snapshot.bubbleAiImageCropLeft,
    bubbleAiImageCropTop: snapshot.bubbleAiImageCropTop,
    bubbleAiImageCropRight: snapshot.bubbleAiImageCropRight,
    bubbleAiImageCropBottom: snapshot.bubbleAiImageCropBottom,
    bubbleAiImageRepeatStart: snapshot.bubbleAiImageRepeatStart,
    bubbleAiImageRepeatEnd: snapshot.bubbleAiImageRepeatEnd,
    bubbleAiImageRepeatYStart: snapshot.bubbleAiImageRepeatYStart,
    bubbleAiImageRepeatYEnd: snapshot.bubbleAiImageRepeatYEnd,
    bubbleAiImageScale: snapshot.bubbleAiImageScale,
    bubbleUserRoundedCornersEnabled: snapshot.bubbleUserRoundedCornersEnabled,
    bubbleAiRoundedCornersEnabled: snapshot.bubbleAiRoundedCornersEnabled,
    bubbleUserContentPaddingLeft: snapshot.bubbleUserContentPaddingLeft,
    bubbleUserContentPaddingRight: snapshot.bubbleUserContentPaddingRight,
    bubbleAiContentPaddingLeft: snapshot.bubbleAiContentPaddingLeft,
    bubbleAiContentPaddingRight: snapshot.bubbleAiContentPaddingRight,
    customUserAvatarUri: snapshot.customUserAvatarUri,
    avatarShape: snapshot.avatarShape,
    avatarCornerRadius: snapshot.avatarCornerRadius,
    useCustomFont: snapshot.useCustomFont,
    fontType: snapshot.fontType,
    systemFontName: snapshot.systemFontName,
    customFontPath: snapshot.customFontPath,
    fontScale: fontScale ?? snapshot.fontScale,
    bubbleUserUseCustomFont: snapshot.bubbleUserUseCustomFont,
    bubbleUserFontType: snapshot.bubbleUserFontType,
    bubbleUserSystemFontName: snapshot.bubbleUserSystemFontName,
    bubbleUserCustomFontPath: snapshot.bubbleUserCustomFontPath,
    bubbleAiUseCustomFont: snapshot.bubbleAiUseCustomFont,
    bubbleAiFontType: snapshot.bubbleAiFontType,
    bubbleAiSystemFontName: snapshot.bubbleAiSystemFontName,
    bubbleAiCustomFontPath: snapshot.bubbleAiCustomFontPath,
    showThinkingProcess: snapshot.showThinkingProcess,
    showModelProvider: snapshot.showModelProvider,
    showModelName: snapshot.showModelName,
    showRoleName: snapshot.showRoleName,
    showUserName: snapshot.showUserName,
    showMessageTokenStats: snapshot.showMessageTokenStats,
    showMessageTimingStats: snapshot.showMessageTimingStats,
    showMessageTimestamp: snapshot.showMessageTimestamp,
    showInputProcessingStatus: snapshot.showInputProcessingStatus,
  );
}

/// Application adapter for the shared background, including private video presets.
class OperitThemeBackground extends StatelessWidget {
  const OperitThemeBackground({
    super.key,
    this.fit = StackFit.expand,
    this.muteVideo = false,
    required this.themePreferenceSnapshot,
    required this.child,
  });

  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final Widget child;
  final StackFit fit;
  final bool muteVideo;

  @override
  Widget build(BuildContext context) {
    final mediaPath = themePreferenceSnapshot.backgroundImageUri;
    final hasMedia =
        themePreferenceSnapshot.useBackgroundImage &&
        mediaPath != null &&
        mediaPath.isNotEmpty;
    return ThemeBackground(
      themePreferenceSnapshot: themePreferenceSnapshot,
      fit: fit,
      media: hasMedia
          ? _ThemeBackgroundMedia(
              key: ValueKey<String>(
                themePreferenceSnapshot.backgroundMediaType,
              ),
              mediaPath: mediaPath,
              mediaType: themePreferenceSnapshot.backgroundMediaType,
              opacity: themePreferenceSnapshot.backgroundImageOpacity,
              muted: muteVideo || themePreferenceSnapshot.videoBackgroundMuted,
              loop: themePreferenceSnapshot.videoBackgroundLoop,
              blurEnabled: themePreferenceSnapshot.useBackgroundBlur,
              blurRadius: themePreferenceSnapshot.backgroundBlurRadius,
            )
          : null,
      child: child,
    );
  }
}

class _ThemeBackgroundMedia extends StatelessWidget {
  const _ThemeBackgroundMedia({
    super.key,
    required this.mediaPath,
    required this.mediaType,
    required this.opacity,
    required this.muted,
    required this.loop,
    required this.blurEnabled,
    required this.blurRadius,
  });

  final String mediaPath;
  final String mediaType;
  final double opacity;
  final bool muted;
  final bool loop;
  final bool blurEnabled;
  final double blurRadius;

  @override
  Widget build(BuildContext context) {
    final media = mediaType == UserPreferencesManager.MEDIA_TYPE_VIDEO
        ? _VideoThemeBackground(
            mediaPath: mediaPath,
            muted: muted,
            loop: loop,
            resolver: ThemeAssetScope.of(context),
            diagnostics: ThemeAssetScope.diagnosticsOf(context),
          )
        : ThemeAssetImage(storagePath: mediaPath, fit: BoxFit.cover);
    return ThemeBackgroundMedia(
      opacity: opacity,
      blurEnabled: blurEnabled,
      blurRadius: blurRadius,
      child: media,
    );
  }
}

class _VideoThemeBackground extends StatefulWidget {
  const _VideoThemeBackground({
    required this.mediaPath,
    required this.muted,
    required this.loop,
    required this.resolver,
    required this.diagnostics,
  });

  final String mediaPath;
  final bool muted;
  final bool loop;
  final ThemeAssetResolver resolver;
  final ThemeAssetDiagnostics? diagnostics;

  /// Creates the video state that owns the runtime asset controller.
  @override
  State<_VideoThemeBackground> createState() => _VideoThemeBackgroundState();
}

class _VideoThemeBackgroundState extends State<_VideoThemeBackground> {
  ThemeVideoAssetLoader? _activeLoader;
  ThemeVideoAssetLoader? _pendingLoader;
  ThemeAssetResolver? _boundResolver;
  ThemeAssetDiagnostics? _boundDiagnostics;
  ThemeAssetFailure? _lastFailure;
  String? _boundReference;
  Future<void> _settingsUpdate = Future<void>.value();
  int _retryEpoch = -1;

  /// Starts loading the runtime video asset when the widget mounts.
  @override
  void initState() {
    super.initState();
    _synchronizeRequest();
  }

  /// Only a retry of this reference renews an otherwise unchanged video.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _synchronizeRequest();
  }

  /// Updates playback settings or reloads bytes when the video asset changes.
  @override
  void didUpdateWidget(covariant _VideoThemeBackground oldWidget) {
    super.didUpdateWidget(oldWidget);
    _synchronizeRequest();
    if (oldWidget.loop != widget.loop || oldWidget.muted != widget.muted) {
      // Settings errors are observed, including when an older successful video
      // remains visible while its replacement is being prepared.
      _settingsUpdate = _settingsUpdate.then((_) => _updatePlaybackSettings());
    }
  }

  void _synchronizeRequest() {
    final epoch = widget.diagnostics?.retryEpoch(widget.mediaPath) ?? 0;
    if (!identical(_boundDiagnostics, widget.diagnostics)) {
      _boundDiagnostics?.release(this);
      _boundDiagnostics = widget.diagnostics;
      final failure = _lastFailure;
      if (failure != null && failure.reference == widget.mediaPath) {
        widget.diagnostics?.report(failure, owner: this);
      }
      if (_boundReference == widget.mediaPath && identical(_boundResolver, widget.resolver)) {
        // Keep a successful player and pending request; only the diagnostic
        // owner moves. Its retry epoch now belongs to the new scope.
        _retryEpoch = epoch;
        return;
      }
    }
    if (_boundReference == widget.mediaPath &&
        identical(_boundResolver, widget.resolver) &&
        epoch == _retryEpoch) {
      return;
    }
    _boundReference = widget.mediaPath;
    _boundResolver = widget.resolver;
    _retryEpoch = epoch;
    widget.diagnostics?.release(this);
    _lastFailure = null;
    final previousRequest = _pendingLoader;
    previousRequest?.cancel();
    if (previousRequest != null) unawaited(previousRequest.dispose());
    late final ThemeVideoAssetLoader loader;
    loader = ThemeVideoAssetLoader(
      resolver: widget.resolver,
      reference: widget.mediaPath,
      diagnostics: widget.diagnostics,
      onPlaybackFailure: (failure) => _handlePlaybackFailure(loader, failure),
    );
    _pendingLoader = loader;
    unawaited(_loadController(loader));
  }

  bool _current(ThemeVideoAssetLoader loader) =>
      mounted && identical(_pendingLoader, loader);

  /// Keeps the active video until the candidate is completely ready.
  Future<void> _loadController(ThemeVideoAssetLoader loader) async {
    try {
      await loader.initialize(
        muted: widget.muted,
        loop: widget.loop,
      );
      if (!_current(loader)) {
        await loader.dispose();
        return;
      }
      while (true) {
        final muted = widget.muted;
        final loop = widget.loop;
        await loader.configure(muted: muted, loop: loop, autoplay: true);
        if (!_current(loader)) {
          await loader.dispose();
          return;
        }
        if (muted == widget.muted && loop == widget.loop) break;
      }
      // No await between this check and publication: a failed candidate must
      // never dispose the previous successful video.
      loader.validateReady();
      final previous = _activeLoader;
      _pendingLoader = null;
      _activeLoader = loader;
      _lastFailure = null;
      setState(() {});
      widget.diagnostics?.resolve(
        loader.reference,
        purpose: ThemeAssetPurpose.background,
        owner: this,
      );
      if (previous != null) unawaited(previous.dispose());
    } on ThemeAssetFailure catch (failure) {
      if (!_current(loader)) return;
      _pendingLoader = null;
      _reportFailure(failure.withContext(retainedPrevious: _activeLoader != null));
    } on FvpCreateCancelledException {
      // Superseded requests are cancellation, not a damaged resource.
    } finally {
      if (!identical(_activeLoader, loader)) unawaited(loader.dispose());
    }
  }

  Future<void> _updatePlaybackSettings() async {
    final loader = _activeLoader;
    if (!mounted || loader == null) return;
    try {
      await loader.configure(muted: widget.muted, loop: widget.loop);
    } on ThemeAssetFailure catch (failure) {
      _handlePlaybackFailure(loader, failure);
    } on FvpCreateCancelledException {
      // The widget may have been replaced while updating native settings.
    }
  }

  void _handlePlaybackFailure(
    ThemeVideoAssetLoader loader,
    ThemeAssetFailure failure,
  ) {
    if (!mounted || !identical(_activeLoader, loader)) return;
    _activeLoader = null;
    _reportFailure(failure);
    unawaited(loader.dispose());
  }

  void _reportFailure(ThemeAssetFailure failure) {
    _lastFailure = failure;
    widget.diagnostics?.report(failure, owner: this);
    setState(() {});
  }

  /// Releases the active video controller.
  @override
  void dispose() {
    _boundDiagnostics?.release(this);
    final pending = _pendingLoader;
    final active = _activeLoader;
    _pendingLoader = null;
    _activeLoader = null;
    pending?.cancel();
    active?.cancel();
    if (pending != null) unawaited(pending.dispose());
    if (active != null) unawaited(active.dispose());
    super.dispose();
  }

  /// Builds the fitted video once the controller is initialized.
  @override
  Widget build(BuildContext context) {
    final controller = _activeLoader?.controller;
    if (controller == null ||
        !controller.value.isInitialized ||
        controller.value.hasError) {
      return const SizedBox.expand();
    }
    return SizedBox.expand(
      child: FittedBox(
        fit: BoxFit.cover,
        child: SizedBox(
          width: controller.value.size.width,
          height: controller.value.size.height,
          child: ValueListenableBuilder<VideoPlayerValue>(
            valueListenable: controller,
            child: VideoPlayer(controller),
            builder: (context, value, child) => LiquidGlassSourceAnimation(
              active: value.isPlaying,
              child: child!,
            ),
          ),
        ),
      ),
    );
  }
}

String operitMessageFontFamily(
  ThemePreferenceSnapshot themePreferenceSnapshot, {
  required bool isUser,
}) {
  if (isUser && themePreferenceSnapshot.bubbleUserUseCustomFont) {
    return _fontFamilyForSettings(
      fontType: themePreferenceSnapshot.bubbleUserFontType,
      systemFontName: themePreferenceSnapshot.bubbleUserSystemFontName,
      customFontPath: themePreferenceSnapshot.bubbleUserCustomFontPath,
    );
  }
  if (!isUser && themePreferenceSnapshot.bubbleAiUseCustomFont) {
    return _fontFamilyForSettings(
      fontType: themePreferenceSnapshot.bubbleAiFontType,
      systemFontName: themePreferenceSnapshot.bubbleAiSystemFontName,
      customFontPath: themePreferenceSnapshot.bubbleAiCustomFontPath,
    );
  }
  return _fontFamily(themePreferenceSnapshot);
}

List<String> operitMessageFontFamilyFallback(
  ThemePreferenceSnapshot themePreferenceSnapshot, {
  required bool isUser,
}) {
  if (isUser && themePreferenceSnapshot.bubbleUserUseCustomFont) {
    return _fontFamilyFallbackForSettings(
      fontType: themePreferenceSnapshot.bubbleUserFontType,
      systemFontName: themePreferenceSnapshot.bubbleUserSystemFontName,
    );
  }
  if (!isUser && themePreferenceSnapshot.bubbleAiUseCustomFont) {
    return _fontFamilyFallbackForSettings(
      fontType: themePreferenceSnapshot.bubbleAiFontType,
      systemFontName: themePreferenceSnapshot.bubbleAiSystemFontName,
    );
  }
  return _fontFamilyFallback(themePreferenceSnapshot);
}

String _fontFamily(ThemePreferenceSnapshot themePreferenceSnapshot) {
  return _fontFamilyForSettings(
    fontType: themePreferenceSnapshot.fontType,
    systemFontName: themePreferenceSnapshot.systemFontName,
    customFontPath: themePreferenceSnapshot.customFontPath,
  );
}

String _fontFamilyForSettings({
  required String fontType,
  required String? systemFontName,
  required String? customFontPath,
}) {
  if (_usesFontAsset(fontType)) {
    if (customFontPath == null || customFontPath.isEmpty) {
      throw StateError('A file font requires a runtime asset reference');
    }
    return _customFontFamilyForPath(customFontPath);
  }
  if (fontType != UserPreferencesManager.FONT_TYPE_SYSTEM) {
    return _defaultFontFamily;
  }
  return switch (systemFontName) {
    UserPreferencesManager.SYSTEM_FONT_SERIF => 'Georgia',
    UserPreferencesManager.SYSTEM_FONT_MONOSPACE => 'Cascadia Mono',
    UserPreferencesManager.SYSTEM_FONT_SANS_SERIF ||
    UserPreferencesManager.SYSTEM_FONT_DEFAULT ||
    null => _defaultFontFamily,
    _ => _defaultFontFamily,
  };
}

List<String> _fontFamilyFallback(
  ThemePreferenceSnapshot themePreferenceSnapshot,
) {
  return _fontFamilyFallbackForSettings(
    fontType: themePreferenceSnapshot.fontType,
    systemFontName: themePreferenceSnapshot.systemFontName,
  );
}

List<String> _fontFamilyFallbackForSettings({
  required String fontType,
  required String? systemFontName,
}) {
  if (_usesFontAsset(fontType)) {
    return const <String>[];
  }
  if (fontType != UserPreferencesManager.FONT_TYPE_SYSTEM) {
    return _defaultFontFamilyFallback;
  }
  return switch (systemFontName) {
    UserPreferencesManager.SYSTEM_FONT_SERIF => _serifFontFamilyFallback,
    UserPreferencesManager.SYSTEM_FONT_MONOSPACE =>
      _monospaceFontFamilyFallback,
    _ => _defaultFontFamilyFallback,
  };
}

/// Prepares a complete snapshot without publishing aliases from a stale load.
Future<Map<String, String>> _prepareCustomFonts(
  ThemePreferenceSnapshot snapshot,
) async {
  final paths = _themeFontReferences(snapshot);
  final families = <String, String>{};
  for (final path in paths) {
    if (path == null || path.isEmpty) {
      throw StateError(
        'A file font requires a nonempty runtime asset reference',
      );
    }
    if (families.containsKey(path)) continue;
    final bytes = await ThemeAssetStore().readFontBytes(path);
    final prepared = await _customFontRegistry.registerBytes(
      reference: path,
      bytes: bytes,
    );
    families[path] = prepared.family;
  }
  return Map<String, String>.unmodifiable(families);
}

/// Identifies the exact font assets that require preparation before publication.
List<String?> _themeFontReferences(ThemePreferenceSnapshot snapshot) =>
    <String?>[
      if (_usesFontAsset(snapshot.fontType)) snapshot.customFontPath,
      if (snapshot.bubbleUserUseCustomFont &&
          _usesFontAsset(snapshot.bubbleUserFontType))
        snapshot.bubbleUserCustomFontPath,
      if (snapshot.bubbleAiUseCustomFont &&
          _usesFontAsset(snapshot.bubbleAiFontType))
        snapshot.bubbleAiCustomFontPath,
    ];

String _customFontFamilyForPath(String path) {
  final family = _loadedCustomFontFamilies[path];
  if (family == null) {
    throw StateError('Theme font has not completed byte preparation: $path');
  }
  return family;
}

/// Identifies persisted faces that must be registered before publishing a theme.
bool _usesFontAsset(String fontType) =>
    fontType == UserPreferencesManager.FONT_TYPE_FILE ||
    fontType == UserPreferencesManager.FONT_TYPE_DEVICE;

final ThemeFontRegistry _customFontRegistry = ThemeFontRegistry();
final Map<String, String> _loadedCustomFontFamilies = <String, String>{};

const String _defaultFontFamily = 'Aptos';

// Resolve Chinese glyphs explicitly on iOS before other fallback families.
List<String> get _defaultFontFamilyFallback =>
    defaultTargetPlatform == TargetPlatform.iOS
    ? const <String>['PingFang SC', ..._defaultFontFamilyFallbackBase]
    : _defaultFontFamilyFallbackBase;

const List<String> _defaultFontFamilyFallbackBase = <String>[
  'Calibri',
  'Segoe UI',
  'Microsoft YaHei UI',
  'Microsoft YaHei',
  'SimHei',
  'Noto Sans CJK SC',
  'Source Han Sans SC',
  'Roboto',
  'Arial',
];

const List<String> _serifFontFamilyFallback = <String>[
  'Times New Roman',
  'Songti SC',
  'SimSun',
  'Noto Serif CJK SC',
  'serif',
];

const List<String> _monospaceFontFamilyFallback = <String>[
  'Consolas',
  'JetBrains Mono',
  'SF Mono',
  'Menlo',
  'monospace',
];

const Color _brandSeedColor = Color(0xFF3B82F6);

/// Resolves the application's Material palette for the requested brightness.
ColorScheme operitColorScheme(
  Brightness brightness,
  ThemePreferenceSnapshot themePreferenceSnapshot,
) {
  final palette = themePreferenceSnapshot.themePaletteJson;
  if (palette != null) {
    return ThemeVisualSpecV1.colorSchemeFromPalette(
      jsonDecode(palette) as Map<String, dynamic>,
      brightness,
    );
  }
  final customPrimary =
      themePreferenceSnapshot.useCustomColors &&
          themePreferenceSnapshot.customPrimaryColor != null
      ? Color(themePreferenceSnapshot.customPrimaryColor!)
      : null;
  final seedColor = customPrimary ?? _brandSeedColor;
  final customSecondary =
      themePreferenceSnapshot.useCustomColors &&
          themePreferenceSnapshot.customSecondaryColor != null
      ? Color(themePreferenceSnapshot.customSecondaryColor!)
      : null;
  final monochrome =
      _isGrayscaleThemeColor(seedColor) &&
      (customSecondary == null || _isGrayscaleThemeColor(customSecondary));
  var scheme = ColorScheme.fromSeed(
    seedColor: seedColor,
    brightness: brightness,
    dynamicSchemeVariant: _isGrayscaleThemeColor(seedColor)
        ? DynamicSchemeVariant.monochrome
        : customPrimary == null
        ? DynamicSchemeVariant.tonalSpot
        : DynamicSchemeVariant.fidelity,
  );

  if (customPrimary != null) {
    final foreground = _themeAccentForeground(customPrimary);
    scheme = scheme.copyWith(
      primary: customPrimary,
      onPrimary: foreground,
      primaryContainer: customPrimary,
      onPrimaryContainer: foreground,
      surfaceTint: customPrimary,
    );
  }
  if (customSecondary != null) {
    final secondaryScheme = ColorScheme.fromSeed(
      seedColor: customSecondary,
      brightness: brightness,
      dynamicSchemeVariant: _isGrayscaleThemeColor(customSecondary)
          ? DynamicSchemeVariant.monochrome
          : DynamicSchemeVariant.fidelity,
    );
    final foreground = _themeAccentForeground(customSecondary);
    scheme = scheme.copyWith(
      secondary: customSecondary,
      onSecondary: foreground,
      secondaryContainer: customSecondary,
      onSecondaryContainer: foreground,
      secondaryFixed: secondaryScheme.primaryFixed,
      secondaryFixedDim: secondaryScheme.primaryFixedDim,
      onSecondaryFixed: secondaryScheme.onPrimaryFixed,
      onSecondaryFixedVariant: secondaryScheme.onPrimaryFixedVariant,
    );
  }

  // Preserve neutral surfaces when both selected accents are grayscale.
  if (monochrome) {
    return scheme;
  }

  // Decouple neutral surface tones from seed color to eliminate cyan/teal tint,
  // providing a clean, professional neutral dark palette and crisp neutral light palette.
  if (brightness == Brightness.dark) {
    return scheme.copyWith(
      surface: const Color(0xFF161616),
      surfaceDim: const Color(0xFF121212),
      surfaceBright: const Color(0xFF262626),
      surfaceContainerLowest: const Color(0xFF0D0E10),
      surfaceContainerLow: const Color(0xFF131314),
      surfaceContainer: const Color(0xFF18181A),
      surfaceContainerHigh: const Color(0xFF202023),
      surfaceContainerHighest: const Color(0xFF26272B),
      outline: const Color(0xFF4C4D53),
      outlineVariant: const Color(0xFF262628),
      onSurface: const Color(0xFFECECEC),
      onSurfaceVariant: const Color(0xFFA0A1A7),
    );
  } else {
    return scheme.copyWith(
      surface: const Color(0xFFFFFFFF),
      surfaceDim: const Color(0xFFEDEDF0),
      surfaceBright: const Color(0xFFFFFFFF),
      surfaceContainerLowest: const Color(0xFFFFFFFF),
      surfaceContainerLow: const Color(0xFFF7F8F9),
      surfaceContainer: const Color(0xFFF0F2F5),
      surfaceContainerHigh: const Color(0xFFE8EAEF),
      surfaceContainerHighest: const Color(0xFFE0E3E8),
      outline: const Color(0xFF8F929A),
      outlineVariant: const Color(0xFFE2E4E8),
      onSurface: const Color(0xFF1B1C1E),
      onSurfaceVariant: const Color(0xFF5A5C63),
    );
  }
}

/// Identifies achromatic seed colors without treating dark chromatic colors as gray.
bool _isGrayscaleThemeColor(Color color) =>
    color.r == color.g && color.g == color.b;

/// Chooses the black or white foreground with the higher WCAG contrast ratio.
Color _themeAccentForeground(Color background) {
  final luminance = background.computeLuminance();
  final blackContrast = (luminance + 0.05) / 0.05;
  final whiteContrast = 1.05 / (luminance + 0.05);
  return blackContrast >= whiteContrast ? Colors.black : Colors.white;
}
