// ignore_for_file: file_names

import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

import '../host/HostEnvironmentDescriptor.dart';
import '../link/CoreLinkProtocol.dart';
import 'CoreProxy.dart';
import 'OperitRuntimeBridge.dart';
import 'PlatformCoreProxy.dart';

class ProxyCoreRuntimeBridge extends OperitRuntimeBridge {
  const ProxyCoreRuntimeBridge({
    this.coreProxy = platformCoreProxy,
  });

  final CoreProxy coreProxy;

  @override
  Future<Object?> call(CoreCallRequest request) {
    return coreProxy.call(request);
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    return coreProxy.watchSnapshot(request);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    return coreProxy.watchStream(request);
  }

  @override
  Future<HostEnvironmentDescriptor> hostDescriptor() {
    return coreProxy.hostDescriptor();
  }
}

class RemoteCoreProxy extends CoreProxy {
  RemoteCoreProxy({required this.session, http.Client? client})
    : client = client ?? http.Client();

  final PairedRemoteSessionRecord session;
  final http.Client client;

  @override
  Future<Object?> call(CoreCallRequest request) async {
    final body = jsonEncode({'request': request.toJson()});
    debugPrint(
      '[OperitRemoteCore] call -> ${request.targetPath.key}.${request.methodName} '
      'id=${request.requestId} url=${session.uri('/link/call')}',
    );
    final response = await client.post(
      session.uri('/link/call'),
      headers: session.signedHeaders(body),
      body: body,
    );
    debugPrint(
      '[OperitRemoteCore] call http <- status=${response.statusCode} '
      'id=${request.requestId} bytes=${response.body.length}',
    );
    _throwIfRemoteError(response);

    final json = jsonDecode(response.body) as Map<String, Object?>;
    final result = json['result'] as Map<String, Object?>;
    if (result.containsKey('Ok')) {
      debugPrint(
        '[OperitRemoteCore] call <- ok ${request.targetPath.key}.${request.methodName} '
        'id=${request.requestId}',
      );
      return result['Ok'];
    }
    if (result.containsKey('Err')) {
      final error = CoreLinkError.fromJson(result['Err'] as Map<String, Object?>);
      debugPrint(
        '[OperitRemoteCore] call <- err ${request.targetPath.key}.${request.methodName} '
        'id=${request.requestId} $error',
      );
      throw error;
    }
    throw const CoreLinkError(
      code: 'INVALID_RESPONSE',
      message: 'remote core call response result is invalid',
    );
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    final body = jsonEncode({'request': request.toJson()});
    debugPrint(
      '[OperitRemoteCore] watchSnapshot -> ${request.targetPath.key}.${request.propertyName} '
      'id=${request.requestId}',
    );
    final response = await client.post(
      session.uri('/link/watch/snapshot'),
      headers: session.signedHeaders(body),
      body: body,
    );
    debugPrint(
      '[OperitRemoteCore] watchSnapshot http <- status=${response.statusCode} '
      'id=${request.requestId} bytes=${response.body.length}',
    );
    _throwIfRemoteError(response);
    return CoreEvent.fromJson(
      jsonDecode(response.body) as Map<String, Object?>,
    );
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    final body = jsonEncode({'request': request.toJson()});
    debugPrint(
      '[OperitRemoteCore] watchStream -> ${request.targetPath.key}.${request.propertyName} '
      'id=${request.requestId}',
    );
    final httpRequest = http.Request('POST', session.uri('/link/watch/stream'))
      ..headers.addAll(session.signedHeaders(body))
      ..body = body;
    final response = await client.send(httpRequest);
    debugPrint(
      '[OperitRemoteCore] watchStream http <- status=${response.statusCode} '
      'id=${request.requestId}',
    );
    if (response.statusCode < 200 || response.statusCode >= 300) {
      final bodyText = await response.stream.bytesToString();
      _throwRemoteErrorBody(response.statusCode, bodyText);
    }
    var buffer = '';
    await for (final text in response.stream.transform(utf8.decoder)) {
      buffer += text;
      var index = buffer.indexOf('\n');
      while (index >= 0) {
        final line = buffer.substring(0, index).trim();
        buffer = buffer.substring(index + 1);
        if (line.isNotEmpty) {
          debugPrint(
            '[OperitRemoteCore] watchStream event lineBytes=${line.length} '
            'id=${request.requestId}',
          );
          yield CoreEvent.fromJson(jsonDecode(line) as Map<String, Object?>);
        }
        index = buffer.indexOf('\n');
      }
    }
  }

  @override
  Future<HostEnvironmentDescriptor> hostDescriptor() async {
    final nonce = 'flutter-${DateTime.now().microsecondsSinceEpoch}';
    final body = jsonEncode({'nonce': nonce});
    final response = await client.post(
      session.uri('/link/session'),
      headers: session.signedHeaders(body),
      body: body,
    );
    _throwIfRemoteError(response);

    final json = jsonDecode(response.body) as Map<String, Object?>;
    final coreDeviceId = json['coreDeviceId'] as String;
    final transports = (json['transports'] as List<Object?>).cast<String>();
    return HostEnvironmentDescriptor(
      id: 'remote:$coreDeviceId',
      displayName: 'Remote Operit Core',
      pathStyleDescriptionEn: 'Remote core path style',
      pathStyleDescriptionCn: '远程核心路径风格',
      examplePaths: const <String>[],
      usesEnvironmentParameter: false,
      environmentParameterDescriptionEn: '',
      environmentParameterDescriptionCn: '',
      capabilities: transports,
      fileSystemHost: true,
      webVisitHost: true,
      systemOperationHost: true,
      managedRuntimeHost: true,
      runtimeStorageHost: true,
      runtimeSqliteHost: true,
    );
  }

  void dispose() {
    client.close();
  }

  void _throwIfRemoteError(http.Response response) {
    if (response.statusCode >= 200 && response.statusCode < 300) {
      return;
    }
    _throwRemoteErrorBody(response.statusCode, response.body);
  }

  void _throwRemoteErrorBody(int statusCode, String body) {
    final decoded = jsonDecode(body);
    if (decoded is Map<String, Object?> &&
        decoded.containsKey('code') &&
        decoded.containsKey('message')) {
      throw CoreLinkError.fromJson(decoded);
    }
    throw CoreLinkError(
      code: 'REMOTE_HTTP_ERROR',
      message: 'remote core returned HTTP $statusCode',
    );
  }
}

class PairedRemoteSessionRecord {
  const PairedRemoteSessionRecord({
    required this.baseUrl,
    required this.sessionId,
    required this.deviceId,
    required this.sessionSecret,
  });

  factory PairedRemoteSessionRecord.fromJson(Map<String, Object?> json) {
    return PairedRemoteSessionRecord(
      baseUrl: json['baseUrl'] as String,
      sessionId: json['sessionId'] as String,
      deviceId: json['deviceId'] as String,
      sessionSecret: json['sessionSecret'] as String,
    );
  }

  final String baseUrl;
  final String sessionId;
  final String deviceId;
  final String sessionSecret;

  Uri uri(String path) {
    return Uri.parse('${baseUrl.replaceFirst(RegExp(r'/$'), '')}$path');
  }

  Map<String, String> signedHeaders(String body) {
    return {
      'content-type': 'application/json',
      'x-operit-session': sessionId,
      'x-operit-device': deviceId,
      'x-operit-signature': _sign(body),
    };
  }

  Map<String, Object?> toJson() {
    return {
      'baseUrl': baseUrl,
      'sessionId': sessionId,
      'deviceId': deviceId,
      'sessionSecret': sessionSecret,
    };
  }

  String _sign(String body) {
    final secret = base64Decode(sessionSecret);
    final hmac = Hmac(sha256, secret);
    return base64Encode(hmac.convert(utf8.encode(body)).bytes);
  }
}
