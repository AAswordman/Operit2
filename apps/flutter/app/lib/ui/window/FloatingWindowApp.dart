// ignore_for_file: file_names
import 'dart:async';

import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/material.dart';
import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/errors/UnhandledErrorReporter.dart';
import '../../core/host/RuntimeWindowCamera.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../data/preferences/UserPreferencesManager.dart';
import '../../l10n/generated/app_localizations.dart';
import '../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import '../theme/OperitTheme.dart' show operitColorScheme;
import 'FloatingWindowFrame.dart';
import 'FloatingWindowScope.dart';

/// Hosts an ordinary live plugin UI session on a transparent, undecorated root.
class FloatingWindowApp extends StatefulWidget {
  /// Creates a transparent host for the launched plugin route.
  const FloatingWindowApp({
    super.key,
    required this.launch,
    this.clients = const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge()),
  });
  final FloatingWindowLaunch launch;
  final GeneratedCoreProxyClients clients;

  /// Owns the plugin session and its appearance subscription.
  @override
  State<FloatingWindowApp> createState() => _FloatingWindowAppState();
}

class _FloatingWindowAppState extends State<FloatingWindowApp>
    with WidgetsBindingObserver {
  final _session = ToolPkgUiSessionController();
  late final _preferences = UserPreferencesManager(
    clients: widget.clients,
  ).windowPreferencesFlow();

  RuntimeWindowCamera? _camera;
  final _frame = GlobalKey<FloatingWindowFrameState>();

  Future<Object?> _contentCommand(String name) async {
    switch (name) {
      case 'window.fullscreen.enter':
        await _setFullscreen(true);
        return null;
      case 'window.fullscreen.exit':
        await _setFullscreen(false);
        return null;
      case windowCameraCaptureCommand:
        return _camera?.capture();
      case windowCameraStopCommand:
        await _camera?.stop();
        return null;
      default:
        throw ArgumentError('Unknown window content command: $name');
    }
  }

  Future<void> _setFullscreen(bool fullscreen) async {
    final frame = _frame.currentState;
    if (frame == null) throw StateError('Window content is not ready');
    await frame.setFullscreen(fullscreen);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.hidden ||
        state == AppLifecycleState.paused ||
        state == AppLifecycleState.detached) {
      unawaited(_camera?.stop());
    }
  }

  @override
  void initState() {
    super.initState();
    if (windowCameraSupported &&
        (!windowCameraNeedsActivity ||
            widget.launch.onBeforeCloseChanged != null)) {
      _camera = RuntimeWindowCamera();
      WidgetsBinding.instance.addObserver(this);
    }
    FloatingWindows.setContentCommandHandler(widget.launch, _contentCommand);
    FloatingWindows.setBeforeClose(widget.launch, () async {
      final stopping = _camera?.stop();
      await _session.close();
      await stopping;
    });
  }

  @override
  void dispose() {
    FloatingWindows.setBeforeClose(widget.launch, null);
    FloatingWindows.setContentCommandHandler(widget.launch, null);
    if (_camera != null) {
      WidgetsBinding.instance.removeObserver(this);
      _camera?.dispose();
    }
    super.dispose();
  }

  /// Waits for the committed palette before mounting the plugin's first frame.
  @override
  Widget build(BuildContext context) {
    return StreamBuilder<
      ({ThemePreferenceSnapshot appearance, AppLanguage language})
    >(
      stream: _preferences,
      builder: (context, preferences) {
        if (preferences.hasError) {
          Error.throwWithStackTrace(
            preferences.error!,
            preferences.stackTrace!,
          );
        }
        final snapshot = preferences.data;
        if (snapshot == null) return const SizedBox.shrink();
        return _buildWindow(snapshot.appearance, snapshot.language);
      },
    );
  }

  /// Applies the selected app palette while keeping the window surface clear.
  Widget _buildWindow(
    ThemePreferenceSnapshot appearance,
    AppLanguage language,
  ) {
    final payload = widget.launch.payload;
    final plugin = core_proxy.ToolPkgContainerRuntime.fromJson(
      (payload['plugin'] as Map).cast<String, Object?>(),
    );

    /// Matches the main app's colors for either system brightness.
    ThemeData transparentTheme(Brightness brightness) => ThemeData(
      useMaterial3: true,
      colorScheme: operitColorScheme(brightness, appearance),
      scaffoldBackgroundColor: Colors.transparent,
      canvasColor: Colors.transparent,
    );

    final launcher = ToolPkgUiLauncherScreen(
      clients: widget.clients,
      plugin: plugin,
      initialRouteId: payload['route'] as String,
      showLauncherChrome: false,
      showLoadingIndicator: false,
      sessionController: _session,
      initialState: (payload['state'] as Map).cast<String, Object?>(),
    );

    return MaterialApp(
      debugShowCheckedModeBanner: false,
      color: Colors.transparent,
      locale: language.locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: transparentTheme(Brightness.light),
      darkTheme: transparentTheme(Brightness.dark),
      themeMode: appearance.themeMode,
      themeAnimationDuration: Duration.zero,
      home: UnhandledErrorHost(
        child: Material(
          type: MaterialType.transparency,
          child: FloatingWindowFrame(
            key: _frame,
            launch: widget.launch,
            builder: (fullscreen, toolbar, resizeHandle) => FloatingWindowScope(
              camera: _camera,
              appearance: appearance,
              fullscreen: fullscreen,
              toolbar: toolbar,
              resizeHandle: resizeHandle,
              child: launcher,
            ),
          ),
        ),
      ),
    );
  }
}
