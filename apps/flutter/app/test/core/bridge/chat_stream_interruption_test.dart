import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';
import 'package:operit2/ui/features/chat/components/part/StructuredMessagePartRenderer.dart';

enum _Interruption { transportError, eofWithoutCompleted, malformedFrame }

void main() {
  for (final interruption in _Interruption.values) {
    test(
      'live chat resumes on its next descriptor after ${interruption.name}, without cancelling AI',
      () async {
        final bridge = _LiveChatBridge();
        final observer = _ChatObserver(bridge)..start();
        addTearDown(() async {
          await observer.dispose();
          await bridge.dispose();
        });
        await _drainEvents();
        bridge.produce('before');
        await _drainEvents();
        expect(observer.text, 'before');
        expect(bridge.messageWatchOpenCount, 1);

        bridge.interrupt(interruption);
        await _drainEvents();
        // The independent producer and state watch are not cancelled by a
        // failure of the message watch. Its next persisted message descriptor
        // must reopen the physical watch and replay a fresh snapshot.
        bridge.produce(' after');
        await _drainEvents();
        expect(bridge.producerTicks, 2);
        expect(observer.stateProgress, [1, 2]);
        expect(bridge.cancelCalls, 0);
        expect(bridge.backendRunning, isTrue);
        expect(observer.text, 'before after');
        expect(observer.outputDone, isFalse);
        expect(bridge.messageWatchOpenCount, 2);
        expect(observer.errors, hasLength(1));

        bridge.produce(' still running');
        await _drainEvents();
        expect(observer.text, 'before after still running');
        expect(bridge.messageWatchOpenCount, 2);
      },
    );

    for (final immediateDescriptor in [false, true]) {
      testWidgets(
        'chat surface resumes after ${interruption.name} on a ${immediateDescriptor ? 'immediate' : 'later'} message delta without duplicate text',
        (tester) async {
          final bridge = _LiveChatBridge(incrementalDescriptors: true);
          final rendererState = StreamMarkdownRendererState();
          final stateProgress = <int>[];
          final states = bridge
              .watchStream(
                const CoreWatchRequest(
                  requestId: 'widget-state',
                  target: r'$core.internal',
                  propertyName: 'chatStateFlow',
                  args: {'chatId': 'chat-1'},
                ),
              )
              .listen((event) {
                stateProgress.add((event.value as Map)['progress'] as int);
              });
          addTearDown(() async {
            await states.cancel();
            await bridge.dispose();
          });
          final messages = GeneratedCoreProxyClients(
            bridge,
          ).chatRuntimeHolderMain.chatMessagesFlow(chatId: 'chat-1');
          await tester.pumpWidget(
            MaterialApp(
              home: Scaffold(
                body: StreamBuilder<List<ChatMessage>>(
                  stream: messages,
                  builder: (context, snapshot) {
                    if (!snapshot.hasData) return const SizedBox();
                    return StreamingStructuredMessageRenderer(
                      parts: snapshot.data!.single.parts,
                      showThinkingProcess: true,
                      nodeGrouper: const NoopMarkdownNodeGrouper(),
                      textColor: Colors.black,
                      backgroundColor: Colors.white,
                      contentStream: snapshot.data!.single.contentStream,
                      rendererId: 'chat-1:1',
                      streamState: rendererState,
                      splitMarkdownContent: (_) async =>
                          <MarkdownStreamEvent>[],
                    );
                  },
                ),
              ),
            ),
          );
          await _pumpOutput(tester);
          bridge.produce('before');
          await _pumpOutput(tester);
          expect(rendererState.collectedContent.toString(), 'before');
          expect(
            find.textContaining('before', findRichText: true),
            findsWidgets,
          );

          bridge.interrupt(interruption);
          if (!immediateDescriptor) await _pumpOutput(tester);
          bridge.produce(' after');
          await _pumpOutput(tester);
          // Use actual generated delta decoding plus the production renderer,
          // rather than forcing a new stream into the widget by hand.
          expect(bridge.producerTicks, 2);
          expect(stateProgress, [1, 2]);
          expect(bridge.backendRunning, isTrue);
          expect(bridge.cancelCalls, 0);
          expect(rendererState.collectedContent.toString(), 'before after');
          expect(rendererState.nodes, hasLength(1));
          expect(rendererState.nodes.single.content.toString(), 'before after');
          expect(
            find.textContaining('before after', findRichText: true),
            findsWidgets,
          );
          expect(bridge.messageWatchOpenCount, 2);
          expect(
            tester
                .widget<StreamMarkdownRenderer>(
                  find.byType(StreamMarkdownRenderer),
                )
                .isStreaming,
            isTrue,
          );

          bridge.produce(' still running');
          await _pumpOutput(tester);
          expect(
            rendererState.collectedContent.toString(),
            'before after still running',
          );
          expect(bridge.messageWatchOpenCount, 2);
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox());
        },
      );
    }
  }

  test(
    'normal Completed stays cached and never restarts the producer',
    () async {
      final bridge = _LiveChatBridge();
      final observer = _ChatObserver(bridge)..start();
      addTearDown(() async {
        await observer.dispose();
        await bridge.dispose();
      });
      await _drainEvents();
      bridge.produce('finished');
      await _drainEvents();
      bridge.complete();
      await _drainEvents();
      final completedStream = observer.currentStream;

      bridge.publishMessageDescriptor();
      await _drainEvents();
      expect(identical(observer.currentStream, completedStream), isTrue);
      expect(observer.text, 'finished');
      expect(observer.outputDone, isTrue);
      expect(observer.errors, isEmpty);
      expect(bridge.messageWatchOpenCount, 1);
      expect(bridge.cancelCalls, 0);
    },
  );

  test('an idle live stream is not treated as disconnected', () async {
    final bridge = _LiveChatBridge();
    final observer = _ChatObserver(bridge)..start();
    addTearDown(() async {
      await observer.dispose();
      await bridge.dispose();
    });
    await _drainEvents();
    final stream = observer.currentStream;
    bridge.publishMessageDescriptor();
    await _drainEvents();
    expect(identical(observer.currentStream, stream), isTrue);
    expect(observer.outputDone, isFalse);
    expect(observer.errors, isEmpty);
    expect(bridge.messageWatchOpenCount, 1);
    expect(bridge.cancelCalls, 0);
  });
}

Future<void> _pumpOutput(WidgetTester tester) async {
  for (var i = 0; i < 4; i += 1) {
    await tester.pump(const Duration(milliseconds: 250));
  }
}

Future<void> _drainEvents() async {
  for (var i = 0; i < 8; i += 1) {
    await Future<void>.delayed(Duration.zero);
  }
}

/// Exercises the real generated chat-flow decoder and embedded stream cache.
/// The observer switches subscriptions only when the descriptor gives it a new
/// stream instance, matching the streaming renderer's didUpdateWidget behavior.
class _ChatObserver {
  _ChatObserver(this.bridge);

  final _LiveChatBridge bridge;
  StreamSubscription<List<ChatMessage>>? _messages;
  StreamSubscription<CoreEvent>? _states;
  StreamSubscription<MarkdownStreamEvent>? _output;
  Stream<MarkdownStreamEvent>? currentStream;
  final List<Object> errors = [];
  final List<int> stateProgress = [];
  String text = '';
  bool outputDone = false;

  void start() {
    _messages = GeneratedCoreProxyClients(bridge).chatRuntimeHolderMain
        .chatMessagesFlow(chatId: 'chat-1')
        .listen((messages) {
          final stream = messages.single.contentStream!;
          if (identical(currentStream, stream)) return;
          unawaited(_output?.cancel());
          currentStream = stream;
          outputDone = false;
          _output = stream.listen(
            (event) {
              if (event.eventType == 'reset') text = '';
              if (event.eventType == 'chunk') text += event.value ?? '';
            },
            onError: (Object error, StackTrace _) => errors.add(error),
            onDone: () {
              if (identical(currentStream, stream)) outputDone = true;
            },
          );
        });
    _states = bridge
        .watchStream(
          const CoreWatchRequest(
            requestId: 'test-state',
            target: r'$core.internal',
            propertyName: 'chatStateFlow',
            args: {'chatId': 'chat-1'},
          ),
        )
        .listen((event) {
          stateProgress.add((event.value as Map)['progress'] as int);
        });
  }

  Future<void> dispose() async {
    await _messages?.cancel();
    await _states?.cancel();
    await _output?.cancel();
  }
}

/// A deterministic backend with independently owned producer and watch channels.
/// Fault injection closes/corrupts only the message watch, never the producer.
class _LiveChatBridge extends OperitRuntimeBridge {
  _LiveChatBridge({this.incrementalDescriptors = false});

  final bool incrementalDescriptors;
  bool _descriptorPublished = false;
  static const streamId = 'chat-message-stream:chat-1:1';
  final _messages = StreamController<CoreEvent>();
  final _states = StreamController<CoreEvent>();
  final List<StreamController<CoreEvent>> _outputs = [];
  CoreWatchRequest? _messagesRequest;
  CoreWatchRequest? _stateRequest;
  CoreWatchRequest? _outputRequest;
  int producerTicks = 0;
  int messageWatchOpenCount = 0;
  int cancelCalls = 0;
  bool backendRunning = true;
  String _content = '';

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    switch (request.propertyName) {
      case 'chatMessagesFlow':
        _messagesRequest = request;
        publishMessageDescriptor();
        return _messages.stream;
      case 'chatStateFlow':
        _stateRequest = request;
        return _states.stream;
      case 'openCoreStream':
        messageWatchOpenCount += 1;
        _outputRequest = request;
        final controller = StreamController<CoreEvent>();
        _outputs.add(controller);
        controller.add(_event(request, 'Changed', _markdown('reset')));
        controller.add(
          _event(request, 'Changed', _markdown('markdownBlockStart', null, 1)),
        );
        if (_content.isNotEmpty) {
          controller.add(
            _event(request, 'Changed', _markdown('chunk', _content)),
          );
          controller.add(
            _event(
              request,
              'Changed',
              _markdown('markdownBlockChunk', _content, 1),
            ),
          );
        }
        return controller.stream;
      default:
        throw StateError('Unexpected watch ${request.propertyName}');
    }
  }

  void produce(String chunk) {
    if (!backendRunning) throw StateError('Producer has finished');
    producerTicks += 1;
    _content += chunk;
    if (_outputs.isNotEmpty && !_outputs.last.isClosed) {
      _outputs.last.add(
        _event(_outputRequest!, 'Changed', _markdown('chunk', chunk)),
      );
      _outputs.last.add(
        _event(
          _outputRequest!,
          'Changed',
          _markdown('markdownBlockChunk', chunk, 1),
        ),
      );
    }
    _states.add(
      _event(_stateRequest!, 'Changed', {
        'isLoading': true,
        'progress': producerTicks,
      }),
    );
    publishMessageDescriptor();
  }

  void interrupt(_Interruption interruption) {
    switch (interruption) {
      case _Interruption.transportError:
        _outputs.last.addError(
          StateError('Injected message-watch interruption'),
        );
      case _Interruption.eofWithoutCompleted:
        unawaited(_outputs.last.close());
      case _Interruption.malformedFrame:
        _outputs.last.add(
          _event(_outputRequest!, 'Changed', {'invalid': true}),
        );
    }
  }

  void complete() {
    backendRunning = false;
    _outputs.last.add(_event(_outputRequest!, 'Completed', null));
    unawaited(_outputs.last.close());
  }

  void publishMessageDescriptor() {
    if (incrementalDescriptors && _descriptorPublished) {
      _messages.add(
        _event(_messagesRequest!, 'Delta', {
          r'$coreDelta': [
            {
              'op': 'set',
              'path': [0, 'outputTokens'],
              'value': producerTicks,
            },
          ],
        }),
      );
      return;
    }
    _descriptorPublished = true;
    _messages.add(
      _event(_messagesRequest!, 'Snapshot', [
        {
          'sender': 'ai',
          'parts': [],
          'timestamp': 1,
          'roleName': '',
          'selectedVariantIndex': 0,
          'variantCount': 1,
          'provider': '',
          'modelName': '',
          'inputTokens': 0,
          'outputTokens': producerTicks,
          'cachedInputTokens': 0,
          'sentAt': 0,
          'outputDurationMs': 0,
          'waitDurationMs': 0,
          'completedAt': backendRunning ? 0 : 1,
          'displayMode': 'NORMAL',
          'isFavorite': false,
          'contentStream': {
            r'$coreStream': {
              'streamId': streamId,
              'target': r'$core.stream.open',
              'propertyName': 'openCoreStream',
              'args': {'streamId': streamId},
            },
          },
        },
      ]),
    );
  }

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    if (request.methodName == 'cancelMessage') cancelCalls += 1;
    throw StateError(
      'No model or cancellation calls should be made during watch recovery',
    );
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  Future<void> dispose() async {
    await _messages.close();
    await _states.close();
    for (final output in _outputs) {
      if (!output.isClosed) await output.close();
    }
  }
}

CoreEvent _event(CoreWatchRequest request, String kind, Object? value) =>
    CoreEvent.raw(
      requestId: request.requestId,
      target: request.target,
      propertyName: request.propertyName,
      kind: kind,
      valueBytes: encodeCoreLink(value),
      decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
    );

Map<String, Object?> _markdown(String type, [String? value, int? blockId]) => {
  'chatId': 'chat-1',
  'type': type,
  'value': value,
  'id': null,
  'blockId': blockId,
  'inlineId': null,
  'parentBlockId': null,
  'nodeType': null,
  'headerLevel': null,
  'xml': null,
};
