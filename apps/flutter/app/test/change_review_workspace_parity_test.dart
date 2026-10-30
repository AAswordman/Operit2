import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'reference/change_review/WorkspaceDiffContent.dart';
import 'reference/change_review/WorkspaceDiffReview.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/features/chat/viewmodel/WorkspaceFileModels.dart';

/// Captures identical host dimensions, typography, theme and actual native widgets.
Future<Uint8List> captureWorkspace(
  WidgetTester tester,
  Widget child,
  Brightness brightness,
  String name, {
  double width = 800,
  double scale = 1,
  String locale = 'zh',
  Future<void> Function()? configure,
}) async {
  final key = GlobalKey();
  tester.view.physicalSize = Size(width, 600);
  tester.view.devicePixelRatio = 1;
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: MaterialApp(
        locale: Locale(locale),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        theme: ThemeData(brightness: brightness),
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(
            context,
          ).copyWith(textScaler: TextScaler.linear(scale)),
          child: child!,
        ),
        home: Scaffold(
          body: RepaintBoundary(key: key, child: child),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  await configure?.call();
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
  return (await tester.runAsync(() async {
    final image =
        await (key.currentContext!.findRenderObject() as RenderRepaintBoundary)
            .toImage();
    final bytes = (await image.toByteData(
      format: ui.ImageByteFormat.rawRgba,
    ))!.buffer.asUint8List();
    final png = (await image.toByteData(
      format: ui.ImageByteFormat.png,
    ))!.buffer.asUint8List();
    File(
      '${Directory.systemTemp.path}/change-review-$name.png',
    ).writeAsBytesSync(png);
    image.dispose();
    return bytes;
  }))!;
}

/// Preserves content-pane pixels while allowing the plugin-owned navigation tree to evolve.
void main() {
  final fixtures =
      jsonDecode(
            File(
              'test/fixtures/change_review_workspace.json',
            ).readAsStringSync(),
          )
          as Map<String, dynamic>;
  for (final brightness in Brightness.values) {
    for (final raw in fixtures['samples'] as List) {
      final sample = raw as Map<String, dynamic>;
      final name = '${brightness.name}-${sample['name']}';
      testWidgets('workspace content pixel parity: $name', (tester) async {
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final reference = await captureWorkspace(
          tester,
          WorkspaceDiffContent(
            review: WorkspaceDiffReview(
              files: sample['empty'] == true
                  ? []
                  : [
                      WorkspaceDiffFile(
                        path: 'src/a.ts',
                        diffContent: sample['diff'] as String,
                        additions:
                            sample['name'].toString().contains('wrap') ||
                                sample['name'] == 'split' ||
                                sample['name'].toString().contains('gap') ||
                                sample['name'].toString().contains('full')
                            ? 2
                            : 1,
                        deletions:
                            sample['name'].toString().contains('gap') ||
                                sample['name'].toString().contains('full')
                            ? 2
                            : 1,
                      ),
                    ],
            ),
            onListWorkspaceFiles: sample['pane'] == null
                ? null
                : (_) async => [
                    WorkspaceFileEntry(
                      name: 'readme.txt',
                      path: '/app/workspaces/test/readme.txt',
                      relativePath: 'readme.txt',
                      isDirectory: false,
                      size: 14,
                      lastModified: '',
                    ),
                  ],
            onReadWorkspaceTextFile: (_) async =>
                sample['full'] == true || sample['fullOff'] == true
                ? List.generate(40, (index) => 'line ${index + 1}').join('\n')
                : 'tree file body',
          ),
          brightness,
          '$name-native',
          width: (sample['width'] as num?)?.toDouble() ?? 800,
          scale: (sample['scale'] as num?)?.toDouble() ?? 1,
          locale: sample['locale'] as String? ?? 'zh',
          configure: () async {
            if (sample['split'] == true) {
              await tester.tap(find.byTooltip('分栏视图 (Split)'));
              await tester.pumpAndSettle();
            }
            if (sample['wrap'] == true) {
              await tester.tap(find.byTooltip('开启换行 (自动换行)'));
              await tester.pumpAndSettle();
            }
            if (sample['full'] == true || sample['fullOff'] == true) {
              await tester.tap(find.byTooltip('完整展示整个文件 (展开未更改区域)'));
              await tester.pumpAndSettle();
              if (sample['fullOff'] == true) {
                await tester.tap(find.byTooltip('仅显示改动附近的行 (收起未更改区域)'));
                await tester.pumpAndSettle();
              }
            }
            if (sample['pane'] != null) {
              await tester.tap(find.byTooltip('切换至项目工作树'));
              await tester.pumpAndSettle();
            }
            if (sample['preview'] != null) {
              await tester.tap(find.text('readme.txt'));
              await tester.pumpAndSettle();
            }
          },
        );
        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pumpAndSettle();
        final plugin = await captureWorkspace(
          tester,
          buildComposeDslLayoutForTest(
            node: fixtures[name] as Map<String, dynamic>,
            hostContext: ComposeDslWebViewHostContext(
              packageName: 'com.operit.change_review',
              routeInstanceId: 'workspace-parity',
              executionContextKey: 'workspace-parity',
              dispatchAction: (_, [payload]) async => null,
              runtimeOptionsProvider: () => {},
            ),
            splitMarkdownContent: (_) async => [],
            onTextInput: (_, _) async => null,
          ),
          brightness,
          '$name-plugin',
          width: (sample['width'] as num?)?.toDouble() ?? 800,
          scale: (sample['scale'] as num?)?.toDouble() ?? 1,
          locale: sample['locale'] as String? ?? 'zh',
        );
        final width = (sample['width'] as num?)?.toInt() ?? 800;
        // The directory tree intentionally replaces the old flat file list.
        // Compare every pixel in the unchanged toolbar, ribbon and diff content.
        for (var row = 0; row < 600; row++) {
          final start = (row * width + 221) * 4;
          final end = (row + 1) * width * 4;
          expect(
            Uint8List.sublistView(plugin, start, end),
            orderedEquals(Uint8List.sublistView(reference, start, end)),
            reason: 'Unchanged content row $row in $name',
          );
        }
      });
    }
  }
}
