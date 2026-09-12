import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/ToolCallResultMergeRender.dart';
import 'package:operit2/ui/features/chat/components/part/XmlCanvasSummaryComponents.dart';

/// Verifies ordered tool/result merging and its interaction with groups.
void main() {
  const mergeRender = ToolCallResultMergeRender();

  group('ToolCallResultMergeRender.match', () {
    test('matches suffixless tool tags', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="read_file" />'),
        _xml('<tool_result name="read_file">ok</tool_result>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      );

      expect(match, isNotNull);
      expect(match!.startIndex, 0);
      expect(match.endIndexInclusive, 1);
    });

    test('matches arbitrary-length tag suffixes', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool_session_identifier name="read_file" />'),
        _xml(
          '<tool_result_session_identifier name="read_file">ok</tool_result_session_identifier>',
        ),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      );

      expect(match, isNotNull);
      expect(match!.endIndexInclusive, 1);
    });

    test('matches two ordered calls followed by their ordered results', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool_A1 name="read_file" />'),
        _xml('<tool_B23456 name="grep_code" />'),
        _xml('<tool_result_A1 name="read_file">file</tool_result_A1>'),
        _xml('<tool_result_B23456 name="grep_code">match</tool_result_B23456>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 3,
      );

      expect(match, isNotNull);
      expect(match!.endIndexInclusive, 3);
    });

    test('matches four calls followed by four ordered results', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="daily_life:get_current_time" />'),
        _xml('<tool name="daily_life:device_status" />'),
        _xml('<tool name="visit_web" />'),
        _xml('<tool name="list_core_nodes" />'),
        _xml(
          '<tool_result name="daily_life:get_current_time" status="success">'
          '<content>12:00</content></tool_result>',
        ),
        _xml(
          '<tool_result name="daily_life:device_status" status="success">'
          '<content>ready</content></tool_result>',
        ),
        _xml(
          '<tool_result name="visit_web" status="error">'
          '<content>blocked</content></tool_result>',
        ),
        _xml(
          '<tool_result name="list_core_nodes" status="success">'
          '<content>core-1</content></tool_result>',
        ),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 7,
      );

      expect(match, isNotNull);
      expect(match!.endIndexInclusive, 7);
    });

    test('matches four results containing raw URL ampersands', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="daily_life:get_current_date"></tool>'),
        _xml('<tool name="daily_life:device_status"></tool>'),
        _xml(
          '<tool name="daily_life:search_weather">'
          '<param name="location">Hong Kong</param></tool>',
        ),
        _xml(
          '<tool name="daily_life:search_weather">'
          '<param name="location">Shanghai</param></tool>',
        ),
        _xml(
          '<tool_result name="daily_life:get_current_date" status="success">'
          '<content>{"iso":"2026-09-11"}</content></tool_result>',
        ),
        _xml(
          '<tool_result name="daily_life:device_status" status="success">'
          '<content>{"batteryLevel":80}</content></tool_result>',
        ),
        _xml(
          '<tool_result name="daily_life:search_weather" status="success">'
          '<content>{"url":"https://example.com/?city=hk&lang=en"}'
          '</content></tool_result>',
        ),
        _xml(
          '<tool_result name="daily_life:search_weather" status="success">'
          '<content>{"url":"https://example.com/?city=sh&lang=zh"}'
          '</content></tool_result>',
        ),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 7,
      );

      expect(match, isNotNull);
      expect(match!.endIndexInclusive, 7);
    });

    test('matches four calls and results separated by line breaks', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="daily_life:get_current_time" />'),
        _break(),
        _xml('<tool name="daily_life:device_status" />'),
        _break(),
        _xml('<tool name="visit_web" />'),
        _break(),
        _xml('<tool name="list_core_nodes" />'),
        _break(),
        _xml(
          '<tool_result name="daily_life:get_current_time">12:00</tool_result>',
        ),
        _break(),
        _xml(
          '<tool_result name="daily_life:device_status">ready</tool_result>',
        ),
        _break(),
        _xml('<tool_result name="visit_web">blocked</tool_result>'),
        _break(),
        _xml('<tool_result name="list_core_nodes">core-1</tool_result>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: nodes.length - 1,
      );

      expect(match, isNotNull);
      expect(match!.endIndexInclusive, nodes.length - 1);
    });

    test('matches repeated tool names by source order', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool_first name="read_file" />'),
        _xml('<tool_second name="read_file" />'),
        _xml('<tool_result_first name="read_file">one</tool_result_first>'),
        _xml('<tool_result_second name="read_file">two</tool_result_second>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 3,
      );

      expect(match, isNotNull);
      expect(match!.endIndexInclusive, 3);
    });

    test('rejects results whose tool names are out of order', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool_one name="read_file" />'),
        _xml('<tool_two name="grep_code" />'),
        _xml('<tool_result_two name="grep_code">match</tool_result_two>'),
        _xml('<tool_result_one name="read_file">file</tool_result_one>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 3,
      );

      expect(match, isNull);
    });

    test('rejects a result with a different tool name', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="read_file" />'),
        _xml('<tool_result name="grep_code">match</tool_result>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      );

      expect(match, isNull);
    });

    test('matches proxy results by their concrete target tool name', () {
      final nodes = <MarkdownNodeStable>[
        _xml(
          '<tool name="proxy"><param name="tool_name">read_file</param>'
          '<param name="params">{"path":"README.md"}</param></tool>',
        ),
        _xml('<tool_result name="read_file">contents</tool_result>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      );

      expect(match, isNotNull);
    });

    test('matches package proxy results by their concrete target tool name', () {
      final nodes = <MarkdownNodeStable>[
        _xml(
          '<tool name="package_proxy"><param name="tool_name">'
          'demo:lookup</param><param name="params">{"query":"x"}</param></tool>',
        ),
        _xml('<tool_result name="demo:lookup">found</tool_result>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      );

      expect(match, isNotNull);
    });
  });

  group('ToolCallResultMergeRender.renderMerge', () {
    testWidgets('renders one pair as one merged row', (tester) async {
      final nodes = <MarkdownNodeStable>[
        _xml(
          '<tool name="read_file"><param name="path">README.md</param></tool>',
        ),
        _xml('<tool_result name="read_file">contents</tool_result>'),
      ];
      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      )!;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: mergeRender.renderMerge(
              match: match,
              nodes: nodes,
              rendererId: 'single',
              textColor: Colors.black,
              xmlRenderer: _emptyXmlRenderer,
              xmlStreamResolver: _emptyStringStreamResolver,
              xmlMarkdownEventStreamResolver: _emptyObjectStreamResolver,
              renderInstanceKey: 'single-pair',
            ),
          ),
        ),
      );

      expect(find.byType(MergedToolCallResultRow), findsOneWidget);
      expect(find.byType(CanvasToolSummaryRow), findsOneWidget);
      expect(find.text('read_file'), findsOneWidget);
      expect(find.text('README.md'), findsOneWidget);
      expect(find.text('contents'), findsNothing);

      await tester.tap(find.text('read_file'));
      await tester.pumpAndSettle();

      expect(find.text('contents'), findsOneWidget);
    });

    testWidgets('renders two pairs as two merged rows', (tester) async {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool_one name="read_file" />'),
        _xml('<tool_two name="grep_code" />'),
        _xml('<tool_result_one name="read_file">file</tool_result_one>'),
        _xml('<tool_result_two name="grep_code">match</tool_result_two>'),
      ];
      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 3,
      )!;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: mergeRender.renderMerge(
              match: match,
              nodes: nodes,
              rendererId: 'batch',
              textColor: Colors.black,
              xmlRenderer: _emptyXmlRenderer,
              xmlStreamResolver: _emptyStringStreamResolver,
              xmlMarkdownEventStreamResolver: _emptyObjectStreamResolver,
              renderInstanceKey: 'two-pairs',
            ),
          ),
        ),
      );

      expect(find.byType(MergedToolCallResultRow), findsNWidgets(2));
      expect(find.text('read_file'), findsOneWidget);
      expect(find.text('grep_code'), findsOneWidget);
    });
  });

  testWidgets('two grouped pairs expand to two merged rows', (tester) async {
    const grouper = ThinkToolsXmlNodeGrouper(showThinkingProcess: true);
    final nodes = <MarkdownNodeStable>[
      _xml('<tool name="read_file" />'),
      _xml('<tool name="grep_code" />'),
      _xml('<tool_result name="read_file">file</tool_result>'),
      _xml('<tool_result name="grep_code">match</tool_result>'),
    ];
    final groupedItem = grouper.group(nodes, 'grouped').single;
    final markdownGroup = groupedItem as MarkdownGroupItem;

    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: grouper.renderGroup(
            group: markdownGroup,
            nodes: nodes,
            rendererId: 'grouped',
            isVisible: true,
            isLastNode: true,
            textColor: Colors.black,
            xmlRenderer: _emptyXmlRenderer,
            mergeRender:
                ({
                  required startIndex,
                  required endIndexInclusive,
                  required renderInstanceKeyPrefix,
                  required shouldRenderNode,
                }) {
                  final match = mergeRender.match(
                    nodes: nodes,
                    startIndex: startIndex,
                    endIndexInclusive: endIndexInclusive,
                  )!;
                  return <Widget>[
                    mergeRender.renderMerge(
                      match: match,
                      nodes: nodes,
                      rendererId: 'grouped',
                      textColor: Colors.black,
                      xmlRenderer: _emptyXmlRenderer,
                      xmlStreamResolver: _emptyStringStreamResolver,
                      xmlMarkdownEventStreamResolver:
                          _emptyObjectStreamResolver,
                      renderInstanceKey: renderInstanceKeyPrefix,
                    ),
                  ];
                },
            xmlStreamResolver: _emptyStringStreamResolver,
            xmlMarkdownEventStreamResolver: _emptyObjectStreamResolver,
            onLinkClick: null,
            fillMaxWidth: true,
            textStyle: const TextStyle(),
          ),
        ),
      ),
    );

    expect(find.text('工具调用（2）'), findsOneWidget);
    expect(find.byType(MergedToolCallResultRow), findsNothing);

    await tester.tap(find.text('工具调用（2）'));
    await tester.pumpAndSettle();

    expect(find.byType(MergedToolCallResultRow), findsNWidgets(2));
  });
}

/// Creates one stable XML Markdown node for merge tests.
MarkdownNodeStable _xml(String content) {
  final trimmed = content.trim();
  final opening = RegExp(
    r'^<([A-Za-z][A-Za-z0-9_]*)([^>]*)>',
  ).firstMatch(trimmed);
  final tagName = opening?.group(1);
  final openingText = opening?.group(2) ?? '';
  final attributes = <String, String>{
    for (final match in RegExp(
      r'([A-Za-z][A-Za-z0-9_-]*)\s*=\s*"([^"]*)"',
    ).allMatches(openingText))
      match.group(1)!: match.group(2)!,
  };
  final openingEnd = opening?.end ?? 0;
  final selfClosing = trimmed.endsWith('/>');
  final closingStart = tagName == null
      ? -1
      : trimmed.toLowerCase().lastIndexOf('</${tagName.toLowerCase()}>');
  final bodyEnd = closingStart >= openingEnd ? closingStart : trimmed.length;
  final body = opening == null ? '' : trimmed.substring(openingEnd, bodyEnd);
  final children = <MarkdownXmlChildStable>[];
  final childPattern = RegExp(
    r'<([A-Za-z][A-Za-z0-9_]*)([^>]*)>([\s\S]*?)</([A-Za-z][A-Za-z0-9_]*)>',
    caseSensitive: false,
  );
  for (final match in childPattern.allMatches(body)) {
    if (match.group(1)!.toLowerCase() != match.group(4)!.toLowerCase()) {
      continue;
    }
    final childAttributes = <String, String>{
      for (final attr in RegExp(
        r'([A-Za-z][A-Za-z0-9_-]*)\s*=\s*"([^"]*)"',
      ).allMatches(match.group(2)!))
        attr.group(1)!: attr.group(2)!,
    };
    children.add(
      MarkdownXmlChildStable(
        index: children.length,
        tagName: match.group(1),
        attributes: childAttributes,
        body: match.group(3)!,
        isClosed: true,
      ),
    );
  }
  return MarkdownNodeStable(
    type: MarkdownNodeType.xmlBlock,
    content: content,
    isStreaming: false,
    xmlTagName: tagName,
    xmlAttributes: attributes,
    xmlBody: body,
    xmlChildren: children,
    xmlIsClosed: selfClosing || closingStart >= openingEnd,
  );
}

/// Creates one rendered Markdown line-break separator.
MarkdownNodeStable _break() {
  return const MarkdownNodeStable(
    type: MarkdownNodeType.htmlBreak,
    content: '\n',
    isStreaming: false,
  );
}

/// Produces an empty widget for unused XML rendering paths.
Widget _emptyXmlRenderer({
  required String xmlContent,
  required bool isStreaming,
  required Color textColor,
  Stream<String>? xmlStream,
  Stream<Object>? xmlMarkdownEventStream,
  String? renderInstanceKey,
}) {
  return const SizedBox.shrink();
}

/// Resolves no XML text stream in isolated merge tests.
Stream<String>? _emptyStringStreamResolver(int index) {
  return null;
}

/// Resolves no XML event stream in isolated merge tests.
Stream<Object>? _emptyObjectStreamResolver(int index) {
  return null;
}
