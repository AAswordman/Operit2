// ignore_for_file: file_names

import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:video_player/video_player.dart';

import '../../core/errors/UnhandledErrorReporter.dart';
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
    _runtimeUiReady =
        widget.initialThemeIsReady && _runtimeManager.runtimeConfigured;
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
    final themeSnapshot = runtimeReady
        ? _controller.themePreferenceSnapshot
        : UserPreferencesManager.defaultThemePreferenceSnapshot;
    final themeMode = runtimeReady
        ? _controller.themeMode
        : widget.initialThemeMode ??
              widget.initialThemePreferenceSnapshot.themeMode;
    final Widget appChild;
    if (_preserveUnconfiguredChild) {
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
      appChild = widget.child;
    }
    return _OperitThemeScope(
      controller: _controller,
      child: _OperitMaterialApp(
        themeMode: themeMode,
        themePreferenceSnapshot: themeSnapshot,
        hostInteractionHostsEnabled:
            runtimeReady && widget.hostInteractionHostsEnabled,
        zoom: _zoom,
        zoomShortcutsEnabled: runtimeReady && _runtimeStartCompleted,
        onZoomChanged: _changeZoom,
        child: appChild,
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
    required this.child,
  });

  final ThemeMode themeMode;
  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final bool hostInteractionHostsEnabled;
  final double zoom;
  final bool zoomShortcutsEnabled;
  final ValueChanged<double> onZoomChanged;
  final Widget child;

  /// Builds the themed application and process-wide host overlays.
  @override
  Widget build(BuildContext context) {
    final lightColorScheme = _seedColorScheme(
      Brightness.light,
      themePreferenceSnapshot,
    );
    final darkColorScheme = _seedColorScheme(
      Brightness.dark,
      themePreferenceSnapshot,
    );
    return MaterialApp(
      title: 'Operit2',
      debugShowCheckedModeBanner: false,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: _themeData(lightColorScheme, themePreferenceSnapshot),
      darkTheme: _themeData(darkColorScheme, themePreferenceSnapshot),
      themeMode: themeMode,
      themeAnimationDuration: OperitTheme.of(context).suppressThemeAnimation
          ? Duration.zero
          : kThemeAnimationDuration,
      builder: (context, materialChild) {
        final appContent = ThemeCircularRevealHost(
          child: AnnotatedRegion<SystemUiOverlayStyle>(
            value: _systemUiOverlayStyle(Theme.of(context).colorScheme),
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
  StreamSubscription<Map<String, String>>? _themePreferencesSubscription;
  ThemePreferenceSnapshot _themePreferenceSnapshot =
      UserPreferencesManager.defaultThemePreferenceSnapshot;
  int _themePreferenceRevision = 0;
  int _lifecycleRevision = 0;
  bool _suppressThemeAnimation = false;

  ThemeMode get themeMode => _themePreferenceSnapshot.themeMode;
  ThemePreferenceSnapshot get themePreferenceSnapshot =>
      _themePreferenceSnapshot;
  bool get suppressThemeAnimation => _suppressThemeAnimation;

  /// Applies the known first-frame theme state before asynchronous startup.
  void setInitialThemePreferenceSnapshot(ThemePreferenceSnapshot snapshot) {
    _themePreferenceSnapshot = snapshot;
  }

  /// Loads ordinary appearance values and observes their authoritative preference changes.
  Future<void> start() async {
    final lifecycleRevision = ++_lifecycleRevision;
    await _themePreferencesSubscription?.cancel();
    _themePreferencesSubscription = null;
    await _reloadThemePreferenceSnapshot();
    if (lifecycleRevision != _lifecycleRevision) {
      return;
    }
    _themePreferencesSubscription = _preferencesManager
        .preferencesFlow()
        .listen((_) => unawaited(_reloadThemePreferenceSnapshot()));
  }

  /// Invalidates in-flight loads and closes the ordinary appearance observer.
  void dispose() {
    _lifecycleRevision++;
    _themePreferenceRevision++;
    unawaited(_themePreferencesSubscription?.cancel());
    _themePreferencesSubscription = null;
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

  /// Persists and applies one ordinary appearance mode without actor-dependent scope.
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
    final snapshot = _themePreferenceSnapshotWith(
      _themePreferenceSnapshot,
      backgroundImageOpacity: backgroundImageOpacity,
      backgroundBlurRadius: backgroundBlurRadius,
      fontScale: fontScale,
    );
    _themePreferenceSnapshot = snapshot;
    _onChanged();
  }

  /// Persists a partial theme update for the active target.
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

  /// Saves the independent user-avatar setting and reloads ordinary appearance.
  Future<void> saveActiveThemeUserAvatarSettings({
    required String customUserAvatarUri,
  }) async {
    await _preferencesManager.saveGlobalUserAvatarSettings(
      customUserAvatarUri: customUserAvatarUri,
    );
    await _reloadThemePreferenceSnapshot();
  }

  /// Clears ordinary message color overrides without reading plugin references.
  Future<void> resetMessageColorSettings() async {
    await _preferencesManager.resetMessageColorSettings();
    await _reloadThemePreferenceSnapshot();
  }

  /// Resets ordinary appearance overrides without selecting or modifying a plugin actor.
  Future<void> resetThemeSettings() async {
    await _preferencesManager.resetThemeSettings();
    await _reloadThemePreferenceSnapshot();
  }

  /// Commits ordinary appearance only while the same load and controller lifetime remain current.
  Future<void> _reloadThemePreferenceSnapshot() async {
    final lifecycleRevision = _lifecycleRevision;
    final preferenceRevision = ++_themePreferenceRevision;
    final snapshot = await _preferencesManager.resolveThemePreferenceSnapshot();
    await _loadCustomFontIfNeeded(snapshot);
    if (lifecycleRevision != _lifecycleRevision ||
        preferenceRevision != _themePreferenceRevision) {
      return;
    }
    await _saveStartupThemeMode(snapshot.themeMode);
    if (lifecycleRevision != _lifecycleRevision ||
        preferenceRevision != _themePreferenceRevision) {
      return;
    }
    _themePreferenceSnapshot = snapshot;
    _onChanged();
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

/// Builds Material theme data for the effective user theme state.
ThemeData _themeData(
  ColorScheme colorScheme,
  ThemePreferenceSnapshot themePreferenceSnapshot,
) {
  final typography = Typography.material2021();
  final fontFamily = _fontFamily(themePreferenceSnapshot);
  final fontFamilyFallback = _fontFamilyFallback(themePreferenceSnapshot);
  final backgroundVisible =
      themePreferenceSnapshot.useBackgroundImage &&
      themePreferenceSnapshot.backgroundImageUri != null &&
      themePreferenceSnapshot.backgroundImageUri!.isNotEmpty;
  final appBarBackgroundColor =
      backgroundVisible || themePreferenceSnapshot.transparentSurfaceEnabled
      ? Colors.transparent
      : colorScheme.surface;
  final textTheme =
      (colorScheme.brightness == Brightness.dark
              ? typography.white
              : typography.black)
          .apply(
            fontFamily: fontFamily,
            fontFamilyFallback: fontFamilyFallback,
            fontSizeFactor: themePreferenceSnapshot.fontScale,
          );
  return ThemeData(
    colorScheme: colorScheme,
    scaffoldBackgroundColor: Colors.transparent,
    canvasColor: colorScheme.surface,
    textTheme: textTheme,
    inputDecorationTheme: _inputDecorationTheme(colorScheme, textTheme),
    dropdownMenuTheme: _dropdownMenuTheme(colorScheme, textTheme),
    menuTheme: _menuTheme(colorScheme, textTheme),
    menuButtonTheme: _menuButtonTheme(colorScheme, textTheme),
    popupMenuTheme: _popupMenuTheme(colorScheme, textTheme),
    buttonTheme: _buttonTheme(colorScheme),
    dialogTheme: _dialogTheme(colorScheme, textTheme),
    // ignore: deprecated_member_use
    progressIndicatorTheme: const ProgressIndicatorThemeData(year2023: false),
    appBarTheme: AppBarTheme(
      backgroundColor: appBarBackgroundColor,
      foregroundColor: colorScheme.onSurface,
      surfaceTintColor: Colors.transparent,
      systemOverlayStyle: _systemUiOverlayStyle(colorScheme),
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      toolbarHeight: 64,
      titleTextStyle: textTheme.titleSmall?.copyWith(
        color: colorScheme.onSurface,
        fontFamily: fontFamily,
        fontFamilyFallback: fontFamilyFallback,
        fontWeight: FontWeight.w600,
      ),
    ),
    fontFamily: fontFamily,
    fontFamilyFallback: fontFamilyFallback,
    useMaterial3: true,
  );
}

/// Builds the shared theme for Material 3 DropdownMenu widgets.
DropdownMenuThemeData _dropdownMenuTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  final menuShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(8),
  );
  return DropdownMenuThemeData(
    textStyle: textTheme.bodyMedium?.copyWith(color: colorScheme.onSurface),
    menuStyle: MenuStyle(
      backgroundColor: WidgetStatePropertyAll(colorScheme.surfaceContainerHigh),
      shadowColor: WidgetStatePropertyAll(
        colorScheme.shadow.withValues(alpha: 0.18),
      ),
      surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
      elevation: const WidgetStatePropertyAll(3),
      padding: const WidgetStatePropertyAll(EdgeInsets.symmetric(vertical: 4)),
      shape: WidgetStatePropertyAll(menuShape),
    ),
  );
}

/// Builds the shared menu theme used by MenuAnchor and MenuItemButton.
MenuThemeData _menuTheme(ColorScheme colorScheme, TextTheme textTheme) {
  final menuShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(8),
  );
  return MenuThemeData(
    style: MenuStyle(
      backgroundColor: WidgetStatePropertyAll(colorScheme.surfaceContainerHigh),
      shadowColor: WidgetStatePropertyAll(
        colorScheme.shadow.withValues(alpha: 0.18),
      ),
      surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
      elevation: const WidgetStatePropertyAll(3),
      padding: const WidgetStatePropertyAll(EdgeInsets.symmetric(vertical: 4)),
      shape: WidgetStatePropertyAll(menuShape),
    ),
  );
}

/// Builds the shared item style used inside MenuAnchor menus.
MenuButtonThemeData _menuButtonTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  return MenuButtonThemeData(
    style: ButtonStyle(
      foregroundColor: WidgetStatePropertyAll(colorScheme.onSurface),
      textStyle: WidgetStatePropertyAll(textTheme.bodyMedium),
      padding: const WidgetStatePropertyAll(
        EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      ),
      minimumSize: const WidgetStatePropertyAll(Size(0, 40)),
      shape: WidgetStatePropertyAll(
        RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      ),
    ),
  );
}

/// Builds the shared popup menu theme used by PopupMenuButton.
PopupMenuThemeData _popupMenuTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  return PopupMenuThemeData(
    color: colorScheme.surfaceContainerHigh,
    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    menuPadding: const EdgeInsets.symmetric(vertical: 4),
    elevation: 3,
    shadowColor: colorScheme.shadow.withValues(alpha: 0.18),
    surfaceTintColor: Colors.transparent,
    textStyle: textTheme.bodyMedium?.copyWith(color: colorScheme.onSurface),
  );
}

/// Builds the legacy button theme values still used by DropdownButton.
ButtonThemeData _buttonTheme(ColorScheme colorScheme) {
  return ButtonThemeData(alignedDropdown: true, colorScheme: colorScheme);
}

DialogThemeData _dialogTheme(ColorScheme colorScheme, TextTheme textTheme) {
  return DialogThemeData(
    backgroundColor: colorScheme.surfaceContainerHigh,
    surfaceTintColor: Colors.transparent,
    elevation: 6,
    shadowColor: colorScheme.shadow.withValues(alpha: 0.18),
    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
    clipBehavior: Clip.antiAlias,
    insetPadding: const EdgeInsets.symmetric(horizontal: 24, vertical: 24),
    actionsPadding: const EdgeInsets.fromLTRB(24, 0, 24, 20),
    titleTextStyle: textTheme.headlineSmall?.copyWith(
      color: colorScheme.onSurface,
      fontWeight: FontWeight.w700,
    ),
    contentTextStyle: textTheme.bodyMedium?.copyWith(
      color: colorScheme.onSurfaceVariant,
    ),
    iconColor: colorScheme.primary,
    barrierColor: Colors.black.withValues(alpha: 0.38),
  );
}

InputDecorationTheme _inputDecorationTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  final radius = BorderRadius.circular(8);
  final baseBorder = OutlineInputBorder(
    borderRadius: radius,
    borderSide: BorderSide(
      color: colorScheme.outlineVariant.withValues(alpha: 0.72),
      width: 1,
    ),
  );
  final disabledBorder = OutlineInputBorder(
    borderRadius: radius,
    borderSide: BorderSide(
      color: colorScheme.outlineVariant.withValues(alpha: 0.36),
      width: 1,
    ),
  );
  final focusedBorder = OutlineInputBorder(
    borderRadius: radius,
    borderSide: BorderSide(color: colorScheme.primary, width: 1.4),
  );
  final errorBorder = OutlineInputBorder(
    borderRadius: radius,
    borderSide: BorderSide(color: colorScheme.error, width: 1.2),
  );

  return InputDecorationTheme(
    filled: true,
    fillColor: colorScheme.surfaceContainerHighest.withValues(alpha: 0.34),
    isDense: true,
    contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 14),
    border: baseBorder,
    enabledBorder: baseBorder,
    disabledBorder: disabledBorder,
    focusedBorder: focusedBorder,
    errorBorder: errorBorder,
    focusedErrorBorder: errorBorder.copyWith(
      borderSide: BorderSide(color: colorScheme.error, width: 1.4),
    ),
    labelStyle: textTheme.labelSmall?.copyWith(
      color: colorScheme.onSurfaceVariant,
      fontWeight: FontWeight.w400,
    ),
    floatingLabelStyle: textTheme.labelSmall?.copyWith(
      color: colorScheme.primary,
      fontWeight: FontWeight.w500,
    ),
    hintStyle: textTheme.bodyMedium?.copyWith(
      color: colorScheme.onSurfaceVariant.withValues(alpha: 0.76),
    ),
    helperStyle: textTheme.bodySmall?.copyWith(
      color: colorScheme.onSurfaceVariant,
    ),
    errorStyle: textTheme.bodySmall?.copyWith(color: colorScheme.error),
    prefixIconColor: colorScheme.onSurfaceVariant,
    suffixIconColor: colorScheme.onSurfaceVariant,
  );
}

/// The shared image/video background used by the app and appearance preview.
class OperitThemeBackground extends StatelessWidget {
  const OperitThemeBackground({
    super.key,
    this.fit = StackFit.expand,
    this.muteVideo = false,
    required this.themePreferenceSnapshot,
    required this.child,
  });

  static const Duration _backgroundAnimationDuration = Duration(
    milliseconds: 360,
  );

  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final Widget child;
  final StackFit fit;

  /// Keeps embedded previews from playing a second copy of background audio.
  final bool muteVideo;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final mediaPath = themePreferenceSnapshot.backgroundImageUri;
    final hasBackgroundMedia =
        themePreferenceSnapshot.useBackgroundImage &&
        mediaPath != null &&
        mediaPath.isNotEmpty;
    return LiquidGlassScope(
      child: Stack(
        fit: fit,
        children: <Widget>[
          Positioned.fill(
            child: GlassBackgroundSource(
              child: Stack(
                fit: StackFit.expand,
                children: <Widget>[
                  AnimatedContainer(
                    duration: _backgroundAnimationDuration,
                    curve: Curves.easeOutCubic,
                    color: colorScheme.surface,
                  ),
                  AnimatedSwitcher(
                    duration: _backgroundAnimationDuration,
                    switchInCurve: Curves.easeOutCubic,
                    switchOutCurve: Curves.easeInCubic,
                    child: hasBackgroundMedia
                        ? _ThemeBackgroundMedia(
                            key: ValueKey<String>(
                              '${themePreferenceSnapshot.backgroundMediaType}|$mediaPath',
                            ),
                            mediaPath: mediaPath,
                            mediaType:
                                themePreferenceSnapshot.backgroundMediaType,
                            opacity:
                                themePreferenceSnapshot.backgroundImageOpacity,
                            muted:
                                muteVideo ||
                                themePreferenceSnapshot.videoBackgroundMuted,
                            loop: themePreferenceSnapshot.videoBackgroundLoop,
                            blurEnabled:
                                themePreferenceSnapshot.useBackgroundBlur,
                            blurRadius:
                                themePreferenceSnapshot.backgroundBlurRadius,
                          )
                        : const SizedBox.expand(
                            key: ValueKey<String>('empty-theme-background'),
                          ),
                  ),
                ],
              ),
            ),
          ),
          child,
        ],
      ),
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
    final blurred = blurEnabled
        ? ImageFiltered(
            imageFilter: ui.ImageFilter.blur(
              sigmaX: blurRadius,
              sigmaY: blurRadius,
            ),
            child: media,
          )
        : media;
    return AnimatedOpacity(
      duration: OperitThemeBackground._backgroundAnimationDuration,
      curve: Curves.easeOutCubic,
      opacity: opacity.clamp(0, 1),
      child: blurred,
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
    if (customFontPath != null && customFontPath.isNotEmpty) {
      return _customFontFamilyForPath(customFontPath);
    }
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

/// Loads every custom font referenced by the active theme snapshot.
Future<void> _loadCustomFontIfNeeded(
  ThemePreferenceSnapshot themePreferenceSnapshot,
) async {
  final fontPaths = <String?>[
    if (themePreferenceSnapshot.fontType ==
        UserPreferencesManager.FONT_TYPE_FILE)
      themePreferenceSnapshot.customFontPath,
    if (themePreferenceSnapshot.bubbleUserUseCustomFont &&
        themePreferenceSnapshot.bubbleUserFontType ==
            UserPreferencesManager.FONT_TYPE_FILE)
      themePreferenceSnapshot.bubbleUserCustomFontPath,
    if (themePreferenceSnapshot.bubbleAiUseCustomFont &&
        themePreferenceSnapshot.bubbleAiFontType ==
            UserPreferencesManager.FONT_TYPE_FILE)
      themePreferenceSnapshot.bubbleAiCustomFontPath,
  ];
  for (final fontPath in fontPaths) {
    if (fontPath == null ||
        fontPath.isEmpty ||
        _loadedCustomFontPaths.contains(fontPath)) {
      continue;
    }
    final fontBytes = await ThemeAssetStore().readBytes(fontPath);
    final loader = FontLoader(_customFontFamilyForPath(fontPath))
      ..addFont(Future<ByteData>.value(ByteData.sublistView(fontBytes)));
    await loader.load();
    _loadedCustomFontPaths.add(fontPath);
  }
}

/// Builds a stable in-process font family name for a theme font asset.
String _customFontFamilyForPath(String fontPath) {
  return 'OperitCustomFont_${fontPath.hashCode.abs()}';
}

final Set<String> _loadedCustomFontPaths = <String>{};

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

/// Keeps system bars transparent so inset surfaces remain visible beneath them.
SystemUiOverlayStyle _systemUiOverlayStyle(ColorScheme colorScheme) {
  final iconBrightness = colorScheme.brightness == Brightness.dark
      ? Brightness.light
      : Brightness.dark;
  return SystemUiOverlayStyle(
    statusBarColor: Colors.transparent,
    statusBarIconBrightness: iconBrightness,
    statusBarBrightness: colorScheme.brightness,
    systemStatusBarContrastEnforced: false,
    systemNavigationBarColor: Colors.transparent,
    systemNavigationBarDividerColor: Colors.transparent,
    systemNavigationBarIconBrightness: iconBrightness,
    systemNavigationBarContrastEnforced: false,
  );
}
