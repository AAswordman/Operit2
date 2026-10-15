import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

final fixture =
    jsonDecode(File('test/fixtures/native_memory_menu.json').readAsStringSync())
        as Map<String, dynamic>;

// Exact reference from main's _MenuSection, _SwitchRow and _ActionRow.
// This test owns no memory business logic; actual plugin fixtures come from its registered SDK route.
Widget reference(bool expanded) => Builder(
  builder: (context) {
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    Widget switchRow(IconData icon, String title) => InkWell(
      onTap: () {},
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 32),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: [
              Icon(icon, size: 16, color: colors.primary),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: text.bodySmall!.copyWith(color: colors.onSurface),
                ),
              ),
              Text(
                '开',
                style: text.bodySmall!.copyWith(
                  color: colors.primary,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(width: 8),
              SizedBox(
                width: 40,
                height: 28,
                child: FittedBox(
                  child: Switch(
                    value: true,
                    onChanged: (_) {},
                    materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        InkWell(
          onTap: () {},
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 36),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                children: [
                  Icon(
                    Icons.data_object_outlined,
                    size: 16,
                    color: colors.onSurfaceVariant.withValues(alpha: 0.7),
                  ),
                  const SizedBox(width: 12),
                  Text('记忆', style: text.bodySmall),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      '用户资料 · 记忆库更新',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.end,
                      style: text.bodySmall!.copyWith(
                        color: colors.primary,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  IconButton(
                    tooltip: '记忆状态说明',
                    onPressed: () {},
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints.tightFor(
                      width: 24,
                      height: 28,
                    ),
                    style: IconButton.styleFrom(
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    ),
                    icon: Icon(
                      Icons.info_outline,
                      size: 16,
                      color: colors.onSurfaceVariant.withValues(alpha: 0.7),
                    ),
                  ),
                  const SizedBox(width: 6),
                  Icon(
                    expanded
                        ? Icons.keyboard_arrow_up
                        : Icons.keyboard_arrow_down,
                    size: 20,
                    color: colors.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),
        ),
        if (expanded)
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 0, 8, 4),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                switchRow(Icons.assignment_ind_outlined, '提供用户资料'),
                switchRow(Icons.save, '自动更新记忆库'),
                InkWell(
                  onTap: () {},
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(minHeight: 32),
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 12),
                      child: Row(
                        children: [
                          Icon(
                            Icons.save_outlined,
                            size: 16,
                            color: colors.onSurfaceVariant,
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Text(
                              '手动更新记忆',
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                              style: text.bodySmall!.copyWith(
                                color: colors.onSurface,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  },
);

Future<Uint8List> capture(
  WidgetTester tester,
  Widget child,
  Brightness brightness,
  GlobalKey key,
) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: ThemeData(brightness: brightness),
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: RepaintBoundary(
            key: key,
            child: SizedBox(
              width: 300,
              height: 160,
              child: ColoredBox(
                color: Colors.white,
                child: Align(alignment: Alignment.topLeft, child: child),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
  return (await tester.runAsync(() async {
    final image =
        await (key.currentContext!.findRenderObject() as RenderRepaintBoundary)
            .toImage(pixelRatio: 1);
    final bytes = (await image.toByteData(
      format: ui.ImageByteFormat.rawRgba,
    ))!.buffer.asUint8List();
    image.dispose();
    return bytes;
  }))!;
}

Future<Uint8List> dialogPixels(
  WidgetTester tester,
  Brightness brightness,
  bool actual,
) async {
  final key = GlobalKey();
  final status = fixture['status'] as Map<String, dynamic>;
  final props = status['props'] as Map<String, dynamic>;
  final host = ComposeDslWebViewHostContext(
    packageName: 'com.operit.character_cards',
    routeInstanceId: 'status-parity',
    executionContextKey: 'status-parity',
    dispatchAction: (id, [payload]) async => null,
    runtimeOptionsProvider: () => {},
  );
  var opened = false;
  await tester.pumpWidget(
    RepaintBoundary(
      key: key,
      child: MaterialApp(
        theme: ThemeData(brightness: brightness),
        home: Scaffold(
          body: Builder(
            builder: (context) {
              if (actual) {
                return buildComposeDslLayoutForTest(
                  node: status,
                  hostContext: host,
                  splitMarkdownContent: (_) async => [],
                  onTextInput: (_, _) async => null,
                );
              }
              if (!opened) {
                opened = true;
                WidgetsBinding.instance.addPostFrameCallback((_) {
                  showDialog<void>(
                    context: context,
                    builder: (context) => AlertDialog(
                      title: const Text('记忆状态'),
                      content: Text(props['text'] as String),
                      actions: [
                        TextButton(
                          onPressed: () => Navigator.of(context).pop(),
                          child: const Text('关闭'),
                        ),
                      ],
                    ),
                  );
                });
              }
              return const SizedBox.shrink();
            },
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
  expect(find.text('关闭'), findsOneWidget);
  return (await tester.runAsync(() async {
    final image =
        await (key.currentContext!.findRenderObject() as RenderRepaintBoundary)
            .toImage(pixelRatio: 1);
    final bytes = (await image.toByteData(
      format: ui.ImageByteFormat.rawRgba,
    ))!.buffer.asUint8List();
    image.dispose();
    return bytes;
  }))!;
}

void main() {
  for (final brightness in Brightness.values) {
    testWidgets(
      'registered memory status dialog matches main pixels: $brightness',
      (tester) async {
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetDevicePixelRatio);
        final expected = await dialogPixels(tester, brightness, false);
        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pumpAndSettle();
        final actual = await dialogPixels(tester, brightness, true);
        expect(actual.length, expected.length);
        var changed = 0;
        for (var i = 0; i < actual.length; i++) {
          if (actual[i] != expected[i]) changed++;
        }
        expect(
          changed,
          0,
          reason:
              'The status dialog must preserve main dimensions, typography, colors and actions.',
        );
      },
    );
    for (final expanded in [false, true]) {
      testWidgets(
        'registered memory DSL matches main pixels: $brightness expanded=$expanded',
        (tester) async {
          tester.view.devicePixelRatio = 1;
          addTearDown(tester.view.resetDevicePixelRatio);
          final expected = await capture(
            tester,
            reference(expanded),
            brightness,
            GlobalKey(),
          );
          await tester.pumpWidget(const SizedBox.shrink());
          await tester.pumpAndSettle();
          final host = ComposeDslWebViewHostContext(
            packageName: 'com.operit.character_cards',
            routeInstanceId: 'memory-parity',
            executionContextKey: 'memory-parity',
            dispatchAction: (id, [payload]) async => null,
            runtimeOptionsProvider: () => {},
          );
          final actual = await capture(
            tester,
            buildComposeDslLayoutForTest(
              node:
                  fixture[expanded ? 'expanded' : 'collapsed']
                      as Map<String, dynamic>,
              hostContext: host,
              splitMarkdownContent: (_) async => [],
              onTextInput: (_, _) async => null,
            ),
            brightness,
            GlobalKey(),
          );
          expect(actual, orderedEquals(expected));
        },
      );
    }
  }
}
