import 'dart:convert';
import 'dart:io';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/code_editor/CodeEditor.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorInput.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorText.dart';
import 'package:operit2/ui/common/code_editor/CodeTextLayout.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorZoomTransform.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Exercises the real plugin renderer and its shared lazy-list scroll controller.
Widget _review({
  bool split = false,
  double width = 400,
  double textScale = 1,
  Map<String, Object?>? node,
}) => MaterialApp(
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(
      context,
    ).copyWith(textScaler: TextScaler.linear(textScale)),
    child: child!,
  ),
  home: Scaffold(
    body: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: width,
        height: 300,
        child: buildComposeDslLayoutForTest(
          node:
              node ??
              {
                'type': 'CodeEditor',
                'props': {
                  'contentWidth': 1200.0,
                  'wrapToViewport': false,
                  'fillMaxSize': true,
                },
                'children': [
                  {
                    'type': 'LazyColumn',
                    'props': {'fillMaxSize': true},
                    'children': [
                      for (var index = 0; index < 1000; index++)
                        {
                          'type': 'Box',
                          'props': {'width': 1200.0, 'height': 22.0},
                          'children': [
                            {
                              'type': split ? 'Row' : 'Box',
                              'children': [
                                {
                                  'type': 'Text',
                                  'props': {'text': 'line $index'},
                                },
                                if (split)
                                  {
                                    'type': 'Text',
                                    'props': {'text': 'right $index'},
                                  },
                              ],
                            },
                          ],
                        },
                    ],
                  },
                ],
              },
          hostContext: ComposeDslWebViewHostContext(
            packageName: 'com.operit.change_review',
            routeInstanceId: 'viewport-test',
            executionContextKey: 'viewport-test',
            dispatchAction: (_, [payload]) async => null,
            runtimeOptionsProvider: () => {},
          ),
          splitMarkdownContent: (_) async => [],
          onTextInput: (_, _) async => null,
        ),
      ),
    ),
  ),
);

CodeEditorZoomTransform _zoom(WidgetTester tester) =>
    tester.widget(find.byType(CodeEditorZoomTransform));
ScrollPosition _position(WidgetTester tester) => tester
    .state<ScrollableState>(
      find.descendant(
        of: find.byType(CodeEditorViewport),
        matching: find.byType(Scrollable),
      ),
    )
    .position;

Iterable<Map> _nodes(Map node) sync* {
  yield node;
  for (final child in node['children'] as List? ?? const []) {
    yield* _nodes(child as Map);
  }
  for (final slot in (node['slots'] as Map? ?? const {}).values) {
    for (final child in slot as List) {
      yield* _nodes(child as Map);
    }
  }
}

void main() {
  final fixtures =
      jsonDecode(
            File(
              'test/fixtures/change_review_workspace.json',
            ).readAsStringSync(),
          )
          as Map;
  testWidgets('actual source preview uses the shared read-only editor', (
    tester,
  ) async {
    await tester.pumpWidget(
      _review(
        node: Map<String, Object?>.from(fixtures['light-preview'] as Map),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(CodeEditor), findsOneWidget);
    final input = tester.widget<CodeEditorInput>(find.byType(CodeEditorInput));
    expect(input.readOnly, isTrue);
    expect(input.controller.text, 'tree file body');
    final focal = tester.getCenter(find.byType(CodeEditorViewport));
    tester.binding.handlePointerEvent(
      PointerPanZoomStartEvent(pointer: 5, position: focal),
    );
    tester.binding.handlePointerEvent(
      PointerPanZoomUpdateEvent(pointer: 5, position: focal, scale: 1.5),
    );
    await tester.pump();
    tester.binding.handlePointerEvent(
      PointerPanZoomEndEvent(pointer: 5, position: focal),
    );
    expect(_zoom(tester).scale, 1.5);
    expect(tester.testTextInput.hasAnyClients, isFalse);
    expect(tester.takeException(), isNull);
  });
  for (final name in [
    'light-unified',
    'dark-split',
    'light-wrap',
    'dark-split-wrap',
    'light-gap',
    'dark-full',
    'light-directory-tree',
    'dark-resized-tree',
  ]) {
    testWidgets('actual review fixture zooms without moving controls: $name', (
      tester,
    ) async {
      final tree = Map<String, Object?>.from(fixtures[name] as Map);
      final actions = <String>[];
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: buildComposeDslLayoutForTest(
              node: tree,
              hostContext: ComposeDslWebViewHostContext(
                packageName: 'com.operit.change_review',
                routeInstanceId: name,
                executionContextKey: name,
                dispatchAction: (id, [payload]) async {
                  actions.add(id);
                  return null;
                },
                runtimeOptionsProvider: () => {},
              ),
              splitMarkdownContent: (_) async => [],
              onTextInput: (_, _) async => null,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      final control = find.byTooltip('分栏视图 (Split)');
      final controlRect = tester.getRect(control);
      final viewportRect = tester.getRect(find.byType(CodeEditorViewport));
      final focal = viewportRect.center;
      tester.binding.handlePointerEvent(
        PointerPanZoomStartEvent(pointer: 5, position: focal),
      );
      tester.binding.handlePointerEvent(
        PointerPanZoomUpdateEvent(pointer: 5, position: focal, scale: 1.5),
      );
      await tester.pump();
      tester.binding.handlePointerEvent(
        PointerPanZoomEndEvent(pointer: 5, position: focal),
      );
      expect(_zoom(tester).scale, 1.5);
      expect(tester.getRect(control), controlRect);
      if (name == 'dark-split-wrap') {
        final splitter = _nodes(tree).singleWhere(
          (node) => (node['props'] as Map?)?['key'] == 'diff-splitter',
        );
        final ops =
            ((splitter['props'] as Map)['modifier'] as Map)['__modifierOps']
                as List;
        final gesture =
            (ops.singleWhere((op) => op['name'] == 'dragGestures')['args']
                        as List)
                    .single
                as Map;
        final handle = tester.getCenter(
          find.descendant(
            of: find.byType(CodeEditorViewport),
            matching: find.byWidgetPredicate(
              (widget) =>
                  widget is MouseRegion &&
                  widget.cursor == SystemMouseCursors.resizeColumn,
            ),
          ),
        );
        final pointer = await tester.startGesture(handle);
        await pointer.moveBy(const Offset(40, 0));
        await tester.pump();
        await pointer.moveBy(const Offset(20, 0));
        await pointer.up();
        await tester.pumpAndSettle();
        expect(actions, contains((gesture['onDrag'] as Map)['__actionId']));
      }
      expect(tester.takeException(), isNull);
    });
  }

  for (final split in [false, true]) {
    for (final wrap in [false, true]) {
      testWidgets(
        'actual Chinese diff reflows at viewport width: split=$split wrap=$wrap',
        (tester) async {
          final tree =
              fixtures['light-chinese-${split ? 'split' : 'unified'}-${wrap ? 'wrap' : 'nowrap'}']
                  as Map;
          final editor = _nodes(
            tree,
          ).singleWhere((node) => node['type'] == 'CodeEditor');
          for (final (width, textScale)
              in (split
                  ? [(360.0, 1.0), (480.0, 1.6)]
                  : [(240.0, 1.0), (400.0, 1.6)])) {
            await tester.pumpWidget(
              _review(
                width: width,
                textScale: textScale,
                node: Map<String, Object?>.from(editor),
              ),
            );
            await tester.pumpAndSettle();
            final editorState = tester.state<CodeEditorState>(
              find.byType(CodeEditor),
            );
            for (final scale in [0.6, 1.0, 1.5, 2.5]) {
              editorState.zoomTo(scale);
              await tester.pumpAndSettle();
              expect(tester.takeException(), isNull);
              final transform = _zoom(tester);
              final body = tester.renderObject<RenderBox>(
                find.descendant(
                  of: find.byType(CodeEditorZoomTransform),
                  matching: find.byType(Scrollable),
                ),
              );
              if (wrap) {
                expect(body.size.width * scale, closeTo(width, 0.01));
                expect(transform.horizontalOffset, 0);
              } else {
                expect(body.size.width, greaterThanOrEqualTo(width / scale));
              }
              var checked = 0;
              for (final element in find.byType(CodeEditorText).evaluate()) {
                final source = element.widget as CodeEditorText;
                if (source.filePath.isEmpty) continue;
                final paragraphFinder = find.descendant(
                  of: find.byWidget(source),
                  matching: find.byType(RichText),
                );
                final paragraph = tester.renderObject<RenderParagraph>(
                  paragraphFinder,
                );
                expect(
                  paragraph.constraints.maxWidth,
                  closeTo(
                    (body.size.width - (split ? 1 : 0)) * (split ? 0.5 : 1) -
                        (split ? 51 : 53),
                    0.01,
                  ),
                  reason: 'Gutter space is subtracted exactly once',
                );
                final boxes = paragraph.getBoxesForSelection(
                  TextSelection(
                    baseOffset: 0,
                    extentOffset: source.text.length,
                  ),
                );
                final layout = CodeTextLayout(source.text);
                layout.configure(
                  wrap ? paragraph.constraints.maxWidth : double.infinity,
                  source.style,
                  MediaQuery.textScalerOf(element),
                  Directionality.of(element),
                  locale: Localizations.maybeLocaleOf(element),
                );
                final native = layout.paragraph(0);
                expect(
                  paragraph.size.height,
                  closeTo(native.height, 0.01),
                  reason: 'Shared editor line height: ${source.text} at $scale',
                );
                final lineTops = boxes.map((box) => box.top).toSet();
                if (source.text.isNotEmpty) {
                  expect(
                    lineTops.length,
                    native.computeLineMetrics().length,
                    reason: 'Same wrap as editor: ${source.text} at $scale',
                  );
                }
                final visibleBoxes = [
                  for (var i = 0; i < source.text.length; i++)
                    if (source.text[i].trim().isNotEmpty)
                      ...paragraph.getBoxesForSelection(
                        TextSelection(baseOffset: i, extentOffset: i + 1),
                      ),
                ];
                for (final box in visibleBoxes) {
                  expect(
                    box.right,
                    lessThanOrEqualTo(paragraph.constraints.maxWidth + 0.01),
                    reason:
                        'Every glyph fits its laid-out column: ${source.text} at width=$width scale=$scale',
                  );
                  if (wrap) {
                    final right = paragraph
                        .localToGlobal(Offset(box.right, box.bottom))
                        .dx;
                    expect(right, lessThanOrEqualTo(width + 0.01));
                  }
                }
                layout.dispose();
                checked++;
              }
              expect(checked, greaterThan(0));
            }
          }
        },
      );
    }
  }

  for (final split in [false, true]) {
    testWidgets('diagonal drag moves both axes in split=$split', (
      tester,
    ) async {
      await tester.pumpWidget(_review(split: split));
      await tester.pumpAndSettle();
      expect(find.text('line 900'), findsNothing);
      await tester.dragFrom(const Offset(240, 180), const Offset(-100, -110));
      await tester.pumpAndSettle();
      expect(_zoom(tester).horizontalOffset, lessThan(-50));
      expect(_position(tester).pixels, greaterThan(50));
      expect(_zoom(tester).scale, 1);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('trackpad zoom keeps the focal point anchored while panning', (
    tester,
  ) async {
    await tester.pumpWidget(_review());
    await tester.pumpAndSettle();
    const focal = Offset(200, 150);
    tester.binding.handlePointerEvent(
      const PointerPanZoomStartEvent(pointer: 5, position: focal),
    );
    tester.binding.handlePointerEvent(
      const PointerPanZoomUpdateEvent(
        pointer: 5,
        position: focal,
        scale: 1.5,
        pan: Offset(-30, -40),
      ),
    );
    await tester.pump();
    expect(_zoom(tester).scale, closeTo(1.5, 0.001));
    expect(_zoom(tester).horizontalOffset, closeTo(-130, 0.01));
    expect(_position(tester).pixels, closeTo(150 - 110 / 1.5, 0.01));
    tester.binding.handlePointerEvent(
      const PointerPanZoomEndEvent(pointer: 5, position: focal),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('two touch pointers zoom and a remaining finger can pan', (
    tester,
  ) async {
    await tester.pumpWidget(_review());
    await tester.pumpAndSettle();
    final first = await tester.startGesture(const Offset(120, 150), pointer: 1);
    final second = await tester.startGesture(
      const Offset(280, 150),
      pointer: 2,
    );
    await first.moveTo(const Offset(100, 150));
    await second.moveTo(const Offset(300, 150));
    await tester.pump();
    await first.moveTo(const Offset(60, 150));
    await second.moveTo(const Offset(340, 150));
    await tester.pump();
    expect(_zoom(tester).scale, greaterThan(1.2));
    await second.up();
    await tester.pump();
    final before = _position(tester).pixels;
    await first.moveBy(const Offset(-30, -70));
    await tester.pump();
    expect(_position(tester).pixels, greaterThan(before));
    await first.up();
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('wheel supports both axes and modifier zoom obeys bounds', (
    tester,
  ) async {
    await tester.pumpWidget(_review());
    await tester.pumpAndSettle();
    void wheel(Offset delta) => tester.binding.handlePointerEvent(
      PointerScrollEvent(position: const Offset(200, 150), scrollDelta: delta),
    );
    wheel(const Offset(65, 90));
    await tester.pump();
    expect(_zoom(tester).horizontalOffset, -65);
    expect(_position(tester).pixels, 90);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    wheel(const Offset(0, -1000));
    await tester.pump();
    expect(_zoom(tester).scale, 2.5);
    wheel(const Offset(0, 2000));
    await tester.pump();
    expect(_zoom(tester).scale, 0.6);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    wheel(const Offset(10000, 100000));
    await tester.pump();
    expect(_zoom(tester).horizontalOffset, closeTo(400 - 1200 * 0.6, 0.01));
    expect(_position(tester).pixels, _position(tester).maxScrollExtent);
    expect(find.text('line 999'), findsOneWidget);
    wheel(const Offset(-10000, -100000));
    await tester.pump();
    expect(_zoom(tester).horizontalOffset, 0);
    expect(_position(tester).pixels, 0);
    expect(tester.takeException(), isNull);
  });
}
