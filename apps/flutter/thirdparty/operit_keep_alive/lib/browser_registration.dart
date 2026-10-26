import 'package:flutter/services.dart';
import 'package:flutter_web_plugins/flutter_web_plugins.dart';

/// Reports browser page residency honestly without pretending to survive tab closure.
class BrowserKeepAlivePlugin {
  final Set<String> _leases = {};
  bool _initialized = false;

  /// Registers the shared channel directly with Flutter's browser plugin registry.
  static void registerWith(Registrar registrar) {
    final plugin = BrowserKeepAlivePlugin();
    final channel = MethodChannel(
      'operit/keep_alive',
      const StandardMethodCodec(),
      registrar,
    );
    channel.setMethodCallHandler(plugin._handle);
  }

  /// Implements lease accounting while returning the browser's unavailable protection.
  Future<Object?> _handle(MethodCall call) async {
    final arguments = call.arguments as Map<Object?, Object?>;
    switch (call.method) {
      case 'workCapabilities':
        return {
          'updatesOwner': 'client',
          'surface': 'none',
          'liveUpdates': false,
        };
      case 'workActionsReady':
        return null;
      case 'publishWork':
        throw UnsupportedError(
          'The browser host has no persistent system work surface',
        );
      case 'initialize':
        _initialized = true;
      case 'acquire':
        if (!_initialized) throw StateError('Keep-alive is not initialized');
        final id = arguments['id'] as String;
        if (!_leases.add(id)) throw StateError('Duplicate lease: $id');
      case 'release':
        final id = arguments['id'] as String;
        if (!_leases.remove(id)) throw StateError('Unknown lease: $id');
      default:
        throw MissingPluginException(
          'Unknown keep-alive operation: ${call.method}',
        );
    }
    return {
      'protection': 'unavailable',
      'closeToTray': false,
      'activeLeases': _leases.length,
      'detail':
          'Browsers can freeze or discard this page; closing the tab ends its local runtime.',
    };
  }
}
