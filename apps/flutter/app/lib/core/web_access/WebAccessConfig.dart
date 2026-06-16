// ignore_for_file: file_names

import 'dart:convert';
import 'dart:math';

import '../link/RemoteRuntimeLinkClient.dart';
import '../path/OperitClientPaths.dart';

class WebAccessConfig {
  const WebAccessConfig({
    required this.enabled,
    required this.bindAddress,
    required this.token,
    required this.updatedAt,
  });

  factory WebAccessConfig.initial() {
    return WebAccessConfig(
      enabled: false,
      bindAddress: '0.0.0.0:37194',
      token: WebAccessToken.generate(),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  factory WebAccessConfig.fromJson(Map<String, Object?> json) {
    return WebAccessConfig(
      enabled: json['enabled'] as bool,
      bindAddress: json['bindAddress'] as String,
      token: json['token'] as String,
      updatedAt: json['updatedAt'] as int,
    );
  }

  final bool enabled;
  final String bindAddress;
  final String token;
  final int updatedAt;

  WebAccessConfig copyWith({
    bool? enabled,
    String? bindAddress,
    String? token,
    int? updatedAt,
  }) {
    return WebAccessConfig(
      enabled: enabled ?? this.enabled,
      bindAddress: bindAddress ?? this.bindAddress,
      token: token ?? this.token,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'enabled': enabled,
      'bindAddress': bindAddress,
      'token': token,
      'updatedAt': updatedAt,
    };
  }
}

class WebAccessConfigStore {
  const WebAccessConfigStore._();

  static Future<WebAccessConfig> read() async {
    final file = await OperitClientPaths.webAccessConfigFile();
    if (!await file.exists()) {
      return WebAccessConfig.initial();
    }
    final content = await file.readAsString();
    return WebAccessConfig.fromJson(
      jsonDecode(content) as Map<String, Object?>,
    );
  }

  static Future<void> write(WebAccessConfig config) async {
    final file = await OperitClientPaths.webAccessConfigFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(
      const JsonEncoder.withIndent('  ').convert(config),
    );
  }
}

class WebAccessAcceptedSessionRecord {
  const WebAccessAcceptedSessionRecord({
    required this.deviceId,
    required this.deviceInfo,
    required this.pairingServiceVersion,
    required this.sessionSecret,
  });

  factory WebAccessAcceptedSessionRecord.fromJson(Map<String, Object?> json) {
    return WebAccessAcceptedSessionRecord(
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

class WebAccessAcceptedSessionStore {
  const WebAccessAcceptedSessionStore._();

  static Future<Map<String, WebAccessAcceptedSessionRecord>> read() async {
    final file = await OperitClientPaths.webAccessAcceptedSessionsFile();
    if (!await file.exists()) {
      return <String, WebAccessAcceptedSessionRecord>{};
    }
    final content = await file.readAsString();
    final decoded = jsonDecode(content) as Map<String, Object?>;
    return decoded.map(
      (key, value) => MapEntry(
        key,
        WebAccessAcceptedSessionRecord.fromJson(value as Map<String, Object?>),
      ),
    );
  }

  static Future<void> write(
    Map<String, WebAccessAcceptedSessionRecord> sessions,
  ) async {
    final file = await OperitClientPaths.webAccessAcceptedSessionsFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(
      const JsonEncoder.withIndent(
        '  ',
      ).convert(sessions.map((key, value) => MapEntry(key, value.toJson()))),
    );
  }
}

class WebAccessPairingCodeRecord {
  const WebAccessPairingCodeRecord({
    required this.pairingId,
    required this.pairingServiceVersion,
    required this.clientDeviceId,
    required this.clientDeviceInfo,
    required this.pairingCode,
    required this.createdAt,
  });

  factory WebAccessPairingCodeRecord.fromJson(Map<String, Object?> json) {
    return WebAccessPairingCodeRecord(
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

class WebAccessPairingCodeStore {
  const WebAccessPairingCodeStore._();

  static Future<WebAccessPairingCodeRecord?> read() async {
    final file = await OperitClientPaths.webAccessPairingCodeFile();
    if (!await file.exists()) {
      return null;
    }
    final content = await file.readAsString();
    return WebAccessPairingCodeRecord.fromJson(
      jsonDecode(content) as Map<String, Object?>,
    );
  }
}

class WebAccessToken {
  const WebAccessToken._();

  static String generate() {
    final random = Random.secure();
    final bytes = List<int>.generate(18, (_) => random.nextInt(256));
    return 'ow-${base64Url.encode(bytes).replaceAll('=', '')}';
  }
}
