// ignore_for_file: file_names
import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/material.dart';
import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/errors/UnhandledErrorReporter.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../data/preferences/UserPreferencesManager.dart';
import '../../l10n/generated/app_localizations.dart';
import '../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import '../theme/OperitTheme.dart' show operitColorScheme;

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

class _FloatingWindowAppState extends State<FloatingWindowApp> {
  final _session = ToolPkgUiSessionController();
  late final _appearance = UserPreferencesManager(
    clients: widget.clients,
  ).themePreferenceSnapshotFlow();

  /// Registers the session drain with the floating window host.
  @override
  void initState() {
    super.initState();
    FloatingWindows.setBeforeClose(widget.launch, _session.close);
  }

  /// Releases the host close hook when the surface leaves the tree.
  @override
  void dispose() {
    FloatingWindows.setBeforeClose(widget.launch, null);
    super.dispose();
  }

  /// Waits for the committed palette before mounting the plugin's first frame.
  @override
  Widget build(BuildContext context) {
    return StreamBuilder<ThemePreferenceSnapshot>(
      stream: _appearance,
      builder: (context, appearance) {
        if (appearance.hasError) {
          Error.throwWithStackTrace(appearance.error!, appearance.stackTrace!);
        }
        final snapshot = appearance.data;
        if (snapshot == null) return const SizedBox.shrink();
        return _buildWindow(snapshot);
      },
    );
  }

  /// Applies the selected app palette while keeping the window surface clear.
  Widget _buildWindow(ThemePreferenceSnapshot appearance) {
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

    return MaterialApp(
      debugShowCheckedModeBanner: false,
      color: Colors.transparent,
      locale: Locale(payload['locale'] as String),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: transparentTheme(Brightness.light),
      darkTheme: transparentTheme(Brightness.dark),
      themeMode: appearance.themeMode,
      themeAnimationDuration: Duration.zero,
      home: UnhandledErrorHost(
        child: Material(
          type: MaterialType.transparency,
          child: ToolPkgUiLauncherScreen(
            clients: widget.clients,
            plugin: plugin,
            initialRouteId: payload['route'] as String,
            showLauncherChrome: false,
            showLoadingIndicator: false,
            sessionController: _session,
            initialState: (payload['state'] as Map).cast<String, Object?>(),
          ),
        ),
      ),
    );
  }
}
