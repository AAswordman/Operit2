import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

import 'change_review_workspace_parity_test.dart' show captureWorkspace;
import 'reference/change_review/HomeCard.dart';

/// Compares the actual plugin entry card against the frozen pre-migration widget.
void main() {
  final fixture =
      jsonDecode(
            File('test/fixtures/change_review_home.json').readAsStringSync(),
          )
          as Map<String, dynamic>;
  for (final brightness in Brightness.values) {
    testWidgets('home card pixel parity: ${brightness.name}', (tester) async {
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final reference = await captureWorkspace(
        tester,
        ChangeReviewHomeReference(onOpenDiff: () {}),
        brightness,
        'home-${brightness.name}-native',
      );
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      final plugin = await captureWorkspace(
        tester,
        buildComposeDslLayoutForTest(
          node: fixture,
          hostContext: ComposeDslWebViewHostContext(
            packageName: 'com.operit.change_review',
            routeInstanceId: 'home-parity',
            executionContextKey: 'home-parity',
            dispatchAction: (_, [payload]) async => null,
            runtimeOptionsProvider: () => {},
          ),
          splitMarkdownContent: (_) async => [],
          onTextInput: (_, _) async => null,
        ),
        brightness,
        'home-${brightness.name}-plugin',
      );
      expect(plugin, orderedEquals(reference));
    });
  }
}
