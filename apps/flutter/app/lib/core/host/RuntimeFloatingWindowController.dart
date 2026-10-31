// ignore_for_file: file_names
import 'dart:async';
import 'dart:convert';
import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/widgets.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../../ui/main/navigation/ToolPkgCatalogChangeBus.dart';

class _OwnedFloatingWindow {
  _OwnedFloatingWindow(this.window, this.route, this.options, this.state);
  final FloatingWindow window;
  final String route;
  FloatingWindowOptions options;
  Map<String, Object?> state;
  bool visible = true;
  Future<Map<String, Object?>> info(String id) async {
    options = options.merge(await window.getBounds());
    return {
      'id': id,
      'route': route,
      'visible': visible,
      'options': options.toMap(),
    };
  }
}

/// The owner engine serializes lifecycle changes and indexes plugin-local IDs.
class RuntimeFloatingWindowController {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  static final _windows = <String, _OwnedFloatingWindow>{};
  static final _owners = <String, String>{};
  static final _ids = <String, String>{};
  static Future<void> _tail = Future<void>.value();
  static Timer? _audit;
  static StreamSubscription<void>? _catalog;

  static Future<Object?> command(Map<String, Object?> request) {
    final work = _tail.then((_) => _command(request));
    _tail = work.then<void>((_) {}, onError: (Object _, StackTrace stack) {});
    return work;
  }

  static Future<Object?> _command(Map<String, Object?> request) async {
    final owner = request['packageName'];
    if (owner is! String || owner.isEmpty) {
      throw ArgumentError('Missing authenticated window owner');
    }
    final operation = request['operation'];
    if (operation == 'capabilities') return FloatingWindows.capabilities();
    if (operation == 'requestPermission') {
      return FloatingWindows.requestPermission();
    }
    await _prune();
    if (operation == 'list') {
      return [
        for (final entry in _windows.entries)
          if (_owners[entry.key] == owner)
            await entry.value.info(_ids[entry.key]!),
      ];
    }
    final id = request['id'];
    if (id is! String || id.isEmpty || id.trim() != id || id.length > 128) {
      throw ArgumentError(
        'Window id must be a trimmed string of 1–128 characters',
      );
    }
    final key = jsonEncode([owner, id]);
    if (operation == 'open') {
      if (_windows.containsKey(key)) {
        throw StateError('Window already exists: $id');
      }
      if (_owners.values.where((value) => value == owner).length >= 16) {
        throw StateError('A plugin may own at most 16 floating windows');
      }
      final options = FloatingWindowOptions.defaults.merge(
        FloatingWindowOptions.fromMap(
          ((request['options'] as Map?) ?? {}).cast<String, Object?>(),
        ),
      );
      final support = await FloatingWindows.capabilities();
      if ((options.x != null || options.y != null) &&
          support['positioning'] != true) {
        throw UnsupportedError(
          'This host does not support absolute window positioning',
        );
      }
      if (request['state'] != null && request['state'] is! Map) {
        throw ArgumentError('Window state must be an object');
      }
      final route = request['route'] as String;
      final window = await FloatingWindows.open(
        payload: {
          'plugin': request['plugin'],
          'route': route,
          'state': request['state'] ?? {},
          'windowId': id,
          'locale':
              WidgetsBinding.instance.platformDispatcher.locale.languageCode,
        },
        options: options,
      );
      final owned = _OwnedFloatingWindow(
        window,
        route,
        options,
        Map<String, Object?>.from((request['state'] as Map?) ?? {}),
      );
      _windows[key] = owned;
      _owners[key] = owner;
      _ids[key] = id;
      _startAudit();
      return owned.info(id);
    }
    final owned = _windows[key];
    if (owned == null) {
      throw StateError('Unknown window owned by this plugin: $id');
    }
    switch (operation) {
      case 'update':
        final patch = FloatingWindowOptions.fromMap(
          (request['options'] as Map).cast<String, Object?>(),
        );
        final rawState = request['state'];
        if (rawState != null && rawState is! Map) {
          throw ArgumentError('Window state must be an object');
        }
        final state = rawState == null
            ? null
            : {...owned.state, ...Map<String, Object?>.from(rawState as Map)};
        await owned.window.update(
          patch,
          payload: state == null ? null : {'state': state},
        );
        owned.options = owned.options.merge(patch);
        if (state != null) owned.state = state;
      case 'show':
        await owned.window.show();
        owned.visible = true;
      case 'hide':
        await owned.window.hide();
        owned.visible = false;
      case 'close':
        await owned.window.close();
        _remove(key);
        return null;
      default:
        throw ArgumentError('Unknown floating window operation: $operation');
    }
    return owned.info(id);
  }

  static void _remove(String key) {
    _windows.remove(key);
    _owners.remove(key);
    _ids.remove(key);
    if (_windows.isEmpty) {
      _audit?.cancel();
      _audit = null;
      _catalog?.cancel();
      _catalog = null;
    }
  }

  static Future<void> _prune() async {
    if (_windows.isEmpty) return;
    final live = await FloatingWindows.liveIds();
    for (final entry in _windows.entries.toList()) {
      if (!live.contains(entry.value.window.id)) _remove(entry.key);
    }
  }

  static void _startAudit() {
    if (_audit != null) return;
    void reconcile() {
      final work = _tail.then((_) async {
        await _prune();
        if (_windows.isEmpty) return;
        final enabled =
            (await _clients.application
                    .packageManager()
                    .getEnabledToolPkgContainerRuntimes())
                .map((plugin) => plugin.packageName)
                .toSet();
        for (final entry in _windows.entries.toList()) {
          if (!enabled.contains(_owners[entry.key])) {
            await entry.value.window.close();
            _remove(entry.key);
          }
        }
      });
      _tail = work.catchError((Object error, StackTrace stack) {
        FlutterError.reportError(
          FlutterErrorDetails(
            exception: error,
            stack: stack,
            library: 'floating windows',
            context: ErrorDescription('reconciling plugin windows'),
          ),
        );
      });
    }

    _audit = Timer.periodic(const Duration(seconds: 5), (_) => reconcile());
    _catalog = ToolPkgCatalogChangeBus.listen(reconcile);
  }
}
