// ignore_for_file: file_names

import 'dart:convert';
import 'dart:math';

import '../link/RemoteRuntimeLinkClient.dart';
import '../path/OperitClientPaths.dart';

enum LinkHostPortMode { automatic, fixed }

class LinkHostConfig {
  const LinkHostConfig({
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

  factory LinkHostConfig.initial() {
    return LinkHostConfig(
      webAccessEnabled: false,
      discoveryEnabled: false,
      portMode: LinkHostPortMode.automatic,
      bindAddress: automaticBindAddress,
      token: LinkHostToken.generate(),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  factory LinkHostConfig.fromJson(Map<String, Object?> json) {
    return LinkHostConfig(
      webAccessEnabled: json['webAccessEnabled'] as bool,
      discoveryEnabled: json['discoveryEnabled'] as bool,
      portMode: _linkHostPortModeFromJson(json['portMode']),
      bindAddress: json['bindAddress'] as String,
      token: json['token'] as String,
      updatedAt: json['updatedAt'] as int,
    );
  }

  final bool webAccessEnabled;
  final bool discoveryEnabled;
  final LinkHostPortMode portMode;
  final String bindAddress;
  final String token;
  final int updatedAt;

  LinkHostConfig copyWith({
    bool? webAccessEnabled,
    bool? discoveryEnabled,
    LinkHostPortMode? portMode,
    String? bindAddress,
    String? token,
    int? updatedAt,
  }) {
    return LinkHostConfig(
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

LinkHostPortMode _linkHostPortModeFromJson(Object? value) {
  if (value is String) {
    for (final mode in LinkHostPortMode.values) {
      if (mode.name == value) {
        return mode;
      }
    }
  }
  return LinkHostPortMode.automatic;
}

class LinkHostConfigStore {
  const LinkHostConfigStore._();

  static Future<LinkHostConfig> read() async {
    final file = await OperitClientPaths.linkHostConfigFile();
    if (!await file.exists()) {
      return LinkHostConfig.initial();
    }
    final content = await file.readAsString();
    return LinkHostConfig.fromJson(jsonDecode(content) as Map<String, Object?>);
  }

  static Future<void> write(LinkHostConfig config) async {
    final file = await OperitClientPaths.linkHostConfigFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(
      const JsonEncoder.withIndent('  ').convert(config),
    );
  }
}

class LinkHostDeviceIdStore {
  const LinkHostDeviceIdStore._();

  static Future<String> read() async {
    final file = await OperitClientPaths.linkHostDeviceIdFile();
    if (await file.exists()) {
      final deviceId = (await file.readAsString()).trim();
      if (deviceId.isEmpty) {
        throw StateError('empty link host device id: ${file.path}');
      }
      return deviceId;
    }
    await file.parent.create(recursive: true);
    final deviceId = 'core-${_uuidV4()}';
    await file.writeAsString(deviceId);
    return deviceId;
  }
}

class InboundLinkSessionRecord {
  const InboundLinkSessionRecord({
    required this.deviceId,
    required this.deviceInfo,
    required this.pairingServiceVersion,
    required this.sessionSecret,
  });

  factory InboundLinkSessionRecord.fromJson(Map<String, Object?> json) {
    return InboundLinkSessionRecord(
      deviceId: json['deviceId'] as String,
      deviceInfo: RemoteDeviceInfo.fromJson(
        json['deviceInfo'] as Map<String, Object?>,
      ),
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      sessionSecret: json['sessionSecret'] as String,
    );
  }

  final String deviceId;
  final RemoteDeviceInfo deviceInfo;
  final int pairingServiceVersion;
  final String sessionSecret;

  Map<String, Object?> toJson() {
    return {
      'deviceId': deviceId,
      'deviceInfo': deviceInfo.toJson(),
      'pairingServiceVersion': pairingServiceVersion,
      'sessionSecret': sessionSecret,
    };
  }
}

String _uuidV4() {
  final random = Random.secure();
  final bytes = List<int>.generate(16, (_) => random.nextInt(256));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  final hex = bytes
      .map((byte) => byte.toRadixString(16).padLeft(2, '0'))
      .join();
  return '${hex.substring(0, 8)}-${hex.substring(8, 12)}-${hex.substring(12, 16)}-${hex.substring(16, 20)}-${hex.substring(20)}';
}

class InboundLinkSessionStore {
  const InboundLinkSessionStore._();

  static Future<Map<String, InboundLinkSessionRecord>> read() async {
    final file = await OperitClientPaths.inboundLinkSessionsFile();
    if (!await file.exists()) {
      return <String, InboundLinkSessionRecord>{};
    }
    final content = await file.readAsString();
    final decoded = jsonDecode(content) as Map<String, Object?>;
    return decoded.map(
      (key, value) => MapEntry(
        key,
        InboundLinkSessionRecord.fromJson(value as Map<String, Object?>),
      ),
    );
  }

  static Future<void> write(
    Map<String, InboundLinkSessionRecord> sessions,
  ) async {
    final file = await OperitClientPaths.inboundLinkSessionsFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(
      const JsonEncoder.withIndent(
        '  ',
      ).convert(sessions.map((key, value) => MapEntry(key, value.toJson()))),
    );
  }
}

class PendingLinkPairingCodeRecord {
  const PendingLinkPairingCodeRecord({
    required this.pairingId,
    required this.pairingServiceVersion,
    required this.clientDeviceId,
    required this.clientDeviceInfo,
    required this.pairingCode,
    required this.createdAt,
  });

  factory PendingLinkPairingCodeRecord.fromJson(Map<String, Object?> json) {
    return PendingLinkPairingCodeRecord(
      pairingId: json['pairingId'] as String,
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      clientDeviceId: json['clientDeviceId'] as String,
      clientDeviceInfo: RemoteDeviceInfo.fromJson(
        json['clientDeviceInfo'] as Map<String, Object?>,
      ),
      pairingCode: json['pairingCode'] as String,
      createdAt: json['createdAt'] as int,
    );
  }

  final String pairingId;
  final int pairingServiceVersion;
  final String clientDeviceId;
  final RemoteDeviceInfo clientDeviceInfo;
  final String pairingCode;
  final int createdAt;
}

class PendingLinkPairingCodeStore {
  const PendingLinkPairingCodeStore._();

  static Future<PendingLinkPairingCodeRecord?> read() async {
    final file = await OperitClientPaths.pendingLinkPairingCodeFile();
    if (!await file.exists()) {
      return null;
    }
    final content = await file.readAsString();
    return PendingLinkPairingCodeRecord.fromJson(
      jsonDecode(content) as Map<String, Object?>,
    );
  }
}

class LinkHostToken {
  const LinkHostToken._();

  static String generate() {
    final random = Random.secure();
    final bytes = List<int>.generate(18, (_) => random.nextInt(256));
    return 'ow-${base64Url.encode(bytes).replaceAll('=', '')}';
  }
}
