import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    show MarkdownStreamEvent;
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'support/markdown_event_fixture.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';

void main() {
  const declaration = ToolDisplayDeclaration(
    labels: {'fr': 'Lire', 'en': 'Read', 'default': 'Action'},
    icon: 'file',
    targetParameter: 'document',
  );

  test('uses declared metadata for arbitrary tool and parameter names', () {
    final presentation = ToolPresentation.fromCall(
      'example:z42',
      '<param name="document">/remote/workspace/a.dart</param>'
          '<param name="headers">important</param>'
          '<param name="user_agent">agent</param>',
      display: declaration,
    );
    expect(
      presentation.display!.labelFor(const Locale('fr'), 'fallback'),
      'Lire',
    );
    expect(
      presentation.display!.labelFor(const Locale('de'), 'fallback'),
      'Action',
    );
    expect(presentation.target, '/remote/workspace/a.dart');
    expect(presentation.parameterSummary, contains('headers: important'));
    expect(presentation.parameterSummary, contains('user_agent: agent'));
    expect(presentation.icon, Icons.description_outlined);
  });

  test('does not infer actions or targets from undeclared tool names', () {
    for (final tool in ['read_file', 'custom:delete_record', 'visit_web']) {
      final presentation = ToolPresentation.fromCall(
        tool,
        '<param name="path">literal</param>',
        result: 'important result',
        isSuccess: true,
      );
      expect(presentation.display, isNull);
      expect(presentation.target, isEmpty);
      expect(presentation.icon, Icons.build_outlined);
      expect(presentation.hasDetails, isTrue);
      expect(presentation.result, 'important result');
    }
  });

  test('directory prose never becomes directory data', () {
    final presentation = ToolPresentation.fromResult(
      'read_file',
      'Directory listing for /sample:\n-rw-r--r-- 12 name="example.txt"',
      isSuccess: true,
    );
    expect(presentation.resultData, isNull);
    expect(presentation.result, contains('Directory listing for'));
  });

  test(
    'declared file diffs work with plugin-owned payloads and arbitrary names',
    () {
      final presentation = ToolPresentation.fromResult(
        'custom:z42',
        'Changed',
        isSuccess: true,
        display: const ToolDisplayDeclaration(
          labels: {'en': 'Edit'},
          resultRenderer: ToolResultRenderer.fileDiff,
        ),
        resultData: {
          'operation': {'path': '/host/a.dart', 'details': 'Updated'},
          'diffContent': '@@ -1 +1 @@\n-old\n+new',
        },
      );
      expect(presentation.fileDiffs.single.path, '/host/a.dart');
      expect(presentation.fileDiffs.single.diffContent, contains('+new'));
    },
  );

  test(
    'result schemas and type names never select a renderer without a declaration',
    () {
      for (final display in <ToolDisplayDeclaration?>[
        null,
        const ToolDisplayDeclaration(labels: {'en': 'Generic'}),
      ]) {
        final diff = ToolPresentation.fromResult(
          'apply_file',
          '<file-diff path="a">+line</file-diff>',
          isSuccess: true,
          display: display,
          resultData: {
            '__type': 'FileApplyResultData',
            'operation': {'path': 'a'},
            'diffContent': '+line',
          },
        );
        expect(diff.fileDiffs, isEmpty);
        final directory = ToolPresentation.fromResult(
          'list_files',
          'Directory listing',
          isSuccess: true,
          display: display,
          resultData: {
            '__type': 'DirectoryListingData',
            'path': '/remote',
            'entries': [],
          },
        );
        expect(directory.hasDirectoryListing, isFalse);
      }
    },
  );

  test(
    'metadata decodes raw events and XML without changing literal entities',
    () {
      final data = {
        'label': {'en': 'Read &quot;literal&quot; & more'},
        'icon': 'file',
      };
      final raw = jsonEncode(data);
      final escaped = raw.replaceAll('&', '&amp;').replaceAll('"', '&quot;');
      expect(decodeToolMetadata(raw), data);
      expect(decodeToolMetadata(escaped), data);
      expect(decodeToolMetadata('not metadata'), isNull);
    },
  );

  test('malformed display attributes fall back without throwing', () {
    for (final data in [
      {'label': 42},
      {
        'label': {'en': 42},
      },
      {
        'label': {'en': 'Read'},
        'icon': [],
      },
      {
        'label': {'en': 'Read'},
        'targetParameter': false,
      },
    ]) {
      expect(toolDisplayFromAttributes({'display': jsonEncode(data)}), isNull);
    }
  });

  test('malformed native data retains generic result presentation', () {
    expect(
      isDirectoryListingData({
        '__type': 'DirectoryListingData',
        'path': 42,
        'entries': [],
      }),
      isFalse,
    );
    final presentation = ToolPresentation.fromResult(
      'arbitrary',
      'complete text',
      isSuccess: true,
      resultData: {
        '__type': 'FileApplyResultData',
        'operation': {'path': false},
        'diffContent': '+line',
      },
    );
    expect(presentation.fileDiffs, isEmpty);
    expect(presentation.result, 'complete text');
  });

  testWidgets(
    'standalone calls read display from native events rather than raw markup',
    (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StreamMarkdownRenderer(
              content:
                  '<tool name="example:z42"><param name="document">/remote/a</param></tool>',
              isStreaming: false,
              textColor: Colors.black,
              backgroundColor: Colors.white,
              splitMarkdownContent: (content) async {
                final events = await splitMarkdownEventFixture(content);
                return events.map((event) {
                  if (event.xml == null) return event;
                  return MarkdownStreamEvent.fromJson({
                    ...event.toJson(),
                    'xml': {
                      ...event.xml!.toJson(),
                      'attributes': {
                        'name': 'example:z42',
                        'display': jsonEncode({
                          'label': {'en': 'Declared action'},
                          'icon': 'file',
                          'targetParameter': 'document',
                        }),
                      },
                    },
                  });
                }).toList();
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Declared action'), findsOneWidget);
      expect(find.text('/remote/a'), findsOneWidget);
    },
  );

  testWidgets('typed directory results render independently of tool names', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ToolCallRow(
            isStreaming: false,
            presentation: ToolPresentation.fromResult(
              'example:z42',
              'server-defined prose',
              isSuccess: true,
              display: const ToolDisplayDeclaration(
                labels: {'en': 'Read'},
                icon: 'folder',
                resultRenderer: ToolResultRenderer.directory,
              ),
              resultData: {
                'path': '/different-host',
                'entries': [
                  {'name': 'document.txt', 'isDirectory': false, 'size': 42},
                ],
              },
            ),
          ),
        ),
      ),
    );
    expect(find.text('Read'), findsOneWidget);
    await tester.tap(find.text('Read'));
    await tester.pumpAndSettle();
    expect(find.byType(DirectoryListingDisplay), findsOneWidget);
    expect(find.text('document.txt'), findsOneWidget);
    expect(find.text('/different-host'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}
