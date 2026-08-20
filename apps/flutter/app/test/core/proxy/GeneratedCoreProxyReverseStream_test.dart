// ignore_for_file: file_names

import 'package:flutter_test/flutter_test.dart';
import 'dart:typed_data';

import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';

/// Verifies generated reverse-stream clients serialize structured item values.
void main() {
  test('browser interaction stream sends a Link map', () async {
    final bridge = _RecordingBridge();
    const command = RuntimeBrowserCommand(
      action: 'interact',
      sessionId: 'session-1',
      url: null,
      script: null,
      payloadJson: '{"type":"pointer"}',
      userAgent: null,
      headers: <String, String>{},
    );
    final client = GeneratedServicesRuntimeBrowserServiceCoreProxy(
      bridge,
      const CoreObjectPath(<String>['services', 'runtimeBrowserService']),
    );

    await client.submitBrowserInteractions(
      commands: Stream<RuntimeBrowserCommand>.value(command),
    );

    expect(bridge.pushRequest?.methodName, 'submitBrowserInteractions');
    expect(bridge.sink.items, <Object?>[command.toJson()]);
    expect(bridge.sink.closed, isTrue);
  });
}

/// Records the values a generated Core client submits to its Link bridge.
class _RecordingBridge extends OperitRuntimeBridge {
  final _RecordingPushSink sink = _RecordingPushSink();
  CorePushRequest? pushRequest;

  /// Rejects encoded calls because this test only exercises reverse streams.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    throw UnimplementedError();
  }

  /// Rejects direct calls because this test only exercises reverse streams.
  @override
  Future<Object?> call(CoreCallRequest request) {
    throw UnimplementedError();
  }

  /// Captures the generated stream open request and returns its recording sink.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    pushRequest = request;
    return sink;
  }

  /// Rejects embedded streams because this test only exercises reverse streams.
  @override
  Stream<T> openEmbeddedCoreStream<T>(
    String streamId,
    CoreObjectPath targetPath,
    String propertyName,
    Object? args,
    T Function(CoreLinkValueReader reader) decode,
  ) {
    throw UnimplementedError();
  }

  /// Rejects snapshots because this test only exercises reverse streams.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw UnimplementedError();
  }

  /// Rejects watch streams because this test only exercises reverse streams.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    throw UnimplementedError();
  }
}

/// Records ordered Link values produced by a generated reverse-stream client.
class _RecordingPushSink implements CorePushSink {
  final List<Object?> items = <Object?>[];
  var closed = false;

  /// Records one submitted Link value.
  @override
  Future<void> add(Object? args) async {
    items.add(args);
  }

  /// Records completion of the reverse stream.
  @override
  Future<void> close() async {
    closed = true;
  }
}
