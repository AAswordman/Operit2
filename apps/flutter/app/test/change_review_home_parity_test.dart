import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

import 'change_review_workspace_parity_test.dart' show captureWorkspace;

/// Keeps the whole review entry clickable across themes, widths and text sizes.
void main() {
  final fixture =
      jsonDecode(
            File('test/fixtures/change_review_home.json').readAsStringSync(),
          )
          as Map<String, dynamic>;
  for (final brightness in Brightness.values) {
    for (final width in [280.0, 400.0]) {
      testWidgets('quiet review entry: ${brightness.name}/$width', (
        tester,
      ) async {
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        var opened = false;
        await captureWorkspace(
          tester,
          Align(
            alignment: Alignment.topLeft,
            child: buildComposeDslLayoutForTest(
              node: fixture,
              hostContext: ComposeDslWebViewHostContext(
                packageName: 'com.operit.change_review',
                routeInstanceId: 'home-layout',
                executionContextKey: 'home-layout',
                dispatchAction: (_, [payload]) async {
                  opened = true;
                  return null;
                },
                runtimeOptionsProvider: () => {},
              ),
              splitMarkdownContent: (_) async => [],
              onTextInput: (_, _) async => null,
            ),
          ),
          brightness,
          'home-${brightness.name}-$width',
          width: width,
          scale: 1.5,
        );
        expect(find.text('代码变更'), findsOneWidget);
        expect(find.text('查看文件改动与对比'), findsOneWidget);
        expect(find.byType(Card), findsNothing);
        await tester.tap(find.text('查看文件改动与对比'));
        await tester.pumpAndSettle();
        expect(opened, isTrue);
        expect(tester.takeException(), isNull);
      });
    }
  }
}
