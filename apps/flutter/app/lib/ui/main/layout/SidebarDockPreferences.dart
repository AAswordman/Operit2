// ignore_for_file: file_names

import 'dart:convert';

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

  Stream<Set<SidebarDirectory>> watchDirectoryPins() {
    return _clients.preferencesPreferenceStorageManager
        .preferencesFlow(fileName: _fileName)
        .map((values) {
          final directories = <SidebarDirectory>{};
          for (final entry in values.entries) {
            if (!entry.key.startsWith(SidebarDirectory._keyPrefix)) continue;
            final directory = SidebarDirectory.fromPreferenceKey(entry.key);
            if (_decodeDirectoryPinned(entry.value)) {
              directories.add(directory);
            }
          }
          return Set<SidebarDirectory>.unmodifiable(directories);
        })
        .distinct((left, right) => setEquals(left, right));
  }

  Future<bool> isDirectoryPinned(SidebarDirectory directory) async {
    final value = await _clients.preferencesPreferenceStorageManager
        .getPreference(fileName: _fileName, key: directory.preferenceKey);
    return _decodeDirectoryPinned(value);
  }

  Future<void> setDirectoryPinned(SidebarDirectory directory, bool pinned) {
    if (pinned) {
      return _clients.preferencesPreferenceStorageManager.setPreference(
        fileName: _fileName,
        key: directory.preferenceKey,
        value: true,
      );
    }
    return _clients.preferencesPreferenceStorageManager.removePreference(
      fileName: _fileName,
      key: directory.preferenceKey,
    );
  }

  Future<void> moveDirectoryPin({
    required SidebarDirectory from,
    required SidebarDirectory to,
  }) async {
    if (!await isDirectoryPinned(from)) return;
    await setDirectoryPinned(to, true);
    await setDirectoryPinned(from, false);
  }

  static bool _decodeDirectoryPinned(Object? value) {
    if (value == null) return false;
    if (value is! bool) {
      throw const FormatException('sidebar directory pin must be a boolean');
    }
    return value;
  }
}

enum SidebarDirectoryKind { workspace, group }

@immutable
class SidebarDirectory {
  const SidebarDirectory.workspace(this.workspaceId)
    : kind = SidebarDirectoryKind.workspace,
      groupName = null;

  const SidebarDirectory.group({
    required this.workspaceId,
    required this.groupName,
  }) : kind = SidebarDirectoryKind.group;

  static const String _keyPrefix = 'directory_pinned:';

  final SidebarDirectoryKind kind;
  final String? workspaceId;
  final String? groupName;

  String get preferenceKey =>
      '$_keyPrefix${jsonEncode(<String, Object?>{'kind': kind.name, 'workspaceId': workspaceId, 'groupName': groupName})}';

  factory SidebarDirectory.fromPreferenceKey(String key) {
    if (!key.startsWith(_keyPrefix)) {
      throw const FormatException('invalid sidebar directory pin key');
    }
    final decoded = jsonDecode(key.substring(_keyPrefix.length));
    if (decoded is! Map ||
        decoded.length != 3 ||
        !decoded.containsKey('kind') ||
        !decoded.containsKey('workspaceId') ||
        !decoded.containsKey('groupName')) {
      throw const FormatException('invalid sidebar directory identity');
    }
    final workspaceId = decoded['workspaceId'];
    if (workspaceId != null &&
        (workspaceId is! String ||
            workspaceId.isEmpty ||
            workspaceId != workspaceId.trim())) {
      throw const FormatException('invalid sidebar directory workspace');
    }
    final groupName = decoded['groupName'];
    switch (decoded['kind']) {
      case 'workspace':
        if (groupName != null) {
          throw const FormatException(
            'workspace directory cannot have a group',
          );
        }
        return SidebarDirectory.workspace(workspaceId as String?);
      case 'group':
        if (groupName != null &&
            (groupName is! String ||
                groupName.isEmpty ||
                groupName != groupName.trim())) {
          throw const FormatException('invalid sidebar directory group');
        }
        return SidebarDirectory.group(
          workspaceId: workspaceId as String?,
          groupName: groupName,
        );
    }
    throw const FormatException('unknown sidebar directory kind');
  }

  @override
  bool operator ==(Object other) =>
      other is SidebarDirectory &&
      other.kind == kind &&
      other.workspaceId == workspaceId &&
      other.groupName == groupName;

  @override
  int get hashCode => Object.hash(kind, workspaceId, groupName);
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
