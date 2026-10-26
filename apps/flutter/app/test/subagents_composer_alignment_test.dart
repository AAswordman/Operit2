import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatPendingMessageQueuePanel.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/PendingQueueMessageItem.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/theme/OperitGlassSurface.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Checks plugin panels against the actual queue and global theme surfaces.
void main() {
  setUp(ClientLogger.initialize);
  late Map<String, Object?> fixture;
  setUpAll(() async {
    final root = Directory.current.parent.parent.parent;
    final result = await Process.run(
      'node',
      [
        '${root.path}/plugins/packages/external/subagents/tests/composer_fixture.mjs',
      ],
      workingDirectory: root.path,
      stdoutEncoding: utf8,
      stderrEncoding: utf8,
    );
    expect(result.exitCode, 0, reason: result.stderr.toString());
    fixture = (jsonDecode(result.stdout as String) as Map)
        .cast<String, Object?>();
  });

  for (final brightness in Brightness.values) {
    for (final width in [320.0, 700.0]) {
      for (final scale in [1.0, 1.5]) {
        for (final transparent in [false, true]) {
          for (final plugin in ['subagents', 'goal']) {
            testWidgets(
              '$plugin matches actual queue at $width / $scale / ${brightness.name} / glass:$transparent',
              (tester) async {
                final collapsedKey = plugin == 'subagents'
                    ? 'node'
                    : 'goalNode';
                final expandedKey = plugin == 'subagents'
                    ? 'expandedNode'
                    : 'goalExpandedNode';
                late ThemeData theme;

                /// Renders the real plugin and native queue under user theme preferences.
                Future<void> mount(String fixtureKey) async {
                  await tester.pumpWidget(
                    OperitTheme(
                      initialThemePreferenceSnapshot:
                          ThemePreferenceSnapshot.fromJson({
                            ...UserPreferencesManager
                                .defaultThemePreferenceSnapshot
                                .toJson(),
                            'chatInputFloating': true,
                            'transparentSurfaceEnabled': transparent,
                            'fontScale': 1.2,
                          }),
                      initialThemeMode: brightness == Brightness.dark
                          ? ThemeMode.dark
                          : ThemeMode.light,
                      initialThemeIsReady: false,
                      unconfiguredChildEnabled: true,
                      hostInteractionHostsEnabled: false,
                      child: Builder(
                        builder: (context) {
                          theme = Theme.of(context);
                          return Scaffold(
                            body: MediaQuery(
                              data: MediaQueryData(
                                textScaler: TextScaler.linear(scale),
                              ),
                              child: Align(
                                alignment: Alignment.topCenter,
                                child: SizedBox(
                                  key: const ValueKey('panel-viewport'),
                                  width: width,
                                  child: Column(
                                    mainAxisSize: MainAxisSize.min,
                                    children: [
                                      Padding(
                                        padding: const EdgeInsets.symmetric(
                                          horizontal: 8,
                                        ),
                                        child: buildComposeDslLayoutForTest(
                                          node: (fixture[fixtureKey] as Map)
                                              .cast<String, Object?>(),
                                          hostContext:
                                              ComposeDslWebViewHostContext(
                                                packageName:
                                                    'com.operit.$plugin',
                                                routeInstanceId: 'alignment',
                                                executionContextKey:
                                                    'alignment',
                                                dispatchAction:
                                                    (id, [payload]) async =>
                                                        null,
                                                runtimeOptionsProvider: () =>
                                                    <String, Object?>{},
                                              ),
                                          onTextInput: (id, text) async => null,
                                          splitMarkdownContent: (_) async => [],
                                        ),
                                      ),
                                      ChatPendingMessageQueuePanel(
                                        queuedMessages: const [
                                          PendingQueueMessageItem(
                                            id: 1,
                                            text: 'Queue message',
                                          ),
                                        ],
                                        expanded: fixtureKey == expandedKey,
                                        onExpandedChange: (_) {},
                                        onDeleteMessage: (_) {},
                                        onEditMessage: (_) {},
                                        onSendMessage: (_) {},
                                      ),
                                    ],
                                  ),
                                ),
                              ),
                            ),
                          );
                        },
                      ),
                    ),
                  );
                }

                await mount(collapsedKey);
                final surface = find.byKey(
                  ValueKey<Object?>('$plugin.surface'),
                );
                final glass = tester.widget<OperitGlassSurface>(
                  find
                      .descendant(
                        of: surface,
                        matching: find.byType(OperitGlassSurface),
                      )
                      .first,
                );
                final queueSurface = find.byKey(
                  const ValueKey('chat.pending-queue.surface'),
                );
                final queueGlass = tester.widget<OperitGlassSurface>(
                  queueSurface,
                );
                expect(glass.color, theme.colorScheme.surfaceContainer);
                expect(glass.color, queueGlass.color);
                expect(glass.layer, queueGlass.layer);
                expect(glass.borderRadius, queueGlass.borderRadius);
                expect(glass.material, isTrue);
                final actualThemeGlass = tester.widget<ThemeGlassSurface>(
                  find
                      .descendant(
                        of: surface,
                        matching: find.byType(ThemeGlassSurface),
                      )
                      .first,
                );
                expect(actualThemeGlass.transparentSurface, transparent);
                expect(
                  find.descendant(
                    of: surface,
                    matching: find.byType(BackdropFilter),
                  ),
                  transparent ? findsOneWidget : findsNothing,
                );
                expect(find.byType(Card), findsNothing);
                final pluginRect = tester.getRect(surface);
                final queueRect = tester.getRect(queueSurface);
                final viewport = tester.getRect(
                  find.byKey(const ValueKey('panel-viewport')),
                );
                expect(pluginRect.left, closeTo(viewport.left + 8, 0.1));
                expect(pluginRect.left, closeTo(queueRect.left, 0.1));
                expect(pluginRect.right, closeTo(queueRect.right, 0.1));
                expect(queueRect.top - pluginRect.bottom, closeTo(8, 0.1));
                expect(pluginRect.height, closeTo(queueRect.height, 0.1));
                expect(pluginRect.height, lessThanOrEqualTo(48));
                expect(find.byType(TextButton), findsNothing);
                expect(find.byType(IconButton), findsNWidgets(3));
                final titleFinder = find.byKey(
                  ValueKey<Object?>('$plugin.title'),
                );
                final titleWidget = tester.widget<Text>(
                  find
                      .descendant(of: titleFinder, matching: find.byType(Text))
                      .first,
                );
                final queueTitle = tester.widget<Text>(
                  find.text(
                    AppLocalizations.of(
                      tester.element(queueSurface),
                    )!.chatPendingQueueTitle(1),
                  ),
                );
                expect(
                  titleWidget.style!.fontSize,
                  theme.textTheme.labelMedium!.fontSize,
                );
                expect(titleWidget.style!.fontSize, closeTo(12 * 1.2, 0.01));
                expect(titleWidget.style, queueTitle.style);
                final header = tester.getRect(
                  find.byKey(ValueKey<Object?>('$plugin.header')),
                );
                final queueHeader = tester.getRect(
                  find.byKey(const ValueKey('chat.pending-queue.header')),
                );
                expect(header.height, closeTo(queueHeader.height, 0.1));
                final icon = tester.getRect(
                  find.byKey(ValueKey<Object?>('$plugin.status-icon')),
                );
                final title = tester.getRect(titleFinder);
                expect(icon.center.dy, closeTo(title.center.dy, 0.1));
                for (final button in find.byType(IconButton).evaluate()) {
                  final rect = tester.getRect(find.byWidget(button.widget));
                  expect(rect.size, const Size(32, 32));
                  expect(rect.center.dy, closeTo(icon.center.dy, 0.1));
                  expect(rect.right, lessThanOrEqualTo(header.right - 12));
                }
                await mount(expandedKey);
                expect(
                  tester.getRect(surface).height,
                  greaterThan(pluginRect.height),
                );
                if (plugin == 'subagents') {
                  final name = tester.getRect(
                    find.text('检查子代理区域在窄屏和大字体下的对齐与长名称显示'),
                  );
                  final close = tester.getRect(find.byTooltip('关闭子代理'));
                  expect(name.center.dy, closeTo(close.center.dy, 0.1));
                  expect(close.size, const Size(28, 28));
                  final taskGlass = tester
                      .widgetList<OperitGlassSurface>(
                        find.descendant(
                          of: surface,
                          matching: find.byType(OperitGlassSurface),
                        ),
                      )
                      .last;
                  expect(taskGlass.layer, OperitGlassSurfaceLayer.control);
                  expect(find.text('运行中 · 第 1 轮'), findsNothing);
                } else {
                  expect(
                    find.byKey(const ValueKey<Object?>('goal.details')),
                    findsOneWidget,
                  );
                  expect(find.textContaining('目标进行中'), findsOneWidget);
                }
                expect(tester.takeException(), isNull);
                await tester.pumpWidget(const SizedBox.shrink());
              },
            );
          }
        }
      }
    }
  }
}
