// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';

/// Stores the persisted placement of plugin views across application launches.
@immutable
class SidebarDockLayout {
  /// Creates one persisted sidebar layout snapshot.
  const SidebarDockLayout({
    required this.primaryEntryIds,
    required this.secondaryEntryIds,
    required this.selectedSecondaryViewId,
  });

  final List<String> primaryEntryIds;
  final List<String> secondaryEntryIds;
  final String selectedSecondaryViewId;

  /// Decodes one strict persisted sidebar layout object.
  factory SidebarDockLayout.fromJson(Object? value) {
    if (value is! Map) {
      throw const FormatException('sidebar dock layout must be an object');
    }
    final map = value.cast<Object?, Object?>();
    final version = map['version'];
    if (version != 1) {
      throw FormatException(
        'unsupported sidebar dock layout version: $version',
      );
    }
    final primaryEntryIds = _decodeStringList(map['primaryEntryIds']);
    final secondaryEntryIds = _decodeStringList(map['secondaryEntryIds']);
    if (primaryEntryIds.toSet().length != primaryEntryIds.length ||
        secondaryEntryIds.toSet().length != secondaryEntryIds.length ||
        primaryEntryIds
            .toSet()
            .intersection(secondaryEntryIds.toSet())
            .isNotEmpty) {
      throw const FormatException(
        'sidebar dock entry ids must be unique across both zones',
      );
    }
    final selectedSecondaryViewId = map['selectedSecondaryViewId'];
    if (selectedSecondaryViewId is! String ||
        selectedSecondaryViewId.trim().isEmpty) {
      throw const FormatException(
        'sidebar dock selected view must be a non-empty string',
      );
    }
    return SidebarDockLayout(
      primaryEntryIds: List<String>.unmodifiable(primaryEntryIds),
      secondaryEntryIds: List<String>.unmodifiable(secondaryEntryIds),
      selectedSecondaryViewId: selectedSecondaryViewId,
    );
  }

  /// Encodes this sidebar layout into the persisted JSON representation.
  Map<String, Object?> toJson() {
    return <String, Object?>{
      'version': 1,
      'primaryEntryIds': primaryEntryIds,
      'secondaryEntryIds': secondaryEntryIds,
      'selectedSecondaryViewId': selectedSecondaryViewId,
    };
  }
}

/// Provides Core-backed persistence for the Flutter sidebar dock state.
class SidebarDockPreferences {
  /// Creates a sidebar preference store using the shared preference host API.
  const SidebarDockPreferences({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients;

  static const String _fileName = 'sidebar_dock.preferences.json';
  static const String _layoutKey = 'layout';

  final GeneratedCoreProxyClients _clients;

  /// Loads the saved sidebar layout, returning null when no layout exists yet.
  Future<SidebarDockLayout?> load() async {
    final values = await _clients.preferencesPreferenceStorageManager
        .getPreferences(fileName: _fileName, keys: <String>[_layoutKey]);
    return _decodeLayout(values[_layoutKey]);
  }

  /// Observes sidebar layouts committed locally or synchronized from peers.
  Stream<SidebarDockLayout?> watch() {
    return _clients.preferencesPreferenceStorageManager
        .preferencesFlow(fileName: _fileName)
        .map((values) => values[_layoutKey])
        .distinct()
        .map(_decodeLayout);
  }

  /// Decodes one layout snapshot using the same schema as startup reads.
  SidebarDockLayout? _decodeLayout(Object? encoded) {
    if (encoded == null) {
      return null;
    }
    return SidebarDockLayout.fromJson(encoded);
  }

  /// Saves one sidebar layout through the shared Core preference store.
  Future<void> save(SidebarDockLayout layout) {
    return _clients.preferencesPreferenceStorageManager.setPreferences(
      fileName: _fileName,
      values: <String, Object?>{_layoutKey: layout.toJson()},
    );
  }
}

/// Decodes a strict list of non-empty persisted identifiers.
List<String> _decodeStringList(Object? value) {
  if (value is! List) {
    throw const FormatException('sidebar dock entry ids must be an array');
  }
  final result = <String>[];
  for (final item in value) {
    if (item is! String || item.trim().isEmpty) {
      throw const FormatException(
        'sidebar dock entry ids must contain non-empty strings',
      );
    }
    result.add(item);
  }
  return result;
}
