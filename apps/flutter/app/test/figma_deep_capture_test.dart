import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit_ui/operit_ui.dart';

void main() {
  testWidgets(
    'real expanded tool result and retained plugin UI export editable text with owner identity',
    (tester) async {
      expect(
        operitDesignBridgeEnabled,
        isTrue,
        reason: 'Run with --dart-define=OPERIT_DESIGN_BRIDGE=true',
      );
      final key = GlobalKey();
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: RepaintBoundary(
            key: key,
            child: Scaffold(
              body: Column(
                children: [
                  ToolCallRow(
                    presentation: ToolPresentation.fromResult(
                      'read_file',
                      'Deep tool payload: README.md',
                      isSuccess: true,
                    ),
                    isStreaming: false,
                  ),
                  buildComposeDslLayoutForTest(
                    node: {
                      'type': 'Column',
                      'children': [
                        {
                          'type': 'Card',
                          'children': [
                            {
                              'type': 'Text',
                              'props': {'text': 'Nested plugin controls'},
                            },
                          ],
                        },
                      ],
                    },
                    hostContext: ComposeDslWebViewHostContext(
                      packageName: 'org.operit.design-fixture',
                      routeInstanceId: 'settings',
                      executionContextKey: 'design-test',
                      dispatchAction: (id, [payload]) async => null,
                      runtimeOptionsProvider: () => {},
                    ),
                    splitMarkdownContent: (_) async => [],
                    onTextInput: (id, text) async => null,
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('read_file'));
      await tester.pumpAndSettle();
      expect(find.textContaining('Deep tool payload'), findsOneWidget);
      final document = await tester.runAsync(
        () => captureOperitDesign(
          key.currentContext!.findRenderObject()! as RenderRepaintBoundary,
          name: 'Real tools and plugin',
        ),
      );
      final textNodes = <Map<String, Object?>>[];
      void visit(Map<String, Object?> n) {
        if (n['type'] == 'text') textNodes.add(n);
        for (final c in n['children'] as List? ?? []) {
          visit(c as Map<String, Object?>);
        }
      }

      for (final scene in document!['scenes'] as List) {
        visit(scene as Map<String, Object?>);
      }
      final tool = textNodes.singleWhere(
        (n) => n['text'].toString().contains('Deep tool payload'),
      );
      final plugin = textNodes.singleWhere(
        (n) => n['text'] == 'Nested plugin controls',
      );
      expect((tool['source'] as Map)['tool'], 'read_file');
      expect((plugin['source'] as Map)['package'], 'org.operit.design-fixture');
      expect((plugin['source'] as Map)['composeId'], isNotEmpty);
      final file = File('.dart_tool/figma-deep-capture.operit-design.json');
      file.parent.createSync(recursive: true);
      file.writeAsStringSync(jsonEncode(document));
      expect(tester.takeException(), isNull);
    },
    skip: !operitDesignBridgeEnabled,
  );
  testWidgets(
    'runtime operates real tool expansion and plugin callbacks while retaining edits',
    (tester) async {
      final boundary = GlobalKey();
      var clicks = 0;
      final actions = <String>[];
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: RepaintBoundary(
            key: boundary,
            child: Scaffold(
              body: StatefulBuilder(
                builder: (context, setState) {
                  rebuild = setState;
                  return Column(
                    children: [
                      ToolCallRow(
                        presentation: ToolPresentation.fromResult(
                          'read_file',
                          'Live expanded tool payload',
                          isSuccess: true,
                        ),
                        isStreaming: false,
                      ),
                      buildComposeDslLayoutForTest(
                        node: {
                          'type': 'Column',
                          'children': [
                            {
                              'type': 'Text',
                              'props': {'text': 'Plugin count $clicks'},
                            },
                            {
                              'type': 'Button',
                              'props': {
                                'text': 'Run plugin action',
                                'onClick': 'increment',
                              },
                            },
                          ],
                        },
                        hostContext: ComposeDslWebViewHostContext(
                          packageName: 'org.operit.design-fixture',
                          routeInstanceId: 'live-settings',
                          executionContextKey: 'live-design-test',
                          dispatchAction: (id, [payload]) async {
                            actions.add(id);
                            rebuild(() => clicks++);
                            return null;
                          },
                          runtimeOptionsProvider: () => {},
                        ),
                        splitMarkdownContent: (_) async => [],
                        onTextInput: (id, text) async => null,
                      ),
                    ],
                  );
                },
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final runtime = OperitDesignRuntime(
        root: () =>
            boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary,
      );
      addTearDown(runtime.dispose);
      Future<List<Map<String, Object?>>> nodes() async {
        await tester.pumpAndSettle();
        final snapshot = (await tester.runAsync(runtime.snapshot))!;
        final result = <Map<String, Object?>>[];
        void visit(Map<String, Object?> node) {
          result.add(node);
          for (final child in node['children'] as List? ?? []) {
            visit(child as Map<String, Object?>);
          }
        }

        for (final scene in (snapshot['document'] as Map)['scenes'] as List) {
          visit(scene as Map<String, Object?>);
        }
        return result;
      }

      String target(Map<String, Object?> node) =>
          (node['source'] as Map)['runtimeTarget'] as String;
      var all = await nodes();
      runtime.dispatch({
        'type': 'tap',
        'target': target(all.singleWhere((n) => n['text'] == 'read_file')),
      });
      all = await nodes();
      expect(
        all.any(
          (n) => n['text'].toString().contains('Live expanded tool payload'),
        ),
        isTrue,
      );
      final label = all.singleWhere((n) => n['text'] == 'Plugin count 0');
      expect((label['source'] as Map)['package'], 'org.operit.design-fixture');
      runtime.edit(target(label), {'text': 'Designer plugin label'});
      all = await nodes();
      runtime.dispatch({
        'type': 'tap',
        'target': target(
          all.singleWhere((n) => n['text'] == 'Run plugin action'),
        ),
      });
      all = await nodes();
      expect(actions, ['increment']);
      expect(clicks, 1);
      final edited = all.singleWhere(
        (n) => n['text'] == 'Designer plugin label',
      );
      runtime.clear(target(edited));
      all = await nodes();
      expect(all.any((n) => n['text'] == 'Plugin count 1'), isTrue);
      expect(tester.takeException(), isNull);
    },
    skip: !operitDesignBridgeEnabled,
  );
  testWidgets(
    'registered DOM surface replaces only the platform subtree and preserves Flutter overlays',
    (tester) async {
      final key = GlobalKey();
      await tester.pumpWidget(
        MaterialApp(
          home: RepaintBoundary(
            key: key,
            child: Scaffold(
              body: Stack(
                children: [
                  Positioned(
                    left: 20,
                    top: 20,
                    width: 300,
                    height: 200,
                    child: OperitDesignSurface(
                      readScene: () async => {
                        'scene': {
                          'id': 'dom-root',
                          'name': 'Web plugin',
                          'type': 'frame',
                          'x': 0,
                          'y': 0,
                          'width': 300,
                          'height': 200,
                          'children': [
                            {
                              'id': 'dom-label',
                              'name': 'DOM text',
                              'type': 'text',
                              'x': 10,
                              'y': 10,
                              'width': 200,
                              'height': 30,
                              'text': 'Editable WebView content',
                              'source': {
                                'kind': 'plugin-webview',
                                'designId': 'plugin-title',
                              },
                            },
                          ],
                        },
                        'assets': [],
                        'warnings': [],
                      },
                      child: const ColoredBox(
                        color: Colors.blue,
                        child: Text('Platform placeholder'),
                      ),
                    ),
                  ),
                  const Positioned(
                    left: 30,
                    top: 40,
                    child: Text('Flutter progress overlay'),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      final document = await tester.runAsync(
        () => captureOperitDesign(
          key.currentContext!.findRenderObject()! as RenderRepaintBoundary,
          includeReference: false,
        ),
      );
      final exported = jsonEncode(document);
      expect(exported, contains('Editable WebView content'));
      expect(exported, contains('Flutter progress overlay'));
      expect(exported, contains('plugin-title'));
      expect(exported, isNot(contains('Platform placeholder')));
      await tester.pumpWidget(const SizedBox());
    },
    skip: !operitDesignBridgeEnabled,
  );
}
