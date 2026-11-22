// ignore_for_file: file_names
import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import '../logging/ClientLogger.dart';

/// Product state of the main UI. Auxiliary engines never initialize this owner.
enum AppVisibility { unknown, visible, hidden }

/// Separates intentional UI hiding from focus and desktop compositor occlusion.
class AppVisibilityService extends ValueNotifier<AppVisibility>
    with WidgetsBindingObserver {
  AppVisibilityService() : super(AppVisibility.unknown);
  static final instance = AppVisibilityService();
  static const channel = MethodChannel('operit/app_visibility');
  bool _initialized = false;
  bool _disposed = false;
  int _revision = 0;

  bool get _nativeDesktop =>
      !kIsWeb &&
      const [
        TargetPlatform.macOS,
        TargetPlatform.windows,
        TargetPlatform.linux,
      ].contains(defaultTargetPlatform);

  /// Installs listeners before reading the initial snapshot to avoid a startup gap.
  Future<void> initialize() async {
    if (_initialized) return;
    _initialized = true;
    WidgetsBinding.instance.addObserver(this);
    if (_nativeDesktop) {
      channel.setMethodCallHandler((call) async {
        if (call.method != 'visibilityChanged') throw MissingPluginException();
        _revision++;
        _accept(call.arguments, 'native-window');
      });
      final revision = _revision;
      try {
        final visible = await channel.invokeMethod<bool>('getVisibility');
        if (revision == _revision) _accept(visible, 'native-snapshot');
      } catch (error, stack) {
        // Unknown is deliberate: a missing native adapter must not turn blur into hiding.
        if (ClientLogger.isInitialized) {
          ClientLogger.e(
            'main UI visibility unavailable',
            tag: 'AppVisibility',
            error: error,
            stackTrace: stack,
          );
        }
      }
    } else {
      final lifecycle = WidgetsBinding.instance.lifecycleState;
      if (lifecycle != null) didChangeAppLifecycleState(lifecycle);
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (_nativeDesktop) return;
    // Mobile / OHOS use scene/ability visibility; Web uses document visibility.
    // inactive retains a visible UI while focus moves to a dialog or another view.
    _accept(switch (state) {
      AppLifecycleState.resumed || AppLifecycleState.inactive => true,
      AppLifecycleState.hidden ||
      AppLifecycleState.paused ||
      AppLifecycleState.detached => false,
    }, 'flutter-${state.name}');
  }

  void _accept(Object? visible, String source) {
    if (_disposed) return;
    if (visible is! bool) throw StateError('Host visibility must be a boolean');
    final next = visible ? AppVisibility.visible : AppVisibility.hidden;
    if (next == value) return;
    if (ClientLogger.isInitialized) {
      ClientLogger.i(
        'main UI ${value.name} -> ${next.name} source=$source',
        tag: 'AppVisibility',
      );
    }
    value = next;
  }

  @override
  void dispose() {
    _disposed = true;
    WidgetsBinding.instance.removeObserver(this);
    if (_nativeDesktop) channel.setMethodCallHandler(null);
    super.dispose();
  }
}
