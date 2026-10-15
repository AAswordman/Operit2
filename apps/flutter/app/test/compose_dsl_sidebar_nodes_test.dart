import 'dart:convert';
import 'dart:io';
import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/material.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/M3LoadingIndicator.dart';
import 'package:operit2/ui/common/components/SwipeActions.dart';
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

/// Finds the generic transition node, without relying on a native rail class.
Finder railIndicator(String id) => find.descendant(
  of: find.byKey(ValueKey<Object?>('sidebar-chat-rail-indicator-$id')),
  matching: find.byType(AnimatedContainer),
);

/// Reuses the production plugin's selected/unselected visual projections.
Map<String, Object?> projectRailSelection(
  Map<String, Object?> initial,
  String id,
) {
  final projections = <bool, Map<String, Object?>>{};
  void record(Map node) {
    final props = node['props'] as Map;
    if ((props['key'] as String? ?? '').startsWith(
      'sidebar-chat-rail-indicator-',
    )) {
      projections[props['height'] == 16] = props.cast<String, Object?>();
    }
    for (final child in node['children'] as List? ?? []) {
      record(child as Map);
    }
  }

  record(initial);
  final tree = (jsonDecode(jsonEncode(initial)) as Map).cast<String, Object?>();
  void update(Map node) {
    final props = node['props'] as Map;
    final key = props['key'] as String? ?? '';
    if (key.startsWith('sidebar-chat-rail-indicator-')) {
      for (final entry
          in projections[key == 'sidebar-chat-rail-indicator-$id']!.entries) {
        if (entry.key != 'key') props[entry.key] = entry.value;
      }
    }
    for (final child in node['children'] as List? ?? []) {
      update(child as Map);
    }
  }

  update(tree);
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

Widget renderSidebar(
  Map<String, Object?> node,
  List<String> actions, {
  Future<Object?> Function(String, [Object?])? onAction,
}) => buildComposeDslLayoutForTest(
  node: node,
  hostContext: ComposeDslWebViewHostContext(
    packageName: 'fixture.character-sidebar',
    routeInstanceId: 'sidebar-layout-test',
    executionContextKey: 'sidebar-layout-test',
    dispatchAction: (id, [payload]) async {
      actions.add('$id:$payload');
      return onAction == null ? null : await onAction(id, payload);
    },
    runtimeOptionsProvider: () => {},
  ),
  onTextInput: (_, _) async => null,
  splitMarkdownContent: (_) async => [],
);

/// Counts real ink features rather than accepting a gesture-only click region.
class RecordingSplashFactory extends InteractiveInkFeatureFactory {
  int splashes = 0;
  final active = <InteractiveInkFeature>{};

  @override
  InteractiveInkFeature create({
    required MaterialInkController controller,
    required RenderBox referenceBox,
    required Offset position,
    required Color color,
    required TextDirection textDirection,
    bool containedInkWell = false,
    RectCallback? rectCallback,
    BorderRadius? borderRadius,
    ShapeBorder? customBorder,
    double? radius,
    VoidCallback? onRemoved,
  }) {
    splashes++;
    late final InteractiveInkFeature feature;
    feature = InkRipple.splashFactory.create(
      controller: controller,
      referenceBox: referenceBox,
      position: position,
      color: color,
      textDirection: textDirection,
      containedInkWell: containedInkWell,
      rectCallback: rectCallback,
      borderRadius: borderRadius,
      customBorder: customBorder,
      radius: radius,
      onRemoved: () {
        active.remove(feature);
        onRemoved?.call();
      },
    );
    active.add(feature);
    return feature;
  }
}

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
    'sidebar chat, category and group clicks create local ink ripples',
    (tester) async {
      final actions = <String>[];
      final splashes = RecordingSplashFactory();
      await tester.pumpWidget(
        themed(
          MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            theme: ThemeData(splashFactory: splashes),
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
      for (final label in ['对话 c1', '对话 c2', '旅行角色', '行程分组']) {
        final target = find.text(label);
        final ink = find
            .ancestor(of: target, matching: find.byType(InkWell))
            .first;
        final material = tester.widget<Material>(
          find.ancestor(of: ink, matching: find.byType(Material)).first,
        );
        expect(material.type, MaterialType.transparency);
        expect(material.clipBehavior, Clip.antiAlias);
        final previous = splashes.splashes;
        await tester.tap(target);
        await tester.pump();
        expect(splashes.splashes, previous + 1);
      }
      expect(actions, hasLength(4));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('selection refresh preserves the in-flight chat ripple', (
    tester,
  ) async {
    final actions = <String>[];
    final splashes = RecordingSplashFactory();
    final initial = sidebarTree();
    final selected = (jsonDecode(jsonEncode(initial)) as Map)
        .cast<String, Object?>();

    Map propsFor(Map<String, Object?> tree, String key) {
      Map? result;
      void visit(Map node) {
        if ((node['props'] as Map?)?['key'] == key)
          result = node['props'] as Map;
        for (final child in node['children'] as List? ?? []) {
          visit(child as Map);
        }
      }

      visit(tree);
      return result!;
    }

    // The fixture contains both real selected and unselected row projections.
    // Swap their decoration operations, keeping each row's own action callbacks.
    final first = propsFor(initial, 'sidebar-chat-hit-c1');
    final second = propsFor(initial, 'sidebar-chat-hit-c2');
    for (final pair in [(first, second, 'c1'), (second, first, 'c2')]) {
      final ownOps = (pair.$1['modifier'] as Map)['__modifierOps'] as List;
      final otherOps = (pair.$2['modifier'] as Map)['__modifierOps'] as List;
      final indicator = propsFor(
        selected,
        'sidebar-chat-rail-indicator-${pair.$3}',
      );
      final opposite = propsFor(
        initial,
        'sidebar-chat-rail-indicator-${pair.$3 == 'c1' ? 'c2' : 'c1'}',
      );
      indicator.addAll(<String, Object?>{
        ...opposite.cast<String, Object?>(),
        'key': indicator['key'],
      });
      propsFor(selected, 'sidebar-chat-hit-${pair.$3}')['modifier'] = {
        '__modifierOps': [
          for (final op in otherOps)
            if ((op as Map)['name'] != 'combinedClickable') op,
          ownOps.firstWhere((op) => (op as Map)['name'] == 'combinedClickable'),
        ],
      };
    }

    var tree = initial;
    var refreshes = 0;
    late StateSetter refresh;
    await tester.pumpWidget(
      themed(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          theme: ThemeData(splashFactory: splashes),
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                refresh = setState;
                return SizedBox(
                  width: 250,
                  height: 620,
                  child: renderSidebar(
                    tree,
                    actions,
                    onAction: (id, [payload]) async {
                      refresh(() {
                        refreshes++;
                        tree = selected;
                      });
                      return null;
                    },
                  ),
                );
              },
            ),
          ),
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 200));
    Finder inkMaterial() => find
        .ancestor(
          of: find
              .ancestor(of: find.text('对话 c2'), matching: find.byType(InkWell))
              .first,
          matching: find.byType(Material),
        )
        .first;
    final materialState = tester.state(inkMaterial());
    final gesture = await tester.startGesture(
      tester.getCenter(find.text('对话 c2')),
    );
    await tester.pump(const Duration(milliseconds: 150));
    final ripple = splashes.active.single;
    await gesture.up();
    await tester.pump(const Duration(milliseconds: 16));
    expect(
      refreshes,
      1,
      reason: 'Selection must update immediately, without delaying the action',
    );
    expect(actions, hasLength(1));
    expect(tester.state(inkMaterial()), same(materialState));
    expect(
      splashes.active,
      contains(ripple),
      reason: 'A refresh must not dispose an unfinished splash',
    );
    // Additional input/metadata refreshes must not reset the same animation.
    refresh(
      () => tree = (jsonDecode(jsonEncode(selected)) as Map)
          .cast<String, Object?>(),
    );
    await tester.pump(const Duration(milliseconds: 100));
    expect(splashes.active, contains(ripple));
    await tester.pump(const Duration(milliseconds: 500));
    expect(
      splashes.active,
      isNot(contains(ripple)),
      reason: 'The original splash should finish naturally',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('swipe backgrounds are rounded and action icons stay 18 by 18', (
    tester,
  ) async {
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
    final swipe = find.byType(SwipeActions).first;
    final bounds = tester.getRect(swipe);
    expect(bounds.height, 31);
    final clip = find
        .ancestor(of: swipe, matching: find.byType(ClipRRect))
        .first;
    expect(
      tester.widget<ClipRRect>(clip).borderRadius,
      BorderRadius.circular(8),
    );
    expect(tester.getRect(clip), bounds);
    for (final right in [true, false]) {
      final gesture = await tester.startGesture(
        Offset(
          bounds.left + bounds.width * (right ? .1 : .9),
          bounds.center.dy,
        ),
      );
      await gesture.moveBy(Offset(bounds.width * (right ? .3 : -.3), 0));
      await tester.pump();
      final icon = find.descendant(
        of: swipe,
        matching: find.byIcon(right ? Icons.edit : Icons.delete),
      );
      expect(icon, findsOneWidget);
      final iconBounds = tester.getRect(icon);
      expect(iconBounds.size, const Size(18, 18));
      expect(iconBounds.center.dy, bounds.center.dy);
      if (right) {
        expect(iconBounds.left, bounds.left + 12);
      } else {
        expect(iconBounds.right, bounds.right - 12);
      }
      await gesture.up();
      await tester.pump(const Duration(milliseconds: 200));
    }
    expect(
      actions,
      isEmpty,
      reason: 'Sub-threshold swipes must not invoke actions',
    );
    expect(tester.takeException(), isNull);
  });

  for (final brightness in Brightness.values) {
    testWidgets(
      'plugin-composed rail keeps hover and selection animations in $brightness',
      (tester) async {
        final actions = <String>[];
        final initial = sidebarTree();
        var tree = initial;
        late StateSetter refresh;
        await tester.pumpWidget(
          themed(
            MaterialApp(
              theme: ThemeData(
                colorScheme: ColorScheme.fromSeed(
                  seedColor: Colors.blue,
                  brightness: brightness,
                ),
              ),
              home: Scaffold(
                body: StatefulBuilder(
                  builder: (context, setState) {
                    refresh = setState;
                    return SizedBox(
                      width: 250,
                      height: 620,
                      child: renderSidebar(
                        tree,
                        actions,
                        onAction: (id, [payload]) async {
                          refresh(
                            () => tree = projectRailSelection(
                              initial,
                              actions.length == 1 ? 'c2' : 'c1',
                            ),
                          );
                          return null;
                        },
                      ),
                    );
                  },
                ),
              ),
            ),
          ),
        );
        await tester.pump(const Duration(milliseconds: 200));
        final rail = find.byKey(
          const ValueKey<Object?>('sidebar-chat-rail-c2'),
        );
        final indicator = railIndicator('c2');
        expect(rail, findsOneWidget);
        expect(indicator, findsOneWidget);
        final indicatorState = tester.state(indicator);
        final scheme = Theme.of(tester.element(rail)).colorScheme;
        expect(tester.getSize(rail), const Size(20, 34));
        expect(tester.getSize(indicator).height, 0);
        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.addPointer(location: Offset.zero);
        await mouse.moveTo(tester.getCenter(find.text('对话 c2')));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 80));
        expect(tester.getSize(indicator).height, inExclusiveRange(0, 10));
        await tester.pump(const Duration(milliseconds: 100));
        expect(tester.getSize(indicator), const Size(2, 10));
        expect(
          (tester.widget<AnimatedContainer>(indicator).decoration
                  as BoxDecoration)
              .color,
          scheme.onSurface.withValues(alpha: 0.55),
        );
        await mouse.moveTo(tester.getCenter(rail));
        await tester.pump(const Duration(milliseconds: 200));
        expect(tester.getSize(indicator), const Size(2, 10));
        expect(actions, isEmpty);
        await tester.tap(find.text('对话 c2'));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 80));
        expect(tester.state(indicator), same(indicatorState));
        expect(tester.getSize(indicator).height, inExclusiveRange(10, 16));
        final midTransitionHeight = tester.getSize(indicator).height;
        // A host refresh mid-transition must not replace or reset the animation.
        refresh(
          () => tree = (jsonDecode(jsonEncode(tree)) as Map)
              .cast<String, Object?>(),
        );
        await tester.pump();
        expect(tester.state(indicator), same(indicatorState));
        expect(tester.getSize(indicator).height, midTransitionHeight);
        await tester.pump(const Duration(milliseconds: 100));
        expect(tester.getSize(indicator), const Size(2, 16));
        expect(
          (tester.widget<AnimatedContainer>(indicator).decoration
                  as BoxDecoration)
              .color,
          scheme.primary,
        );
        await mouse.moveTo(Offset.zero);
        await tester.pump(const Duration(milliseconds: 200));
        expect(tester.getSize(indicator), const Size(2, 16));
        await tester.tap(find.text('对话 c1'));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 200));
        expect(tester.getSize(indicator).height, 0);
        expect(actions, hasLength(2));
        expect(tester.takeException(), isNull);
        await mouse.removePointer();
      },
    );
  }

  testWidgets(
    'open chat menu keeps the plugin-composed rail active outside the row',
    (tester) async {
      final actions = <String>[];
      await tester.pumpWidget(
        themed(
          MaterialApp(
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
      final indicator = railIndicator('c2');
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer(location: Offset.zero);
      await mouse.moveTo(tester.getCenter(find.text('对话 c2')));
      await tester.pump(const Duration(milliseconds: 200));
      final row = find
          .ancestor(of: find.text('对话 c2'), matching: find.byType(SwipeActions))
          .first;
      await tester.tap(
        find.descendant(of: row, matching: find.byType(PopupMenuButton<int>)),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 350));
      await mouse.moveTo(Offset.zero);
      await tester.pump(const Duration(milliseconds: 200));
      expect(tester.getSize(indicator), const Size(2, 10));
      await tester.tapAt(const Offset(500, 500));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 350));
      expect(tester.getSize(indicator).height, 0);
      expect(actions, isEmpty);
      expect(tester.takeException(), isNull);
      await mouse.removePointer();
    },
  );

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
    'generic Box hover styles honor plugin geometry and timing on touch platforms',
    (tester) async {
      final actions = <String>[];
      await tester.pumpWidget(
        MaterialApp(
          theme: ThemeData(platform: TargetPlatform.android),
          home: Scaffold(
            body: Center(
              child: renderSidebar({
                'type': 'HoverRegion',
                'props': {'width': 100, 'height': 80},
                'children': [
                  {
                    'type': 'Box',
                    'props': {
                      'width': 100,
                      'height': 80,
                      'contentAlignment': 'center',
                    },
                    'children': [
                      {
                        'type': 'Box',
                        'props': {
                          'key': 'custom-visual',
                          'width': 18,
                          'height': 12,
                          'backgroundColor': '#FFFF0000',
                          'backgroundShape': {'cornerRadius': 3},
                          'animation': {
                            'durationMillis': 240,
                            'curve': 'linear',
                          },
                          'hoverStyle': {
                            'height': 28,
                            'backgroundColor': '#FF00FF00',
                            'backgroundShape': {'cornerRadius': 9},
                          },
                        },
                        'children': <Object?>[],
                      },
                    ],
                  },
                ],
              }, actions),
            ),
          ),
        ),
      );
      final indicator = find.descendant(
        of: find.byKey(const ValueKey<Object?>('custom-visual')),
        matching: find.byType(AnimatedContainer),
      );
      expect(
        tester.getSize(indicator),
        const Size(18, 12),
        reason: 'Touch platforms must not imply pointer hover',
      );
      final state = tester.state(indicator);
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer(location: Offset.zero);
      await mouse.moveTo(tester.getCenter(indicator));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 120));
      expect(tester.getSize(indicator), const Size(18, 20));
      await tester.pump(const Duration(milliseconds: 120));
      expect(tester.getSize(indicator), const Size(18, 28));
      expect(tester.state(indicator), same(state));
      final decoration =
          tester.widget<AnimatedContainer>(indicator).decoration
              as BoxDecoration;
      expect(decoration.color, const Color(0xFF00FF00));
      expect(decoration.borderRadius, BorderRadius.circular(9));
      await mouse.moveTo(Offset.zero);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 240));
      expect(tester.getSize(indicator), const Size(18, 12));
      expect(actions, isEmpty);
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
      for (final type in ['UnregisteredWidget', 'HistoryRail']) {
        expect(
          () => renderSidebar({'type': type, 'children': <Object?>[]}, []),
          throwsFormatException,
        );
      }
    },
  );
}
