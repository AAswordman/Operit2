// ignore_for_file: file_names

import 'dart:math' as math;

class RuntimeIdentity {
  const RuntimeIdentity({
    required this.id,
    required this.name,
    required this.createdAt,
  });

  /// Creates one isolated user identity with a stable storage-safe identifier.
  factory RuntimeIdentity.create([String name = '']) {
    final normalizedName = name.trim();
    final now = DateTime.now().millisecondsSinceEpoch;
    final random = math.Random.secure().nextInt(0x7fffffff);
    return RuntimeIdentity(
      id: 'identity-$now-${random.toRadixString(16).padLeft(8, '0')}',
      name: normalizedName,
      createdAt: now,
    );
  }

  /// Creates an identity from its persisted bootstrap representation.
  factory RuntimeIdentity.fromJson(Map<String, Object?> json) {
    final identity = RuntimeIdentity(
      id: json['id'] as String,
      name: json['name'] as String,
      createdAt: json['createdAt'] as int,
    );
    identity.validate();
    return identity;
  }

  final String id;
  final String name;
  final int createdAt;

  /// Validates the identity before it is used as a storage path segment.
  void validate() {
    if (!RegExp(r'^identity-[a-z0-9-]+$').hasMatch(id)) {
      throw FormatException('invalid runtime identity id: $id');
    }
    if (name.trim().length > 80) {
      throw FormatException('runtime identity name is invalid');
    }
    if (createdAt <= 0) {
      throw FormatException('runtime identity creation time is invalid');
    }
  }

  /// Creates a renamed identity while preserving its storage identifier.
  RuntimeIdentity rename(String name) {
    final renamed = RuntimeIdentity(
      id: id,
      name: name.trim(),
      createdAt: createdAt,
    );
    renamed.validate();
    return renamed;
  }

  /// Converts this identity into its persisted bootstrap representation.
  Map<String, Object?> toJson() {
    return <String, Object?>{'id': id, 'name': name, 'createdAt': createdAt};
  }
}

class LocalRuntimeStorageConfig {
  const LocalRuntimeStorageConfig({
    required this.confirmed,
    required this.runtimeRoot,
    required this.workspaceRoot,
    required this.identities,
    required this.activeIdentityId,
    required this.updatedAt,
  });

  /// Creates an unconfirmed local runtime storage config.
  factory LocalRuntimeStorageConfig.platformDefault() {
    final identity = RuntimeIdentity.create();
    return LocalRuntimeStorageConfig(
      confirmed: false,
      runtimeRoot: '',
      workspaceRoot: '',
      identities: <RuntimeIdentity>[identity],
      activeIdentityId: identity.id,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  /// Creates a storage configuration from its persisted representation.
  factory LocalRuntimeStorageConfig.fromJson(Map<String, Object?> json) {
    final config = LocalRuntimeStorageConfig(
      confirmed: json['confirmed'] as bool,
      runtimeRoot: json['runtimeRoot'] as String,
      workspaceRoot: json['workspaceRoot'] as String,
      identities: (json['identities'] as List<Object?>)
          .map(
            (item) => RuntimeIdentity.fromJson(item! as Map<String, Object?>),
          )
          .toList(growable: false),
      activeIdentityId: json['activeIdentityId'] as String,
      updatedAt: json['updatedAt'] as int,
    );
    config.validate();
    return config;
  }

  final bool confirmed;
  final String runtimeRoot;
  final String workspaceRoot;
  final List<RuntimeIdentity> identities;
  final String activeIdentityId;
  final int updatedAt;

  /// Returns the identity selected for the next runtime process.
  RuntimeIdentity get activeIdentity {
    return identities.singleWhere(
      (identity) => identity.id == activeIdentityId,
    );
  }

  /// Validates identity ownership and base storage roots.
  void validate() {
    if (identities.isEmpty) {
      throw const FormatException('at least one runtime identity is required');
    }
    final ids = <String>{};
    for (final identity in identities) {
      identity.validate();
      if (!ids.add(identity.id)) {
        throw FormatException('duplicate runtime identity id: ${identity.id}');
      }
    }
    if (!ids.contains(activeIdentityId)) {
      throw FormatException('active runtime identity does not exist');
    }
    if (confirmed &&
        (runtimeRoot.trim().isEmpty || workspaceRoot.trim().isEmpty)) {
      throw const FormatException(
        'confirmed runtime storage roots must not be empty',
      );
    }
  }

  /// Creates a copy with updated fields.
  LocalRuntimeStorageConfig copyWith({
    bool? confirmed,
    String? runtimeRoot,
    String? workspaceRoot,
    List<RuntimeIdentity>? identities,
    String? activeIdentityId,
    int? updatedAt,
  }) {
    return LocalRuntimeStorageConfig(
      confirmed: confirmed ?? this.confirmed,
      runtimeRoot: runtimeRoot ?? this.runtimeRoot,
      workspaceRoot: workspaceRoot ?? this.workspaceRoot,
      identities: identities ?? this.identities,
      activeIdentityId: activeIdentityId ?? this.activeIdentityId,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  /// Converts this configuration into its persisted representation.
  Map<String, Object?> toJson() {
    return <String, Object?>{
      'confirmed': confirmed,
      'runtimeRoot': runtimeRoot,
      'workspaceRoot': workspaceRoot,
      'identities': identities.map((identity) => identity.toJson()).toList(),
      'activeIdentityId': activeIdentityId,
      'updatedAt': updatedAt,
    };
  }
}

class RuntimeStoragePaths {
  const RuntimeStoragePaths({
    required this.runtimeRoot,
    required this.workspaceRoot,
  });

  /// Creates a storage path snapshot from native channel values.
  factory RuntimeStoragePaths.fromMap(Map<Object?, Object?> map) {
    return RuntimeStoragePaths(
      runtimeRoot: map['runtimeRoot'] as String,
      workspaceRoot: map['workspaceRoot'] as String,
    );
  }

  final String runtimeRoot;
  final String workspaceRoot;
}
