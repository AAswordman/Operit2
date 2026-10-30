import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

import 'change_review_workspace_parity_test.dart' show captureWorkspace;

/// Exercises the redesigned summary through the production Compose renderer.
void main() {
  final fixtures =
      jsonDecode(
            File('test/fixtures/change_review_summary.json').readAsStringSync(),
          )
          as Map<String, dynamic>;
  for (final brightness in Brightness.values) {
    for (final locale in ['zh', 'en']) {
      for (final expanded in [true, false]) {
        for (final count in [1, 6, 12]) {
          testWidgets('summary layout: $brightness/$locale/$expanded/$count', (
            tester,
          ) async {
            addTearDown(tester.view.resetPhysicalSize);
            addTearDown(tester.view.resetDevicePixelRatio);
            for (final width in [320.0, 740.0]) {
              final node =
                  fixtures['${brightness.name}-$locale-$count-${expanded ? 'expanded' : 'collapsed'}']
                      as Map<String, dynamic>;
              await captureWorkspace(
                tester,
                Align(
                  alignment: Alignment.topLeft,
                  child: buildComposeDslLayoutForTest(
                    node: node,
                    hostContext: ComposeDslWebViewHostContext(
                      packageName: 'com.operit.change_review',
                      routeInstanceId: 'summary-layout',
                      executionContextKey: 'summary-layout',
                      dispatchAction: (_, [payload]) async => null,
                      runtimeOptionsProvider: () => {},
                    ),
                    splitMarkdownContent: (_) async => [],
                    onTextInput: (_, _) async => null,
                  ),
                ),
                brightness,
                'summary-${brightness.name}-$locale-$expanded-$count-$width',
                width: width,
                scale: width == 740 ? 1.5 : 1,
                locale: locale,
              );
              expect(
                find.text('+${count * 2}'),
                count == 1 && expanded ? findsNWidgets(2) : findsOneWidget,
              );
              if (expanded) {
                final rows = tester
                    .widgetList<RichText>(find.byType(RichText))
                    .where(
                      (text) => text.text.toPlainText().startsWith('file'),
                    );
                expect(rows.length, count);
                for (final row in rows) {
                  expect(row.text.toPlainText(), contains('  src/'));
                  expect(row.maxLines, 1);
                }
                expect(find.text('+2'), findsNWidgets(count == 1 ? 2 : count));
              }
              expect(
                find.text('−$count'),
                count == 1 && expanded ? findsNWidgets(2) : findsOneWidget,
              );
              final added = tester.widget<Text>(
                find.text('+${count * 2}').first,
              );
              final colors = Theme.of(
                tester.element(find.text('+${count * 2}').first),
              ).colorScheme;
              final ink = added.style!.color!.computeLuminance();
              final background = colors.surfaceContainer.computeLuminance();
              final contrast =
                  (ink > background ? ink + 0.05 : background + 0.05) /
                  (ink > background ? background + 0.05 : ink + 0.05);
              expect(contrast, greaterThanOrEqualTo(4.5));
              expect(find.byIcon(Icons.undo_rounded), findsOneWidget);
              expect(
                find.byIcon(Icons.insert_drive_file_outlined),
                expanded ? findsNWidgets(count) : findsNothing,
              );
              expect(find.byType(RichText), isNot(findsNothing));
            }
          });
        }
      }
    }
  }
}
