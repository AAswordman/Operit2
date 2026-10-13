// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:operit_theme/operit_theme.dart';
import 'package:video_player/video_player.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/errors/UnhandledErrorReporter.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../core/runtime/RuntimeBootstrapManager.dart';
import '../../data/preferences/UserPreferencesManager.dart';
import '../../l10n/generated/app_localizations.dart';
import '../common/AppToastHost.dart';
import '../common/RuntimeBootstrapScreen.dart';
import '../common/layout/ApplicationZoom.dart';
import '../common/layout/ApplicationZoomPreferences.dart';
import '../features/chat/tts/TtsFloatingPanel.dart';
import '../features/startup/PluginLoadingOverlay.dart';
import '../../core/host/browser/RuntimeBrowserOwnerHost.dart';
import '../features/chat/components/workspace/browser/automation/WorkspaceWebVisitHost.dart';
import 'OperitThemeAssets.dart';
import 'ThemeCircularRevealHost.dart';
import 'ThemeLibraryService.dart';
import '../features/settings/appearance/ThemeLibraryPanel.dart';

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
  double _zoom = 1.0;
  Future<void> _zoomSave = Future<void>.value();
  StreamSubscription<LongPastedTextInputSettings>? _longPasteSubscription;
  late final OperitThemeController _controller = OperitThemeController(
    onChanged: () {
      if (mounted) {
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
    // The 78-field first-frame transport cannot express a package's full
    // palette/resources. Each engine reads and prepares the CoreNode choice.
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
    _cancelPreferenceSubscriptions();
    super.dispose();
  }

  /// Synchronizes the theme lifecycle with runtime configuration changes.
  void _handleRuntimeBootstrapChanged() {
    if (!_runtimeManager.runtimeConfigured) {
      _runtimeGeneration++;
      _stopRuntimeStartupStatusPolling();
      _controller.dispose();
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
      if (!mounted ||
          generation != _runtimeGeneration ||
          !_runtimeManager.runtimeConfigured) {
        return;
      }
      _longPasteSubscription = const UserPreferencesManager()
          .longPastedTextInputSettingsFlow()
          .listen((settings) {
            if (!mounted || generation != _runtimeGeneration) {
              return;
            }
            UserPreferencesManager.longPastedTextInputSettings.value = settings;
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
    final activeThemeReady = runtimeReady && !_controller.themeIsPreparing &&
        _controller.themeLoadError == null;
    // Bootstrap/recovery chrome is not an applied theme. App content stays
    // blocked on selection errors; no selection is replaced automatically.
    final themeSnapshot = activeThemeReady
        ? _controller.themePreferenceSnapshot
        : UserPreferencesManager.defaultThemePreferenceSnapshot;
    final themeMode = runtimeReady
        ? _controller.themeMode
        : widget.initialThemeMode ??
              widget.initialThemePreferenceSnapshot.themeMode;
    final Widget appChild;
    if (_preserveUnconfiguredChild && !runtimeReady) {
      appChild = Stack(
        fit: StackFit.expand,
        children: <Widget>[
          widget.child,
          if (runtimeConfigured && !runtimeReady)
            RuntimeBootstrapScreen(
              message: _runtimeStartupMessage,
              errorText: _runtimeStartupError?.toString(),
            ),
        ],
      );
    } else if (!runtimeConfigured) {
      appChild = widget.unconfiguredChildEnabled
          ? widget.child
          : const RuntimeBootstrapScreen(message: '请先在主窗口配置运行时目录和工作区目录');
    } else if (!runtimeReady) {
      appChild = RuntimeBootstrapScreen(
        message: _runtimeStartupMessage,
        errorText: _runtimeStartupError?.toString(),
      );
    } else {
      // Keep chat/navigation state mounted across a theme preparation barrier.
      // Offstage content is not painted or interactive and retains its own
      // validated asset resolver; recovery chrome cannot reinterpret its paths.
      appChild = Stack(fit: StackFit.expand, children: <Widget>[
        Offstage(
          offstage: !activeThemeReady,
          child: TickerMode(
            enabled: activeThemeReady,
            child: ExcludeFocus(
              excluding: !activeThemeReady,
              child: ThemeAssetScope(
                resolver: _controller.preparedVisual == null
                    ? _themeAssets : _controller.preparedVisual!.resources,
                child: widget.child,
              ),
            ),
          ),
        ),
        if (_controller.themeLoadError != null)
          ThemeLibraryRecoveryScreen(controller: _controller),
        if (_controller.themeIsPreparing && _controller.themeLoadError == null)
          const RuntimeBootstrapScreen(message: '正在校验并准备本节点主题资源'),
      ]);
    }
    return _OperitThemeScope(
      controller: _controller,
      child: ThemeAssetScope(
        resolver: _controller.preparedVisual == null
            ? _themeAssets : _controller.preparedVisual!.resources,
        child: _OperitMaterialApp(
          themeMode: themeMode,
          themePreferenceSnapshot: themeSnapshot,
          preparedVisual: activeThemeReady ? _controller.preparedVisual : null,
          hostInteractionHostsEnabled:
              activeThemeReady && widget.hostInteractionHostsEnabled,
          zoom: _zoom,
          zoomShortcutsEnabled: runtimeReady && _runtimeStartCompleted,
          onZoomChanged: _changeZoom,
          child: appChild,
        ),
      ),
    );
  }
}

class _OperitMaterialApp extends StatelessWidget {
  const _OperitMaterialApp({
    required this.themeMode,
    required this.themePreferenceSnapshot,
    required this.preparedVisual,
    required this.hostInteractionHostsEnabled,
    required this.zoom,
    required this.zoomShortcutsEnabled,
    required this.onZoomChanged,
    required this.child,
  });

  final ThemeMode themeMode;
  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final PreparedThemeVisual? preparedVisual;
  final bool hostInteractionHostsEnabled;
  final double zoom;
  final bool zoomShortcutsEnabled;
  final ValueChanged<double> onZoomChanged;
  final Widget child;

  /// Builds the themed application and process-wide host overlays.
  @override
  Widget build(BuildContext context) {
    final visual = preparedVisual;
    final lightColorScheme = visual == null
        ? _seedColorScheme(Brightness.light, themePreferenceSnapshot)
        : visual.spec.colorScheme(Brightness.light);
    final darkColorScheme = visual == null
        ? _seedColorScheme(Brightness.dark, themePreferenceSnapshot)
        : visual.spec.colorScheme(Brightness.dark);
    return ThemeRenderRoot(
      title: 'Operit2',
      themePreferenceSnapshot: themePreferenceSnapshot,
      lightColorScheme: lightColorScheme,
      darkColorScheme: darkColorScheme,
      fontFamily: visual == null ? _fontFamily(themePreferenceSnapshot) : visual.resources.sans.family,
      fontFamilyFallback: visual == null ? _fontFamilyFallback(themePreferenceSnapshot) : const <String>[],
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
              child: materialChild!,
            ),
          ),
        );
        return ApplicationZoomHost(
          zoom: zoom,
          shortcutsEnabled: zoomShortcutsEnabled,
          onZoomChanged: onZoomChanged,
          child: appContent,
        );
      },
      home: UnhandledErrorHost(
        child: RuntimeBrowserOwnerHost(
          enabled: hostInteractionHostsEnabled,
          child: WorkspaceWebVisitHost(
            child: AppToastHost(
              child: Stack(
                fit: StackFit.expand,
                children: <Widget>[
                  Positioned.fill(child: child),
                  const TtsFloatingPanel(),
                  PluginLoadingOverlay(
                    key: const ValueKey<String>('plugin-loading-overlay'),
                    enabled: hostInteractionHostsEnabled,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Isolated client preview: real chat widgets see a private theme scope. It has
/// no start/subscription, persistence callback or selection write.
class OperitThemePreview extends StatelessWidget {
  const OperitThemePreview({
    super.key, required this.visual, required this.settings, required this.child,
  });

  final PreparedThemeVisual visual;
  final ThemePreferenceSnapshot settings;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final snapshot = visual.snapshot(settings);
    _loadedCustomFontFamilies.addAll(_preparedFontFamilies(visual));
    final controller = OperitThemeController(onChanged: () {});
    controller.setInitialThemePreferenceSnapshot(snapshot);
    return _OperitThemeScope(
      controller: controller,
      child: ThemeAssetScope(
        resolver: visual.resources,
        child: ThemeRenderRoot(
          title: 'Operit2 · 独立主题预览',
          themePreferenceSnapshot: snapshot,
          lightColorScheme: visual.spec.colorScheme(Brightness.light),
          darkColorScheme: visual.spec.colorScheme(Brightness.dark),
          fontFamily: visual.resources.sans.family,
          fontFamilyFallback: const <String>[],
          themeMode: settings.themeMode,
          themeAnimationDuration: Duration.zero,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          builder: (context, child) => OperitThemeBackground(
            themePreferenceSnapshot: snapshot, child: child!,
          ),
          home: child,
        ),
      ),
    );
  }
}

Map<String, String> _preparedFontFamilies(PreparedThemeVisual visual) => <String, String>{
  for (final font in <PreparedThemeFont>[
    visual.resources.sans, visual.resources.mono, visual.resources.user, visual.resources.ai,
  ]) font.family: font.family,
};

class OperitThemeController {
  /// Creates the controller that owns the effective application theme.
  OperitThemeController({
    required VoidCallback onChanged,
    Future<void> Function(ThemeMode)? saveStartupThemeMode,
    UserPreferencesManager preferencesManager = const UserPreferencesManager(),
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _onChanged = onChanged,
       _saveStartupThemeMode =
           saveStartupThemeMode ?? _saveStartupThemeModeDefault,
       _preferencesManager = preferencesManager,
       _clients = clients;

  final VoidCallback _onChanged;
  final Future<void> Function(ThemeMode) _saveStartupThemeMode;
  final UserPreferencesManager _preferencesManager;
  final GeneratedCoreProxyClients _clients;
  StreamSubscription<core_proxy.ThemeNodeState>? _nodeThemeSubscription;
  late final ThemeLibraryService library = ThemeLibraryService(_clients);
  ThemePreferenceSnapshot _themePreferenceSnapshot =
      UserPreferencesManager.defaultThemePreferenceSnapshot;
  ThemePreferenceSnapshot _personalSnapshot =
      UserPreferencesManager.defaultThemePreferenceSnapshot;
  core_proxy.ThemeSelection? _selection;
  PreparedThemeVisual? _preparedVisual;
  (String, String, String)? _preparedIdentity;
  PreparedThemeVisual? _stagedVisual;
  (String, String, String)? _stagedIdentity;
  bool _applyingSelection = false;
  Object? _themeLoadError;
  bool _themeIsPreparing = false;
  bool _hasLoadedState = false;
  int _themePreferenceRevision = 0;
  int _lifecycleRevision = 0;
  bool _suppressThemeAnimation = false;

  ThemeMode get themeMode => _themePreferenceSnapshot.themeMode;
  ThemePreferenceSnapshot get themePreferenceSnapshot => _themePreferenceSnapshot;
  core_proxy.ThemeSelection? get selection => _selection;
  PreparedThemeVisual? get preparedVisual => _preparedVisual;
  Object? get themeLoadError => _themeLoadError;
  bool get themeIsPreparing => _themeIsPreparing;
  bool get packageVisualsLocked => _selection != null;
  bool get suppressThemeAnimation => _suppressThemeAnimation;

  /// Applies the known first-frame theme state before asynchronous startup.
  void setInitialThemePreferenceSnapshot(ThemePreferenceSnapshot snapshot) {
    _themePreferenceSnapshot = snapshot;
  }

  /// Initializes fresh node-global settings and observes this CoreNode only.
  /// Internal-test role preferences are intentionally neither read nor migrated.
  Future<void> start() async {
    final lifecycle = ++_lifecycleRevision;
    await _nodeThemeSubscription?.cancel();
    _nodeThemeSubscription = null;
    await _clients.application.initializeNodeThemeAppearance(
      snapshotJson: jsonEncode(UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()),
    );
    if (lifecycle != _lifecycleRevision) return;
    await _reloadThemePreferenceSnapshot();
    if (lifecycle != _lifecycleRevision) return;
    _nodeThemeSubscription = _clients.application.nodeThemeStateFlow().listen(
      (state) {
        if (_applyingSelection) return;
        unawaited(_acceptNodeState(state));
      },
      onError: (Object error, StackTrace stack) {
        if (lifecycle != _lifecycleRevision) return;
        _themePreferenceRevision++;
        _themeIsPreparing = false;
        _themeLoadError = error;
        _onChanged();
      },
    );
  }

  void dispose() {
    _lifecycleRevision++;
    _themePreferenceRevision++;
    unawaited(_nodeThemeSubscription?.cancel());
    _nodeThemeSubscription = null;
    _stagedVisual?.dispose();
    _stagedVisual = null;
    _stagedIdentity = null;
    _applyingSelection = false;
    _preparedVisual?.dispose();
    _preparedVisual = null;
    _preparedIdentity = null;
    _selection = null;
    _themePreferenceSnapshot =
        UserPreferencesManager.defaultThemePreferenceSnapshot;
    _personalSnapshot =
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
    await _preferencesManager.saveThemeSettings(
      themeMode: themeMode,
    );
    await _reloadThemePreferenceSnapshot();
  }

  /// Applies transient slider values without persisting them.
  void previewThemeSettings({
    double? backgroundImageOpacity,
    double? backgroundBlurRadius,
    double? fontScale,
  }) {
    // Allow live slider preview for opacity, blur and fontScale even when a package is active.
    final snapshot = _themePreferenceSnapshotWith(
      _themePreferenceSnapshot,
      backgroundImageOpacity: backgroundImageOpacity,
      backgroundBlurRadius: backgroundBlurRadius,
      fontScale: fontScale,
    );
    _themePreferenceSnapshot = snapshot;
    _onChanged();
  }

  /// Persists a partial node-global theme update.
  Future<void> saveThemeSettings({
    String? inputStyle,
    String? chatStyle,
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
      chatStyle: chatStyle,
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
    await _preferencesManager.saveNodeThemeSettings(<String, Object?>{
      'cursorUserBubbleColor': null, 'bubbleUserBubbleColor': null,
      'bubbleAiBubbleColor': null, 'bubbleUserTextColor': null, 'bubbleAiTextColor': null,
    });
    await _reloadThemePreferenceSnapshot();
  }

  /// Resets personal appearance only; never removes a package selection.
  Future<void> resetThemeSettings() async {
    if (packageVisualsLocked) throw StateError('Select personal appearance before resetting visuals');
    await _preferencesManager.saveNodeThemeSettings(
      UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
    );
    await _reloadThemePreferenceSnapshot();
  }

  Future<List<core_proxy.ThemeLibraryItem>> listThemeLibrary() =>
      _clients.application.listThemeLibrary();

  /// Deletes a theme package from the Space library. When [unselectIfActive]
  /// is true and this node currently selects the target package, switches this
  /// node to personal appearance first before deleting the package.
  Future<void> deleteLibraryTheme({
    required String themeId,
    required String archiveSha256,
    bool unselectIfActive = false,
  }) async {
    final current = _selection;
    final isActive = current != null &&
        current.themeId == themeId &&
        current.archiveSha256 == archiveSha256;
    if (isActive) {
      if (!unselectIfActive) {
        throw StateError('请先切回本节点个人外观后再删除当前生效的主题包');
      }
      await selectPersonalAppearance();
    }
    await library.deleteTheme(themeId, archiveSha256);
  }

  Future<core_proxy.ThemeInstallReceipt> applyLibraryTheme(core_proxy.ThemeLibraryEntry entry) async {
    // Preflight happens before the CoreNode selection write. Preview owns a
    // different prepared instance and can never submit a selection itself.
    final prepared = await library.prepare(entry.themeId, entry.archiveSha256, expectedVersion: entry.version);
    final targetIdentity = (entry.themeId, entry.version, entry.archiveSha256);
    _stagedVisual?.dispose();
    _stagedVisual = prepared;
    _stagedIdentity = targetIdentity;
    _applyingSelection = true;
    try {
      final packageDefaults = prepared.spec
          .toSnapshot(_personalSnapshot, preserveUserSettings: false)
          .toJson();
      await _preferencesManager.saveNodeThemeSettings(<String, Object?>{
        for (final key in kUserConfigurableThemeFields)
          key: packageDefaults[key],
      });
      final receipt = await _clients.application.applyTheme(
        themeId: entry.themeId, archiveSha256: entry.archiveSha256,
      );
      final next = receipt.selection;
      if (receipt.status != core_proxy.ThemeInstallStatus.applied || next == null ||
          next.themeId != entry.themeId || next.version != entry.version ||
          next.archiveSha256 != entry.archiveSha256) {
        throw StateError('Theme application receipt differs from the explicit selection');
      }
      await _reloadThemePreferenceSnapshot();
      return receipt;
    } finally {
      _applyingSelection = false;
      if (identical(_stagedVisual, prepared)) {
        _stagedVisual = null;
        _stagedIdentity = null;
        if (!identical(_preparedVisual, prepared)) {
          prepared.dispose();
        }
      }
    }
  }

  Future<void> selectPersonalAppearance() async {
    _applyingSelection = true;
    try {
      await _prepareCustomFonts(_personalSnapshot);
      await _clients.application.selectPersonalThemeAppearance();
      await _reloadThemePreferenceSnapshot();
    } finally {
      _applyingSelection = false;
    }
  }

  Future<void> reloadTheme() => _reloadThemePreferenceSnapshot(revalidatePackage: true);

  Future<void> saveNodeThemeSettings(Map<String, Object?> updates) async {
    await _preferencesManager.saveNodeThemeSettings(updates);
    await _reloadThemePreferenceSnapshot();
  }

  Future<void> _reloadThemePreferenceSnapshot({bool revalidatePackage = false}) async {
    final lifecycle = _lifecycleRevision;
    final revision = ++_themePreferenceRevision;
    final state = await _clients.application.nodeThemeState();
    if (lifecycle != _lifecycleRevision || revision != _themePreferenceRevision) return;
    await _acceptNodeState(state, requestLifecycle: lifecycle, requestRevision: revision,
      revalidatePackage: revalidatePackage);
  }

  Future<void> _acceptNodeState(core_proxy.ThemeNodeState state, {
    int? requestLifecycle, int? requestRevision, bool revalidatePackage = false,
  }) async {
    final lifecycle = requestLifecycle ?? _lifecycleRevision;
    final revision = requestRevision ?? ++_themePreferenceRevision;
    bool current() => lifecycle == _lifecycleRevision && revision == _themePreferenceRevision;
    if (!current()) return;
    PreparedThemeVisual? candidate;
    final selectedIdentity = state.selection == null ? null :
        (state.selection!.themeId, state.selection!.version, state.selection!.archiveSha256);
    final hasStaged = !revalidatePackage &&
        selectedIdentity != null &&
        _stagedIdentity == selectedIdentity &&
        _stagedVisual != null;
    _themeIsPreparing = !hasStaged &&
        (revalidatePackage ||
            !_hasLoadedState ||
            _themeLoadError != null ||
            (selectedIdentity != null && selectedIdentity != _preparedIdentity));
    // Mode/scale/visibility edits reuse prepared package resources and preserve
    // the live app subtree. A new or failed visual identity uses the barrier.
    if (_themeIsPreparing) _onChanged();
    try {
      final source = state.privateAppearanceJson;
      if (state.schemaVersion != 1 || source == null) {
        throw StateError('Node-global appearance state is not initialized');
      }
      final personal = ThemePreferenceSnapshot.fromJson(jsonDecode(source) as Map<String, dynamic>);
      final selected = state.selection;
      final ThemePreferenceSnapshot snapshot;
      final Map<String, String> families;
      (String, String, String)? identity;
      if (selected == null) {
        snapshot = personal;
        families = await _prepareCustomFonts(personal);
      } else {
        identity = (selected.themeId, selected.version, selected.archiveSha256);
        if (!revalidatePackage && _stagedIdentity == identity && _stagedVisual != null) {
          candidate = _stagedVisual;
          _stagedVisual = null;
          _stagedIdentity = null;
        } else if (!revalidatePackage && _preparedIdentity == identity && _preparedVisual != null) {
          candidate = _preparedVisual;
        } else {
          candidate = await library.prepare(selected.themeId, selected.archiveSha256, expectedVersion: selected.version);
        }
        final prepared = candidate!;
        snapshot = prepared.snapshot(personal, preserveUserSettings: true);
        families = _preparedFontFamilies(prepared);
      }
      if (!current()) return;
      await _saveStartupThemeMode(snapshot.themeMode);
      if (!current()) return;
      _loadedCustomFontFamilies.addAll(families);
      final previous = _preparedVisual;
      _personalSnapshot = personal;
      _selection = state.selection;
      _preparedVisual = candidate;
      candidate = null;
      _preparedIdentity = identity;
      _themePreferenceSnapshot = snapshot;
      _themeLoadError = null;
      _themeIsPreparing = false;
      _hasLoadedState = true;
      _onChanged();
      if (previous != null && !identical(previous, _preparedVisual)) {
        WidgetsBinding.instance.addPostFrameCallback((_) => previous.dispose());
      }
    } catch (error) {
      if (!current()) return;
      _selection = state.selection;
      _themeLoadError = error;
      _themeIsPreparing = false;
      _onChanged();
    } finally {
      if (candidate != null && !identical(candidate, _preparedVisual)) candidate.dispose();
    }
  }

}

/// Persists the first-frame theme mode through the existing bootstrap host.
Future<void> _saveStartupThemeModeDefault(ThemeMode themeMode) {
  return RuntimeBootstrapManager.instance.saveStartupThemeMode(themeMode);
}

class _OperitThemeScope extends InheritedWidget {
  const _OperitThemeScope({required this.controller, required super.child});

  final OperitThemeController controller;

  @override
  bool updateShouldNotify(_OperitThemeScope oldWidget) {
    return true;
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
    chatStyle: snapshot.chatStyle,
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
                '${themePreferenceSnapshot.backgroundMediaType}|$mediaPath',
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
        ? _VideoThemeBackground(mediaPath: mediaPath, muted: muted, loop: loop)
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
  });

  final String mediaPath;
  final bool muted;
  final bool loop;

  /// Creates the video state that owns the runtime asset controller.
  @override
  State<_VideoThemeBackground> createState() => _VideoThemeBackgroundState();
}

class _VideoThemeBackgroundState extends State<_VideoThemeBackground> {
  VideoPlayerController? _controller;
  int _loadToken = 0;

  /// Starts loading the runtime video asset when the widget mounts.
  @override
  void initState() {
    super.initState();
    unawaited(_loadController());
  }

  /// Updates playback settings or reloads bytes when the video asset changes.
  @override
  void didUpdateWidget(covariant _VideoThemeBackground oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.mediaPath != widget.mediaPath) {
      unawaited(_loadController());
      return;
    }
    final controller = _controller;
    if (controller == null) {
      return;
    }
    if (oldWidget.loop != widget.loop) {
      unawaited(controller.setLooping(widget.loop));
    }
    if (oldWidget.muted != widget.muted) {
      unawaited(controller.setVolume(widget.muted ? 0 : 1));
    }
  }

  /// Creates and starts a controller from imported runtime asset bytes.
  Future<void> _loadController() async {
    final token = ++_loadToken;
    final bytes = await ThemeAssetStore().readBytes(widget.mediaPath);
    final controller =
        VideoPlayerController.networkUrl(
            Uri.dataFromBytes(
              bytes,
              mimeType: themeAssetVideoMimeType(widget.mediaPath),
            ),
          )
          ..setLooping(widget.loop)
          ..setVolume(widget.muted ? 0 : 1);
    await controller.initialize();
    await controller.play();
    if (!mounted || token != _loadToken) {
      await controller.dispose();
      return;
    }
    final previous = _controller;
    setState(() {
      _controller = controller;
    });
    await previous?.dispose();
  }

  /// Releases the active video controller.
  @override
  void dispose() {
    _loadToken++;
    unawaited(_controller?.dispose());
    super.dispose();
  }

  /// Builds the fitted video once the controller is initialized.
  @override
  Widget build(BuildContext context) {
    final controller = _controller;
    if (controller == null || !controller.value.isInitialized) {
      return const SizedBox.expand();
    }
    return SizedBox.expand(
      child: FittedBox(
        fit: BoxFit.cover,
        child: SizedBox(
          width: controller.value.size.width,
          height: controller.value.size.height,
          child: VideoPlayer(controller),
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
  final path = isUser && themePreferenceSnapshot.bubbleUserUseCustomFont
      ? themePreferenceSnapshot.bubbleUserCustomFontPath
      : !isUser && themePreferenceSnapshot.bubbleAiUseCustomFont
      ? themePreferenceSnapshot.bubbleAiCustomFontPath
      : themePreferenceSnapshot.customFontPath;
  if (path != null && _loadedCustomFontFamilies[path] == path) return const <String>[];
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
  if (fontType == UserPreferencesManager.FONT_TYPE_FILE) {
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
  final path = themePreferenceSnapshot.customFontPath;
  if (path != null && _loadedCustomFontFamilies[path] == path) return const <String>[];
  return _fontFamilyFallbackForSettings(
    fontType: themePreferenceSnapshot.fontType,
    systemFontName: themePreferenceSnapshot.systemFontName,
  );
}

List<String> _fontFamilyFallbackForSettings({
  required String fontType,
  required String? systemFontName,
}) {
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
  final paths = <String?>[
    if (snapshot.fontType == UserPreferencesManager.FONT_TYPE_FILE)
      snapshot.customFontPath,
    if (snapshot.bubbleUserUseCustomFont &&
        snapshot.bubbleUserFontType == UserPreferencesManager.FONT_TYPE_FILE)
      snapshot.bubbleUserCustomFontPath,
    if (snapshot.bubbleAiUseCustomFont &&
        snapshot.bubbleAiFontType == UserPreferencesManager.FONT_TYPE_FILE)
      snapshot.bubbleAiCustomFontPath,
  ];
  final families = <String, String>{};
  for (final path in paths) {
    if (path == null || path.isEmpty) {
      throw StateError(
        'A file font requires a nonempty runtime asset reference',
      );
    }
    if (families.containsKey(path)) continue;
    final bytes = await ThemeAssetStore().readBytes(path);
    final prepared = await _customFontRegistry.registerBytes(
      reference: path,
      bytes: bytes,
    );
    families[path] = prepared.family;
  }
  return Map<String, String>.unmodifiable(families);
}

String _customFontFamilyForPath(String path) {
  final family = _loadedCustomFontFamilies[path];
  if (family == null) {
    throw StateError('Theme font has not completed byte preparation: $path');
  }
  return family;
}

final ThemeFontRegistry _customFontRegistry = ThemeFontRegistry();
final Map<String, String> _loadedCustomFontFamilies = <String, String>{};

const String _defaultFontFamily = 'Aptos';

const List<String> _defaultFontFamilyFallback = <String>[
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

const Color _brandSeedColor = Color(0xFFBBDEFB);

/// Builds one Material color scheme and applies the secondary accent color.
ColorScheme _seedColorScheme(
  Brightness brightness,
  ThemePreferenceSnapshot themePreferenceSnapshot,
) {
  final seedColor =
      themePreferenceSnapshot.useCustomColors &&
          themePreferenceSnapshot.customPrimaryColor != null
      ? Color(themePreferenceSnapshot.customPrimaryColor!)
      : _brandSeedColor;
  final scheme = ColorScheme.fromSeed(
    seedColor: seedColor,
    brightness: brightness,
    dynamicSchemeVariant: DynamicSchemeVariant.tonalSpot,
  );
  if (!themePreferenceSnapshot.useCustomColors ||
      themePreferenceSnapshot.customSecondaryColor == null) {
    return scheme;
  }
  return scheme.copyWith(
    secondary: Color(themePreferenceSnapshot.customSecondaryColor!),
  );
}
