// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart' show Response;
import 'package:http/http.dart' as http;
import 'package:web_socket_channel/web_socket_channel.dart';

import '../link/CoreLinkCodec.dart';
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
    _pushChannel = _RemotePushChannel(
      session: session,
      onConnectionIssue: (error) => _notifyConnectionIssue(error),
    );
  }

  final PairedRemoteSessionRecord session;
  final http.Client client;
  RemoteConnectionIssueCallback? _onConnectionIssue;
  late final _RemoteWatchChannelPool _watchPool;
  late final _RemotePushChannel _pushChannel;
  bool _disposed = false;

  void setConnectionIssueHandler(RemoteConnectionIssueCallback? handler) {
    _onConnectionIssue = handler;
  }

  void _notifyConnectionIssue(CoreLinkError error) {
    if (_disposed) return;
    _onConnectionIssue?.call(error);
  }

  Future<Response> _postRequest(String path, Uint8List body) async {
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
    final body = encodeCoreLink(<String, Object?>{'request': request.toJson()});
    final response = await _postRequest('/link/call', body);
    _throwIfRemoteError(response);

    final json = decodeCoreLinkMap(response.bodyBytes);
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

  /// Opens a client-owned stream on the remote Link push carrier.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    return _pushChannel.open(request);
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    final body = encodeCoreLink(<String, Object?>{'request': request.toJson()});
    final response = await _postRequest('/link/watch/snapshot', body);
    _throwIfRemoteError(response);
    return CoreEvent.fromJson(decodeCoreLinkMap(response.bodyBytes));
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

  Future<RemoteSessionInfo> sessionInfo() async {
    final nonce = 'flutter-${DateTime.now().microsecondsSinceEpoch}';
    final body = encodeCoreLink(<String, Object?>{'nonce': nonce});
    final response = await _postRequest('/link/session', body);
    _throwIfRemoteError(response);
    final info = RemoteSessionInfo.fromJson(
      decodeCoreLinkMap(response.bodyBytes),
    );
    if (info.protocolVersion != 3) {
      throw CoreLinkError(
        code: 'LINK_VERSION_MISMATCH',
        message:
            'remote Link protocol version is ${info.protocolVersion}, expected 3',
      );
    }
    return info;
  }

  void dispose() {
    _disposed = true;
    _watchPool.dispose();
    _pushChannel.dispose();
    client.close();
  }

  void _throwIfRemoteError(http.Response response) {
    if (response.statusCode >= 200 && response.statusCode < 300) {
      return;
    }
    if (response.statusCode == 401 || response.statusCode == 403) {
      _notifyConnectionIssue(
        _parseErrorBody(response.statusCode, response.bodyBytes),
      );
    }
    _throwRemoteErrorBody(response.statusCode, response.bodyBytes);
  }

  CoreLinkError _parseErrorBody(int statusCode, Uint8List body) {
    try {
      final decoded = decodeCoreLink(body);
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

  void _throwRemoteErrorBody(int statusCode, Uint8List body) {
    throw _parseErrorBody(statusCode, body);
  }
}

class _RemotePushChannel {
  /// Creates one persistent remote push carrier.
  _RemotePushChannel({required this.session, this.onConnectionIssue});

  final PairedRemoteSessionRecord session;
  final RemoteConnectionIssueCallback? onConnectionIssue;
  WebSocketChannel? _channel;
  StreamSubscription<Object?>? _subscription;

  /// Opens one logical push stream on the shared websocket.
  Future<CorePushSink> open(CorePushRequest request) async {
    final channel = await _ensureChannel();
    _send(channel, <String, Object?>{
      'type': 'PushOpen',
      'body': request.toJson(),
    });
    return _RemoteCorePushSink(channel: this, pushId: request.requestId);
  }

  /// Sends one signed protocol payload in websocket order.
  void send(Map<String, Object?> payload) {
    final channel = _channel;
    if (channel == null) {
      throw StateError('Link push carrier is not open');
    }
    _send(channel, payload);
  }

  /// Closes the shared push carrier.
  void dispose() {
    unawaited(_subscription?.cancel());
    unawaited(_channel?.sink.close());
    _subscription = null;
    _channel = null;
  }

  /// Creates and observes the shared websocket carrier.
  Future<WebSocketChannel> _ensureChannel() async {
    final current = _channel;
    if (current != null) {
      return current;
    }
    final channel = WebSocketChannel.connect(session.wsUri('/link/ws'));
    await channel.ready;
    _channel = channel;
    _subscription = channel.stream.listen(
      _handleResponse,
      onError: (Object error, StackTrace stackTrace) {
        onConnectionIssue?.call(
          CoreLinkError(code: 'REMOTE_PUSH_ERROR', message: error.toString()),
        );
      },
      onDone: () {
        onConnectionIssue?.call(
          const CoreLinkError(
            code: 'REMOTE_PUSH_CLOSED',
            message: 'remote Link push carrier closed',
          ),
        );
      },
    );
    return channel;
  }

  /// Reports structured server errors received on the push carrier.
  void _handleResponse(Object? frame) {
    final bytes = Uint8List.fromList((frame as List<int>));
    final response = decodeCoreLinkMap(bytes);
    if (response['type'] == 'Error') {
      onConnectionIssue?.call(
        CoreLinkError.fromJson(response['body'] as Map<String, Object?>),
      );
    }
  }

  /// Signs and writes one websocket payload.
  void _send(WebSocketChannel channel, Map<String, Object?> payload) {
    final payloadBytes = encodeCoreLink(payload);
    channel.sink.add(
      encodeCoreLink(<String, Object?>{
        'protocolVersion': 3,
        'sessionId': session.sessionId,
        'deviceId': session.deviceId,
        'signature': session.signature(payloadBytes),
        'payload': payload,
      }),
    );
  }
}

class _RemoteCorePushSink implements CorePushSink {
  /// Creates one logical stream on a remote push carrier.
  _RemoteCorePushSink({required this.channel, required this.pushId});

  final _RemotePushChannel channel;
  final String pushId;
  int _sequence = 0;
  bool _closed = false;

  /// Sends one ordered item without opening an HTTP request.
  @override
  Future<void> add(Object? args) {
    if (_closed) {
      throw StateError('Link push stream is closed');
    }
    channel.send(<String, Object?>{
      'type': 'PushItem',
      'body': <String, Object?>{
        'pushId': pushId,
        'sequence': _sequence++,
        'args': args,
      },
    });
    return Future<void>.value();
  }

  /// Completes this logical stream after its preceding items.
  @override
  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;
    channel.send(<String, Object?>{'type': 'PushClose', 'body': pushId});
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
    final body = encodeCoreLink(<String, Object?>{
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
      _throwRemoteErrorBody(response.statusCode, response.bodyBytes);
    }
    final decoded = decodeCoreLinkMap(response.bodyBytes);
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
    final body = encodeCoreLink(<String, Object?>{
      'channelId': subscription.channelId,
      'subscriptionId': subscription.subscriptionId,
    });
    final response = await client.post(
      session.uri('/link/watch/channel/close'),
      headers: session.signedHeaders(body),
      body: body,
    );
    if (response.statusCode < 200 || response.statusCode >= 300) {
      _throwRemoteErrorBody(response.statusCode, response.bodyBytes);
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
    required StreamSubscription<List<int>> eventSubscription,
  }) : _eventSubscription = eventSubscription;

  static Future<_RemoteWatchChannel> open({
    required PairedRemoteSessionRecord session,
    required http.Client client,
    RemoteConnectionIssueCallback? onConnectionIssue,
    required String channelId,
  }) async {
    final subscriptions = <String, StreamController<CoreEvent>>{};
    final body = encodeCoreLink(<String, Object?>{'channelId': channelId});
    final request =
        http.Request('POST', session.uri('/link/watch/channel/events'))
          ..headers.addAll(session.signedHeaders(body))
          ..bodyBytes = body;
    final response = await client.send(request);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      final bodyBytes = Uint8List.fromList(await response.stream.toBytes());
      _throwRemoteErrorBody(response.statusCode, bodyBytes);
    }
    late final _RemoteWatchChannel channel;
    final buffer = <int>[];
    final eventSubscription = response.stream.listen(
      (bytes) {
        buffer.addAll(bytes);
        while (buffer.length >= 4) {
          final frameLength = ByteData.sublistView(
            Uint8List.fromList(buffer.sublist(0, 4)),
          ).getUint32(0);
          if (buffer.length < 4 + frameLength) {
            break;
          }
          final frame = Uint8List.fromList(buffer.sublist(4, 4 + frameLength));
          buffer.removeRange(0, 4 + frameLength);
          channel._dispatch(frame);
        }
      },
      onError: (Object error, StackTrace stackTrace) {
        if (channel._closing) {
          return;
        }
        onConnectionIssue?.call(
          CoreLinkError(code: 'REMOTE_WATCH_ERROR', message: error.toString()),
        );
        channel._fail(error, stackTrace);
      },
      onDone: () {
        if (buffer.isNotEmpty) {
          channel._fail(
            const CoreLinkError(
              code: 'INVALID_FRAME',
              message: 'remote watch channel ended with an incomplete frame',
            ),
            StackTrace.current,
          );
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
  final StreamSubscription<List<int>> _eventSubscription;
  int subscriptionCount = 0;
  bool _closing = false;

  void _dispatch(Uint8List frame) {
    final decoded = decodeCoreLinkMap(frame);
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

void _throwRemoteErrorBody(int statusCode, Uint8List body) {
  final decoded = decodeCoreLink(body);
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

  /// Creates a copy of this paired session record with a new endpoint URL.
  PairedRemoteSessionRecord withBaseUrl(String value) {
    final normalizedBaseUrl = value.endsWith('/')
        ? value.substring(0, value.length - 1)
        : value;
    return PairedRemoteSessionRecord(
      baseUrl: normalizedBaseUrl,
      sessionId: sessionId,
      deviceId: deviceId,
      coreDeviceId: coreDeviceId,
      remoteDeviceInfo: remoteDeviceInfo,
      pairingServiceVersion: pairingServiceVersion,
      sessionSecret: sessionSecret,
    );
  }

  Uri uri(String path) {
    final normalizedBaseUrl = baseUrl.endsWith('/')
        ? baseUrl.substring(0, baseUrl.length - 1)
        : baseUrl;
    return Uri.parse('$normalizedBaseUrl$path');
  }

  /// Builds one websocket URI rooted at this remote session.
  Uri wsUri(String path) {
    final value = uri(path);
    final scheme = switch (value.scheme) {
      'http' => 'ws',
      'https' => 'wss',
      _ => throw StateError('unsupported remote websocket scheme'),
    };
    return value.replace(scheme: scheme);
  }

  Map<String, String> signedHeaders(Uint8List body) {
    return {
      'content-type': 'application/msgpack',
      'x-operit-link-version': '3',
      'x-operit-session': sessionId,
      'x-operit-device': deviceId,
      'x-operit-signature': signature(body),
    };
  }

  /// Signs one remote link message body with this session secret.
  String signature(Uint8List body) {
    return _sign(body);
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

  String _sign(Uint8List body) {
    final secret = base64Decode(sessionSecret);
    final hmac = Hmac(sha256, secret);
    return base64Encode(hmac.convert(body).bytes);
  }
}
