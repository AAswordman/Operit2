// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart' show Response;
import 'package:http/http.dart' as http;

import '../host/HostEnvironmentDescriptor.dart';
import '../link/CoreLinkProtocol.dart';
import '../bridge/CoreProxy.dart';

typedef RemoteConnectionIssueCallback = void Function(CoreLinkError error);

class RemoteRuntimeLinkClient extends CoreProxy {
  RemoteRuntimeLinkClient({
    required this.session,
    http.Client? client,
    RemoteConnectionIssueCallback? onConnectionIssue,
  }) : client = client ?? http.Client(),
       _onConnectionIssue = onConnectionIssue {
    _watchPool = _RemoteWatchChannelPool(
      session: session,
      client: this.client,
      onConnectionIssue: (error) => _notifyConnectionIssue(error),
    );
  }

  final PairedRemoteSessionRecord session;
  final http.Client client;
  RemoteConnectionIssueCallback? _onConnectionIssue;
  late final _RemoteWatchChannelPool _watchPool;
  bool _disposed = false;

  void setConnectionIssueHandler(RemoteConnectionIssueCallback? handler) {
    _onConnectionIssue = handler;
  }

  void _notifyConnectionIssue(CoreLinkError error) {
    if (_disposed) return;
    _onConnectionIssue?.call(error);
  }

  Future<Response> _postRequest(String path, String body) async {
    try {
      return await client.post(
        session.uri(path),
        headers: session.signedHeaders(body),
        body: body,
      );
    } catch (error) {
      _notifyConnectionIssue(
        CoreLinkError(code: 'REMOTE_UNREACHABLE', message: error.toString()),
      );
      rethrow;
    }
  }

  @override
  Future<Object?> call(CoreCallRequest request) async {
    final body = jsonEncode({'request': request.toJson()});
    final response = await _postRequest('/link/call', body);
    _throwIfRemoteError(response);

    final json = jsonDecode(response.body) as Map<String, Object?>;
    final result = json['result'] as Map<String, Object?>;
    if (result.containsKey('Ok')) {
      return result['Ok'];
    }
    if (result.containsKey('Err')) {
      final error = CoreLinkError.fromJson(
        result['Err'] as Map<String, Object?>,
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
    final response = await _postRequest('/link/watch/snapshot', body);
    _throwIfRemoteError(response);
    return CoreEvent.fromJson(
      jsonDecode(response.body) as Map<String, Object?>,
    );
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    final subscription = await _watchPool.open(request);
    try {
      await for (final event in subscription.events) {
        yield event;
      }
    } finally {
      await _watchPool.close(subscription);
    }
  }

  @override
  Future<HostEnvironmentDescriptor> hostDescriptor() async {
    final info = await sessionInfo();
    return HostEnvironmentDescriptor(
      id: 'remote:${info.coreDeviceId}',
      displayName: 'Remote ${info.coreDeviceInfo.displayName}',
      pathStyleDescriptionEn: 'Remote core path style',
      pathStyleDescriptionCn: '远程核心路径风格',
      examplePaths: const <String>[],
      usesEnvironmentParameter: false,
      environmentParameterDescriptionEn: '',
      environmentParameterDescriptionCn: '',
      capabilities: info.transports,
      fileSystemHost: true,
      webVisitHost: true,
      systemOperationHost: true,
      managedRuntimeHost: true,
      runtimeStorageHost: true,
      runtimeSqliteHost: true,
    );
  }

  Future<RemoteSessionInfo> sessionInfo() async {
    final nonce = 'flutter-${DateTime.now().microsecondsSinceEpoch}';
    final body = jsonEncode(<String, Object?>{'nonce': nonce});
    final response = await _postRequest('/link/session', body);
    _throwIfRemoteError(response);
    return RemoteSessionInfo.fromJson(
      jsonDecode(response.body) as Map<String, Object?>,
    );
  }

  Future<RemoteHostInteractionRequest?> pollHostInteraction({
    required int timeoutMs,
  }) async {
    final body = jsonEncode(<String, Object?>{'timeoutMs': timeoutMs});
    final response = await _postRequest('/host/interaction/poll', body);
    _throwIfRemoteError(response);
    final decoded = jsonDecode(response.body) as Map<String, Object?>;
    final request = decoded['request'];
    if (request == null) {
      return null;
    }
    return RemoteHostInteractionRequest.fromJson(
      request as Map<String, Object?>,
    );
  }

  Future<void> respondHostInteraction({
    required String requestId,
    required String result,
  }) async {
    final body = jsonEncode({
      'requestId': requestId,
      'response': <String, String>{'result': result},
    });
    final response = await _postRequest('/host/interaction/respond', body);
    _throwIfRemoteError(response);
  }

  void dispose() {
    _disposed = true;
    _watchPool.dispose();
    client.close();
  }

  void _throwIfRemoteError(http.Response response) {
    if (response.statusCode >= 200 && response.statusCode < 300) {
      return;
    }
    if (response.statusCode == 401 || response.statusCode == 403) {
      _notifyConnectionIssue(
        _parseErrorBody(response.statusCode, response.body),
      );
    }
    _throwRemoteErrorBody(response.statusCode, response.body);
  }

  CoreLinkError _parseErrorBody(int statusCode, String body) {
    try {
      final decoded = jsonDecode(body);
      if (decoded is Map<String, Object?> &&
          decoded.containsKey('code') &&
          decoded.containsKey('message')) {
        return CoreLinkError.fromJson(decoded);
      }
    } catch (_) {}
    return CoreLinkError(
      code: 'REMOTE_HTTP_ERROR',
      message: 'remote core returned HTTP $statusCode',
    );
  }

  void _throwRemoteErrorBody(int statusCode, String body) {
    throw _parseErrorBody(statusCode, body);
  }
}

class _RemoteWatchChannelPool {
  _RemoteWatchChannelPool({
    required this.session,
    required this.client,
    RemoteConnectionIssueCallback? onConnectionIssue,
  }) : _onConnectionIssue = onConnectionIssue;

  static const int maxSubscriptionsPerChannel = 16;

  final PairedRemoteSessionRecord session;
  final http.Client client;
  final RemoteConnectionIssueCallback? _onConnectionIssue;
  final List<_RemoteWatchChannel> _channels = <_RemoteWatchChannel>[];
  int _nextChannelId = 0;
  int _nextSubscriptionId = 0;

  Future<_RemoteWatchSubscription> open(CoreWatchRequest request) async {
    final channel = await _acquireChannel();
    final subscriptionId = 'watch-${_nextSubscriptionId++}';
    final controller = StreamController<CoreEvent>();
    channel.subscriptions[subscriptionId] = controller;
    channel.subscriptionCount += 1;
    final body = jsonEncode({
      'channelId': channel.channelId,
      'subscriptionId': subscriptionId,
      'request': request.toJson(),
    });
    final response = await client.post(
      session.uri('/link/watch/channel/open'),
      headers: session.signedHeaders(body),
      body: body,
    );
    if (response.statusCode < 200 || response.statusCode >= 300) {
      channel.subscriptions.remove(subscriptionId);
      channel.subscriptionCount -= 1;
      await controller.close();
      _throwRemoteErrorBody(response.statusCode, response.body);
    }
    final decoded = jsonDecode(response.body) as Map<String, Object?>;
    if (decoded['subscriptionId'] != subscriptionId) {
      channel.subscriptions.remove(subscriptionId);
      channel.subscriptionCount -= 1;
      await controller.close();
      throw const CoreLinkError(
        code: 'INVALID_RESPONSE',
        message: 'remote watch channel subscription id mismatch',
      );
    }
    return _RemoteWatchSubscription(
      channelId: channel.channelId,
      subscriptionId: subscriptionId,
      events: controller.stream,
    );
  }

  Future<void> close(_RemoteWatchSubscription subscription) async {
    final channel = _channel(subscription.channelId);
    final controller = channel.subscriptions.remove(
      subscription.subscriptionId,
    );
    channel.subscriptionCount -= 1;
    await controller?.close();
    final body = jsonEncode({
      'channelId': subscription.channelId,
      'subscriptionId': subscription.subscriptionId,
    });
    final response = await client.post(
      session.uri('/link/watch/channel/close'),
      headers: session.signedHeaders(body),
      body: body,
    );
    if (response.statusCode < 200 || response.statusCode >= 300) {
      _throwRemoteErrorBody(response.statusCode, response.body);
    }
    if (channel.subscriptionCount == 0) {
      await channel.dispose();
      _channels.remove(channel);
    }
  }

  void dispose() {
    for (final channel in List<_RemoteWatchChannel>.from(_channels)) {
      channel.dispose();
    }
    _channels.clear();
  }

  Future<_RemoteWatchChannel> _acquireChannel() async {
    for (final channel in _channels) {
      if (channel.subscriptionCount < maxSubscriptionsPerChannel) {
        return channel;
      }
    }
    final channel = await _RemoteWatchChannel.open(
      session: session,
      client: client,
      onConnectionIssue: _onConnectionIssue,
      channelId: 'watch-channel-${_nextChannelId++}',
    );
    _channels.add(channel);
    return channel;
  }

  _RemoteWatchChannel _channel(String channelId) {
    return _channels.firstWhere((channel) => channel.channelId == channelId);
  }
}

class _RemoteWatchChannel {
  _RemoteWatchChannel._({
    required this.channelId,
    required this.subscriptions,
    required StreamSubscription<String> eventSubscription,
  }) : _eventSubscription = eventSubscription;

  static Future<_RemoteWatchChannel> open({
    required PairedRemoteSessionRecord session,
    required http.Client client,
    RemoteConnectionIssueCallback? onConnectionIssue,
    required String channelId,
  }) async {
    final subscriptions = <String, StreamController<CoreEvent>>{};
    final body = jsonEncode({'channelId': channelId});
    final request =
        http.Request('POST', session.uri('/link/watch/channel/events'))
          ..headers.addAll(session.signedHeaders(body))
          ..body = body;
    final response = await client.send(request);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      final bodyText = await response.stream.bytesToString();
      _throwRemoteErrorBody(response.statusCode, bodyText);
    }
    late final _RemoteWatchChannel channel;
    var buffer = '';
    final eventSubscription = response.stream
        .transform(utf8.decoder)
        .listen(
          (text) {
            buffer += text;
            var index = buffer.indexOf('\n');
            while (index >= 0) {
              final line = buffer.substring(0, index).trim();
              buffer = buffer.substring(index + 1);
              if (line.isNotEmpty) {
                channel._dispatch(line);
              }
              index = buffer.indexOf('\n');
            }
          },
          onError: (Object error, StackTrace stackTrace) {
            if (channel._closing) {
              return;
            }
            onConnectionIssue?.call(
              CoreLinkError(
                code: 'REMOTE_WATCH_ERROR',
                message: error.toString(),
              ),
            );
            channel._fail(error, stackTrace);
          },
          onDone: () {
            final tail = buffer.trim();
            if (tail.isNotEmpty) {
              channel._dispatch(tail);
            }
            if (channel._closing) {
              channel._done();
              return;
            }
            channel._done();
            onConnectionIssue?.call(
              const CoreLinkError(
                code: 'REMOTE_WATCH_CLOSED',
                message: 'remote watch channel closed',
              ),
            );
          },
        );
    channel = _RemoteWatchChannel._(
      channelId: channelId,
      subscriptions: subscriptions,
      eventSubscription: eventSubscription,
    );
    return channel;
  }

  final String channelId;
  final Map<String, StreamController<CoreEvent>> subscriptions;
  final StreamSubscription<String> _eventSubscription;
  int subscriptionCount = 0;
  bool _closing = false;

  void _dispatch(String line) {
    final decoded = jsonDecode(line) as Map<String, Object?>;
    final subscriptionId = decoded['subscriptionId'] as String;
    final event = CoreEvent.fromJson(decoded['event'] as Map<String, Object?>);
    final controller = subscriptions[subscriptionId];
    controller?.add(event);
    if (event.kind == 'Completed') {
      unawaited(controller?.close());
    }
  }

  void _fail(Object error, StackTrace stackTrace) {
    for (final controller in subscriptions.values) {
      if (!controller.isClosed) {
        controller.addError(error, stackTrace);
      }
    }
    _closeAll();
  }

  void _done() {
    if (_closing) {
      _closeAll();
      return;
    }
    _fail(
      const CoreLinkError(
        code: 'REMOTE_WATCH_CLOSED',
        message: 'remote watch channel closed',
      ),
      StackTrace.current,
    );
  }

  void _closeAll() {
    for (final controller in subscriptions.values) {
      if (!controller.isClosed) {
        controller.close();
      }
    }
    subscriptions.clear();
    subscriptionCount = 0;
  }

  Future<void> dispose() {
    _closing = true;
    _closeAll();
    return _eventSubscription.cancel();
  }
}

class _RemoteWatchSubscription {
  const _RemoteWatchSubscription({
    required this.channelId,
    required this.subscriptionId,
    required this.events,
  });

  final String channelId;
  final String subscriptionId;
  final Stream<CoreEvent> events;
}

class RemoteHostInteractionRequest {
  const RemoteHostInteractionRequest({
    required this.requestId,
    required this.kind,
    required this.payload,
  });

  factory RemoteHostInteractionRequest.fromJson(Map<String, Object?> json) {
    return RemoteHostInteractionRequest(
      requestId: json['requestId'] as String,
      kind: json['kind'] as String,
      payload: json['payload'] as Map<String, Object?>,
    );
  }

  final String requestId;
  final String kind;
  final Map<String, Object?> payload;
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

class RemoteDeviceInfo {
  const RemoteDeviceInfo({required this.platform, required this.model});

  factory RemoteDeviceInfo.fromJson(Map<String, Object?> json) {
    return RemoteDeviceInfo(
      platform: json['platform'] as String,
      model: json['model'] as String,
    );
  }

  final String platform;
  final String model;

  String get displayName => '$platform-$model';

  Map<String, Object?> toJson() {
    return {'platform': platform, 'model': model};
  }
}

class RemoteSessionInfo {
  const RemoteSessionInfo({
    required this.protocolVersion,
    required this.pairingServiceVersion,
    required this.coreDeviceId,
    required this.coreDeviceInfo,
    required this.clientDeviceId,
    required this.clientDeviceInfo,
    required this.transports,
    required this.nonce,
  });

  factory RemoteSessionInfo.fromJson(Map<String, Object?> json) {
    return RemoteSessionInfo(
      protocolVersion: json['protocolVersion'] as int,
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      coreDeviceId: json['coreDeviceId'] as String,
      coreDeviceInfo: RemoteDeviceInfo.fromJson(
        json['coreDeviceInfo'] as Map<String, Object?>,
      ),
      clientDeviceId: json['clientDeviceId'] as String,
      clientDeviceInfo: RemoteDeviceInfo.fromJson(
        json['clientDeviceInfo'] as Map<String, Object?>,
      ),
      transports: (json['transports'] as List<Object?>).cast<String>(),
      nonce: json['nonce'] as String,
    );
  }

  final int protocolVersion;
  final int pairingServiceVersion;
  final String coreDeviceId;
  final RemoteDeviceInfo coreDeviceInfo;
  final String clientDeviceId;
  final RemoteDeviceInfo clientDeviceInfo;
  final List<String> transports;
  final String nonce;
}

class PairedRemoteSessionRecord {
  const PairedRemoteSessionRecord({
    required this.baseUrl,
    required this.sessionId,
    required this.deviceId,
    required this.coreDeviceId,
    required this.remoteDeviceInfo,
    required this.pairingServiceVersion,
    required this.sessionSecret,
  });

  factory PairedRemoteSessionRecord.fromJson(Map<String, Object?> json) {
    return PairedRemoteSessionRecord(
      baseUrl: json['baseUrl'] as String,
      sessionId: json['sessionId'] as String,
      deviceId: json['deviceId'] as String,
      coreDeviceId: json['coreDeviceId'] as String,
      remoteDeviceInfo: RemoteDeviceInfo.fromJson(
        json['remoteDeviceInfo'] as Map<String, Object?>,
      ),
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      sessionSecret: json['sessionSecret'] as String,
    );
  }

  final String baseUrl;
  final String sessionId;
  final String deviceId;
  final String coreDeviceId;
  final RemoteDeviceInfo remoteDeviceInfo;
  final int pairingServiceVersion;
  final String sessionSecret;

  Uri uri(String path) {
    final normalizedBaseUrl = baseUrl.endsWith('/')
        ? baseUrl.substring(0, baseUrl.length - 1)
        : baseUrl;
    return Uri.parse('$normalizedBaseUrl$path');
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
      'coreDeviceId': coreDeviceId,
      'remoteDeviceInfo': remoteDeviceInfo.toJson(),
      'pairingServiceVersion': pairingServiceVersion,
      'sessionSecret': sessionSecret,
    };
  }

  String _sign(String body) {
    final secret = base64Decode(sessionSecret);
    final hmac = Hmac(sha256, secret);
    return base64Encode(hmac.convert(utf8.encode(body)).bytes);
  }
}
