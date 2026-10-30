/// Debug-only scenario registry shared by host routes and plugin surfaces.
library;

import 'dart:async';

/// A real application state that can be opened for UI capture.
class OperitDesignScenario {
  const OperitDesignScenario({
    required this.id,
    required this.name,
    required this.group,
    required this.open,
    this.source = const {},
  });

  final String id;
  final String name;
  final String group;
  final Map<String, Object?> source;
  final FutureOr<void> Function() open;

  Map<String, Object?> toJson() => {
    'id': id,
    'name': name,
    'group': group,
    'source': source,
  };
}

/// The app supplies real routes; capture never guesses or invokes tool actions.
class OperitDesignScenarios {
  static Object? _owner;
  static List<OperitDesignScenario> Function()? _list;
  static FutureOr<void> Function()? _restore;

  static void install({
    required Object owner,
    required List<OperitDesignScenario> Function() list,
    required FutureOr<void> Function() restore,
  }) {
    _owner = owner;
    _list = list;
    _restore = restore;
  }

  static void uninstall(Object owner) {
    if (!identical(owner, _owner)) return;
    _owner = null;
    _list = null;
    _restore = null;
  }

  static List<Map<String, Object?>> list() {
    final provider = _list;
    if (provider == null) {
      throw StateError('Software design catalog is not mounted');
    }
    return provider().map((scenario) => scenario.toJson()).toList();
  }

  static Future<void> open(String id) async {
    final provider = _list;
    if (provider == null) {
      throw StateError('Software design catalog is not mounted');
    }
    final matches = provider().where((scenario) => scenario.id == id);
    if (matches.length != 1) {
      throw StateError('Unknown or ambiguous design scenario: $id');
    }
    await _restore?.call();
    await matches.single.open();
  }

  static Future<void> restore() async => await _restore?.call();
}
