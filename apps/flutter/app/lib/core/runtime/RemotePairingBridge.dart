// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/services.dart';

import '../link/CoreLinkProtocol.dart';
import '../link/RemoteRuntimeLinkClient.dart';
import 'RuntimeDeviceInfoProvider.dart';

class RemotePairingBridge {
  const RemotePairingBridge({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;

  Future<RemotePairStartResult> start({
    required String baseUrl,
    required String token,
  }) async {
    final clientDeviceInfo = await RuntimeDeviceInfoProvider.current();
    final response = await _channel.invokeMethod<String>('remotePairStart', {
      'baseUrl': baseUrl,
      'token': token,
      'clientDeviceInfo': jsonEncode(clientDeviceInfo.toJson()),
    });
    if (response == null) {
      throw StateError('remote pair start returned empty response');
    }
    final decoded = jsonDecode(response) as Map<String, Object?>;
    if (decoded.containsKey('code') && decoded.containsKey('message')) {
      throw CoreLinkError.fromJson(decoded);
    }
    return RemotePairStartResult.fromJson(decoded);
  }

  Future<PairedRemoteSessionRecord> finish({
    required String pairingId,
    required String pairingCode,
  }) async {
    final response = await _channel.invokeMethod<String>('remotePairFinish', {
      'pairingId': pairingId,
      'pairingCode': pairingCode,
    });
    if (response == null) {
      throw StateError('remote pair finish returned empty response');
    }
    final decoded = jsonDecode(response) as Map<String, Object?>;
    if (decoded.containsKey('code') && decoded.containsKey('message')) {
      throw CoreLinkError.fromJson(decoded);
    }
    return PairedRemoteSessionRecord.fromJson(decoded);
  }
}

class RemotePairStartResult {
  const RemotePairStartResult({
    required this.pairingId,
    required this.pairingServiceVersion,
    required this.coreDeviceId,
    required this.coreDeviceInfo,
  });

  factory RemotePairStartResult.fromJson(Map<String, Object?> json) {
    return RemotePairStartResult(
      pairingId: json['pairingId'] as String,
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      coreDeviceId: json['coreDeviceId'] as String,
      coreDeviceInfo: RemoteDeviceInfo.fromJson(
        json['coreDeviceInfo'] as Map<String, Object?>,
      ),
    );
  }

  final String pairingId;
  final int pairingServiceVersion;
  final String coreDeviceId;
  final RemoteDeviceInfo coreDeviceInfo;
}
