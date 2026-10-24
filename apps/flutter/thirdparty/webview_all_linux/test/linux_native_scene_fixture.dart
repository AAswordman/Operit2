import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:webview_all_linux/src/linux_webview_controller.dart';
import 'package:webview_all_linux/src/linux_webview_widget.dart';
import 'package:webview_platform_interface/webview_platform_interface.dart';

const _probe = MethodChannel('operit/linux_native_scene_probe');

/// Starts the Linux host fixture using real WebKit views and Flutter routes.
void main() {
  runApp(const _SceneFixture());
}

class _SceneFixture extends StatefulWidget {
  /// Creates the deterministic composition fixture.
  const _SceneFixture();

  /// Keeps browser controllers alive across modal route changes.
  @override
  State<_SceneFixture> createState() => _SceneFixtureState();
}

class _SceneFixtureState extends State<_SceneFixture> {
  final _navigator = GlobalKey<NavigatorState>();
  late final LinuxWebViewController _first;
  late final LinuxWebViewController _second;
  bool _showFirst = true;
  bool _showSecond = true;
  int _ready = 0;

  /// Registers host controls before loading the two native documents.
  @override
  void initState() {
    super.initState();
    _probe.setMethodCallHandler(_handleProbe);
    _first = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    _second = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    unawaited(_load(_first, 'rgba(0,255,0,0.5)', 'first'));
    unawaited(_load(_second, 'rgba(0,0,255,0.5)', 'second'));
  }

  /// Reports real DOM readiness and clicks without relying on load delays.
  Future<void> _load(
    LinuxWebViewController controller,
    String color,
    String name,
  ) async {
    await controller.setBackgroundColor(const Color(0x00000000));
    await controller.addJavaScriptChannel(
      JavaScriptChannelParams(
        name: 'ProbeEvents',
        onMessageReceived: (message) {
          if (message.message == 'ready') {
            _ready++;
            if (_ready == 2) {
              unawaited(_probe.invokeMethod<void>('ready'));
            }
          } else if (message.message == 'double') {
            unawaited(_probe.invokeMethod<void>('browserDoubleClick', name));
          } else if (message.message == 'key') {
            unawaited(_probe.invokeMethod<void>('browserKey', name));
          } else if (message.message == 'wheel') {
            unawaited(_probe.invokeMethod<void>('browserWheel', name));
          } else if (message.message == 'click') {
            unawaited(_probe.invokeMethod<void>('browserClick', name));
          }
        },
      ),
    );
    await controller.loadHtmlString('''
<html style="background:transparent"><body style="margin:0;width:100vw;height:100vh;background:$color">
<script>
window.addEventListener('load', () => ProbeEvents.postMessage('ready'));
window.addEventListener('click', () => ProbeEvents.postMessage('click'));
window.addEventListener('dblclick', () => ProbeEvents.postMessage('double'));
window.addEventListener('keydown', () => ProbeEvents.postMessage('key'));
window.addEventListener('wheel', () => ProbeEvents.postMessage('wheel'));
</script></body></html>
''');
  }

  /// Applies host requests through actual Flutter navigation and widget updates.
  Future<Object?> _handleProbe(MethodCall call) async {
    switch (call.method) {
      case 'showDialog':
        unawaited(
          showGeneralDialog<void>(
            context: _navigator.currentContext!,
            barrierDismissible: false,
            barrierColor: const Color(0x80000000),
            transitionDuration: Duration.zero,
            pageBuilder: (context, animation, secondaryAnimation) => Center(
              child: Focus(
                autofocus: true,
                onKeyEvent: (node, event) {
                  if (event is KeyDownEvent) {
                    unawaited(_probe.invokeMethod<void>('flutterKey'));
                  }
                  return KeyEventResult.handled;
                },
                child: const ColoredBox(
                  color: Colors.white,
                  child: SizedBox(width: 40, height: 30),
                ),
              ),
            ),
          ),
        );
      case 'dismissDialog':
        _navigator.currentState!.pop();
      case 'removeSecond':
        setState(() => _showSecond = false);
      case 'dispose':
        setState(() {
          _showFirst = false;
          _showSecond = false;
        });
        await WidgetsBinding.instance.endOfFrame;
        await _first.dispose();
        await _second.dispose();
      default:
        throw MissingPluginException(
          'Unknown fixture operation ${call.method}',
        );
    }
    await WidgetsBinding.instance.endOfFrame;
    return null;
  }

  /// Builds the native host widget without changing application platform routing.
  Widget _browser(LinuxWebViewController controller) => Builder(
    builder: LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    ).build,
  );

  /// Interleaves Flutter pixels with two partially overlapping native widgets.
  @override
  Widget build(BuildContext context) => MaterialApp(
    navigatorKey: _navigator,
    debugShowCheckedModeBanner: false,
    home: ColoredBox(
      color: const Color(0xffff0000),
      child: Stack(
        children: [
          if (_showFirst)
            Positioned(
              left: 20,
              top: 20,
              width: 140,
              height: 120,
              child: _browser(_first),
            ),
          const Positioned(
            left: 60,
            top: 30,
            width: 120,
            height: 90,
            child: ColoredBox(color: Color(0x80ffffff)),
          ),
          if (_showSecond)
            Positioned(
              left: 80,
              top: 20,
              width: 140,
              height: 120,
              child: _browser(_second),
            ),
        ],
      ),
    ),
  );
}
