import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/common/components/OperitDialog.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

// The production TS entry generates this fixture through the real SDK.
// Reference geometry comes from main's CharacterCardEditorDialog.dart and
// OperitDialogScaffold, without importing or reusing native character business UI.
final fixture =
    jsonDecode(
          File('test/fixtures/native_character_editor.json').readAsStringSync(),
        )
        as Map<String, dynamic>;
final png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=',
);

Widget theme(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: child,
);
Widget reference() => Builder(
  builder: (context) {
    Widget field(String label, String value, [int lines = 1]) => Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: TextFormField(
        initialValue: value,
        style: Theme.of(context).textTheme.bodyMedium,
        maxLines: lines,
        decoration: InputDecoration(
          labelText: label,
          suffixIconConstraints: lines == 1
              ? null
              : const BoxConstraints.tightFor(width: 36, height: 36),
          suffixIcon: lines == 1
              ? null
              : IconButton(
                  onPressed: () {},
                  iconSize: 18,
                  visualDensity: VisualDensity.compact,
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints.tightFor(
                    width: 28,
                    height: 28,
                  ),
                  icon: const Icon(Icons.fullscreen),
                ),
        ),
      ),
    );
    return OperitDialogScaffold(
      title: '编辑角色卡',
      maxWidth: 760,
      maxHeight: 820,
      showCloseButton: true,
      onClose: () {},
      contentPadding: EdgeInsets.zero,
      actions: [
        TextButton(onPressed: () {}, child: const Text('删除')),
        TextButton(onPressed: () {}, child: const Text('导出')),
        TextButton(onPressed: () {}, child: const Text('取消')),
        FilledButton(onPressed: () {}, child: const Text('保存')),
      ],
      child: DefaultTabController(
        length: 3,
        child: Column(
          children: [
            const TabBar(
              tabs: [
                Tab(text: '基础'),
                Tab(text: '内容'),
                Tab(text: '绑定'),
              ],
            ),
            Expanded(
              child: TabBarView(
                children: [
                  SingleChildScrollView(
                    padding: const EdgeInsets.fromLTRB(18, 10, 18, 10),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        field('角色名称', '原生角色卡'),
                        field('描述', '来自 main 的对照布局'),
                        Padding(
                          padding: const EdgeInsets.only(bottom: 12),
                          child: InputDecorator(
                            decoration: const InputDecoration(
                              labelText: '角色头像',
                            ),
                            child: Row(
                              children: [
                                SizedBox(
                                  width: 44,
                                  height: 44,
                                  child: ClipOval(
                                    child: Image.memory(png, fit: BoxFit.cover),
                                  ),
                                ),
                                const SizedBox(width: 12),
                                Expanded(
                                  child: Text(
                                    '未设置',
                                    style: Theme.of(
                                      context,
                                    ).textTheme.bodyMedium,
                                  ),
                                ),
                                const SizedBox(width: 8),
                                TextButton(
                                  onPressed: () {},
                                  child: const Text('选择'),
                                ),
                              ],
                            ),
                          ),
                        ),
                        field('角色设定', '角色设定正文', 6),
                        field('开场白', '你好。', 3),
                        Padding(
                          padding: const EdgeInsets.only(bottom: 12),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                children: [
                                  const Expanded(
                                    child: Text(
                                      '标签',
                                      style: TextStyle(
                                        fontWeight: FontWeight.w700,
                                      ),
                                    ),
                                  ),
                                  TextButton.icon(
                                    onPressed: () {},
                                    icon: const Icon(
                                      Icons.tune_outlined,
                                      size: 18,
                                    ),
                                    label: const Text('管理标签'),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 6),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox.shrink(),
                  const SizedBox.shrink(),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  },
);

Map<String, Rect> geometry(WidgetTester tester) {
  final result = <String, Rect>{
    'dialog': tester.getRect(find.byType(Dialog)),
    'tabs': tester.getRect(find.byType(TabBar)),
    'title': tester.getRect(find.text('编辑角色卡')),
    'close': tester.getRect(find.byIcon(Icons.close).first),
    'save': tester.getRect(find.widgetWithText(FilledButton, '保存')),
  };
  for (final label in ['角色名称', '描述', '角色头像', '角色设定', '开场白']) {
    final match = find.byWidgetPredicate(
      (widget) =>
          widget is InputDecorator && widget.decoration.labelText == label,
    );
    if (match.evaluate().isNotEmpty)
      result[label] = tester.getRect(match.first);
  }
  return result;
}

void main() {
  testWidgets('retains the actual TS editor while the window resizes', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(1100, 950);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);
    final host = ComposeDslWebViewHostContext(
      packageName: 'com.operit.character_cards',
      routeInstanceId: 'native-resize',
      executionContextKey: 'native-resize',
      dispatchAction: (id, [payload]) async => null,
      runtimeOptionsProvider: () => {},
      readFileBytes: (_) async => png,
    );
    await tester.pumpWidget(
      theme(
        buildComposeDslLayoutForTest(
          node: fixture,
          hostContext: host,
          splitMarkdownContent: (_) async => [],
          onTextInput: (_, text) async => null,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    final editor = tester.state(find.byType(EditableText).first);
    await tester.enterText(find.byType(EditableText).first, '窗口缩放保留的输入');
    await tester.pump();
    for (final size in [
      const Size(800, 850),
      const Size(390, 844),
      const Size(1200, 1000),
    ]) {
      tester.view.physicalSize = size;
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(tester.state(find.byType(EditableText).first), same(editor));
      expect(find.text('窗口缩放保留的输入'), findsOneWidget);
      final rects = geometry(tester);
      expect(rects['角色名称']!.width, greaterThan(0));
      expect(rects['角色名称']!.width, lessThan(size.width));
      expect(rects['dialog']!.width, lessThanOrEqualTo(size.width));
    }
  });

  for (final size in [const Size(1100, 950), const Size(390, 844)]) {
    testWidgets('floating name label fits the actual app theme at $size', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = size;
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final host = ComposeDslWebViewHostContext(
        packageName: 'com.operit.character_cards',
        routeInstanceId: 'native-label',
        executionContextKey: 'native-label',
        dispatchAction: (id, [payload]) async => null,
        runtimeOptionsProvider: () => {},
        readFileBytes: (_) async => png,
      );
      // Inherit Operit's outlined input theme without a nested MaterialApp.
      await tester.pumpWidget(theme(reference()));
      await tester.pumpAndSettle();
      final expectedName = geometry(tester)['角色名称'];
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      await tester.pumpWidget(
        theme(
          buildComposeDslLayoutForTest(
            node: fixture,
            hostContext: host,
            splitMarkdownContent: (_) async => [],
            onTextInput: (_, text) async => null,
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(geometry(tester)['角色名称'], expectedName);
      for (final focused in [false, true]) {
        if (focused) {
          await tester.tap(find.byType(EditableText).first);
          await tester.pumpAndSettle();
        }
        final labelBox = tester.renderObject<RenderBox>(find.text('角色名称'));
        final paintedLabel = MatrixUtils.transformRect(
          labelBox.getTransformTo(null),
          Offset.zero & labelBox.size,
        );
        final viewportBox = RenderAbstractViewport.of(labelBox) as RenderBox;
        final viewport = MatrixUtils.transformRect(
          viewportBox.getTransformTo(null),
          Offset.zero & viewportBox.size,
        );
        expect(
          paintedLabel.top,
          greaterThanOrEqualTo(viewport.top),
          reason:
              'The floating label must fit inside the scrolling clip (focused=$focused).',
        );
        expect(paintedLabel.bottom, lessThanOrEqualTo(viewport.bottom));
      }
    });

    testWidgets('actual TS DSL matches main dialog geometry at $size', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = size;
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      await tester.pumpWidget(theme(MaterialApp(home: reference())));
      await tester.pumpAndSettle();
      final expected = geometry(tester);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      final actions = <String>[];
      final host = ComposeDslWebViewHostContext(
        packageName: 'com.operit.character_cards',
        routeInstanceId: 'native',
        executionContextKey: 'native',
        dispatchAction: (id, [payload]) async {
          actions.add(id);
          return null;
        },
        runtimeOptionsProvider: () => {},
        readFileBytes: (_) async => png,
      );
      await tester.pumpWidget(
        theme(
          MaterialApp(
            home: buildComposeDslLayoutForTest(
              node: fixture,
              hostContext: host,
              splitMarkdownContent: (_) async => [],
              onTextInput: (_, text) async => null,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      final actual = geometry(tester);
      for (final entry in expected.entries) {
        expect(
          actual[entry.key],
          entry.value,
          reason: 'main geometry mismatch: ${entry.key}',
        );
      }
      expect(find.byType(AlertDialog), findsNothing);
      expect(find.byType(PageView), findsOneWidget);
      await tester.tap(find.text('绑定'));
      await tester.pumpAndSettle();
      expect(actions, isNotEmpty);
      expect(tester.takeException(), isNull);
    });
  }
}
