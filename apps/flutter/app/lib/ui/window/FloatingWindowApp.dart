// ignore_for_file: file_names
import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/material.dart';
import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/errors/UnhandledErrorReporter.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../l10n/generated/app_localizations.dart';
import '../features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Hosts an ordinary live plugin UI session on a transparent, undecorated root.
class FloatingWindowApp extends StatefulWidget {
  const FloatingWindowApp({super.key, required this.launch});
  final FloatingWindowLaunch launch;
  @override
  State<FloatingWindowApp> createState() => _FloatingWindowAppState();
}

class _FloatingWindowAppState extends State<FloatingWindowApp> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  final _session = ToolPkgUiSessionController();
  @override
  void initState() {
    super.initState();
    FloatingWindows.setBeforeClose(widget.launch, _session.close);
  }

  @override
  void dispose() {
    FloatingWindows.setBeforeClose(widget.launch, null);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final payload = widget.launch.payload;
    final plugin = core_proxy.ToolPkgContainerRuntime.fromJson(
      (payload['plugin'] as Map).cast<String, Object?>(),
    );
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      color: Colors.transparent,
      locale: Locale(payload['locale'] as String),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: ThemeData(
        useMaterial3: true,
        scaffoldBackgroundColor: Colors.transparent,
        canvasColor: Colors.transparent,
      ),
      home: UnhandledErrorHost(
        child: Material(
          type: MaterialType.transparency,
          child: ToolPkgUiLauncherScreen(
            clients: _clients,
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
