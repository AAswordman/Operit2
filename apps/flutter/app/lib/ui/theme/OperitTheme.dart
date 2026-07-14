// ignore_for_file: file_names

import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:video_player/video_player.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/errors/UnhandledErrorReporter.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../core/runtime/RuntimeConnectionManager.dart';
import '../../data/preferences/UserPreferencesManager.dart';
import '../../l10n/generated/app_localizations.dart';
import '../common/RuntimeBootstrapScreen.dart';
import '../features/chat/tts/TtsFloatingPanel.dart';
import '../../core/host/browser/RuntimeBrowserOwnerHost.dart';
import '../features/chat/components/workspace/browser/automation/WorkspaceWebVisitHost.dart';
import '../permissions/ToolApprovalHost.dart';

class OperitTheme extends StatefulWidget {
  const OperitTheme({
    super.key,
    required this.child,
    this.hostInteractionHostsEnabled = true,
    this.unconfiguredChildEnabled = false,
  });

  final Widget child;
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
  final RuntimeConnectionManager _runtimeManager =
      RuntimeConnectionManager.instance;
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
  bool _preserveUnconfiguredChild = false;
  int _runtimeGeneration = 0;

  /// Subscribes to runtime readiness and starts theme services when configured.
  @override
  void initState() {
    super.initState();
    _runtimeManager.addListener(_handleRuntimeConnectionChanged);
    _preserveUnconfiguredChild =
        widget.unconfiguredChildEnabled && !_runtimeManager.runtimeConfigured;
    if (_runtimeManager.runtimeConfigured) {
      unawaited(_startRuntimeUi());
    }
  }

  /// Releases runtime readiness and theme subscriptions.
  @override
  void dispose() {
    _runtimeManager.removeListener(_handleRuntimeConnectionChanged);
    _controller.dispose();
    super.dispose();
  }

  /// Synchronizes the theme lifecycle with runtime configuration changes.
  void _handleRuntimeConnectionChanged() {
    if (!_runtimeManager.runtimeConfigured) {
      _runtimeGeneration++;
      _controller.dispose();
      setState(() {
        _runtimeUiReady = false;
        _runtimeStartupError = null;
        _runtimeStartFuture = null;
        _preserveUnconfiguredChild = widget.unconfiguredChildEnabled;
      });
      return;
    }
    unawaited(_startRuntimeUi());
  }

  /// Starts core-backed theme state after the runtime is configured.
  Future<void> _startRuntimeUi() async {
    if (_runtimeUiReady ||
        _runtimeStartFuture != null ||
        !_runtimeManager.runtimeConfigured) {
      return;
    }
    final generation = ++_runtimeGeneration;
    final startFuture = _controller.start();
    _runtimeStartFuture = startFuture;
    try {
      await startFuture;
      if (!mounted ||
          generation != _runtimeGeneration ||
          !_runtimeManager.runtimeConfigured) {
        return;
      }
      setState(() {
        _runtimeUiReady = true;
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
        _runtimeStartupError = error;
      });
    } finally {
      if (generation == _runtimeGeneration) {
        _runtimeStartFuture = null;
      }
    }
  }

  /// Builds the bootstrap or fully initialized application theme.
  @override
  Widget build(BuildContext context) {
    final runtimeConfigured = _runtimeManager.runtimeConfigured;
    final runtimeReady = runtimeConfigured && _runtimeUiReady;
    final themeSnapshot = runtimeReady
        ? _controller.themePreferenceSnapshot
        : UserPreferencesManager.defaultThemePreferenceSnapshot;
    final themeMode = runtimeReady ? _controller.themeMode : ThemeMode.system;
    final Widget appChild;
    if (!runtimeConfigured) {
      appChild = widget.unconfiguredChildEnabled
          ? widget.child
          : const RuntimeBootstrapScreen(message: '请先在主窗口配置运行时目录和工作区目录');
    } else if (!runtimeReady) {
      final bootstrap = RuntimeBootstrapScreen(
        errorText: _runtimeStartupError?.toString(),
      );
      appChild = _preserveUnconfiguredChild
          ? Stack(
              fit: StackFit.expand,
              children: <Widget>[widget.child, bootstrap],
            )
          : bootstrap;
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
    required this.child,
  });

  final ThemeMode themeMode;
  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final bool hostInteractionHostsEnabled;
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
      navigatorKey: UnhandledErrorReporter.navigatorKey,
      title: 'Operit2',
      debugShowCheckedModeBanner: false,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: _themeData(lightColorScheme, themePreferenceSnapshot),
      darkTheme: _themeData(darkColorScheme, themePreferenceSnapshot),
      themeMode: themeMode,
      builder: (context, materialChild) {
        return AnnotatedRegion<SystemUiOverlayStyle>(
          value: _systemUiOverlayStyle(Theme.of(context).colorScheme),
          child: _OperitThemeBackground(
            themePreferenceSnapshot: themePreferenceSnapshot,
            child: materialChild!,
          ),
        );
      },
      home: RuntimeBrowserOwnerHost(
        enabled: hostInteractionHostsEnabled,
        child: WorkspaceWebVisitHost(
          child: ToolApprovalHost(
            child: Stack(
              fit: StackFit.expand,
              children: <Widget>[
                Positioned.fill(child: child),
                const TtsFloatingPanel(),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class OperitThemeController {
  OperitThemeController({
    required VoidCallback onChanged,
    UserPreferencesManager preferencesManager = const UserPreferencesManager(),
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _onChanged = onChanged,
       _preferencesManager = preferencesManager,
       _clients = clients;

  final VoidCallback _onChanged;
  final UserPreferencesManager _preferencesManager;
  final GeneratedCoreProxyClients _clients;
  StreamSubscription<Object?>? _activePromptSubscription;
  ThemeMode _themeMode = ThemeMode.system;
  ThemePreferenceSnapshot _themePreferenceSnapshot =
      UserPreferencesManager.defaultThemePreferenceSnapshot;
  String? _activeCharacterCardId;
  String? _activeCharacterGroupId;
  String? _activeThemeTargetName;

  ThemeMode get themeMode => _themeMode;
  ThemePreferenceSnapshot get themePreferenceSnapshot =>
      _themePreferenceSnapshot;
  String get activeThemeTargetName {
    final name = _activeThemeTargetName;
    if (name == null || name.trim().isEmpty) {
      throw StateError('No active theme target name');
    }
    return name;
  }

  bool get hasActiveThemeTarget =>
      _activeCharacterGroupId != null || _activeCharacterCardId != null;
  bool get isActiveThemeTargetGroup => _activeCharacterGroupId != null;

  /// Starts core-backed theme preferences and active prompt subscriptions.
  Future<void> start() async {
    await _activePromptSubscription?.cancel();
    _activePromptSubscription = null;
    final activePrompt = await _clients.preferencesActivePromptManager
        .getActivePrompt();
    _applyActivePrompt(activePrompt);
    await _loadActiveThemeTargetName();
    _activePromptSubscription = _clients.preferencesActivePromptManager
        .activePromptFlowChanges()
        .listen((activePrompt) {
          unawaited(_handleActivePromptChange(activePrompt));
        });
    await loadThemeMode();
  }

  /// Cancels the active prompt subscription.
  void dispose() {
    unawaited(_activePromptSubscription?.cancel());
    _activePromptSubscription = null;
  }

  Future<void> loadThemeMode() async {
    final snapshot = await _resolveThemePreferenceSnapshot();
    await _loadCustomFontIfNeeded(snapshot);
    final themeMode = snapshot.flutterThemeMode;
    if (_themeMode == themeMode && _themePreferenceSnapshot == snapshot) {
      return;
    }
    _themePreferenceSnapshot = snapshot;
    _themeMode = themeMode;
    _onChanged();
  }

  bool isDark(BuildContext context) {
    return Theme.of(context).brightness == Brightness.dark;
  }

  void toggle(BuildContext context) {
    unawaited(setThemeMode(isDark(context) ? ThemeMode.light : ThemeMode.dark));
  }

  Future<void> setThemeMode(ThemeMode themeMode) async {
    if (_themeMode == themeMode) {
      return;
    }
    await _saveThemeModeToCurrentTarget(themeMode);
    await _reloadThemePreferenceSnapshot();
  }

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
    _themeMode = snapshot.flutterThemeMode;
    _onChanged();
  }

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
      characterCardId: _activeCharacterCardId,
      characterGroupId: _activeCharacterGroupId,
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

  Future<void> saveActiveThemeUserAvatarSettings({
    required String customUserAvatarUri,
  }) async {
    await _preferencesManager.saveGlobalUserAvatarSettings(
      customUserAvatarUri: customUserAvatarUri,
    );
    await _reloadThemePreferenceSnapshot();
  }

  Future<void> resetMessageColorSettings() async {
    await _resetCurrentTargetMessageColorSettings();
    await _reloadThemePreferenceSnapshot();
  }

  Future<void> resetThemeSettings() async {
    await _resetCurrentTargetThemeSettings();
    await _reloadThemePreferenceSnapshot();
  }

  Future<void> _reloadThemePreferenceSnapshot() async {
    final snapshot = await _resolveThemePreferenceSnapshot();
    await _loadCustomFontIfNeeded(snapshot);
    _themePreferenceSnapshot = snapshot;
    _themeMode = _themePreferenceSnapshot.flutterThemeMode;
    _onChanged();
  }

  Future<ThemePreferenceSnapshot> _resolveThemePreferenceSnapshot() {
    return _preferencesManager.resolveThemePreferenceSnapshot(
      characterCardId: _activeCharacterCardId,
      characterGroupId: _activeCharacterGroupId,
    );
  }

  Future<void> _handleActivePromptChange(
    core_proxy.ActivePrompt? activePrompt,
  ) async {
    if (_applyActivePrompt(activePrompt)) {
      await _loadActiveThemeTargetName();
      await _reloadThemePreferenceSnapshot();
    }
  }

  Future<void> _saveThemeModeToCurrentTarget(ThemeMode themeMode) {
    return switch (themeMode) {
      ThemeMode.system => _preferencesManager.saveThemeSettings(
        characterCardId: _activeCharacterCardId,
        characterGroupId: _activeCharacterGroupId,
        useSystemTheme: true,
      ),
      ThemeMode.light => _preferencesManager.saveThemeSettings(
        characterCardId: _activeCharacterCardId,
        characterGroupId: _activeCharacterGroupId,
        themeMode: UserPreferencesManager.THEME_MODE_LIGHT,
        useSystemTheme: false,
      ),
      ThemeMode.dark => _preferencesManager.saveThemeSettings(
        characterCardId: _activeCharacterCardId,
        characterGroupId: _activeCharacterGroupId,
        themeMode: UserPreferencesManager.THEME_MODE_DARK,
        useSystemTheme: false,
      ),
    };
  }

  Future<void> _resetCurrentTargetThemeSettings() async {
    final groupId = _activeCharacterGroupId;
    final cardId = _activeCharacterCardId;
    if (groupId != null) {
      await _preferencesManager.deleteCharacterGroupTheme(groupId);
    } else if (cardId != null) {
      await _preferencesManager.deleteCharacterCardTheme(cardId);
    } else {
      throw StateError('No active theme target for theme settings');
    }
  }

  Future<void> _resetCurrentTargetMessageColorSettings() async {
    final groupId = _activeCharacterGroupId;
    final cardId = _activeCharacterCardId;
    if (groupId != null) {
      await _preferencesManager.resetMessageColorSettingsForCharacterGroup(
        groupId,
      );
    } else if (cardId != null) {
      await _preferencesManager.resetMessageColorSettingsForCharacterCard(
        cardId,
      );
    } else {
      throw StateError('No active theme target for message color settings');
    }
  }

  Future<void> _loadActiveThemeTargetName() async {
    final groupId = _activeCharacterGroupId;
    final cardId = _activeCharacterCardId;
    if (groupId != null) {
      final group = await _clients.preferencesCharacterGroupCardManager
          .getCharacterGroupCard(groupId: groupId);
      _activeThemeTargetName = group?.name;
    } else if (cardId != null) {
      final card = await _clients.preferencesCharacterCardManager
          .getCharacterCard(id: cardId);
      _activeThemeTargetName = card.name;
    } else {
      _activeThemeTargetName = null;
    }
  }

  bool _applyActivePrompt(core_proxy.ActivePrompt? activePrompt) {
    String? nextCardId;
    String? nextGroupId;
    if (activePrompt != null) {
      if (activePrompt.tag == 'CharacterCard' &&
          activePrompt.id.trim().isNotEmpty) {
        nextCardId = activePrompt.id.trim();
      } else if (activePrompt.tag == 'CharacterGroup' &&
          activePrompt.id.trim().isNotEmpty) {
        nextGroupId = activePrompt.id.trim();
      }
    }
    if (_activeCharacterCardId == nextCardId &&
        _activeCharacterGroupId == nextGroupId) {
      return false;
    }
    _activeCharacterCardId = nextCardId;
    _activeCharacterGroupId = nextGroupId;
    return true;
  }
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
    useSystemTheme: snapshot.useSystemTheme,
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
    toolCollapseMode: snapshot.toolCollapseMode,
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

ThemeData _themeData(
  ColorScheme colorScheme,
  ThemePreferenceSnapshot themePreferenceSnapshot,
) {
  final typography = Typography.material2021();
  final fontFamily = _fontFamily(themePreferenceSnapshot);
  final fontFamilyFallback = _fontFamilyFallback(themePreferenceSnapshot);
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
    dialogTheme: _dialogTheme(colorScheme, textTheme),
    // ignore: deprecated_member_use
    progressIndicatorTheme: const ProgressIndicatorThemeData(year2023: false),
    appBarTheme: AppBarTheme(
      backgroundColor: colorScheme.surface,
      foregroundColor: colorScheme.onSurface,
      surfaceTintColor: Colors.transparent,
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

DropdownMenuThemeData _dropdownMenuTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  return DropdownMenuThemeData(
    textStyle: textTheme.bodyMedium?.copyWith(color: colorScheme.onSurface),
  );
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

class _OperitThemeBackground extends StatelessWidget {
  const _OperitThemeBackground({
    required this.themePreferenceSnapshot,
    required this.child,
  });

  static const Duration _backgroundAnimationDuration = Duration(
    milliseconds: 360,
  );

  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final Widget child;

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
        fit: StackFit.expand,
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
                            muted: themePreferenceSnapshot.videoBackgroundMuted,
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
        : SizedBox.expand(
            child: Image.file(File(mediaPath), fit: BoxFit.cover),
          );
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
      duration: _OperitThemeBackground._backgroundAnimationDuration,
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

  @override
  State<_VideoThemeBackground> createState() => _VideoThemeBackgroundState();
}

class _VideoThemeBackgroundState extends State<_VideoThemeBackground> {
  late VideoPlayerController _controller;

  @override
  void initState() {
    super.initState();
    _controller = _createController();
    unawaited(_initializeController());
  }

  @override
  void didUpdateWidget(covariant _VideoThemeBackground oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.mediaPath != widget.mediaPath) {
      final previous = _controller;
      _controller = _createController();
      unawaited(_initializeController());
      previous.dispose();
      return;
    }
    if (oldWidget.loop != widget.loop) {
      unawaited(_controller.setLooping(widget.loop));
    }
    if (oldWidget.muted != widget.muted) {
      unawaited(_controller.setVolume(widget.muted ? 0 : 1));
    }
  }

  VideoPlayerController _createController() {
    return VideoPlayerController.file(File(widget.mediaPath))
      ..setLooping(widget.loop)
      ..setVolume(widget.muted ? 0 : 1);
  }

  Future<void> _initializeController() async {
    await _controller.initialize();
    await _controller.play();
    if (mounted) {
      setState(() {});
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (!_controller.value.isInitialized) {
      return const SizedBox.expand();
    }
    return SizedBox.expand(
      child: FittedBox(
        fit: BoxFit.cover,
        child: SizedBox(
          width: _controller.value.size.width,
          height: _controller.value.size.height,
          child: VideoPlayer(_controller),
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
    final fontBytes = await File(fontPath).readAsBytes();
    final loader = FontLoader(_customFontFamilyForPath(fontPath))
      ..addFont(Future<ByteData>.value(ByteData.sublistView(fontBytes)));
    await loader.load();
    _loadedCustomFontPaths.add(fontPath);
  }
}

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

SystemUiOverlayStyle _systemUiOverlayStyle(ColorScheme colorScheme) {
  final iconBrightness = colorScheme.brightness == Brightness.dark
      ? Brightness.light
      : Brightness.dark;
  return SystemUiOverlayStyle(
    statusBarColor: Colors.transparent,
    statusBarIconBrightness: iconBrightness,
    statusBarBrightness: colorScheme.brightness,
    systemNavigationBarColor: colorScheme.surface,
    systemNavigationBarIconBrightness: iconBrightness,
  );
}
