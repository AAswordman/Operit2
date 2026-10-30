import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/FileDiffDisplay.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/ToolCallResultMergeRender.dart';

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

    test('matches a call without any result yet', () {
      final nodes = <MarkdownNodeStable>[_xml('<tool name="read_file" />')];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 0,
      );

      expect(match, isNotNull);
      expect(match!.startIndex, 0);
      expect(match.endIndexInclusive, 0);
    });

    test('matches partial results and keeps later calls pending', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="read_file" />'),
        _xml('<tool name="grep_code" />'),
        _xml('<tool_result name="read_file">contents</tool_result>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 2,
      );

      expect(match, isNotNull);
      expect(match!.startIndex, 0);
      expect(match.endIndexInclusive, 2);
    });

    test('stops before results whose tool names are out of order', () {
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

      expect(match, isNotNull);
      expect(match!.endIndexInclusive, 1);
    });

    test('stops before a result with a different tool name', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="read_file" />'),
        _xml('<tool_result name="grep_code">match</tool_result>'),
      ];

      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      );

      expect(match, isNotNull);
      expect(match!.endIndexInclusive, 0);
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

    test('tracks only calls without results as pending', () {
      final nodes = <MarkdownNodeStable>[
        _xml('<tool name="read_file" />'),
        _xml('<tool name="grep_code" />'),
        _xml('<tool_result name="read_file">contents</tool_result>'),
      ];

      expect(resolvePendingToolName(nodes), 'grep_code');
      expect(
        resolvePendingToolName(<MarkdownNodeStable>[
          ...nodes,
          _xml('<tool_result name="grep_code">match</tool_result>'),
        ]),
        isNull,
      );
    });

    test('completes pending proxy calls by concrete target name', () {
      final call = _xml(
        '<tool name="proxy"><param name="tool_name">read_file</param></tool>',
      );
      final result = _xml(
        '<tool_result name="read_file">contents</tool_result>',
      );

      expect(resolvePendingToolName(<MarkdownNodeStable>[call]), 'proxy');
      expect(
        resolvePendingToolName(<MarkdownNodeStable>[call, result]),
        isNull,
      );
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
      expect(find.text('Read'), findsOneWidget);
      expect(find.text('README.md'), findsOneWidget);
      expect(find.text('contents'), findsNothing);

      await tester.tap(find.text('Read'));
      await tester.pumpAndSettle();

      expect(find.textContaining('path: README.md'), findsOneWidget);
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
      expect(find.text('Read'), findsOneWidget);
      expect(find.text('Grep'), findsOneWidget);
    });

    testWidgets('renders an edit diff inline without dialog', (tester) async {
      final nodes = <MarkdownNodeStable>[
        _xml(
          '<tool name="edit_file"><param name="path">lib/a.dart</param></tool>',
        ),
        _xml(
          '<tool_result name="edit_file" status="success"><content>'
          'Successfully applied\n'
          '<file-diff path="lib/a.dart" details="updated"><![CDATA['
          '@@ -1,1 +1,1 @@\n-old\n+fresh line\n]]></file-diff>'
          '</content></tool_result>',
        ),
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
              rendererId: 'edit',
              textColor: Colors.black,
              xmlRenderer: _emptyXmlRenderer,
              xmlStreamResolver: _emptyStringStreamResolver,
              xmlMarkdownEventStreamResolver: _emptyObjectStreamResolver,
              renderInstanceKey: 'edit-pair',
            ),
          ),
        ),
      );

      expect(find.text('Edit'), findsWidgets);
      expect(find.byType(SvgPicture), findsOneWidget);
      expect(find.text('lib/a.dart'), findsWidgets);
      expect(find.byType(FileDiffDisplay), findsOneWidget);
      expect(find.text('fresh line'), findsOneWidget);
      expect(find.textContaining('Successfully applied'), findsNothing);
      expect(find.textContaining('<file-diff'), findsNothing);

      await tester.tap(find.text('Edit').first);
      await tester.pumpAndSettle();

      expect(find.byType(FileDiffDisplay), findsNothing);
    });

    testWidgets(
      'tapping merged row inside SelectionArea expands inline detail without selection error',
      (tester) async {
        final nodes = <MarkdownNodeStable>[
          _xml(
            '<tool name="read_file"><param name="path">lib/test.dart</param></tool>',
          ),
          _xml(
            '<tool_result name="read_file" status="success"><content>file content line</content></tool_result>',
          ),
        ];
        final match = mergeRender.match(
          nodes: nodes,
          startIndex: 0,
          endIndexInclusive: 1,
        )!;

        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: SelectionArea(
                child: mergeRender.renderMerge(
                  match: match,
                  nodes: nodes,
                  rendererId: 'select-test',
                  textColor: Colors.black,
                  xmlRenderer: _emptyXmlRenderer,
                  xmlStreamResolver: _emptyStringStreamResolver,
                  xmlMarkdownEventStreamResolver: _emptyObjectStreamResolver,
                  renderInstanceKey: 'select-pair',
                ),
              ),
            ),
          ),
        );

        expect(find.byType(MergedToolCallResultRow), findsOneWidget);
        expect(find.text('file content line'), findsNothing);

        await tester.tap(find.byType(MergedToolCallResultRow));
        await tester.pumpAndSettle();

        expect(find.text('file content line'), findsOneWidget);
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets('renders terminal tool with cleaned and unescaped command', (
      tester,
    ) async {
      final nodes = <MarkdownNodeStable>[
        _xml(
          '<tool name="super_admin:bash"><param name="command">cd \'D:/Operit2\' &amp;&amp; git status</param></tool>',
        ),
        _xml(
          '<tool_result name="super_admin:bash" status="success"><content>clean tree</content></tool_result>',
        ),
      ];
      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      )!;

      await tester.pumpWidget(
        MaterialApp(
          locale: const Locale('zh'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: mergeRender.renderMerge(
              match: match,
              nodes: nodes,
              rendererId: 'bash-test',
              textColor: Colors.black,
              xmlRenderer: _emptyXmlRenderer,
              xmlStreamResolver: _emptyStringStreamResolver,
              xmlMarkdownEventStreamResolver: _emptyObjectStreamResolver,
              renderInstanceKey: 'bash-pair',
            ),
          ),
        ),
      );

      expect(find.text('终端'), findsOneWidget);
      expect(find.textContaining('git status'), findsOneWidget);

      await tester.tap(find.text('终端'));
      await tester.pumpAndSettle();

      expect(find.textContaining('command: '), findsOneWidget);
      expect(find.textContaining('&amp;'), findsNothing);
    });

    testWidgets('expanded row shows a compact parameter summary and result', (
      tester,
    ) async {
      final nodes = <MarkdownNodeStable>[
        _xml(
          '<tool name="read_file"><param name="path">lib/test.dart</param></tool>',
        ),
        _xml(
          '<tool_result name="read_file" status="success"><content>file output content</content></tool_result>',
        ),
      ];
      final match = mergeRender.match(
        nodes: nodes,
        startIndex: 0,
        endIndexInclusive: 1,
      )!;

      await tester.pumpWidget(
        MaterialApp(
          locale: const Locale('zh'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: mergeRender.renderMerge(
              match: match,
              nodes: nodes,
              rendererId: 'toggle-test',
              textColor: Colors.black,
              xmlRenderer: _emptyXmlRenderer,
              xmlStreamResolver: _emptyStringStreamResolver,
              xmlMarkdownEventStreamResolver: _emptyObjectStreamResolver,
              renderInstanceKey: 'toggle-pair',
            ),
          ),
        ),
      );

      await tester.tap(find.byType(MergedToolCallResultRow));
      await tester.pumpAndSettle();

      expect(find.textContaining('path: lib/test.dart'), findsOneWidget);
      expect(find.text('file output content'), findsOneWidget);
      expect(find.textContaining('<param'), findsNothing);
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

    expect(find.text('已完成 · 2 次调用'), findsOneWidget);
    expect(find.byType(MergedToolCallResultRow), findsNothing);

    await tester.tap(find.text('已完成 · 2 次调用'));
    await tester.pumpAndSettle();

    expect(find.byType(MergedToolCallResultRow), findsNWidgets(2));
  });

  test('keeps intermediate Markdown outside thinking and tool groups', () {
    const grouper = ThinkToolsXmlNodeGrouper(showThinkingProcess: true);
    final nodes = <MarkdownNodeStable>[
      _xml('<think>Start round 1</think>'),
      _xml('<tool name="read_file" />'),
      _xml('<tool_result name="read_file">ok</tool_result>'),
      const MarkdownNodeStable(
        type: MarkdownNodeType.plainText,
        content: 'Intermediate step 1 note',
        isStreaming: false,
      ),
      _xml('<think>Start round 2</think>'),
      _xml('<tool name="grep_code" />'),
      _xml('<tool_result name="grep_code">found</tool_result>'),
      const MarkdownNodeStable(
        type: MarkdownNodeType.plainText,
        content: 'Final answer to user',
        isStreaming: false,
      ),
    ];

    final groups = grouper.group(nodes, 'multi-turn');
    expect(groups.length, 4);
    final firstGroup = groups[0] as MarkdownGroupItem;
    expect(firstGroup.startIndex, 0);
    expect(firstGroup.endIndexInclusive, 2);
    expect((groups[1] as MarkdownSingleItem).index, 3);
    final secondGroup = groups[2] as MarkdownGroupItem;
    expect(secondGroup.startIndex, 4);
    expect(secondGroup.endIndexInclusive, 6);
    expect((groups[3] as MarkdownSingleItem).index, 7);
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
  final declaredDisplays = <String, Map<String, dynamic>>{
    'read_file': {
      'label': {'en': 'Read', 'zh': '读取'},
      'icon': 'file',
      'targetParameter': 'path',
    },
    'grep_code': {
      'label': {'en': 'Grep', 'zh': '搜索'},
      'icon': 'search',
      'targetParameter': 'pattern',
    },
    'edit_file': {
      'label': {'en': 'Edit', 'zh': '编辑'},
      'resultRenderer': 'fileDiff',
      'icon': 'edit',
      'targetParameter': 'path',
    },
    'super_admin:bash': {
      'label': {'en': 'Terminal', 'zh': '终端'},
      'icon': 'terminal',
      'targetParameter': 'command',
    },
    'execute_in_terminal_session': {
      'label': {'en': 'Terminal', 'zh': '终端'},
      'icon': 'terminal',
      'targetParameter': 'command',
    },
  };
  final declaredDisplay = declaredDisplays[attributes['name']];
  if (declaredDisplay != null) {
    attributes['display'] = jsonEncode(declaredDisplay);
  }
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
