import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/code_editor/CodeEditor.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

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

/// Exercises the current plugin workspace fixtures across themes, layout modes and text scales.
/// The retired native workspace is no longer a pixel oracle for the plugin UI.
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
      testWidgets('workspace content renders: $name', (tester) async {
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
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
        expect(plugin.length, width * 600 * 4);
        if (sample['empty'] == true) {
          expect(find.text('尚无文件变更'), findsWidgets);
          expect(find.text('项目变更将显示在此处'), findsOneWidget);
        } else if (sample['preview'] != null) {
          expect(
            find.byWidgetPredicate(
              (widget) =>
                  widget is CodeEditor &&
                  widget.text == 'tree file body' &&
                  widget.filePath == '/operitapp/workspaces/test/readme.txt' &&
                  widget.readOnly,
            ),
            findsOneWidget,
          );
          expect(find.text('readme.txt'), findsWidgets);
        } else {
          final diff = sample['diff'] as String;
          final expectedContent = sample['full'] == true
              ? 'line 1'
              : diff
                    .split('\n')
                    .firstWhere(
                      (line) =>
                          (line.startsWith('+') || line.startsWith('-')) &&
                          line.substring(1).trim().isNotEmpty,
                    )
                    .substring(1)
                    .trim();
          expect(
            find.textContaining(expectedContent, findRichText: true),
            findsWidgets,
          );
        }
        expect(tester.takeException(), isNull);
      });
    }
  }
}
