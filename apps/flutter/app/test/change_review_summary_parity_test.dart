import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'reference/change_review/TurnDiffSummary.dart';
import 'reference/change_review/WorkspaceDiffReview.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

final fixtures =
    jsonDecode(
          File('test/fixtures/change_review_summary.json').readAsStringSync(),
        )
        as Map<String, dynamic>;

/// Captures the actual native renderer under the same localization and theme.
Future<({Uint8List bytes, Uint8List png})> capture(
  WidgetTester tester,
  Widget child,
  Brightness brightness,
  String locale,
  bool expanded,
  bool reference,
  int count,
) async {
  final key = GlobalKey();
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
        home: Scaffold(
          body: RepaintBoundary(
            key: key,
            child: SizedBox(
              width: 740,
              height: 350,
              child: Align(alignment: Alignment.topLeft, child: child),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  if (reference && !expanded) {
    await tester.tap(
      find.text(
        locale == 'en'
            ? 'Edited $count ${count == 1 ? 'file' : 'files'}'
            : '已编辑 $count 个文件',
      ),
    );
    await tester.pumpAndSettle();
  }
  expect(tester.takeException(), isNull);
  return (await tester.runAsync(() async {
    final image =
        await (key.currentContext!.findRenderObject() as RenderRepaintBoundary)
            .toImage(pixelRatio: 1);
    final bytes = (await image.toByteData(
      format: ui.ImageByteFormat.rawRgba,
    ))!.buffer.asUint8List();
    final png = (await image.toByteData(
      format: ui.ImageByteFormat.png,
    ))!.buffer.asUint8List();
    image.dispose();
    return (bytes: bytes, png: png);
  }))!;
}

/// Compares the plugin's production output with the actual pre-migration widget.
void main() {
  for (final brightness in Brightness.values) {
    for (final locale in ['zh', 'en']) {
      for (final expanded in [true, false]) {
        for (final count in [1, 6, 12]) {
          testWidgets(
            'summary pixel parity: $brightness/$locale/$expanded/$count',
            (tester) async {
              tester.view.devicePixelRatio = 1;
              addTearDown(tester.view.resetDevicePixelRatio);
              final expected = await capture(
                tester,
                TurnDiffSummary(
                  review: WorkspaceDiffReview(
                    files: [
                      for (var index = 0; index < count; index++)
                        WorkspaceDiffFile(
                          path: 'src/file$index.ts',
                          diffContent: '@@ -1 +1,2 @@\n-old\n+new\n+second',
                          additions: 2,
                          deletions: 1,
                        ),
                    ],
                  ),
                  onUndo: () {},
                ),
                brightness,
                locale,
                expanded,
                true,
                count,
              );
              await tester.pumpWidget(const SizedBox.shrink());
              await tester.pumpAndSettle();
              final actual = await capture(
                tester,
                buildComposeDslLayoutForTest(
                  node:
                      fixtures['$locale-$count-${expanded ? 'expanded' : 'collapsed'}']
                          as Map<String, dynamic>,
                  hostContext: ComposeDslWebViewHostContext(
                    packageName: 'com.operit.change_review',
                    routeInstanceId: 'summary-parity',
                    executionContextKey: 'summary-parity',
                    dispatchAction: (_, [payload]) async => null,
                    runtimeOptionsProvider: () => {},
                  ),
                  splitMarkdownContent: (_) async => [],
                  onTextInput: (_, _) async => null,
                ),
                brightness,
                locale,
                expanded,
                false,
                count,
              );
              expect(actual.bytes.length, expected.bytes.length);
              final differences = Iterable<int>.generate(actual.bytes.length)
                  .where(
                    (index) => actual.bytes[index] != expected.bytes[index],
                  )
                  .length;
              if (differences > 0) {
                final directory = Directory('.dart_tool/change_review_parity')
                  ..createSync(recursive: true);
                final suffix = '$locale-${brightness.name}-$expanded-$count';
                File(
                  '${directory.path}/reference-$suffix.png',
                ).writeAsBytesSync(expected.png);
                File(
                  '${directory.path}/plugin-$suffix.png',
                ).writeAsBytesSync(actual.png);
              }
              expect(
                differences,
                0,
                reason:
                    'Preserve original dimensions, typography, file icons, colors and controls.',
              );
            },
          );
        }
      }
    }
  }
}
