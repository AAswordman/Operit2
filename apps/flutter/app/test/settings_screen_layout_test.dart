import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/components/RetainedPage.dart';
import 'package:operit2/ui/features/settings/about/AboutOperitScreen.dart';
import 'package:operit2/ui/features/settings/components/SettingsCategoryList.dart';
import 'package:operit2/ui/features/settings/models/SettingsModels.dart';
import 'package:operit2/ui/features/settings/screens/SettingsScreen.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:package_info_plus/package_info_plus.dart';

/// Verifies responsive settings layouts with installed application metadata.
void main() {
  setUp(() {
    PackageInfo.setMockInitialValues(
      appName: 'Operit2',
      packageName: 'app.operit',
      version: '2.0.0',
      buildNumber: '15',
      buildSignature: '',
    );
  });

  testWidgets('phone settings link directly to the token usage report', (
    tester,
  ) async {
    RouteEntry? openedRoute;
    AppRouterGateway.install(
      handler: (routeId, args, source) {
        openedRoute = RouteEntry(routeId: routeId, args: args, source: source);
      },
      reset: (routeId, args, source) => fail('Unexpected route reset'),
    );
    addTearDown(AppRouterGateway.clear);

    final availableWidth = ValueNotifier<double>(390);
    final hidden = ValueNotifier<bool>(false);
    final topBar = TopBarController();
    addTearDown(availableWidth.dispose);
    addTearDown(hidden.dispose);
    addTearDown(topBar.dispose);
    await _pumpSettings(
      tester,
      availableWidth,
      hidden,
      topBar,
      initialCategory: null,
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Token Usage Report'));
    expect(
      openedRoute?.routeId,
      ScreenRouteRegistry.routeIdOf(const SettingsScreenRoute()),
    );
    expect(openedRoute?.args['category'], SettingsCategory.usage.name);
    expect(
      ScreenRouteRegistry.screenFromEntry(openedRoute!),
      isA<SettingsScreenRoute>().having(
        (screen) => screen.category,
        'category',
        SettingsCategory.usage,
      ),
    );
  });

  for (final width in <double>[340, 520, 759, 760, 920]) {
    testWidgets('settings use their available width: $width', (tester) async {
      final availableWidth = ValueNotifier<double>(width);
      final hidden = ValueNotifier<bool>(false);
      final topBar = TopBarController();
      addTearDown(availableWidth.dispose);
      addTearDown(hidden.dispose);
      addTearDown(topBar.dispose);
      await _pumpSettings(tester, availableWidth, hidden, topBar);
      await tester.pumpAndSettle();

      expect(
        find.byType(SettingsCategoryList),
        width >= 760 ? findsOneWidget : findsNothing,
      );
      expect(
        tester.getSize(find.byType(AboutOperitScreen)).width,
        width >= 760 ? width - 260 : width,
      );
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('cached settings retain layout until reactivated after resizing', (
    tester,
  ) async {
    final availableWidth = ValueNotifier<double>(920);
    final hidden = ValueNotifier<bool>(false);
    final topBar = TopBarController();
    addTearDown(availableWidth.dispose);
    addTearDown(hidden.dispose);
    addTearDown(topBar.dispose);
    await _pumpSettings(tester, availableWidth, hidden, topBar);
    await tester.pumpAndSettle();
    final state = tester.state(find.byType(SettingsScreen));

    hidden.value = true;
    await tester.pumpAndSettle();
    for (final width in <double>[800, 760, 744, 640, 520, 340, 520, 760, 920]) {
      availableWidth.value = width;
      await tester.pump();
      expect(
        tester.state(find.byType(SettingsScreen, skipOffstage: false)),
        same(state),
      );
      expect(
        find.byType(SettingsCategoryList, skipOffstage: false),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    }
    availableWidth.value = 340;
    await tester.pump();
    hidden.value = false;
    await tester.pumpAndSettle();
    expect(find.byType(SettingsCategoryList), findsNothing);
    expect(tester.getSize(find.byType(AboutOperitScreen)).width, 340);
    expect(tester.state(find.byType(SettingsScreen)), same(state));
    expect(tester.takeException(), isNull);
  });
}

/// Renders responsive settings inside a resizable desktop navigation area.
Future<void> _pumpSettings(
  WidgetTester tester,
  ValueNotifier<double> availableWidth,
  ValueNotifier<bool> hidden,
  TopBarController topBar, {
  SettingsCategory? initialCategory = SettingsCategory.about,
}) async {
  // A desktop window stays wide while navigation leaves less space for settings.
  tester.view.physicalSize = const Size(1200, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: TopBarScope(
        controller: topBar,
        child: Scaffold(
          body: Align(
            alignment: Alignment.centerLeft,
            child: ValueListenableBuilder<double>(
              valueListenable: availableWidth,
              child: ValueListenableBuilder<bool>(
                valueListenable: hidden,
                child: SettingsScreen(initialCategory: initialCategory),
                builder: (context, offstage, child) =>
                    RetainedPage(active: !offstage, child: child!),
              ),
              builder: (context, width, child) =>
                  SizedBox(width: width, child: child),
            ),
          ),
        ),
      ),
    ),
  );
}
