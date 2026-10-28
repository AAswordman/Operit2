import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/features/chat/components/ChatMessageExtent.dart';

import 'support/markdown_event_fixture.dart';
import 'support/semantic_markdown_fixture.dart';

MarkdownStreamEvent _event(String type, {int? blockId, String? value}) =>
    MarkdownStreamEvent(
      chatId: 'semantics-test',
      eventType: type,
      blockId: blockId,
      value: value,
      id: null,
      inlineId: null,
      parentBlockId: null,
      nodeType: null,
      headerLevel: null,
      xml: null,
    );

Iterable<SemanticsNode> _semanticsTree(SemanticsNode root) sync* {
  yield root;
  final children = <SemanticsNode>[];
  root.visitChildren((child) {
    children.add(child);
    return true;
  });
  for (final child in children) {
    yield* _semanticsTree(child);
  }
}

void main() {
  testWidgets(
    'streaming message keeps its semantics separate through growth and scrolling',
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        final scroll = ScrollController();
        addTearDown(scroll.dispose);
        final stream = StreamController<Object>();
        addTearDown(stream.close);
        final extents = List.generate(2, (_) => ChatMessageExtent());
        addTearDown(() {
          for (final extent in extents) {
            extent.dispose();
          }
        });
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: SizedBox(
                width: 400,
                height: 300,
                child: CustomScrollView(
                  controller: scroll,
                  slivers: [
                    ChatMessageSliverList(
                      extents: extents,
                      preserveViewport: () => false,
                      itemBuilder: (_, index) => observeChatMessageExtent(
                        key: ValueKey(index),
                        extent: extents[index],
                        child: index == 0
                            ? const Text('Previous message')
                            : StreamMarkdownRenderer(
                                rendererId: 'streaming-semantics',
                                content: '',
                                isStreaming: true,
                                contentStream: stream.stream,
                                textColor: Colors.black,
                                backgroundColor: Colors.white,
                                splitMarkdownContent: splitMarkdownEventFixture,
                              ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
        stream.add(_event('markdownBlockStart', blockId: 1));
        stream.add(
          _event('markdownBlockChunk', blockId: 1, value: 'Streaming tail'),
        );
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 200));
        await tester.pumpAndSettle();
        final historyFinder = find.text('Previous message');
        final tailFinder = find.textContaining(
          'Streaming tail',
          findRichText: true,
        );
        final historyId = tester.getSemantics(historyFinder).id;
        final messageId = tester.getSemantics(tailFinder).id;
        expect(historyId, isNot(messageId));
        expect(tester.getSemantics(historyFinder).label, 'Previous message');
        expect(tester.getSemantics(tailFinder).label, 'Streaming tail');
        for (var chunk = 0; chunk < 4; chunk++) {
          stream.add(
            _event(
              'markdownBlockChunk',
              blockId: 1,
              value: List.filled(6, '\nNext line $chunk').join(),
            ),
          );
          await tester.pump();
          await tester.pump(const Duration(milliseconds: 200));
          await tester.pumpAndSettle();
          expect(tester.getSemantics(historyFinder).id, historyId);
          expect(tester.getSemantics(historyFinder).label, 'Previous message');
          expect(tester.getSemantics(tailFinder).id, messageId);
          expect(
            tester.getSemantics(tailFinder).label,
            contains('Next line $chunk'),
          );
          expect(
            tester.getSemantics(tailFinder).label,
            isNot(contains('Previous message')),
          );
        }
        expect(scroll.position.maxScrollExtent, greaterThan(24));
        scroll.jumpTo(24);
        await tester.pump();
        expect(tester.getSemantics(tailFinder).id, messageId);
        expect(tester.getSemantics(tailFinder).label, contains('Next line 3'));
        stream.add(_event('completed'));
        await tester.pump();
        await tester.pumpAndSettle();
        expect(tester.getSemantics(tailFinder).id, messageId);
        expect(tester.getSemantics(tailFinder).label, contains('Next line 3'));
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
      } finally {
        semantics.dispose();
      }
    },
  );
  for (final live in [false, true]) {
    testWidgets('message boundaries preserve link semantics for live=$live', (
      tester,
    ) async {
      final semantics = tester.ensureSemantics();
      try {
        final source = semanticMarkdownFixtures.first['source'] as String;
        final stream = StreamController<Object>.broadcast();
        addTearDown(stream.close);
        final extent = ChatMessageExtent();
        addTearDown(extent.dispose);
        final opened = <String>[];
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: observeChatMessageExtent(
                key: const ValueKey('links-message'),
                extent: extent,
                child: StreamMarkdownRenderer(
                  content: live ? '' : source,
                  isStreaming: live,
                  contentStream: live ? stream.stream : null,
                  textColor: Colors.black,
                  backgroundColor: Colors.white,
                  splitMarkdownContent: splitSemanticMarkdownFixture,
                  onLinkClick: opened.add,
                ),
              ),
            ),
          ),
        );
        if (live) {
          for (final event in semanticMarkdownEvents(
            source,
          ).where((event) => event.eventType != 'completed')) {
            stream.add(event);
          }
          await tester.pump();
          await tester.pump(const Duration(milliseconds: 200));
        }
        await tester.pumpAndSettle();
        final owner = tester.binding.renderViews.single.owner!.semanticsOwner!;
        final nodes = _semanticsTree(owner.rootSemanticsNode!).toList();
        final link = nodes.singleWhere((node) => node.label == '链接');
        final plainLink = nodes.singleWhere(
          (node) => node.label == 'https://example.org',
        );
        expect(link.getSemanticsData().flagsCollection.isLink, isTrue);
        expect(
          link.getSemanticsData().hasAction(ui.SemanticsAction.tap),
          isTrue,
        );
        owner.performAction(link.id, ui.SemanticsAction.tap);
        owner.performAction(plainLink.id, ui.SemanticsAction.tap);
        expect(opened, ['https://example.com/a_(b)', 'https://example.org']);
        if (live) {
          stream.add(_event('completed'));
          await tester.pump();
          await tester.pumpAndSettle();
          expect(
            _semanticsTree(
              owner.rootSemanticsNode!,
            ).singleWhere((node) => node.label == '链接').id,
            link.id,
          );
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
      } finally {
        semantics.dispose();
      }
    });
  }
}
