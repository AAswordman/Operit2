// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart' as http;

import '../host/HostEnvironmentDescriptor.dart';
import '../link/CoreLinkProtocol.dart';
import 'CoreProxy.dart';

class RemoteCoreProxy extends CoreProxy {
  RemoteCoreProxy({required this.session, http.Client? client})
    : client = client ?? http.Client() {
    _watchPool = _RemoteWatchChannelPool(session: session, client: this.client);
  }

  final PairedRemoteSessionRecord session;
  final http.Client client;
  late final _RemoteWatchChannelPool _watchPool;

  @override
  Future<Object?> call(CoreCallRequest request) async {
    final body = jsonEncode({'request': request.toJson()});
    final response = await client.post(
      session.uri('/link/call'),
      headers: session.signedHeaders(body),
      body: body,
    );
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
    final response = await client.post(
      session.uri('/link/watch/snapshot'),
      headers: session.signedHeaders(body),
      body: body,
    );
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
    _watchPool.dispose();
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

class _RemoteWatchChannelPool {
  _RemoteWatchChannelPool({required this.session, required this.client});

  static const int maxSubscriptionsPerChannel = 16;

  final PairedRemoteSessionRecord session;
  final http.Client client;
  final List<_RemoteWatchChannel> _channels = <_RemoteWatchChannel>[];

  Future<_RemoteWatchSubscription> open(CoreWatchRequest request) async {
    final channel = await _acquireChannel();
    final subscriptionId =
        'watch-${DateTime.now().microsecondsSinceEpoch}-${_channels.length}';
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
      channelId:
          'watch-channel-${DateTime.now().microsecondsSinceEpoch}-${_channels.length}',
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
            channel._fail(error, stackTrace);
          },
          onDone: () {
            final tail = buffer.trim();
            if (tail.isNotEmpty) {
              channel._dispatch(tail);
            }
            channel._closeAll();
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

  void _dispatch(String line) {
    final decoded = jsonDecode(line) as Map<String, Object?>;
    final subscriptionId = decoded['subscriptionId'] as String;
    final event = CoreEvent.fromJson(decoded['event'] as Map<String, Object?>);
    subscriptions[subscriptionId]?.add(event);
  }

  void _fail(Object error, StackTrace stackTrace) {
    for (final controller in subscriptions.values) {
      controller.addError(error, stackTrace);
    }
    _closeAll();
  }

  void _closeAll() {
    for (final controller in subscriptions.values) {
      controller.close();
    }
    subscriptions.clear();
    subscriptionCount = 0;
  }

  Future<void> dispose() {
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
      'sessionSecret': sessionSecret,
    };
  }

  String _sign(String body) {
    final secret = base64Decode(sessionSecret);
    final hmac = Hmac(sha256, secret);
    return base64Encode(hmac.convert(utf8.encode(body)).bytes);
  }
}
