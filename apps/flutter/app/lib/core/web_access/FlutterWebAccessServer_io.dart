// ignore_for_file: file_names, unused_element

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:crypto/crypto.dart';
import 'package:flutter/services.dart';

import '../link/CoreLinkProtocol.dart';
import '../path/OperitClientPaths.dart';
import '../permissions/ToolApprovalBridgeNative.dart';
import '../permissions/ToolApprovalModels.dart';
import '../runtime/RuntimeConnectionManager.dart';
import 'WebAccessConfig.dart';

class FlutterWebAccessServer {
  FlutterWebAccessServer._();

  static final FlutterWebAccessServer instance = FlutterWebAccessServer._();
  static const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');

  bool _running = false;
  WebAccessConfig? _config;
  String? _shutdownToken;
  _LocalWebAccessSession? _localSession;
  final Map<String, _WatchChannel> _watchChannels = <String, _WatchChannel>{};

  bool get isRunning => _running;

  String? get baseUrl {
    final config = _config;
    if (config == null || !_running) {
      return null;
    }
    return _baseUrlForBindAddress(config.bindAddress);
  }

  Future<void> initializeFromConfig() async {
    final config = await WebAccessConfigStore.read();
    if (config.enabled) {
      await start(config);
    }
  }

  Future<void> start(WebAccessConfig config) async {
    if (_running) {
      await stop(updateConfig: false);
    }
    final webRoot = await _materializeWebAccessBundle();
    _config = config;
    _shutdownToken = WebAccessToken.generate();
    _localSession = RuntimeConnectionManager.instance.config.mode ==
            RuntimeConnectionMode.local
        ? _LocalWebAccessSession.generate()
        : null;
    await _startNativeWebAccessServer(config, _shutdownToken!, webRoot);
    _running = true;
    await _writeState(config);
  }

  Future<void> stop({bool updateConfig = true}) async {
    if (!_running) {
      return;
    }
    final shutdownToken = _shutdownToken;
    final baseUrl = this.baseUrl;
    if (shutdownToken != null && baseUrl != null) {
      await _requestNativeWebAccessClose(baseUrl, shutdownToken);
    }
    await _stopNativeWebAccessServer();
    _running = false;
    _config = null;
    _shutdownToken = null;
    _localSession = null;
    for (final channel in _watchChannels.values.toList(growable: false)) {
      await channel.close();
    }
    _watchChannels.clear();
    await _removeState();
    if (updateConfig) {
      final config = await WebAccessConfigStore.read();
      await WebAccessConfigStore.write(
        config.copyWith(
          enabled: false,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
      );
    }
  }

  Future<void> _serve(HttpServer server) async {
    await for (final request in server) {
      unawaited(_handle(request));
    }
  }

  Future<void> _handle(HttpRequest request) async {
    try {
      final path = request.uri.path;
      if (request.method == 'GET' && path == '/') {
        await _writeWebAsset(request.response, 'index.html');
        return;
      }
      if (request.method == 'POST' && path == '/client/web-access/close') {
        await _handleClose(request);
        return;
      }
      if (request.method == 'POST' && path == '/link/session') {
        await _handleLinkSession(request);
        return;
      }
      if (request.method == 'POST' && path == '/link/call') {
        await _handleLinkCall(request);
        return;
      }
      if (request.method == 'POST' && path == '/link/watch/snapshot') {
        await _handleLinkWatchSnapshot(request);
        return;
      }
      if (request.method == 'POST' && path == '/link/watch/channel/events') {
        await _handleWatchChannelEvents(request);
        return;
      }
      if (request.method == 'POST' && path == '/link/watch/channel/open') {
        await _handleWatchChannelOpen(request);
        return;
      }
      if (request.method == 'POST' && path == '/link/watch/channel/close') {
        await _handleWatchChannelClose(request);
        return;
      }
      if (request.method == 'POST' && path == '/host/interaction/poll') {
        await _handleHostInteractionPoll(request);
        return;
      }
      if (request.method == 'POST' && path == '/host/interaction/respond') {
        await _handleHostInteractionRespond(request);
        return;
      }
      if (request.method == 'GET') {
        await _writeWebAsset(request.response, _webAssetPath(request.uri));
        return;
      }
      await _writeJson(request.response, HttpStatus.notFound, {
        'code': 'NOT_FOUND',
        'message': 'web access route not found',
      });
    } on _UnauthorizedLinkRequest {
      await _writeCoreLinkError(
        request.response,
        HttpStatus.unauthorized,
        'UNAUTHORIZED',
        'invalid link session',
      );
    } on _WebAssetNotFound catch (error) {
      await _writeCoreLinkError(
        request.response,
        HttpStatus.notFound,
        'NOT_FOUND',
        error.message,
      );
    } catch (error) {
      await _writeJson(request.response, HttpStatus.internalServerError, {
        'code': 'INTERNAL_ERROR',
        'message': error.toString(),
      });
    }
  }

  Future<void> _handleLinkSession(HttpRequest request) async {
    final body = await _verifiedLocalLinkBody(request);
    final json = jsonDecode(body) as Map<String, Object?>;
    await _writeJson(request.response, HttpStatus.ok, {
      'protocolVersion': 1,
      'coreDeviceId': 'flutter-local-core',
      'clientDeviceId': _requiredLocalSession().deviceId,
      'transports': ['http'],
      'nonce': json['nonce'],
    });
  }

  Future<void> _handleLinkCall(HttpRequest request) async {
    final body = await _verifiedLocalLinkBody(request);
    final envelope = jsonDecode(body) as Map<String, Object?>;
    final callRequest = _coreCallRequestFromJson(
      envelope['request'] as Map<String, Object?>,
    );
    try {
      final result = await RuntimeConnectionManager.instance.coreProxy.call(
        callRequest,
      );
      await _writeJson(request.response, HttpStatus.ok, {
        'requestId': callRequest.requestId,
        'result': {'Ok': result},
      });
    } on CoreLinkError catch (error) {
      await _writeJson(request.response, HttpStatus.ok, {
        'requestId': callRequest.requestId,
        'result': {
          'Err': {'code': error.code, 'message': error.message},
        },
      });
    }
  }

  Future<void> _handleLinkWatchSnapshot(HttpRequest request) async {
    final body = await _verifiedLocalLinkBody(request);
    final envelope = jsonDecode(body) as Map<String, Object?>;
    final watchRequest = _coreWatchRequestFromJson(
      envelope['request'] as Map<String, Object?>,
    );
    final event = await RuntimeConnectionManager.instance.coreProxy
        .watchSnapshot(watchRequest);
    await _writeJson(request.response, HttpStatus.ok, event.toJson());
  }

  Future<void> _handleWatchChannelEvents(HttpRequest request) async {
    final body = await _verifiedLocalLinkBody(request);
    final envelope = jsonDecode(body) as Map<String, Object?>;
    final channelId = envelope['channelId'] as String;
    final previous = _watchChannels.remove(channelId);
    await previous?.close();
    request.response.statusCode = HttpStatus.ok;
    request.response.headers.contentType = ContentType.parse(
      'application/x-ndjson',
    );
    _watchChannels[channelId] = _WatchChannel(
      id: channelId,
      response: request.response,
    );
  }

  Future<void> _handleWatchChannelOpen(HttpRequest request) async {
    final body = await _verifiedLocalLinkBody(request);
    final envelope = jsonDecode(body) as Map<String, Object?>;
    final channelId = envelope['channelId'] as String;
    final subscriptionId = envelope['subscriptionId'] as String;
    final channel = _watchChannels[channelId];
    if (channel == null) {
      await _writeCoreLinkError(
        request.response,
        HttpStatus.badRequest,
        'WATCH_CHANNEL_NOT_FOUND',
        'watch channel not found',
      );
      return;
    }
    final watchRequest = _coreWatchRequestFromJson(
      envelope['request'] as Map<String, Object?>,
    );
    final subscription = RuntimeConnectionManager.instance.coreProxy
        .watchStream(watchRequest)
        .listen(
          (event) {
            unawaited(channel.write(subscriptionId, event));
          },
          onError: (Object error) {
            unawaited(channel.writeError(subscriptionId, error));
          },
        );
    channel.subscriptions[subscriptionId] = subscription;
    await _writeJson(request.response, HttpStatus.ok, {
      'subscriptionId': subscriptionId,
    });
  }

  Future<void> _handleWatchChannelClose(HttpRequest request) async {
    final body = await _verifiedLocalLinkBody(request);
    final envelope = jsonDecode(body) as Map<String, Object?>;
    final channelId = envelope['channelId'] as String;
    final subscriptionId = envelope['subscriptionId'] as String;
    final channel = _watchChannels[channelId];
    await channel?.closeSubscription(subscriptionId);
    if (channel != null && channel.subscriptions.isEmpty) {
      _watchChannels.remove(channelId);
      await channel.close();
    }
    await _writeJson(request.response, HttpStatus.ok, <String, Object?>{});
  }

  Future<void> _handleHostInteractionPoll(HttpRequest request) async {
    await _verifiedLocalLinkBody(request);
    final pending = await const ToolApprovalBridge().currentPermissionRequest();
    await _writeJson(request.response, HttpStatus.ok, {
      'request': pending == null
          ? null
          : {
              'requestId': 'flutter-tool-approval',
              'kind': 'tool_permission',
              'payload': {
                'tool': {
                  'name': pending.tool.name,
                  'parameters': pending.tool.parameters
                      .map(
                        (parameter) => {
                          'name': parameter.name,
                          'value': parameter.value,
                        },
                      )
                      .toList(growable: false),
                },
                'description': pending.description,
              },
            },
    });
  }

  Future<void> _handleHostInteractionRespond(HttpRequest request) async {
    final body = await _verifiedLocalLinkBody(request);
    final envelope = jsonDecode(body) as Map<String, Object?>;
    final response = envelope['response'] as Map<String, Object?>;
    final result = switch (response['result'] as String) {
      'allow' => ToolApprovalResult.allow,
      'always_allow' => ToolApprovalResult.alwaysAllow,
      'deny' => ToolApprovalResult.deny,
      final value => throw FormatException('invalid permission result: $value'),
    };
    await const ToolApprovalBridge().handlePermissionResult(result);
    await _writeJson(request.response, HttpStatus.ok, {'ok': true});
  }

  Future<String> _verifiedLocalLinkBody(HttpRequest request) async {
    final session = _requiredLocalSession();
    final body = await utf8.decoder.bind(request).join();
    final sessionId = request.headers.value('x-operit-session');
    final deviceId = request.headers.value('x-operit-device');
    final signature = request.headers.value('x-operit-signature');
    if (sessionId != session.sessionId ||
        deviceId != session.deviceId ||
        signature != session.sign(body)) {
      throw const _UnauthorizedLinkRequest();
    }
    return body;
  }

  _LocalWebAccessSession _requiredLocalSession() {
    final session = _localSession;
    if (session == null) {
      throw StateError('local web access session is not available');
    }
    return session;
  }

  Future<void> _handleClose(HttpRequest request) async {
    final shutdownToken = request.headers.value(
      'x-operit-web-access-shutdown-token',
    );
    if (shutdownToken != _shutdownToken) {
      await _writeUnauthorized(request.response);
      return;
    }
    await _writeJson(request.response, HttpStatus.ok, {'ok': true});
    scheduleMicrotask(() {
      unawaited(stop());
    });
  }

  Future<void> _writeState(WebAccessConfig config) async {
    final file = await OperitClientPaths.webAccessStateFile();
    await file.parent.create(recursive: true);
    final content = const JsonEncoder.withIndent('  ').convert({
      'bindAddress': config.bindAddress,
      'baseUrl': _baseUrlForBindAddress(config.bindAddress),
      'shutdownToken': _shutdownToken,
      'processId': pid,
      'startedAt': DateTime.now().millisecondsSinceEpoch,
    });
    await file.writeAsString(content);
  }

  Future<void> _removeState() async {
    final file = await OperitClientPaths.webAccessStateFile();
    if (await file.exists()) {
      await file.delete();
    }
  }

  Future<Directory> _materializeWebAccessBundle() async {
    final directory = await OperitClientPaths.webAccessBundleDir();
    final manifest = await AssetManifest.loadFromAssetBundle(rootBundle);
    final assetKeys = manifest
        .listAssets()
        .where((key) => key.startsWith('assets/web_access/'))
        .toList(growable: false)
      ..sort();
    for (final assetKey in assetKeys) {
      final relativePath = assetKey.substring('assets/web_access/'.length);
      final bytes = await rootBundle.load(assetKey);
      final file = File(_joinPath(<String>[
        directory.path,
        ...relativePath.split('/'),
      ]));
      await file.parent.create(recursive: true);
      await file.writeAsBytes(
        bytes.buffer.asUint8List(bytes.offsetInBytes, bytes.lengthInBytes),
      );
    }
    return directory;
  }

  Future<void> _startNativeWebAccessServer(
    WebAccessConfig config,
    String shutdownToken,
    Directory webRoot,
  ) async {
    final responseText = await _runtimeChannel.invokeMethod<String>(
      'startWebAccessServer',
      <String, Object?>{
        'bindAddress': config.bindAddress,
        'token': config.token,
        'shutdownToken': shutdownToken,
        'webRoot': webRoot.path,
      },
    );
    _throwNativeWebAccessError(responseText);
  }

  Future<void> _stopNativeWebAccessServer() async {
    final responseText = await _runtimeChannel.invokeMethod<String>(
      'stopWebAccessServer',
    );
    _throwNativeWebAccessError(responseText);
  }

  Future<void> _requestNativeWebAccessClose(
    String baseUrl,
    String shutdownToken,
  ) async {
    final client = HttpClient();
    try {
      final request = await client.postUrl(
        Uri.parse('$baseUrl/client/web-access/close'),
      );
      request.headers.set('x-operit-web-access-shutdown-token', shutdownToken);
      final response = await request.close();
      final body = await utf8.decoder.bind(response).join();
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw StateError('web access close failed: $body');
      }
    } finally {
      client.close(force: true);
    }
  }

  void _throwNativeWebAccessError(String? responseText) {
    if (responseText == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty web access response',
      );
    }
    final response = jsonDecode(responseText) as Map<String, Object?>;
    if (response['ok'] == true) {
      return;
    }
    if (response.containsKey('code') && response.containsKey('message')) {
      throw CoreLinkError.fromJson(response);
    }
    throw CoreLinkError(
      code: 'INVALID_RESPONSE',
      message: 'runtime bridge web access response is invalid',
    );
  }

  Future<void> _writeUnauthorized(HttpResponse response) {
    return _writeJson(response, HttpStatus.unauthorized, {
      'code': 'UNAUTHORIZED',
      'message': 'invalid web access token',
    });
  }

  Future<void> _writeCoreLinkError(
    HttpResponse response,
    int statusCode,
    String code,
    String message,
  ) {
    return _writeJson(response, statusCode, {'code': code, 'message': message});
  }

  Future<void> _writeJson(
    HttpResponse response,
    int statusCode,
    Object? value,
  ) async {
    response.statusCode = statusCode;
    response.headers.contentType = ContentType.json;
    response.write(jsonEncode(value));
    await response.close();
  }

  Future<void> _writeWebAsset(HttpResponse response, String assetPath) async {
    Uint8List bytes = await _readWebAsset(assetPath);
    if (assetPath == 'index.html') {
      bytes = utf8.encode(
        _injectRuntimeConnectionConfig(utf8.decode(bytes)),
      );
    }
    response.statusCode = HttpStatus.ok;
    response.headers.contentType = ContentType.parse(
      _contentTypeForPath(assetPath),
    );
    response.add(bytes);
    await response.close();
  }

  Future<Uint8List> _readWebAsset(String assetPath) async {
    if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
      final root = File(Platform.resolvedExecutable).parent.path;
      final path = _joinPath(<String>[
        root,
        'data',
        'flutter_assets',
        'assets',
        'web_access',
        ...assetPath.split('/'),
      ]);
      final file = File(path);
      final stat = await file.stat();
      if (stat.type != FileSystemEntityType.file) {
        throw _WebAssetNotFound('web asset not found: $assetPath');
      }
      return file.readAsBytes();
    }
    final data = await rootBundle.load('assets/web_access/$assetPath');
    return data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
  }
}

class _UnauthorizedLinkRequest implements Exception {
  const _UnauthorizedLinkRequest();
}

class _WebAssetNotFound implements Exception {
  const _WebAssetNotFound(this.message);

  final String message;
}

class _LocalWebAccessSession {
  const _LocalWebAccessSession({
    required this.sessionId,
    required this.deviceId,
    required this.sessionSecret,
  });

  factory _LocalWebAccessSession.generate() {
    final random = Random.secure();
    final secret = Uint8List.fromList(
      List<int>.generate(32, (_) => random.nextInt(256), growable: false),
    );
    return _LocalWebAccessSession(
      sessionId: 'flutter-web-access-${WebAccessToken.generate()}',
      deviceId: 'flutter-web-access-client-${WebAccessToken.generate()}',
      sessionSecret: base64Encode(secret),
    );
  }

  final String sessionId;
  final String deviceId;
  final String sessionSecret;

  String sign(String body) {
    final hmac = Hmac(sha256, base64Decode(sessionSecret));
    return base64Encode(hmac.convert(utf8.encode(body)).bytes);
  }
}

class _WatchChannel {
  _WatchChannel({required this.id, required this.response});

  final String id;
  final HttpResponse response;
  final Map<String, StreamSubscription<CoreEvent>> subscriptions =
      <String, StreamSubscription<CoreEvent>>{};
  bool closed = false;
  Future<void> _writeChain = Future<void>.value();

  Future<void> write(String subscriptionId, CoreEvent event) {
    if (closed) {
      return Future<void>.value();
    }
    _writeChain = _writeChain.then((_) async {
      if (closed) {
        return;
      }
      response.write(
        '${jsonEncode({'subscriptionId': subscriptionId, 'event': event.toJson()})}\n',
      );
      await response.flush();
    });
    return _writeChain;
  }

  Future<void> writeError(String subscriptionId, Object error) {
    return write(
      subscriptionId,
      CoreEvent(
        requestId: null,
        targetPath: const CoreObjectPath(<String>[]),
        propertyName: 'watch',
        kind: 'Completed',
        value: {
          'code': 'LINK_WATCH_CHANNEL_ERROR',
          'message': error.toString(),
        },
      ),
    );
  }

  Future<void> closeSubscription(String subscriptionId) async {
    final subscription = subscriptions.remove(subscriptionId);
    await subscription?.cancel();
  }

  Future<void> close() async {
    if (closed) {
      return;
    }
    closed = true;
    for (final subscription in subscriptions.values.toList(growable: false)) {
      await subscription.cancel();
    }
    subscriptions.clear();
    await _writeChain;
    await response.close();
  }
}

class _BindEndpoint {
  const _BindEndpoint({required this.host, required this.port});

  final String host;
  final int port;
}

_BindEndpoint _parseBindAddress(String bindAddress) {
  final index = bindAddress.lastIndexOf(':');
  if (index <= 0 || index == bindAddress.length - 1) {
    throw FormatException('invalid bind address: $bindAddress');
  }
  return _BindEndpoint(
    host: bindAddress.substring(0, index),
    port: int.parse(bindAddress.substring(index + 1)),
  );
}

String _baseUrlForBindAddress(String bindAddress) {
  final endpoint = _parseBindAddress(bindAddress);
  final host = switch (endpoint.host) {
    '0.0.0.0' => '127.0.0.1',
    '::' => '127.0.0.1',
    _ => endpoint.host,
  };
  return 'http://$host:${endpoint.port}';
}

String _webAssetPath(Uri uri) {
  final segments = uri.pathSegments;
  if (segments.isEmpty) {
    return 'index.html';
  }
  for (final segment in segments) {
    if (segment.isEmpty || segment == '.' || segment == '..') {
      throw FormatException('invalid web asset path: ${uri.path}');
    }
    if (segment.runes.any((rune) => rune == 92)) {
      throw FormatException('invalid web asset path: ${uri.path}');
    }
  }
  return segments.join('/');
}

String _injectRuntimeConnectionConfig(String html) {
  final runtimeConfig = RuntimeConnectionManager.instance.config;
  final config = switch (runtimeConfig.mode) {
    RuntimeConnectionMode.local => _localRuntimeConnectionConfig(),
    RuntimeConnectionMode.remote => _remoteRuntimeConnectionConfig(runtimeConfig),
  };
  return html.replaceFirst(
    '<script src="operit_runtime_bridge.js"></script>',
    '<script>window.__OPERIT_WEB_ACCESS__ = $config;</script>\n  <script src="operit_runtime_bridge.js"></script>',
  );
}

String _localRuntimeConnectionConfig() {
  final session = FlutterWebAccessServer.instance._requiredLocalSession();
  return jsonEncode({
    'mode': 'link',
    'baseUrl': '',
    'sessionId': session.sessionId,
    'deviceId': session.deviceId,
    'sessionSecret': session.sessionSecret,
  });
}

String _remoteRuntimeConnectionConfig(RuntimeConnectionConfig runtimeConfig) {
  final session = runtimeConfig.remoteSession;
  if (session == null) {
    throw StateError('remote runtime session is required');
  }
  return jsonEncode({
    'mode': 'link',
    'baseUrl': session.baseUrl,
    'sessionId': session.sessionId,
    'deviceId': session.deviceId,
    'sessionSecret': session.sessionSecret,
  });
}

CoreCallRequest _coreCallRequestFromJson(Map<String, Object?> json) {
  return CoreCallRequest(
    requestId: json['requestId'] as String,
    targetPath: _coreObjectPathFromJson(
      json['targetPath'] as Map<String, Object?>,
    ),
    methodName: json['methodName'] as String,
    args: json['args'],
  );
}

CoreWatchRequest _coreWatchRequestFromJson(Map<String, Object?> json) {
  return CoreWatchRequest(
    requestId: json['requestId'] as String,
    targetPath: _coreObjectPathFromJson(
      json['targetPath'] as Map<String, Object?>,
    ),
    propertyName: json['propertyName'] as String,
    args: json['args'],
  );
}

CoreObjectPath _coreObjectPathFromJson(Map<String, Object?> json) {
  return CoreObjectPath((json['segments'] as List<Object?>).cast<String>());
}

String _joinPath(List<String> segments) {
  return segments.join(Platform.pathSeparator);
}

String _contentTypeForPath(String path) {
  final extension = path.split('.').last.toLowerCase();
  return switch (extension) {
    'html' => 'text/html; charset=utf-8',
    'js' => 'application/javascript; charset=utf-8',
    'css' => 'text/css; charset=utf-8',
    'json' => 'application/json; charset=utf-8',
    'wasm' => 'application/wasm',
    'png' => 'image/png',
    'jpg' => 'image/jpeg',
    'jpeg' => 'image/jpeg',
    'svg' => 'image/svg+xml',
    'ico' => 'image/x-icon',
    'woff' => 'font/woff',
    'woff2' => 'font/woff2',
    _ => 'application/octet-stream',
  };
}
