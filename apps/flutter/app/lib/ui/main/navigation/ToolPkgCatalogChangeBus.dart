// ignore_for_file: file_names

import 'dart:async';

/// Distinguishes registration-only toggles from possible plugin code replacements.
class ToolPkgCatalogChange {
  const ToolPkgCatalogChange({required this.reloadPluginCode});

  final bool reloadPluginCode;
}

/// Broadcasts ToolPkg catalog mutations so navigation surfaces can reload
/// the sidebar entries and routes exposed by enabled packages.
class ToolPkgCatalogChangeBus {
  const ToolPkgCatalogChangeBus._();

  static final StreamController<ToolPkgCatalogChange> _changes =
      StreamController<ToolPkgCatalogChange>.broadcast();

  /// Unknown mutations may replace code even when registration metadata is equal.
  /// Enable/disable operations can preserve sessions for unchanged registrations.
  static void notifyCatalogChanged({bool reloadPluginCode = true}) {
    _changes.add(ToolPkgCatalogChange(reloadPluginCode: reloadPluginCode));
  }

  /// Subscribes a listener to catalog change events.
  static StreamSubscription<void> listen(void Function() listener) {
    return _changes.stream.listen((_) => listener());
  }

  /// Subscribes a session owner that must distinguish code reloads from toggles.
  static StreamSubscription<ToolPkgCatalogChange> listenChanges(
    void Function(ToolPkgCatalogChange change) listener,
  ) => _changes.stream.listen(listener);
}
