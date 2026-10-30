import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/ui/main/design/SoftwareDesignCatalog.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';

void main() {
  test(
    'catalog covers every host tab/category and dynamically registered plugin route',
    () async {
      final initial = RouteEntry(routeId: 'native.ai_chat');
      final router = AppRouterState(initial);
      final routes = [
        const RouteSpec(
          routeId: 'native.ai_chat',
          runtime: RouteRuntime.native,
        ),
        RouteSpec(
          routeId: ScreenRouteRegistry.routeIdOf(const SettingsScreenRoute()),
          runtime: RouteRuntime.native,
        ),
        RouteSpec(
          routeId: ScreenRouteRegistry.routeIdOf(
            const PackageManagerScreenRoute(),
          ),
          runtime: RouteRuntime.native,
        ),
        RouteSpec(
          routeId: ScreenRouteRegistry.routeIdOf(const MarketScreenRoute()),
          runtime: RouteRuntime.native,
        ),
        const RouteSpec(
          routeId: 'plugin.settings',
          runtime: RouteRuntime.toolPkgComposeDsl,
          ownerPackageName: 'demo',
          toolPkgUiModuleId: 'settings',
        ),
      ];
      final navigated = <Map<String, Object?>>[];
      final catalog = SoftwareDesignCatalog(
        router: router,
        routes: () => routes,
        navigate: (id, args, source) {
          navigated.add({'route': id, 'args': args});
          router.navigate(routeId: id, args: args, source: source);
        },
      );
      final scenarios = catalog.scenarios();
      expect(scenarios.map((s) => s.id).toSet().length, scenarios.length);
      expect(scenarios.where((s) => s.group == 'settings').length, 12);
      expect(scenarios.where((s) => s.group == 'packages').length, 4);
      expect(scenarios.where((s) => s.group == 'market').length, 3);
      expect(
        scenarios.singleWhere((s) => s.group == 'plugins').source['package'],
        'demo',
      );
      await scenarios.singleWhere((s) => s.id.endsWith('/about')).open();
      expect(navigated.single['args'], {'category': 'about'});
      expect(router.currentEntry.routeId, 'native.settings');
      catalog.restore();
      expect(identical(router.currentEntry, initial), isTrue);
      expect(router.backStack.length, 1);
      router.dispose();
    },
  );
  test(
    'scenario registry restores temporary navigation and rejects ambiguous ids',
    () async {
      final owner = Object();
      var opened = 0, restored = 0;
      OperitDesignScenarios.install(
        owner: owner,
        list: () => [
          OperitDesignScenario(
            id: 'settings',
            name: 'Settings',
            group: 'settings',
            open: () {
              opened++;
            },
          ),
        ],
        restore: () {
          restored++;
        },
      );
      expect(OperitDesignScenarios.list().single['group'], 'settings');
      await OperitDesignScenarios.open('settings');
      expect(opened, 1);
      expect(restored, 1);
      await expectLater(
        OperitDesignScenarios.open('unknown'),
        throwsStateError,
      );
      OperitDesignScenarios.uninstall(Object());
      expect(OperitDesignScenarios.list().length, 1);
      OperitDesignScenarios.uninstall(owner);
      expect(OperitDesignScenarios.list, throwsStateError);
    },
  );
}
