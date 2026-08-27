// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';

import 'package:operit2/core/bridge/CoreProxy.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
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
    final client = GeneratedCoreProxyClients(
      bridge,
    ).servicesRuntimeBrowserService;

    await client.submitBrowserInteractions(
      commands: Stream<RuntimeBrowserCommand>.value(command),
    );

    expect(bridge.pushRequest?.methodName, 'submitBrowserInteractions');
    expect(bridge.sink.items, <Object?>[command.toJson()]);
    expect(bridge.sink.closed, isTrue);
  });

  test('chatMessagesFlow reconstructs delta stream descriptors', () async {
    final bridge = _GeneratedFlowBridge();
    final client = GeneratedCoreProxyClients(bridge).chatRuntimeHolderMain;

    final snapshots = await client
        .chatMessagesFlow(chatId: 'chat-1')
        .take(2)
        .toList();

    expect(bridge.watchRequest?.propertyName, 'chatMessagesFlow');
    expect(snapshots.first.single.contentStream, isNull);
    final contentStream = snapshots.last.single.contentStream;
    expect(contentStream, isNotNull);
    expect(await contentStream!.first.then((event) => event.value), 'hello');
    expect(bridge.openedStreamIds, <String>['stream-ai']);
  });

  test(
    'chatMessagesFlow preserves embedded stream identity by stream id',
    () async {
      final proxy = _GeneratedFlowCoreProxy();
      final client = GeneratedCoreProxyClients(
        ProxyCoreRuntimeBridge(coreProxy: proxy),
      ).chatRuntimeHolderMain;

      final snapshots = await client
          .chatMessagesFlow(chatId: 'chat-1')
          .take(2)
          .toList();

      final firstStream = snapshots.first.single.contentStream;
      final secondStream = snapshots.last.single.contentStream;
      expect(firstStream, isNotNull);
      expect(identical(firstStream, secondStream), isTrue);
      expect(await firstStream!.first.then((event) => event.value), 'hello');
      expect(proxy.embeddedWatchOpenCount, 1);
    },
  );
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
    int targetObjectId,
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

/// Emits generated watch events that mimic a streamed AI message update.
class _GeneratedFlowBridge extends OperitRuntimeBridge {
  CoreWatchRequest? watchRequest;
  final openedStreamIds = <String>[];

  /// Rejects encoded calls because this test only exercises generated watches.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    throw UnimplementedError();
  }

  /// Rejects direct calls because this test only exercises generated watches.
  @override
  Future<Object?> call(CoreCallRequest request) {
    throw UnimplementedError();
  }

  /// Rejects client-owned streams because this test only exercises generated watches.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    throw UnimplementedError();
  }

  /// Opens one deterministic embedded Markdown stream.
  @override
  Stream<T> openEmbeddedCoreStream<T>(
    String streamId,
    int targetObjectId,
    String propertyName,
    Object? args,
    T Function(CoreLinkValueReader reader) decode,
  ) {
    openedStreamIds.add(streamId);
    return Stream<T>.value(
      const MarkdownStreamEvent(
            chatId: 'chat-1',
            eventType: 'text',
            value: 'hello',
            id: null,
            blockId: null,
            inlineId: null,
            parentBlockId: null,
            nodeType: null,
            headerLevel: null,
          )
          as T,
    );
  }

  /// Rejects snapshots because this test only exercises stream watches.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw UnimplementedError();
  }

  /// Emits a full chat snapshot followed by an incremental stream descriptor.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    watchRequest = request;
    yield _rawGeneratedEvent(request, 'Snapshot', <Object?>[
      _chatMessageValue(contentStream: null),
    ]);
    yield _rawGeneratedEvent(request, 'Delta', <String, Object?>{
      r'$coreDelta': <Object?>[
        <String, Object?>{
          'op': 'set',
          'path': <Object?>[0, 'contentStream'],
          'value': _streamDescriptorValue('stream-ai'),
        },
      ],
    });
  }
}

/// Emits generated watch events through the production ProxyCoreRuntimeBridge path.
class _GeneratedFlowCoreProxy extends CoreProxy {
  CoreWatchRequest? outerWatchRequest;
  var embeddedWatchOpenCount = 0;

  /// Rejects storage default reads because this test only exercises watch streams.
  @override
  Future<Map<Object?, Object?>> runtimeStorageDefaults() {
    throw UnimplementedError();
  }

  /// Rejects storage path normalization because this test only exercises watch streams.
  @override
  Future<Map<Object?, Object?>> runtimeStoragePaths(
    String runtimeRoot,
    String workspaceRoot,
  ) {
    throw UnimplementedError();
  }

  /// Rejects storage root writes because this test only exercises watch streams.
  @override
  Future<void> setRuntimeStorageRoots(
    String runtimeRoot,
    String workspaceRoot,
  ) {
    throw UnimplementedError();
  }

  /// Rejects bootstrap reads because this test only exercises watch streams.
  @override
  Future<String?> runtimeBootstrapRead() {
    throw UnimplementedError();
  }

  /// Rejects bootstrap writes because this test only exercises watch streams.
  @override
  Future<void> runtimeBootstrapWrite(String content) {
    throw UnimplementedError();
  }

  /// Rejects application restarts because this test only exercises watch streams.
  @override
  Future<void> restartApplication() {
    throw UnimplementedError();
  }

  /// Rejects encoded calls because this test only exercises generated watches.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    throw UnimplementedError();
  }

  /// Rejects client-owned streams because this test only exercises generated watches.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    throw UnimplementedError();
  }

  /// Rejects snapshots because this test only exercises stream watches.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw UnimplementedError();
  }

  /// Emits chat flow updates and embedded markdown stream events.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    if (request.propertyName == 'chatMessagesFlow') {
      outerWatchRequest = request;
      final descriptor = _streamDescriptorValue('stream-ai');
      yield _rawGeneratedEvent(request, 'Snapshot', <Object?>[
        _chatMessageValue(contentStream: descriptor),
      ]);
      yield _rawGeneratedEvent(request, 'Delta', <String, Object?>{
        r'$coreDelta': <Object?>[
          <String, Object?>{
            'op': 'set',
            'path': <Object?>[0, 'contentStream'],
            'value': descriptor,
          },
        ],
      });
      return;
    }
    if (request.propertyName == 'openCoreStream') {
      embeddedWatchOpenCount += 1;
      yield _rawGeneratedEvent(
        request,
        'Snapshot',
        _markdownStreamEventValue('hello'),
      );
      yield _rawGeneratedEvent(request, 'Completed', null);
      return;
    }
    throw StateError('unexpected core watch: ${request.propertyName}');
  }
}

/// Creates one raw watch event for the generated proxy test bridge.
CoreEvent _rawGeneratedEvent(
  CoreWatchRequest request,
  String kind,
  Object? value,
) {
  return CoreEvent.raw(
    requestId: request.requestId,
    targetObjectId: request.targetObjectId,
    propertyName: request.propertyName,
    kind: kind,
    valueBytes: encodeCoreLink(value),
    decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
  );
}

/// Builds the minimal complete ChatMessage Link map required by generated decoding.
Map<String, Object?> _chatMessageValue({required Object? contentStream}) {
  return <String, Object?>{
    'sender': 'ai',
    'parts': <Object?>[],
    'timestamp': 1,
    'roleName': '',
    'selectedVariantIndex': 0,
    'variantCount': 1,
    'provider': '',
    'modelName': '',
    'inputTokens': 0,
    'outputTokens': 0,
    'cachedInputTokens': 0,
    'sentAt': 0,
    'outputDurationMs': 0,
    'waitDurationMs': 0,
    'completedAt': 0,
    'displayMode': 'NORMAL',
    'isFavorite': false,
    'contentStream': contentStream,
  };
}

/// Builds one embedded stream descriptor in the Link wire shape.
Map<String, Object?> _streamDescriptorValue(String streamId) {
  return <String, Object?>{
    r'$coreStream': <String, Object?>{
      'streamId': streamId,
      'targetObjectId': 64,
      'propertyName': 'openCoreStream',
      'args': <String, Object?>{'streamId': streamId},
    },
  };
}

/// Builds the minimal MarkdownStreamEvent Link map required by embedded decoding.
Map<String, Object?> _markdownStreamEventValue(String value) {
  return const MarkdownStreamEvent(
    chatId: 'chat-1',
    eventType: 'text',
    value: null,
    id: null,
    blockId: null,
    inlineId: null,
    parentBlockId: null,
    nodeType: null,
    headerLevel: null,
  ).toJson()..['value'] = value;
}
