// ignore_for_file: file_names

import 'package:desktop_widgets/desktop_widgets.dart';
import 'package:flutter/material.dart';
import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/errors/UnhandledErrorReporter.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../l10n/generated/app_localizations.dart';
import '../../data/preferences/UserPreferencesManager.dart';
import '../features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Renders only plugin content on a transparent desktop surface.
class DesktopWidgetWindowApp extends StatefulWidget {
  /// Creates a window without application chrome or an opaque root material.
  const DesktopWidgetWindowApp({super.key, required this.launch});
  final DesktopWidgetLaunch launch;

  /// Creates refresh and host-window lifecycle state.
  @override
  State<DesktopWidgetWindowApp> createState() => _DesktopWidgetWindowAppState();
}

class _DesktopWidgetWindowAppState extends State<DesktopWidgetWindowApp> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  int _revision = 0;
  double _scale = 1;
  Object? _error;
  late final _definition = core_proxy.ToolPkgDesktopWidget.fromJson(
    (widget.launch.payload['definition'] as Map).cast<String, Object?>(),
  );
  late final _language = const UserPreferencesManager().appLanguageFlow();

  /// Opens plugin application UI in the owning window, preserving this content surface.
  Future<void> _openRoute(String package, String route) async {
    await widget.launch.invokeOwner<void>('openDesktopWidgetRoute', {
      'packageName': package,
      'routeId': route,
    });
  }

  /// Presents controls only in response to an explicit secondary click.
  Future<void> _menu(BuildContext context, Offset position) async {
    final l10n = AppLocalizations.of(context)!;
    final overlay =
        Overlay.of(context).context.findRenderObject()! as RenderBox;
    final choice = await showMenu<String>(
      context: context,
      position: RelativeRect.fromSize(position & Size.zero, overlay.size),
      items: [
        PopupMenuItem(value: 'refresh', child: Text(l10n.refresh)),
        PopupMenuItem(value: 'larger', child: Text(l10n.zoomIn)),
        PopupMenuItem(value: 'smaller', child: Text(l10n.zoomOut)),
        PopupMenuItem(value: 'close', child: Text(l10n.close)),
      ],
    );
    if (!mounted || choice == null) return;
    switch (choice) {
      case 'refresh':
        setState(() {
          _revision++;
          _error = null;
        });
      case 'larger':
      case 'smaller':
        final scale = (_scale + (choice == 'larger' ? 0.25 : -0.25)).clamp(
          0.5,
          3.0,
        );
        await DesktopWidgetWindow.resize(Size(360 * scale, 240 * scale));
        _scale = scale;
      case 'close':
        await DesktopWidgetWindow.close();
    }
  }

  /// Reports host interaction failures on the content surface without opening another window.
  Future<void> _perform(Future<void> Function() action) async {
    try {
      await action();
    } catch (error) {
      if (mounted) setState(() => _error = error);
    }
  }

  /// Builds a transparent application root with no title, padding, frame, or shadow.
  @override
  Widget build(BuildContext context) {
    return StreamBuilder<AppLanguage>(
      stream: _language,
      builder: (context, language) {
        if (language.hasError) {
          Error.throwWithStackTrace(language.error!, language.stackTrace!);
        }
        final selected = language.data;
        if (selected == null) return const SizedBox.shrink();
        return _buildWindow(selected);
      },
    );
  }

  /// Applies committed language changes without replacing the plugin session.
  Widget _buildWindow(AppLanguage language) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      color: Colors.transparent,
      locale: language.locale,
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
          child: Builder(
            builder: (context) => GestureDetector(
              behavior: HitTestBehavior.translucent,
              onLongPressStart: (_) => _perform(DesktopWidgetWindow.move),
              onSecondaryTapUp: (details) =>
                  _perform(() => _menu(context, details.localPosition)),
              child: _error == null
                  ? ToolPkgDesktopWidgetView(
                      key: ValueKey(_revision),
                      clients: _clients,
                      definition: _definition,
                      instanceId: widget.launch.payload['instanceId'] as String,
                      onOpenRoute: (package, route) =>
                          _perform(() => _openRoute(package, route)),
                    )
                  : Center(child: Text(_error.toString())),
            ),
          ),
        ),
      ),
    );
  }
}
