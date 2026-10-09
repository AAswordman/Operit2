import 'dart:convert';
import 'dart:io';
import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/material.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/M3LoadingIndicator.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// This tree is serialized by the actual SDK from the production sidebar screen.
/// Recreate with OPERIT_NATIVE_SIDEBAR_FIXTURE=... node --test tests/sidebar-native.test.mjs.
Map<String, Object?> sidebarTree({
  String fixture = 'native_character_sidebar.json',
}) {
  final tree =
      jsonDecode(File('test/fixtures/$fixture').readAsStringSync())
          as Map<String, Object?>;
  void replaceTestFile(Map<String, Object?> node) {
    // Explicit transport fixture: a valid image instead of the test host's absent /test file.
    if (node['type'] == 'Image') {
      (node['props'] as Map)['uri'] =
          'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==';
    }
    for (final child in node['children'] as List) {
      replaceTestFile((child as Map).cast<String, Object?>());
    }
  }

  replaceTestFile(tree);
  return tree;
}

Widget themed(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: child,
);

Widget renderSidebar(Map<String, Object?> node, List<String> actions) =>
    buildComposeDslLayoutForTest(
      node: node,
      hostContext: ComposeDslWebViewHostContext(
        packageName: 'fixture.character-sidebar',
        routeInstanceId: 'sidebar-layout-test',
        executionContextKey: 'sidebar-layout-test',
        dispatchAction: (id, [payload]) async {
          actions.add('$id:$payload');
          return null;
        },
        runtimeOptionsProvider: () => {},
      ),
      onTextInput: (_, _) async => null,
      splitMarkdownContent: (_) async => [],
    );

void main() {
  setUpAll(ClientLogger.initialize);
  for (final brightness in Brightness.values) {
    testWidgets(
      'actual serialized sidebar parses and renders all native nodes in $brightness',
      (tester) async {
        final actions = <String>[];
        final scheme = ColorScheme.fromSeed(
          seedColor: Colors.blue,
          brightness: brightness,
        );
        await tester.pumpWidget(
          themed(
            MaterialApp(
              localizationsDelegates: AppLocalizations.localizationsDelegates,
              supportedLocales: AppLocalizations.supportedLocales,
              theme: ThemeData(colorScheme: scheme),
              home: Scaffold(
                body: SizedBox(
                  width: 250,
                  height: 620,
                  child: renderSidebar(sidebarTree(), actions),
                ),
              ),
            ),
          ),
        );
        await tester.pump(const Duration(milliseconds: 200));
        expect(tester.takeException(), isNull);
        expect(find.text('旅行角色'), findsOneWidget);
        expect(find.text('行程分组'), findsOneWidget);
        expect(find.text('对话 c1'), findsOneWidget);
        expect(find.text('1791059541109'), findsNothing);
        expect(find.byType(Image), findsOneWidget);
        expect(find.byType(PopupMenuButton<int>), findsNWidgets(5));
        final decorations = tester
            .widgetList<DecoratedBox>(find.byType(DecoratedBox))
            .map((widget) => widget.decoration)
            .whereType<BoxDecoration>()
            .toList();
        expect(
          decorations.any(
            (decoration) =>
                decoration.borderRadius == BorderRadius.circular(12) &&
                decoration.color ==
                    scheme.surfaceContainerLow.withValues(alpha: 0.72),
          ),
          isTrue,
        );
        expect(
          decorations.any(
            (decoration) =>
                decoration.borderRadius == BorderRadius.circular(8) &&
                decoration.color ==
                    scheme.secondaryContainer.withValues(alpha: 0.78),
          ),
          isTrue,
        );
        expect(
          decorations.any(
            (decoration) => decoration.gradient is LinearGradient,
          ),
          isTrue,
        );
        final title = tester.widget<Text>(find.text('对话 c1'));
        expect(title.style?.fontSize, 13);
        expect(title.style?.letterSpacing, -0.1);
      },
    );
  }

  testWidgets(
    'hover is local and anchored popup dispatches the selected indexed action',
    (tester) async {
      final actions = <String>[];
      await tester.pumpWidget(
        themed(
          MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: Scaffold(
              body: SizedBox(
                width: 250,
                height: 620,
                child: renderSidebar(sidebarTree(), actions),
              ),
            ),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 200));
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer(location: Offset.zero);
      await mouse.moveTo(tester.getCenter(find.text('行程分组')));
      await tester.pump(const Duration(milliseconds: 200));
      expect(
        actions,
        isEmpty,
        reason: 'Mouse hover must not schedule a backend action',
      );
      final popup = find.byType(PopupMenuButton<int>).first;
      await tester.tap(popup);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 350));
      expect(find.text('编辑名称'), findsOneWidget);
      await tester.tap(find.text('编辑名称'));
      await tester.pump(const Duration(milliseconds: 350));
      await tester.pump();
      expect(actions, hasLength(1));
      expect(actions.single, endsWith(':1'));
      expect(tester.takeException(), isNull);
      await mouse.removePointer();
    },
  );

  testWidgets(
    'generic LoadingIndicator uses the app rotating polygon, not a legacy Material spinner',
    (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: renderSidebar({
              'type': 'LoadingIndicator',
              'props': {'size': 24},
              'children': <Object?>[],
            }, []),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 400));
      expect(find.byType(M3LoadingIndicator), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(
        tester.widget<M3LoadingIndicator>(find.byType(M3LoadingIndicator)).size,
        24,
      );
      expect(tester.takeException(), isNull);
    },
  );

  test(
    'unknown nodes are still rejected instead of being silently rendered as columns',
    () {
      expect(
        () => renderSidebar({
          'type': 'UnregisteredWidget',
          'children': <Object?>[],
        }, []),
        throwsFormatException,
      );
    },
  );
}
