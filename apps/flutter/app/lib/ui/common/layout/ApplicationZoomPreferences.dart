// ignore_for_file: file_names

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import 'ApplicationZoom.dart';

/// Stores interface zoom through Core's node-local runtime storage API.
class ApplicationZoomPreferences {
  /// Uses the same Core/Host storage chain on every platform.
  const ApplicationZoomPreferences({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients;

  final GeneratedCoreProxyClients _clients;

  /// Reads this node's saved zoom, defaulting to 100% for a new local store.
  Future<double> load() async {
    final storage = _clients.repositoryRuntimeStorageRepository;
    final encoded = await storage.readText(
      path: await storage.applicationZoomPath(),
    );
    if (encoded == null) {
      return 1.0;
    }
    final zoom = double.parse(encoded);
    if (!ApplicationZoom.levels.contains(zoom)) {
      throw FormatException('Invalid application zoom: $encoded');
    }
    return zoom;
  }

  /// Core classifies this path as CoreNode-owned, so no sync operation is made.
  Future<void> save(double zoom) async {
    if (!ApplicationZoom.levels.contains(zoom)) {
      throw ArgumentError.value(zoom, 'zoom', 'Unknown zoom level');
    }
    final storage = _clients.repositoryRuntimeStorageRepository;
    await storage.writeText(
      path: await storage.applicationZoomPath(),
      content: zoom.toString(),
    );
  }
}
