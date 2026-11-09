import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/appearance/ThemeConfigurationControl.dart';
import 'package:operit2/ui/features/settings/appearance/ThemeAppearancePreview.dart';
import 'package:operit2/ui/features/settings/appearance/ChatAppearancePreview.dart';
import 'package:operit2/ui/features/settings/appearance/AppearanceSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit_ui/operit_ui.dart' show buildOperitThemeData;

/// Verifies the real named-theme selector and generated proxy calls without a platform runtime.
void main() {
  setUp(ClientLogger.initialize);
  testWidgets(
    'creation exposes grouped sources and changes the explicit selection',
    (tester) async {
      final bridge = _ThemeBridge()..packages.add(_templatePackage());
      await _mount(tester, bridge, size: const Size(1000, 800));
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      final dialog = find.byType(Dialog);
      expect(
        find.descendant(
          of: dialog,
          matching: find.byType(DropdownButtonFormField<String>),
        ),
        findsNothing,
      );
      expect(find.text('Default appearance'), findsOneWidget);
      expect(find.text('Current appearance'), findsOneWidget);
      expect(find.text('Inherit a saved configuration'), findsOneWidget);
      expect(find.text('Use a plugin template'), findsOneWidget);
      expect(find.text('Plugin / night'), findsOneWidget);
      final defaults = find.byKey(const ValueKey('theme-source-defaults'));
      final current = find.byKey(const ValueKey('theme-source-current'));
      expect(
        find.descendant(
          of: defaults,
          matching: find.byIcon(Icons.check_circle),
        ),
        findsOneWidget,
      );
      await tester.tap(current);
      await tester.pumpAndSettle();
      expect(
        find.descendant(of: current, matching: find.byIcon(Icons.check_circle)),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: defaults,
          matching: find.byIcon(Icons.check_circle),
        ),
        findsNothing,
      );
      await tester.enterText(find.byType(TextFormField), 'Current copy');
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(bridge.configs.last.name, 'Current copy');
      expect(
        bridge.calls.where((call) => call.methodName == 'getPreferences'),
        hasLength(1),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'narrow creation keeps actions visible and scrolls long configuration lists',
    (tester) async {
      final bridge = _ThemeBridge();
      bridge.configs.addAll(
        List.generate(
          12,
          (index) => _config(
            'saved-$index',
            'A longer saved appearance configuration $index',
          ),
        ),
      );
      await _mount(tester, bridge, size: const Size(320, 500));
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('Create').hitTestable(), findsOneWidget);
      expect(find.text('Cancel').hitTestable(), findsOneWidget);
      final last = find.byKey(const ValueKey('theme-source-config:saved-11'));
      await tester.scrollUntilVisible(
        last,
        200,
        scrollable: find
            .descendant(
              of: find.byKey(const ValueKey('theme-creation-scroll')),
              matching: find.byType(Scrollable),
            )
            .first,
      );
      await tester.tap(last);
      await tester.pumpAndSettle();
      expect(
        find.descendant(of: last, matching: find.byIcon(Icons.check_circle)),
        findsOneWidget,
      );
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(
        bridge.calls.where((call) => call.methodName == 'create'),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'creation reflows and scrolls through short wide windows, narrow panes and keyboard insets',
    (tester) async {
      final bridge = _ThemeBridge();
      await _mount(tester, bridge, size: const Size(1100, 500));
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('theme-creation-details-scroll')),
        findsOneWidget,
      );
      expect(find.byType(TextFormField).hitTestable(), findsOneWidget);
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.enterText(find.byType(TextFormField), 'Responsive theme');
      for (final size in [
        const Size(1100, 360),
        const Size(700, 420),
        const Size(280, 500),
        const Size(240, 360),
        const Size(520, 320),
        const Size(1100, 500),
      ]) {
        tester.view.physicalSize = size;
        await tester.binding.setSurfaceSize(size);
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull, reason: 'dialog at $size');
        if (size == const Size(1100, 360)) {
          final details = find.byKey(
            const ValueKey('theme-creation-details-scroll'),
          );
          final pane = tester.widget<SingleChildScrollView>(details);
          expect(pane.controller!.position.maxScrollExtent, greaterThan(0));
          await tester.drag(details, const Offset(0, -100));
          await tester.pumpAndSettle();
          expect(pane.controller!.offset, greaterThan(0));
        }
        expect(find.text('Create').hitTestable(), findsOneWidget);
        expect(find.text('Cancel').hitTestable(), findsOneWidget);
        await tester.ensureVisible(find.byType(TextFormField));
        await tester.pumpAndSettle();
        expect(find.byType(TextFormField).hitTestable(), findsOneWidget);
        expect(
          find.widgetWithText(TextFormField, 'Responsive theme'),
          findsOneWidget,
        );
      }
      tester.view.viewInsets = const FakeViewPadding(bottom: 220);
      addTearDown(tester.view.resetViewInsets);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('Create').hitTestable(), findsOneWidget);
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(bridge.configs.last.name, 'Responsive theme');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'migrated theme shows its background without writes when opening the library',
    (tester) async {
      final bridge = _ThemeBridge()
        ..configs.clear()
        ..active = null;
      bridge.assets['theme-media/current.png'] = (await tester.runAsync(
        _landscapeBackground,
      ))!;
      final current = ThemePreferenceSnapshot.fromJson({
        ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
        'useBackgroundImage': true,
        'backgroundImageUri': 'theme-media/current.png',
        'backgroundImageOpacity': 1.0,
        'customPrimaryColor': 0xFF32775C,
      });
      bridge.configs.add(
        core.CoreDataPreferencesThemeConfigManagerThemeConfig(
          id: 'migrated',
          name: 'Default theme',
          snapshot: current.toJson(),
          createdAt: 0,
          updatedAt: 0,
        ),
      );
      bridge.active = 'migrated';
      await _mount(
        tester,
        bridge,
        currentSnapshot: current,
        size: const Size(1000, 600),
      );
      await tester.runAsync(() async {
        await Future<void>.delayed(const Duration(milliseconds: 100));
      });
      await tester.pumpAndSettle();
      final preview = tester.widget<ThemeAppearancePreview>(
        find.byKey(const ValueKey('theme-thumbnail:migrated')),
      );
      expect(preview.snapshot.toJson(), current.toJson());
      expect(find.byType(RawImage), findsOneWidget);
      expect(
        bridge.calls
            .where((call) => call.methodName == 'readBase64')
            .single
            .args,
        {'path': 'theme-media/current.png'},
      );
      expect(bridge.active, 'migrated');
      expect(bridge.configs.single.snapshot, current.toJson());
      expect(find.byKey(const ValueKey('theme-edit:migrated')), findsOneWidget);
      expect(find.byKey(const ValueKey('theme-menu:migrated')), findsOneWidget);
      expect(find.byKey(const ValueKey('theme-edit:night')), findsNothing);
      // Outer appearance changes cannot replace the mode stored in a saved theme.
      await _mount(
        tester,
        bridge,
        currentSnapshot: ThemePreferenceSnapshot.fromJson({
          ...current.toJson(),
          'themeMode': 'dark',
          'useBackgroundImage': false,
        }),
      );
      expect(
        tester
            .widget<ThemeAppearancePreview>(
              find.byKey(const ValueKey('theme-thumbnail:migrated')),
            )
            .snapshot
            .toJson(),
        current.toJson(),
      );
      expect(
        bridge.calls.where(
          (call) => !{
            'readBase64',
            'list',
            'getPreferences',
          }.contains(call.methodName),
        ),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
    },
  );

  for (final size in [
    const Size(240, 360),
    const Size(320, 260),
    const Size(560, 340),
  ]) {
    testWidgets('library reflows with large text at $size', (tester) async {
      await _mount(tester, _ThemeBridge(), size: size, textScale: 1.5);
      expect(tester.takeException(), isNull);
      await tester.scrollUntilVisible(
        find.byKey(const ValueKey('theme-edit:night')),
        150,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('theme-edit:night')).hitTestable(),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets(
    'saved theme selection follows acknowledged catalog state and shows apply failures',
    (tester) async {
      final bridge = _ThemeBridge();
      await _mount(tester, bridge);
      bridge.failure = StateError('Theme owner rejected selection');
      await tester.tap(find.byKey(const ValueKey('theme-apply:night')));
      await tester.pumpAndSettle();
      expect(
        find.textContaining('Theme owner rejected selection'),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('theme-card:day')),
          matching: find.text('In use'),
        ),
        findsOneWidget,
      );
      bridge.failure = null;
      await tester.tap(find.byKey(const ValueKey('theme-apply:night')));
      await tester.pumpAndSettle();
      expect(bridge.active, 'night');
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('theme-card:night')),
          matching: find.text('In use'),
        ),
        findsOneWidget,
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'apply').last.args,
        {'id': 'night'},
      );
    },
  );

  testWidgets('creating a theme from defaults selects its actual ID', (
    tester,
  ) async {
    final bridge = _ThemeBridge();
    await _mount(tester, bridge);
    await tester.tap(find.byKey(const ValueKey('theme-config-create')));
    await tester.pumpAndSettle();
    expect(
      find.descendant(
        of: find.byType(Dialog),
        matching: find.text('New theme configuration'),
      ),
      findsOneWidget,
    );
    await tester.enterText(find.byType(TextFormField), 'My theme');
    await tester.tap(find.text('Create'));
    await tester.pumpAndSettle();
    final create = bridge.calls.singleWhere(
      (call) => call.methodName == 'create',
    );
    final args = create.args as Map;
    expect(args['name'], 'My theme');
    expect(
      args['snapshot'],
      UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
    );
    expect(bridge.active, 'created');
    expect(find.text('My theme'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'new themes start from defaults and preserve the existing named appearance',
    (tester) async {
      final bridge = _ThemeBridge();
      final previous = bridge.configs.first;
      final edited = Map<String, Object?>.from(previous.snapshot)
        ..['themeMode'] = 'dark';
      bridge.configs[0] = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
        id: previous.id,
        name: previous.name,
        snapshot: edited,
        createdAt: previous.createdAt,
        updatedAt: 2,
      );
      await _mount(tester, bridge);
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), 'Fresh theme');
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(bridge.configs.first.snapshot, edited);
      expect(
        bridge.configs.last.snapshot,
        UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      );
      expect(bridge.active, 'created');
      expect(
        bridge.calls.where((call) => call.methodName == 'getPreferences'),
        hasLength(1),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'inheriting a saved configuration copies its appearance without editing the source',
    (tester) async {
      final bridge = _ThemeBridge();
      final previous = bridge.configs.first;
      final edited = Map<String, Object?>.from(previous.snapshot)
        ..['fontScale'] = 1.4;
      bridge.configs[0] = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
        id: previous.id,
        name: previous.name,
        snapshot: edited,
        createdAt: previous.createdAt,
        updatedAt: 2,
      );
      await _mount(tester, bridge);
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), 'Inherited');
      final source = find.byKey(const ValueKey('theme-source-config:day'));
      await tester.ensureVisible(source);
      await tester.tap(source);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(bridge.configs.first.snapshot, edited);
      expect(bridge.configs.last.snapshot, edited);
      expect(bridge.active, 'created');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'renaming keeps the stored snapshot and deletion lists only inactive configurations',
    (tester) async {
      final bridge = _ThemeBridge();
      await _mount(tester, bridge);
      final original = Map<String, Object?>.from(bridge.configs.first.snapshot);
      await tester.tap(find.byKey(const ValueKey('theme-menu:day')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Rename theme configuration'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), 'Renamed day');
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(
        bridge.calls.singleWhere((call) => call.methodName == 'rename').args,
        {'id': 'day', 'name': 'Renamed day'},
      );
      expect(bridge.configs.first.snapshot, original);
      await tester.tap(find.byKey(const ValueKey('theme-menu:day')));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<PopupMenuItem<String>>(
              find.ancestor(
                of: find.text('Delete'),
                matching: find.byType(PopupMenuItem<String>),
              ),
            )
            .enabled,
        isFalse,
      );
      await tester.tapAt(const Offset(10, 10));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('theme-menu:night')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();
      expect(
        find.text('Delete “Night”? This cannot be undone.'),
        findsOneWidget,
      );
      await tester.tap(find.text('Delete').last);
      await tester.pumpAndSettle();
      expect(bridge.configs.map((config) => config.id), ['day']);
      expect(bridge.active, 'day');
    },
  );

  testWidgets(
    'saved themes share one card layout without an extra appearance entry',
    (tester) async {
      final bridge = _ThemeBridge();
      await _mount(tester, bridge);
      expect(find.byKey(const ValueKey('theme-config-custom')), findsNothing);
      expect(find.byKey(const ValueKey('theme-card:day')), findsOneWidget);
      expect(find.byKey(const ValueKey('theme-card:night')), findsOneWidget);
      expect(
        bridge.calls.where(
          (call) =>
              {'create', 'apply', 'patchAppearance'}.contains(call.methodName),
        ),
        isEmpty,
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'useCustomAppearance'),
        isEmpty,
      );
    },
  );
  testWidgets(
    'edit applies the chosen card before opening its editor and blocks failures',
    (tester) async {
      final bridge = _ThemeBridge();
      final edited = <String>[];
      await _mount(
        tester,
        bridge,
        onEdit: (name) async {
          expect(bridge.active, 'night');
          edited.add(name);
        },
      );
      bridge.failure = StateError('Cannot apply');
      await tester.tap(find.byKey(const ValueKey('theme-edit:night')));
      await tester.pumpAndSettle();
      expect(edited, isEmpty);
      expect(bridge.active, 'day');
      bridge.failure = null;
      await tester.tap(find.byKey(const ValueKey('theme-edit:night')));
      await tester.pumpAndSettle();
      expect(edited, ['Night']);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'preview shows the saved snapshot without selecting or writing it',
    (tester) async {
      final bridge = _ThemeBridge();
      await _mount(tester, bridge);
      await tester.tap(find.byKey(const ValueKey('theme-preview:night')));
      await tester.pumpAndSettle();
      final previewContext = tester.element(
        find.descendant(
          of: find.byType(Dialog),
          matching: find.byType(ChatAppearancePreview),
        ),
      );
      expect(
        OperitTheme.of(previewContext).themePreferenceSnapshot.themeMode,
        ThemeMode.dark,
      );
      final renderer = tester.widget<StreamMarkdownRenderer>(
        find.descendant(
          of: find.byType(Dialog),
          matching: find.byType(StreamMarkdownRenderer),
        ),
      );
      expect(renderer.state!.renderNodes, isNotEmpty);
      expect(renderer.state!.streamParsingCompletedSuccessfully, isTrue);
      expect(bridge.active, 'day');
      expect(bridge.calls, isEmpty);
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
      expect(bridge.active, 'day');
    },
  );

  testWidgets(
    'creation opens the editor only after the new theme is selected',
    (tester) async {
      final bridge = _ThemeBridge();
      final edited = <String>[];
      await _mount(
        tester,
        bridge,
        onEdit: (name) async {
          expect(bridge.active, 'created');
          edited.add(name);
        },
      );
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), 'My new theme');
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(edited, ['My new theme']);
    },
  );
  testWidgets(
    'appearance opens at the library and returns there from its editor',
    (tester) async {
      final bridge = _ThemeBridge();
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Scaffold(
            body: AppearanceSettingsPanel(
              clients: GeneratedCoreProxyClients(bridge),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(TabBar), findsNothing);
      await tester.tap(find.byKey(const ValueKey('theme-edit:day')));
      await tester.pumpAndSettle();
      expect(find.byType(TabBar), findsOneWidget);
      expect(find.widgetWithText(AppBar, 'Day'), findsOneWidget);
      await tester.tap(find.byType(BackButton));
      await tester.pumpAndSettle();
      expect(find.byType(TabBar), findsNothing);
      expect(find.byKey(const ValueKey('theme-card:day')), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets(
    'horizontal thumbnail loads the saved background without writing preferences',
    (tester) async {
      final bridge = _ThemeBridge();
      bridge.assets['theme-media/forest.png'] = (await tester.runAsync(
        _landscapeBackground,
      ))!;
      final original = bridge.configs.first;
      bridge.configs[0] = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
        id: original.id,
        name: original.name,
        createdAt: 1,
        updatedAt: 1,
        snapshot: {
          ...original.snapshot,
          'useBackgroundImage': true,
          'backgroundImageUri': 'theme-media/forest.png',
          'backgroundImageOpacity': 1.0,
        },
      );
      await _mount(tester, bridge, size: const Size(390, 844));
      await tester.runAsync(() async {
        await Future<void>.delayed(const Duration(milliseconds: 100));
      });
      await tester.pumpAndSettle();
      expect(
        bridge.calls
            .where((call) => call.methodName == 'readBase64')
            .single
            .args,
        {'path': 'theme-media/forest.png'},
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'patchAppearance'),
        isEmpty,
      );
      final preview = tester.getRect(
        find.byKey(const ValueKey('theme-thumbnail:day')),
      );
      final edit = tester.getRect(find.byKey(const ValueKey('theme-edit:day')));
      expect(preview.right, lessThan(edit.left));
      expect(find.byType(RawImage), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'mode shortcut follows each theme snapshot and edits only the active theme',
    (tester) async {
      final bridge = _ThemeBridge();
      final night = Map<String, Object?>.from(bridge.configs.last.snapshot);
      await _mount(
        tester,
        bridge,
        currentSnapshot: ThemePreferenceSnapshot.fromJson({
          ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
          'themeMode': 'dark',
        }),
      );
      Set<ThemeMode> selected() => tester
          .widget<SegmentedButton<ThemeMode>>(
            find.byType(SegmentedButton<ThemeMode>),
          )
          .selected;
      expect(selected(), {ThemeMode.light});
      await tester.tap(find.text('System').first);
      await tester.pumpAndSettle();
      expect(selected(), {ThemeMode.system});
      expect(bridge.configs.first.snapshot['themeMode'], 'system');
      expect(bridge.configs.last.snapshot, night);
      final firstSaved = Map<String, Object?>.from(
        bridge.configs.first.snapshot,
      );
      await tester.tap(find.byKey(const ValueKey('theme-apply:night')));
      await tester.pumpAndSettle();
      expect(selected(), {ThemeMode.dark});
      await tester.tap(find.text('Light').first);
      await tester.pumpAndSettle();
      expect(selected(), {ThemeMode.light});
      expect(bridge.configs.last.snapshot['themeMode'], 'light');
      expect(bridge.configs.first.snapshot, firstSaved);
      await tester.tap(find.byKey(const ValueKey('theme-apply:day')));
      await tester.pumpAndSettle();
      expect(selected(), {ThemeMode.system});
      bridge.failure = StateError('Mode save failed');
      await tester.tap(find.text('Dark').first);
      await tester.pumpAndSettle();
      expect(selected(), {ThemeMode.system});
      expect(bridge.configs.first.snapshot, firstSaved);
      expect(find.textContaining('Mode save failed'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  if (const bool.fromEnvironment('THEME_DESIGN_CAPTURE')) {
    testWidgets('render the theme library, creation and editor screens', (
      tester,
    ) async {
      final font = FontLoader('Aptos')
        ..addFont(
          rootBundle.load('packages/operit_ui/assets/fonts/noto-sans-sc.ttf'),
        );
      await font.load();
      final icons = FontLoader('MaterialIcons')
        ..addFont(rootBundle.load('fonts/MaterialIcons-Regular.otf'));
      await icons.load();
      final bridge = _ThemeBridge()..configs.clear();
      final background = await tester.runAsync(_landscapeBackground);
      bridge.assets['theme-media/forest.png'] = background!;
      core.CoreDataPreferencesThemeConfigManagerThemeConfig sample(
        String id,
        String name,
        Map<String, Object?> overrides,
      ) => core.CoreDataPreferencesThemeConfigManagerThemeConfig(
        id: id,
        name: name,
        snapshot: {
          ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
          'themeMode': 'light',
          'showMessageTimestamp': false,
          'showModelProvider': false,
          'showModelName': false,
          'showMessageTokenStats': false,
          'showMessageTimingStats': false,
          ...overrides,
        },
        createdAt: 1,
        updatedAt: 1,
      );
      bridge.configs.addAll([
        sample('day', '澄空', {
          'useCustomColors': true,
          'customPrimaryColor': 0xFF3B82F6,
        }),
        sample('night', '子夜', {
          'themeMode': 'dark',
          'userChatStyle': 'bubble',
          'aiChatStyle': 'bubble',
          'useCustomColors': true,
          'customPrimaryColor': 0xFFB7A0EC,
          'bubbleUserBubbleColor': 0xFF51426B,
        }),
        sample('forest', '松间', {
          'useBackgroundImage': true,
          'backgroundImageUri': 'theme-media/forest.png',
          'backgroundImageOpacity': 1.0,
          'useBackgroundBlur': false,
          'useCustomColors': true,
          'customPrimaryColor': 0xFF32775C,
          'userChatStyle': 'bubble',
          'aiChatStyle': 'bubble',
          'bubbleUserBubbleColor': 0xFFD7EBDD,
          'bubbleAiBubbleColor': 0xFFF0F6EE,
          'inputBackgroundColor': 0xFFEAF1E8,
        }),
        sample('sunset', '落日', {
          'useCustomColors': true,
          'customPrimaryColor': 0xFFAE6540,
          'cursorUserBubbleColor': 0xFFF7E0CB,
          'inputStyle': 'classic',
          'inputBackgroundColor': 0xFFFFF5EC,
        }),
        sample('lavender', '雾紫', {
          'useCustomColors': true,
          'customPrimaryColor': 0xFF8065A7,
          'userChatStyle': 'bubble',
          'aiChatStyle': 'bubble',
          'bubbleUserBubbleColor': 0xFFE8DDF4,
          'bubbleAiBubbleColor': 0xFFF4F0F9,
          'inputBackgroundColor': 0xFFF4F0F9,
        }),
      ]);
      final boundary = GlobalKey();
      tester.binding.platformDispatcher.localesTestValue = const [Locale('zh')];
      addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1200, 980);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      await tester.binding.setSurfaceSize(const Size(1200, 980));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      Widget appearance() => RepaintBoundary(
        key: boundary,
        child: OperitTheme(
          key: ValueKey('render-appearance:${bridge.active}'),
          initialThemePreferenceSnapshot: ThemePreferenceSnapshot.fromJson(
            bridge.configs.first.snapshot,
          ),
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Scaffold(
            appBar: AppBar(title: const Text('主题与外观')),
            body: AppearanceSettingsPanel(
              clients: GeneratedCoreProxyClients(bridge),
            ),
          ),
        ),
      );
      await tester.pumpWidget(appearance());
      await tester.pumpAndSettle();
      Future<void> capture(String name) async {
        // Asset decoding uses the engine; let it finish before painting the capture.
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 100));
        });
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        await tester.runAsync(() async {
          final image =
              await (boundary.currentContext!.findRenderObject()!
                      as RenderRepaintBoundary)
                  .toImage(pixelRatio: 2);
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          final folder =
              Platform.environment['OPERIT_THEME_RENDER_DIR'] ??
              '/tmp/operit-theme-review';
          await Directory(folder).create(recursive: true);
          await File(
            '$folder/$name.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
          image.dispose();
        });
        debugPrint('Captured $name');
      }

      expect(find.byType(TabBar), findsNothing);
      await capture('01-theme-library');
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), '我的新主题');
      await tester.tap(
        find.byKey(const ValueKey('theme-source-config:forest')),
      );
      await tester.pumpAndSettle();
      await capture('02-create-theme');
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('theme-edit:day')));
      await tester.pumpAndSettle();
      expect(find.byType(TabBar), findsOneWidget);
      await capture('03-theme-editor');
      await tester.tap(find.byType(BackButton));
      await tester.pumpAndSettle();
      expect(find.byType(TabBar), findsNothing);
      tester.view.physicalSize = const Size(390, 844);
      await tester.binding.setSurfaceSize(const Size(390, 844));
      await tester.pumpAndSettle();
      await capture('04-theme-library-mobile');
      await tester.tap(find.byKey(const ValueKey('theme-edit:day')));
      await tester.pumpAndSettle();
      await capture('05-theme-editor-mobile');
      await tester.tap(find.byType(BackButton));
      await tester.pumpAndSettle();
      tester.view.physicalSize = const Size(1100, 500);
      await tester.binding.setSurfaceSize(const Size(1100, 500));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), '自适应主题');
      await capture('06-create-theme-short-window');
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      bridge.configs
        ..clear()
        ..add(
          sample('migrated', '默认主题', {
            'themeMode': 'dark',
            'useBackgroundImage': true,
            'backgroundImageUri': 'theme-media/forest.png',
            'backgroundImageOpacity': 1.0,
            'useCustomColors': true,
            'customPrimaryColor': 0xFF32775C,
          }),
        );
      bridge.active = 'migrated';
      bridge._publish();
      tester.view.physicalSize = const Size(1000, 600);
      await tester.binding.setSurfaceSize(const Size(1000, 600));
      await tester.pumpAndSettle();
      await tester.pumpWidget(appearance());
      await tester.pumpAndSettle();
      await capture('07-default-theme-from-current-appearance');
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }
}

/// Mounts a compact actual selector with the formal localization and generated proxy dependencies.
Future<void> _mount(
  WidgetTester tester,
  _ThemeBridge bridge, {
  Size size = const Size(1000, 800),
  Locale locale = const Locale('en'),
  Future<void> Function(String)? onEdit,
  ThemePreferenceSnapshot? currentSnapshot,
  double textScale = 1,
}) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      locale: locale,
      theme: buildOperitThemeData(
        ColorScheme.fromSeed(seedColor: const Color(0xFFBBDEFB)),
        currentSnapshot ??
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        fontFamily: 'Aptos',
        fontFamilyFallback: const <String>[],
      ),
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(
          context,
        ).copyWith(textScaler: TextScaler.linear(textScale)),
        child: child!,
      ),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: StatefulBuilder(
          builder: (context, update) => ThemeConfigurationControl(
            clients: GeneratedCoreProxyClients(bridge),
            onEdit: onEdit,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
}

/// Controls only the external theme-owner boundary while the actual widget and codecs execute.
class _ThemeBridge extends OperitRuntimeBridge {
  /// Declares the two genuine configuration records supplied to the selector test.
  _ThemeBridge() : configs = [_config('day', 'Day'), _config('night', 'Night')];

  final List<core.CoreDataPreferencesThemeConfigManagerThemeConfig> configs;
  final List<Map<String, Object?>> packages = [];
  final Map<String, Object?> preferences = {};
  final Map<String, Uint8List> assets = {};
  final List<CoreCallRequest> calls = [];
  final Map<CoreWatchRequest, StreamController<CoreEvent>> watches = {};
  String? active = 'day';
  Object? failure;

  /// Encodes only supported owner operations and publishes committed state after acknowledgements.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map;
    Object? result;
    switch (request.methodName) {
      case 'list':
        result = configs.map((config) => config.toJson()).toList();
      case 'getToolPkgContainerRuntimes':
        result = packages;
      case 'getPreferences':
        result = preferences;
      case 'patchAppearance':
        if (failure case final Object error) throw error;
        preferences.addAll(Map<String, Object?>.from(args['values'] as Map));
        final index = configs.indexWhere((config) => config.id == active);
        final old = configs[index];
        configs[index] = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
          id: old.id,
          name: old.name,
          createdAt: old.createdAt,
          updatedAt: 2,
          snapshot: {
            ...old.snapshot,
            'themeMode': (args['values'] as Map)['theme_mode'],
          },
        );
        result = configs[index].toJson();
        _publish();
      case 'readBase64':
        result = base64Encode(assets[args['path']]!);
      case 'apply':
        if (failure case final Object error) throw error;
        active = args['id'] as String;
        result = configs.singleWhere((config) => config.id == active).toJson();
        _publish();
      case 'create':
        final config = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
          id: 'created',
          name: args['name'] as String,
          snapshot: Map<String, Object?>.from(args['snapshot'] as Map),
          createdAt: 1,
          updatedAt: 1,
        );
        configs.add(config);
        result = config.toJson();
        _publish();
      case 'get':
        result = configs
            .singleWhere((config) => config.id == args['id'])
            .toJson();
      case 'rename':
        final index = configs.indexWhere((config) => config.id == args['id']);
        final old = configs[index];
        configs[index] = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
          id: old.id,
          name: args['name'] as String,
          snapshot: old.snapshot,
          createdAt: old.createdAt,
          updatedAt: 2,
        );
        result = configs[index].toJson();
        _publish();
      case 'delete':
        if (args['id'] == active) {
          throw StateError('Cannot delete active configuration');
        }
        configs.removeWhere((config) => config.id == args['id']);
        result = null;
        _publish();
      default:
        throw StateError(
          'Unexpected theme owner operation: ${request.methodName}',
        );
    }
    return encodeCoreLink([0, result]);
  }

  /// Publishes the real declared owner input over the generated typed watch codec.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'watch') {
      throw StateError('Unexpected theme watch');
    }
    late StreamController<CoreEvent> controller;
    controller = StreamController<CoreEvent>(
      onListen: () => controller.add(_event(request)),
      onCancel: () => watches.remove(request),
    );
    watches[request] = controller;
    return controller.stream;
  }

  /// Emits one committed catalog update to every actual widget subscription.
  void _publish() {
    for (final entry in watches.entries) {
      entry.value.add(_event(entry.key));
    }
  }

  /// Encodes a complete typed catalog event without a stringified payload.
  CoreEvent _event(CoreWatchRequest request) => CoreEvent.raw(
    requestId: request.requestId,
    target: request.target,
    propertyName: request.propertyName,
    kind: 'Changed',
    decodeValue: decodeCoreLink<Object?>,
    valueBytes: encodeCoreLink(
      core.ThemeConfigState(
        configs: configs,
        activeThemeConfigId: active,
      ).toJson(),
    ),
  );

  /// Rejects unrelated push requests instead of simulating another host capability.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw StateError('Unexpected theme push');

  /// Rejects snapshot requests because this selector observes the actual stream contract.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw StateError('Unexpected theme snapshot');
}

/// Builds one complete explicit named-theme record for the external catalog fixture.
core.CoreDataPreferencesThemeConfigManagerThemeConfig _config(
  String id,
  String name,
) => core.CoreDataPreferencesThemeConfigManagerThemeConfig(
  id: id,
  name: name,
  snapshot: {
    ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
    'themeMode': id == 'night' ? 'dark' : 'light',
  },
  createdAt: 1,
  updatedAt: 1,
);

/// Supplies an ordinary plugin registry entry with a visible template declaration.
Map<String, Object?> _templatePackage() => <String, Object?>{
  'packageName': 'dev.example.plugin',
  'displayName': <String, Object?>{
    'values': <String, String>{'en': 'Plugin'},
  },
  'description': <String, Object?>{
    'values': <String, String>{'en': 'Example'},
  },
  'logoResource': null,
  'marketOrigin': null,
  'version': '1.0',
  'apiVersion': '1',
  'mainEntry': '',
  'sourceType': 'EXTERNAL',
  'sourcePath': 'host:///plugin.toolpkg',
  'themes': <Object?>[
    <String, Object?>{
      'id': 'night',
      'path': 'theme.json',
      'preview': 'preview.png',
      'files': <Object?>[],
    },
  ],
  for (final key in <String>[
    'requires',
    'author',
    'subpackages',
    'resources',
    'wasmModules',
    'workflowTemplates',
    'workspaceTemplates',
    'uiModules',
    'uiRoutes',
    'chatComposerSlots',
    'navigationEntries',
    'desktopWidgets',
    'appLifecycleHooks',
    'messageProcessingPlugins',
    'xmlRenderPlugins',
    'inputMenuTogglePlugins',
    'chatInputHooks',
    'chatViewHooks',
    'chatMessageHooks',
    'chatLifecycleHooks',
    'chatMessageMenuItems',
    'chatRuntimeHooks',
    'hostEventHooks',
    'toolLifecycleHooks',
    'promptInputHooks',
    'promptHistoryHooks',
    'promptEstimateHistoryHooks',
    'systemPromptComposeHooks',
    'toolPromptComposeHooks',
    'promptFinalizeHooks',
    'promptEstimateFinalizeHooks',
    'summaryGenerateHooks',
    'coreCommands',
    'aiProviders',
    'manifestExtensionHandlers',
  ])
    key: <Object?>[],
};

/// Creates a real saved background asset for render and storage-path coverage.
Future<Uint8List> _landscapeBackground() async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  const bounds = Rect.fromLTWH(0, 0, 640, 400);
  canvas.drawRect(
    bounds,
    Paint()
      ..shader = const LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [Color(0xFFC8E0DB), Color(0xFFF4E7C9)],
      ).createShader(bounds),
  );
  canvas.drawCircle(
    const Offset(475, 94),
    38,
    Paint()..color = const Color(0xFFFDF0C6),
  );
  for (final layer in [
    (const Color(0xFF8DAF9E), 200.0, 35.0),
    (const Color(0xFF5C8977), 270.0, -10.0),
    (const Color(0xFF315F51), 345.0, 55.0),
  ]) {
    final path = Path()
      ..moveTo(0, layer.$2)
      ..cubicTo(
        150,
        layer.$2 - 120 + layer.$3,
        280,
        layer.$2 + 60,
        420,
        layer.$2 - 30,
      )
      ..quadraticBezierTo(540, layer.$2 - 100, 640, layer.$2 - 20)
      ..lineTo(640, 400)
      ..lineTo(0, 400)
      ..close();
    canvas.drawPath(path, Paint()..color = layer.$1);
  }
  final picture = recorder.endRecording();
  final image = await picture.toImage(640, 400);
  final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
  image.dispose();
  picture.dispose();
  return bytes!.buffer.asUint8List();
}
