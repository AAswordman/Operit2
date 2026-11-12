import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/theme/OperitGlassSurface.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Exercises the authored plugin input in the native renderer at narrow widths.
void main() {
  setUpAll(ClientLogger.initialize);
  late Map<String, Object?> inputs;
  setUpAll(() async {
    final root = Directory.current.parent.parent.parent;
    final result = await Process.run('node', [
      '${root.path}/plugins/packages/external/subagents/tests/composer_fixture.mjs',
    ], workingDirectory: root.path);
    expect(result.exitCode, 0, reason: result.stderr.toString());
    inputs =
        ((jsonDecode(result.stdout as String) as Map)['conversationInputs']
                as Map)
            .cast<String, Object?>();
  });

  for (final brightness in Brightness.values) {
    for (final width in [280.0, 600.0]) {
      for (final scale in [1.0, 1.5]) {
        for (final status in ['running', 'completed', 'closed']) {
          testWidgets(
            'conversation input $status / $width / $scale / ${brightness.name}',
            (tester) async {
              Future<void> mount(String key) async {
                await tester.pumpWidget(
                  OperitTheme(
                    initialThemePreferenceSnapshot:
                        UserPreferencesManager.defaultThemePreferenceSnapshot,
                    initialThemeMode: brightness == Brightness.dark
                        ? ThemeMode.dark
                        : ThemeMode.light,
                    initialThemeIsReady: false,
                    unconfiguredChildEnabled: true,
                    hostInteractionHostsEnabled: false,
                    child: Scaffold(
                      body: MediaQuery(
                        data: MediaQueryData(
                          textScaler: TextScaler.linear(scale),
                        ),
                        child: Align(
                          alignment: Alignment.bottomCenter,
                          child: SizedBox(
                            width: width,
                            child: buildComposeDslLayoutForTest(
                              node: (inputs[key] as Map)
                                  .cast<String, Object?>(),
                              hostContext: ComposeDslWebViewHostContext(
                                packageName: 'com.operit.subagents',
                                routeInstanceId: 'conversation-input',
                                executionContextKey: 'conversation-input',
                                dispatchAction: (id, [payload]) async => null,
                                runtimeOptionsProvider: () =>
                                    <String, Object?>{},
                              ),
                              onTextInput: (id, text) async => null,
                              splitMarkdownContent: (_) async => [],
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                );
                await tester.pump();
              }

              await mount(status);
              final surface = find.byKey(
                const ValueKey<Object?>('subagents.input-surface'),
              );
              final compactHeight = tester.getSize(surface).height;
              expect(compactHeight, lessThanOrEqualTo(150));
              final editor = tester.widget<TextField>(find.byType(TextField));
              expect(editor.decoration!.labelText, isNull);
              expect(editor.decoration!.border, InputBorder.none);
              expect(editor.readOnly, status == 'closed');
              expect(find.byType(OperitGlassSurface), findsOneWidget);
              final action = find.byKey(
                ValueKey<Object?>(
                  status == 'running'
                      ? 'subagents.interrupt'
                      : 'subagents.send',
                ),
              );
              expect(tester.getSize(action), const Size(40, 40));
              final bounds = tester.getRect(surface);
              final actionBounds = tester.getRect(action);
              expect(bounds.contains(actionBounds.topLeft), isTrue);
              expect(bounds.contains(actionBounds.bottomRight), isTrue);
              expect(tester.takeException(), isNull);

              await mount('${status}Expanded');
              expect(
                tester.getSize(surface).height,
                greaterThan(compactHeight),
              );
              expect(tester.takeException(), isNull);
              await tester.pumpWidget(const SizedBox.shrink());
            },
          );
        }
      }
    }
  }
}
