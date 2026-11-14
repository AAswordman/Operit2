import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/common/ChatResponsePresentation.dart';
import 'package:operit2/ui/features/chat/components/TransformedChatMessage.dart';

/// Retains projection requests so tests can complete them out of order.
class _ProjectionBridge extends OperitRuntimeBridge {
  final revisions = StreamController<CoreEvent>.broadcast(sync: true);
  final requests = <Map<String, Object?>>[];
  final completions = <Completer<Uint8List>>[];
  CoreWatchRequest? subscription;

  /// Captures the exact source snapshot passed through the real generated client.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    expect(request.methodName, 'projectMessagesForDisplay');
    requests.add(Map<String, Object?>.from(request.args as Map));
    final pending = Completer<Uint8List>();
    completions.add(pending);
    return pending.future;
  }

  /// Emits transformed content while retaining all other canonical fields.
  void complete(int index, String text, [List<String>? texts]) {
    final messages = requests[index]['messages'] as List;
    final projected = [
      for (var row = 0; row < messages.length; row++)
        {
          ...Map<String, Object?>.from(messages[row] as Map),
          'parts': [
            {
              ...Map<String, Object?>.from(
                (messages[row]['parts'] as List).single as Map,
              ),
              'content': texts?[row] ?? text,
            },
          ],
        },
    ];
    completions[index].complete(encodeCoreLink([0, projected]));
  }

  /// Triggers the same revision watch used after plugin rule changes.
  void invalidate(int revision) {
    final request = subscription!;
    revisions.add(
      CoreEvent.raw(
        requestId: request.requestId,
        target: request.target,
        propertyName: request.propertyName,
        kind: 'Changed',
        valueBytes: encodeCoreLink(revision),
        decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
      ),
    );
  }

  /// Opens the dedicated revision stream through the production generated API.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    expect(request.propertyName, 'messageTransformRevisionFlow');
    subscription = request;
    return Stream.multi((controller) {
      controller.add(
        CoreEvent.raw(
          requestId: request.requestId,
          target: request.target,
          propertyName: request.propertyName,
          kind: 'Snapshot',
          valueBytes: encodeCoreLink(0),
          decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
        ),
      );
      final listener = revisions.stream.listen(controller.add);
      controller.onCancel = listener.cancel;
    });
  }

  /// Rejects unrelated one-shot subscriptions in this focused harness.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects client-owned output streams in this focused harness.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
}

/// Creates canonical text with a stable identity and optional live source.
ChatMessage _message([
  Stream<MarkdownStreamEvent>? stream,
  bool presentation = false,
]) => ChatMessage(
  sender: 'ai',
  parts: const [
    MessagePart(
      partId: 'body',
      sequence: 0,
      kind: MessagePartKind.markdown,
      content: 'canonical',
      toolCallId: null,
      toolName: null,
      attributes: {},
    ),
  ],
  timestamp: 42,
  roleName: '',
  pluginExtensions: presentation
      ? {
          responsePresentationExtension: {'bubbles': [], 'pending': true},
        }
      : {},
  selectedVariantIndex: 0,
  variantCount: 1,
  provider: '',
  modelName: '',
  inputTokens: 0,
  outputTokens: 0,
  cachedInputTokens: 0,
  sentAt: 0,
  outputDurationMs: 0,
  waitDurationMs: 0,
  completedAt: 0,
  displayMode: ChatMessageDisplayMode.normal,
  isFavorite: false,
  contentStream: stream,
);

/// Exercises actual generated projection requests, revision watches, and widget lifetime.
void main() {
  testWidgets(
    'only mounted rows acquire projections and inactive rows stay idle',
    (tester) async {
      final bridge = _ProjectionBridge();
      addTearDown(bridge.revisions.close);
      final clients = GeneratedCoreProxyClients(bridge);
      final cache = MessageTransformCache(clients);
      addTearDown(cache.dispose);
      final messages = [
        for (var index = 0; index < 80; index++)
          ChatMessage.fromJson({..._message().toJson(), 'timestamp': index}),
      ];
      await tester.pumpWidget(
        _rows(clients, [messages.last], cache: cache, depthAnchor: 79),
      );
      expect(bridge.requests, hasLength(1));
      expect(bridge.requests.single['messages'], hasLength(1));
      bridge.complete(0, 'shown');
      await tester.pumpAndSettle();
      for (var repeat = 0; repeat < 20; repeat++) {
        await tester.pumpWidget(
          _rows(clients, [messages.last], cache: cache, depthAnchor: 79),
        );
      }
      expect(bridge.requests, hasLength(1));
      await tester.pumpWidget(const SizedBox());
      bridge.invalidate(1);
      await tester.pump();
      expect(bridge.requests, hasLength(1));
      await tester.pumpWidget(
        _rows(clients, [messages[78]], cache: cache, depthAnchor: 79),
      );
      expect(bridge.requests, hasLength(2));
      expect(bridge.requests.last['messages'], hasLength(1));
      bridge.complete(1, 'next row');
      await tester.pumpAndSettle();
      expect(find.text('next row'), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('malformed batch responses become explicit display errors', (
    tester,
  ) async {
    final bridge = _ProjectionBridge();
    addTearDown(bridge.revisions.close);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: TransformedChatMessage(
          message: _message(),
          chatId: 'chat',
          depthAnchor: 42,
          clients: GeneratedCoreProxyClients(bridge),
          builder: (message) => Text(message.parts.single.content),
        ),
      ),
    );
    bridge.completions.single.complete(encodeCoreLink([0, []]));
    await tester.pumpAndSettle();
    expect(find.textContaining('requested message order'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'mounted batches fail explicitly and stale batches cannot reveal another chat',
    (tester) async {
      final bridge = _ProjectionBridge();
      addTearDown(bridge.revisions.close);
      final clients = GeneratedCoreProxyClients(bridge);
      final cache = MessageTransformCache(clients);
      addTearDown(cache.dispose);
      final first = _message();
      final second = ChatMessage.fromJson({...first.toJson(), 'timestamp': 43});
      await tester.pumpWidget(
        _rows(clients, [first, second], cache: cache, depthAnchor: 43),
      );
      expect(bridge.requests, hasLength(1));
      expect(bridge.requests.single['messages'], hasLength(2));
      bridge.completions[0].completeError(StateError('invalid pattern'));
      await tester.pumpAndSettle();
      expect(find.textContaining('invalid pattern'), findsNWidgets(2));
      // Shared cache watches are subscribed before the row's fake-async build.
      await tester.runAsync(() async => bridge.invalidate(1));
      await tester.pump();
      expect(bridge.requests, hasLength(2));
      await tester.pumpWidget(
        _rows(
          clients,
          [first],
          cache: cache,
          chatId: 'other-chat',
          depthAnchor: 42,
        ),
      );
      expect(bridge.requests, hasLength(3));
      bridge.complete(1, 'old chat');
      await tester.pump();
      expect(find.text('old chat'), findsNothing);
      bridge.complete(2, 'new chat');
      await tester.pumpAndSettle();
      expect(find.text('new chat'), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
    },
  );

  for (final presentation in [false, true]) {
    testWidgets(
      'defers display regex until live output completes, presentation=$presentation',
      (tester) async {
        final bridge = _ProjectionBridge();
        addTearDown(bridge.revisions.close);
        final clients = GeneratedCoreProxyClients(bridge);
        final live = StreamController<MarkdownStreamEvent>.broadcast();
        final sourceStream = live.stream;
        addTearDown(live.close);
        ChatMessage? displayed;
        Widget render(ChatMessage message) => MaterialApp(
          home: TransformedChatMessage(
            message: message,
            chatId: 'chat',
            depthAnchor: 42,
            clients: clients,
            builder: (value) {
              displayed = value;
              return Text(value.parts.single.content);
            },
          ),
        );
        for (var update = 0; update < 8; update++) {
          await tester.pumpWidget(render(_message(sourceStream, presentation)));
          expect(bridge.requests, isEmpty);
          expect(displayed!.contentStream, same(sourceStream));
        }
        bridge.invalidate(1);
        await tester.pump();
        expect(bridge.requests, isEmpty);

        final completed = _message(null, presentation);
        await tester.pumpWidget(render(completed));
        expect(bridge.requests, hasLength(1));
        expect(find.text('canonical'), findsOneWidget);
        bridge.complete(0, 'regex result');
        await tester.pumpAndSettle();
        expect(find.text('regex result'), findsOneWidget);
        expect(displayed!.contentStream, isNull);
        expect(completed.parts.single.content, 'canonical');
        await tester.pumpWidget(const SizedBox());
      },
    );
  }

  testWidgets(
    'history cache projects new viewport rows and reuses completed remounts',
    (tester) async {
      final bridge = _ProjectionBridge();
      addTearDown(bridge.revisions.close);
      final clients = GeneratedCoreProxyClients(bridge);
      final cache = MessageTransformCache(clients);
      addTearDown(cache.dispose);
      final first = _message();
      final second = ChatMessage.fromJson({...first.toJson(), 'timestamp': 43});
      await tester.pumpWidget(
        _rows(clients, [first], cache: cache, depthAnchor: 43),
      );
      bridge.complete(0, 'first shown');
      await tester.pumpAndSettle();
      await tester.pumpWidget(
        _rows(clients, [second], cache: cache, depthAnchor: 43),
      );
      expect(bridge.requests, hasLength(2));
      bridge.complete(1, 'second shown');
      await tester.pumpAndSettle();
      expect(find.text('second shown'), findsOneWidget);
      await tester.pumpWidget(
        _rows(clients, [first], cache: cache, depthAnchor: 43),
      );
      expect(find.text('first shown'), findsOneWidget);
      expect(bridge.requests, hasLength(2));
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'display invalidation projects the original and ignores stale completions',
    (tester) async {
      final bridge = _ProjectionBridge();
      addTearDown(bridge.revisions.close);
      final canonical = _message();
      await tester.pumpWidget(
        MaterialApp(
          home: TransformedChatMessage(
            message: canonical,
            chatId: 'chat',
            depthAnchor: 42,
            clients: GeneratedCoreProxyClients(bridge),
            builder: (message) => Text(message.parts.single.content),
          ),
        ),
      );
      expect(bridge.requests, hasLength(1));
      bridge.complete(0, 'shown');
      await tester.pumpAndSettle();
      expect(find.text('shown'), findsOneWidget);
      expect(canonical.parts.single.content, 'canonical');
      bridge.invalidate(1);
      await tester.pump();
      bridge.invalidate(2);
      await tester.pump();
      expect(bridge.requests, hasLength(3));
      expect(
        (((bridge.requests[2]['messages'] as List).single as Map)['parts']
                as List)
            .single['content'],
        'canonical',
      );
      bridge.complete(2, 'latest');
      await tester.pumpAndSettle();
      bridge.complete(1, 'obsolete');
      await tester.pumpAndSettle();
      expect(find.text('latest'), findsOneWidget);
      expect(find.text('obsolete'), findsNothing);
      await tester.pumpWidget(const SizedBox());
      expect(bridge.revisions.hasListener, false);
    },
  );

  for (final language in ['en', 'zh']) {
    testWidgets(
      'projection errors are localized and live messages retain their runtime stream ($language)',
      (tester) async {
        final bridge = _ProjectionBridge();
        addTearDown(bridge.revisions.close);
        final clients = GeneratedCoreProxyClients(bridge);
        Widget render(ChatMessage message) => MaterialApp(
          locale: Locale(language),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: TransformedChatMessage(
            message: message,
            chatId: 'chat',
            depthAnchor: 42,
            clients: clients,
            builder: (value) => Text(value.parts.single.content),
          ),
        );
        await tester.pumpWidget(render(_message()));
        bridge.completions.single.completeError(StateError('invalid pattern'));
        await tester.pumpAndSettle();
        expect(find.textContaining('invalid pattern'), findsOneWidget);
        expect(
          find.textContaining(
            language == 'en'
                ? 'Failed to process message display:'
                : '消息显示处理失败：',
          ),
          findsOneWidget,
        );
        final live = StreamController<MarkdownStreamEvent>.broadcast();
        await tester.pumpWidget(render(_message(live.stream)));
        expect(find.text('canonical'), findsOneWidget);
        expect(bridge.requests, hasLength(1));
        await tester.pumpWidget(const SizedBox());
        await live.close();
      },
    );
  }
}

/// Mounts only the supplied viewport rows against one shared projection owner.
Widget _rows(
  GeneratedCoreProxyClients clients,
  List<ChatMessage> messages, {
  required MessageTransformCache cache,
  required int depthAnchor,
  String chatId = 'chat',
}) => MaterialApp(
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Column(
    children: [
      for (final message in messages)
        TransformedChatMessage(
          key: ValueKey(message.timestamp),
          message: message,
          chatId: chatId,
          clients: clients,
          depthAnchor: depthAnchor,
          cache: cache,
          builder: (projected) => Text(projected.parts.single.content),
        ),
    ],
  ),
);
