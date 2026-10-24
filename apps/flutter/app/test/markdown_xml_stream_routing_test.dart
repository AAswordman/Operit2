import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';

/// Verifies nested event ownership and renderer-local parsing failures.
void main() {
  setUpAll(ClientLogger.initialize);

  for (final live in [false, true]) {
    testWidgets('routes nested thinking events for live=$live', (tester) async {
      final controller = StreamController<Object>.broadcast();
      final state = StreamMarkdownRendererState();
      final events = _nestedEvents();
      await tester.pumpWidget(
        _surface(
          state: state,
          stream: live ? controller.stream : null,
          split: (_) async => [...events, _event('completed')],
        ),
      );
      if (live) controller.addStream(Stream.fromIterable(events));
      if (live) {
        for (var i = 0; i < 5; i++) {
          await tester.pump(const Duration(milliseconds: 250));
        }
      } else {
        await tester.pumpAndSettle();
      }

      expect(tester.takeException(), isNull);
      expect(state.streamParsingCompletedSuccessfully, !live);
      final outer = state.nodes.first;
      expect(outer.type, MarkdownNodeType.xmlBlock);
      expect(outer.xmlTagName, 'think');
      expect(outer.xmlIsClosed, isFalse);
      if (live) {
        final renderers = tester.widgetList<StreamMarkdownRenderer>(
          find.byType(StreamMarkdownRenderer),
        );
        expect(renderers.length, greaterThanOrEqualTo(3));
        expect(
          find.textContaining('inner body', findRichText: true),
          findsWidgets,
        );
        expect(find.textContaining('read_file'), findsWidgets);
        controller.add(_event('completed'));
        await tester.pumpAndSettle();
        expect(state.streamParsingCompletedSuccessfully, isTrue);
        expect(tester.takeException(), isNull);
      }
      await tester.pumpWidget(const SizedBox());
      await controller.close();
    });
  }

  test(
    'restores nested stream ownership with the saved event history',
    () async {
      final state = StreamMarkdownRendererState();
      final builder = state.eventBuilder;
      builder.startBlock(
        blockId: 1,
        type: MarkdownNodeType.xmlBlock,
        headerLevel: null,
      );
      builder.appendXmlMarkdownEvent(
        parentBlockId: 1,
        event: 'nested start',
        xmlBlockId: 3,
      );
      builder.savepoint('saved');
      builder.appendXmlMarkdownEvent(parentBlockId: 3, event: 'discarded body');
      builder.rollback('saved');

      expect(builder.xmlMarkdownStreamOwner(3), 1);
      final replay = state.xmlMarkdownEventStreams[0]!.toList();
      builder.appendXmlMarkdownEvent(parentBlockId: 3, event: 'retained body');
      builder.complete();
      expect(await replay, ['nested start', 'retained body']);
      expect(() => builder.xmlMarkdownStreamOwner(3), throwsStateError);
      state.reset();
    },
  );

  testWidgets('keeps static event parsing failures local', (tester) async {
    final state = StreamMarkdownRendererState();
    var errors = 0;
    await tester.pumpWidget(
      _surface(
        state: state,
        stream: null,
        split: (_) async => [
          _event('chunk', value: 'retained static content'),
          _event('markdownBlockChunk', blockId: 99, value: 'invalid'),
        ],
        onError: () => errors++,
      ),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(errors, 1);
    expect(state.streamParsingCompletedSuccessfully, isFalse);
    expect(state.collectedContent.toString(), 'retained static content');
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('keeps event parsing errors inside the renderer stream', (
    tester,
  ) async {
    final controller = StreamController<Object>.broadcast();
    final state = StreamMarkdownRendererState();
    var errors = 0;
    await tester.pumpWidget(
      _surface(
        state: state,
        stream: controller.stream,
        split: (_) async => const [],
        onError: () => errors++,
      ),
    );
    controller.add(_event('chunk', value: 'original text'));
    controller.add(_event('markdownBlockChunk', blockId: 99, value: 'invalid'));
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(errors, 1);
    expect(state.streamParsingCompletedSuccessfully, isFalse);
    expect(state.collectedContent.toString(), 'original text');
    final closed = controller.close();
    await tester.pump();
    await closed;
    expect(state.streamParsingCompletedSuccessfully, isFalse);
    expect(errors, 1);
    await tester.pumpWidget(const SizedBox());
  });
}

/// Builds an isolated localized renderer using an explicit event source.
Widget _surface({
  required StreamMarkdownRendererState state,
  required Stream<Object>? stream,
  required MarkdownContentSplitter split,
  VoidCallback? onError,
}) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(
      body: StreamMarkdownRenderer(
        content:
            'nested thinking routing regression ${identityHashCode(state)}',
        contentStream: stream,
        isStreaming: stream != null,
        state: state,
        textColor: Colors.black,
        backgroundColor: Colors.white,
        onStreamError: onError,
        splitMarkdownContent: split,
      ),
    ),
  );
}

/// Models the depth-first Rust event order with unique block identities.
List<MarkdownStreamEvent> _nestedEvents() {
  return [
    _event('markdownBlockStart', blockId: 1, nodeType: 'XmlBlock'),
    _event(
      'markdownBlockChunk',
      blockId: 1,
      nodeType: 'XmlBlock',
      value: '<think>outer body:<think>inner body',
      xml: _metadata('think'),
    ),
    _event('markdownBlockStart', blockId: 2, parent: 1),
    _event('markdownInlineStart', blockId: 2, inlineId: 1, parent: 1),
    _event(
      'markdownInlineChunk',
      blockId: 2,
      inlineId: 1,
      parent: 1,
      value: 'outer body:',
    ),
    _event('markdownBlockStart', blockId: 3, parent: 1, nodeType: 'XmlBlock'),
    _event(
      'markdownBlockChunk',
      blockId: 3,
      parent: 1,
      nodeType: 'XmlBlock',
      value: '<think>inner body',
      xml: _metadata('think'),
    ),
    _event('chunk', parent: 3, value: 'inner body'),
    _event('markdownBlockStart', blockId: 4, parent: 3),
    _event('markdownInlineStart', blockId: 4, inlineId: 1, parent: 3),
    _event(
      'markdownInlineChunk',
      blockId: 4,
      inlineId: 1,
      parent: 3,
      value: 'inner body',
    ),
    _event('markdownBlockStart', blockId: 5, parent: 3, nodeType: 'XmlBlock'),
    _event(
      'markdownBlockChunk',
      blockId: 5,
      parent: 3,
      nodeType: 'XmlBlock',
      value: '<tool name="read_file">file.txt</tool>',
      xml: _metadata('tool', body: 'file.txt', closed: true),
    ),
  ];
}

/// Creates transport metadata without requiring Flutter to reparse XML.
MarkdownXmlStreamEvent _metadata(
  String tag, {
  String? body,
  bool closed = false,
}) {
  return MarkdownXmlStreamEvent(
    tagName: tag,
    attributes: tag == 'tool' ? {'name': 'read_file'} : {},
    bodyChunk: body,
    children: const [],
    isClosed: closed,
  );
}

/// Creates one Rust-shaped event with explicit nesting and optional XML data.
MarkdownStreamEvent _event(
  String type, {
  int? blockId,
  int? inlineId,
  int? parent,
  String? value,
  String? nodeType,
  MarkdownXmlStreamEvent? xml,
}) {
  return MarkdownStreamEvent(
    chatId: 'routing-test',
    eventType: type,
    value: value,
    id: null,
    blockId: blockId,
    inlineId: inlineId,
    parentBlockId: parent,
    nodeType: nodeType,
    headerLevel: null,
    xml: xml,
  );
}
