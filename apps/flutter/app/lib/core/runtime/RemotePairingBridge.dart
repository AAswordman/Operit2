// ignore_for_file: file_names

import 'dart:convert';

import 'package:crypto/crypto.dart';

import '../bridge/PlatformCoreProxy.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart' as generated;
import 'RuntimeDeviceInfoProvider.dart';

class RemotePairingBridge {
  /// Creates a bridge that forwards pairing actions to the local runtime.
  const RemotePairingBridge();

  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
  );

  /// Starts one runtime-owned pairing after hashing the user-supplied Link token.
  Future<RemotePairStartResult> startWithToken({
    required String baseUrl,
    required String token,
  }) {
    return startWithTokenHash(
      baseUrl: baseUrl,
      tokenHash: _linkTokenHash(token),
    );
  }

  /// Starts one runtime-owned pairing using an already-derived Link token hash.
  Future<RemotePairStartResult> startWithTokenHash({
    required String baseUrl,
    required String tokenHash,
  }) async {
    final clientDeviceInfo = await RuntimeDeviceInfoProvider.current();
    final result = await _clients.runtimeRemoteLinkService.startPairedRemote(
      baseUrl: baseUrl,
      tokenHash: tokenHash,
      clientDeviceInfo: clientDeviceInfo,
    );
    return RemotePairStartResult(
      pairingId: result.pairingId,
      pairingServiceVersion: result.pairingServiceVersion,
      coreDeviceId: result.coreDeviceId,
      coreDeviceInfo: result.coreDeviceInfo,
    );
  }

  /// Completes one runtime-owned pairing and selects the resulting named remote runtime.
  Future<generated.PairedRemoteSessionRecord> finish({
    required String pairingId,
    required String pairingCode,
    required String name,
  }) {
    return _clients.runtimeRemoteLinkService.finishPairedRemote(
      pairingId: pairingId,
      pairingCode: pairingCode,
      name: name,
    );
  }
}

/// Derives the Link protocol token hash from the user-provided secret.
String _linkTokenHash(String token) {
  return base64Encode(sha256.convert(utf8.encode(token)).bytes);
}

class RemotePairStartResult {
  /// Creates the UI representation of a runtime-owned pairing start result.
  const RemotePairStartResult({
    required this.pairingId,
    required this.pairingServiceVersion,
    required this.coreDeviceId,
    required this.coreDeviceInfo,
  });

  /// Decodes a pairing start result received through a Core Link response.
  factory RemotePairStartResult.fromJson(Map<String, Object?> json) {
    return RemotePairStartResult(
      pairingId: json['pairingId'] as String,
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      coreDeviceId: json['coreDeviceId'] as String,
      coreDeviceInfo: generated.RemoteDeviceInfo.fromJson(
        json['coreDeviceInfo'] as Map<String, Object?>,
      ),
    );
  }

  final String pairingId;
  final int pairingServiceVersion;
  final String coreDeviceId;
  final generated.RemoteDeviceInfo coreDeviceInfo;
}
