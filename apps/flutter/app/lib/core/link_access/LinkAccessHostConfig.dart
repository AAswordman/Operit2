// ignore_for_file: file_names

import 'dart:convert';
import 'dart:math';

import '../path/OperitClientPaths.dart';

enum LinkAccessHostPortMode { automatic, fixed }

class LinkAccessHostConfig {
  const LinkAccessHostConfig({
    required this.webAccessEnabled,
    required this.discoveryEnabled,
    required this.portMode,
    required this.bindAddress,
    required this.token,
    required this.updatedAt,
  });

  static const List<int> automaticPortSequence = <int>[
    37194,
    37195,
    37196,
    37197,
    37198,
    37199,
    37200,
    37201,
    37202,
    37203,
  ];
  static const String automaticBindAddress = '0.0.0.0:37194';

  factory LinkAccessHostConfig.initial() {
    return LinkAccessHostConfig(
      webAccessEnabled: false,
      discoveryEnabled: false,
      portMode: LinkAccessHostPortMode.automatic,
      bindAddress: automaticBindAddress,
      token: LinkAccessHostToken.generate(),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  factory LinkAccessHostConfig.fromJson(Map<String, Object?> json) {
    return LinkAccessHostConfig(
      webAccessEnabled: json['webAccessEnabled'] as bool,
      discoveryEnabled: json['discoveryEnabled'] as bool,
      portMode: _linkAccessHostPortModeFromJson(json['portMode']),
      bindAddress: json['bindAddress'] as String,
      token: json['token'] as String,
      updatedAt: json['updatedAt'] as int,
    );
  }

  final bool webAccessEnabled;
  final bool discoveryEnabled;
  final LinkAccessHostPortMode portMode;
  final String bindAddress;
  final String token;
  final int updatedAt;

  LinkAccessHostConfig copyWith({
    bool? webAccessEnabled,
    bool? discoveryEnabled,
    LinkAccessHostPortMode? portMode,
    String? bindAddress,
    String? token,
    int? updatedAt,
  }) {
    return LinkAccessHostConfig(
      webAccessEnabled: webAccessEnabled ?? this.webAccessEnabled,
      discoveryEnabled: discoveryEnabled ?? this.discoveryEnabled,
      portMode: portMode ?? this.portMode,
      bindAddress: bindAddress ?? this.bindAddress,
      token: token ?? this.token,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'webAccessEnabled': webAccessEnabled,
      'discoveryEnabled': discoveryEnabled,
      'portMode': portMode.name,
      'bindAddress': bindAddress,
      'token': token,
      'updatedAt': updatedAt,
    };
  }
}

/// Decodes the persisted Link Access Host port selection.
LinkAccessHostPortMode _linkAccessHostPortModeFromJson(Object? value) {
  if (value is String) {
    for (final mode in LinkAccessHostPortMode.values) {
      if (mode.name == value) {
        return mode;
      }
    }
  }
  throw FormatException('invalid Link Access Host port mode: $value');
}

class LinkAccessHostConfigStore {
  const LinkAccessHostConfigStore._();

  /// Reads host listener settings without loading Link Access session state.
  static Future<LinkAccessHostConfig> read() async {
    final file = await OperitClientPaths.linkAccessHostConfigFile();
    if (!await file.exists()) {
      return LinkAccessHostConfig.initial();
    }
    final content = await file.readAsString();
    return LinkAccessHostConfig.fromJson(jsonDecode(content) as Map<String, Object?>);
  }

  /// Writes host listener settings without persisting Link Access session state.
  static Future<void> write(LinkAccessHostConfig config) async {
    final file = await OperitClientPaths.linkAccessHostConfigFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(
      const JsonEncoder.withIndent('  ').convert(config),
    );
  }
}

class LinkAccessHostToken {
  const LinkAccessHostToken._();

  /// Generates the local listener control token.
  static String generate() {
    final random = Random.secure();
    final bytes = List<int>.generate(18, (_) => random.nextInt(256));
    return 'ow-${base64Url.encode(bytes).replaceAll('=', '')}';
  }
}
