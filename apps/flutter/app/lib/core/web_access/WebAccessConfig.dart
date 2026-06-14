// ignore_for_file: file_names

import 'dart:convert';
import 'dart:math';

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
      bindAddress: '127.0.0.1:37194',
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
    await file.writeAsString(const JsonEncoder.withIndent('  ').convert(config));
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
