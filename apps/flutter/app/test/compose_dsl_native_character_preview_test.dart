import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

import 'compose_dsl_native_character_parity_test.dart' as editor;

/// Reads the real TS route's SDK output, including its actual preview actions.
Map<String, dynamic> _fixture(String name) {
  final root =
      jsonDecode(File('test/fixtures/$name.json').readAsStringSync())
          as Map<String, dynamic>;
  // Model the installed archive's VFS avatar, served by the real image loader.
  void visit(Map<String, dynamic> node) {
    if (node['type'] == 'Image') {
      final props = node['props'] as Map<String, dynamic>;
      if (props['path'] != null)
        props['path'] = '/app/preview/operit-avatar.png';
    }
    for (final child in node['children'] as List) {
      visit(child as Map<String, dynamic>);
    }
    for (final slot in (node['slots'] as Map<String, dynamic>).values) {
      for (final child in slot as List) {
        visit(child as Map<String, dynamic>);
      }
    }
  }

  visit(root);
  return root;
}

Finder _readingScrollable() => find
    .descendant(
      of: find.byWidgetPredicate((widget) => widget is ListView),
      matching: find.byType(Scrollable),
    )
    .first;

void main() {
  final screenshotDirectory =
      Platform.environment['OPERIT_PREVIEW_SCREENSHOTS'];
  setUpAll(() async {
    // Render readable Chinese in optional local review captures on macOS.
    if (screenshotDirectory != null) {
      final icons = File(
        '.fvm/flutter_sdk/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
      );
      if (icons.existsSync()) {
        await (FontLoader('MaterialIcons')..addFont(
              Future.value(ByteData.sublistView(icons.readAsBytesSync())),
            ))
            .load();
      }
      final font = File('/System/Library/Fonts/STHeiti Light.ttc');
      if (font.existsSync()) {
        await (FontLoader('Aptos')..addFont(
              Future.value(ByteData.sublistView(font.readAsBytesSync())),
            ))
            .load();
      }
    }
  });

  for (final entity in ['character', 'group']) {
    for (final size in [const Size(1100, 950), const Size(390, 844)]) {
      testWidgets('real TS $entity preview reads and scrolls at $size', (
        tester,
      ) async {
        tester.view.devicePixelRatio = 1;
        tester.view.physicalSize = size;
        addTearDown(tester.view.resetDevicePixelRatio);
        addTearDown(tester.view.resetPhysicalSize);
        final avatar = File(
          '../../../plugins/packages/buildin/character_cards/resources/operit-avatar.png',
        ).readAsBytesSync();
        final actions = <String>[];
        final host = ComposeDslWebViewHostContext(
          packageName: 'com.operit.character_cards',
          routeInstanceId: 'preview-$entity',
          executionContextKey: 'preview-$entity',
          dispatchAction: (id, [payload]) async {
            actions.add(id);
            return null;
          },
          runtimeOptionsProvider: () => {},
          readFileBytes: (_) async => avatar,
        );
        const captureKey = ValueKey('preview-capture');
        await tester.pumpWidget(
          RepaintBoundary(
            key: captureKey,
            child: editor.theme(
              buildComposeDslLayoutForTest(
                node: _fixture('native_${entity}_preview'),
                hostContext: host,
                splitMarkdownContent: (_) async => [],
                onTextInput: (_, text) async => null,
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        await tester.runAsync(
          () => precacheImage(
            MemoryImage(avatar),
            tester.element(find.byType(Dialog)),
          ),
        );
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(
          find.byType(Image),
          findsNWidgets(entity == 'character' ? 1 : 2),
        );
        expect(find.byType(Dialog), findsOneWidget);
        expect(find.byType(PageView), findsNothing);
        expect(find.byType(TabBar), findsNothing);
        expect(find.byType(EditableText), findsNothing);
        final name = find.text(entity == 'character' ? '温和的伙伴' : '一起想想');
        expect(name, findsOneWidget);
        final nameRect = tester.getRect(name);
        expect(nameRect.width, greaterThan(60));
        expect(nameRect.left, greaterThanOrEqualTo(0));
        expect(nameRect.right, lessThanOrEqualTo(size.width));
        expect(find.widgetWithText(FilledButton, '编辑'), findsOneWidget);
        expect(tester.getRect(find.text('关闭')).bottom, lessThan(size.height));
        if (entity == 'character') {
          expect(find.text('开场白'), findsOneWidget);
          expect(find.text('角色设定'), findsOneWidget);
          expect(find.text('温和可靠'), findsOneWidget);
          expect(find.text('未设置'), findsNothing);
        } else {
          expect(find.text('2 位角色'), findsOneWidget);
          expect(
            tester.getRect(find.text('温和的倾听者')).top,
            lessThan(tester.getRect(find.text('理性的思考者')).top),
          );
        }
        if (screenshotDirectory != null) {
          final boundary = tester.renderObject<RenderRepaintBoundary>(
            find.byKey(captureKey),
          );
          await tester.runAsync(() async {
            final image = await boundary.toImage(pixelRatio: 2);
            final data = await image.toByteData(format: ui.ImageByteFormat.png);
            await Directory(screenshotDirectory).create(recursive: true);
            await File(
              '$screenshotDirectory/$entity-${size.width.toInt()}.png',
            ).writeAsBytes(data!.buffer.asUint8List());
            image.dispose();
          });
        }
        await tester.scrollUntilVisible(
          find.text('配置摘要'),
          160,
          scrollable: _readingScrollable(),
        );
        await tester.tap(find.text('配置摘要'));
        await tester.pump();
        expect(actions, hasLength(1));
        expect(tester.takeException(), isNull);
        tester.view.physicalSize = const Size(600, 850);
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(find.widgetWithText(FilledButton, '编辑'), findsOneWidget);
      });
    }
  }

  testWidgets(
    'expanded configuration uses readable names and remains scrollable',
    (tester) async {
      final host = ComposeDslWebViewHostContext(
        packageName: 'com.operit.character_cards',
        routeInstanceId: 'preview-expanded',
        executionContextKey: 'preview-expanded',
        dispatchAction: (id, [payload]) async => null,
        runtimeOptionsProvider: () => {},
        readFileBytes: (_) async => editor.png,
      );
      await tester.pumpWidget(
        editor.theme(
          buildComposeDslLayoutForTest(
            node: _fixture('native_character_preview_expanded'),
            hostContext: host,
            splitMarkdownContent: (_) async => [],
            onTextInput: (_, text) async => null,
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text('工具访问'),
        200,
        scrollable: _readingScrollable(),
      );
      expect(find.text('日常对话记忆'), findsOneWidget);
      expect(find.text('日常对话记忆（可读）'), findsOneWidget);
      expect(find.text('测试语音'), findsOneWidget);
      expect(find.text('DashScope · qwen-plus'), findsOneWidget);
      expect(find.byType(EditableText), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );
}
