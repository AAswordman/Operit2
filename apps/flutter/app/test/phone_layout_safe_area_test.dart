import 'support/native_sidebar_bridge.dart';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/main/components/CollapsedDrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/PhoneLayout.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/theme/OperitGlassSurface.dart';

/// Verifies edge-to-edge drawer backgrounds with safely inset controls.
void main() {
  for (final animated in <bool>[true, false]) {
    for (final bottomInset in <double>[0, 24, 48, 80]) {
      testWidgets(
        'phone drawer actions avoid $bottomInset bottom inset (effects: $animated)',
        (tester) async {
          tester.view.physicalSize = const Size(400, 800);
          tester.view.devicePixelRatio = 1;
          tester.view.padding = FakeViewPadding(top: 24, bottom: bottomInset);
          tester.view.viewPadding = FakeViewPadding(
            top: 24,
            bottom: bottomInset,
          );
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);
          addTearDown(tester.view.resetPadding);
          addTearDown(tester.view.resetViewPadding);

          final drawerOpen = ValueNotifier<bool>(false);
          final conversations = ValueNotifier<DrawerConversationState>(
            const DrawerConversationState(loading: false),
          );
          addTearDown(drawerOpen.dispose);
          addTearDown(conversations.dispose);
          final selectedEntries = <String>[];

          await tester.pumpWidget(
            OperitTheme(
              initialThemePreferenceSnapshot:
                  UserPreferencesManager.defaultThemePreferenceSnapshot,
              initialThemeIsReady: false,
              unconfiguredChildEnabled: true,
              hostInteractionHostsEnabled: false,
              child: Scaffold(
                body: PhoneLayout(
                  bridge: NativeSidebarBridge(),
                  content: const SizedBox.expand(),
                  navigationEntries: const <NavigationEntrySpec>[
                    NavigationEntrySpec(
                      entryId: 'main.package_manager',
                      routeId: 'package_manager',
                      surface: NavigationSurface.mainSidebarSystem,
                      title: '包管理',
                      icon: Icons.inventory_2_outlined,
                    ),
                    NavigationEntrySpec(
                      entryId: 'main.settings',
                      routeId: 'settings',
                      surface: NavigationSurface.mainSidebarSystem,
                      title: '设置',
                      icon: Icons.settings_outlined,
                    ),
                  ],
                  pluginSidebarEntries: const [],
                  selectedRouteId: 'ai_chat',
                  drawerConversationState: conversations,
                  drawerWidth: 300,
                  drawerOpenState: drawerOpen,
                  enableNavigationAnimation: animated,
                  onOpenDrawer: () => drawerOpen.value = true,
                  onCloseDrawer: () => drawerOpen.value = false,
                  onNavigationEntrySelected: (entry) =>
                      selectedEntries.add(entry.entryId),
                  onConversationActivated: () {},
                ),
              ),
            ),
          );
          await tester.pumpAndSettle();
          drawerOpen.value = true;
          await tester.pumpAndSettle();

          /// Checks surface coverage without moving drawer content under bars.
          void expectActionsAboveNavigationBar(double inset, double topInset) {
            final surface = find.descendant(
              of: find.byKey(const ValueKey<String>('phoneDrawerLayer')),
              matching: find.byType(OperitGlassSurface),
            );
            // The spring can settle with a subpixel horizontal translation.
            expect(tester.getSize(surface), const Size(300, 800));
            expect(tester.getRect(surface).top, closeTo(0, 0.1));
            expect(tester.getRect(surface).bottom, closeTo(800, 0.1));
            expect(
              tester.getRect(find.byType(DrawerContent)).top,
              closeTo(topInset, 0.1),
            );
            expect(
              tester.getRect(find.byType(DrawerContent)).bottom,
              closeTo(800 - inset, 0.1),
            );
            for (final action in tester.widgetList<BottomSidebarAction>(
              find.byType(BottomSidebarAction),
            )) {
              expect(
                tester.getRect(find.byWidget(action)).bottom,
                lessThanOrEqualTo(800 - inset - 15.9),
              );
            }
          }

          expectActionsAboveNavigationBar(bottomInset, 24);
          await tester.tap(find.text('包管理'));
          await tester.tap(find.text('设置'));
          await tester.pumpAndSettle();
          expect(selectedEntries, <String>[
            'main.package_manager',
            'main.settings',
          ]);

          // Switching system navigation modes must update the mounted drawer.
          final drawerState = tester.state(find.byType(DrawerContent));
          final updatedInset = bottomInset == 48 ? 24.0 : 48.0;
          tester.view.padding = FakeViewPadding(top: 36, bottom: updatedInset);
          tester.view.viewPadding = FakeViewPadding(
            top: 36,
            bottom: updatedInset,
          );
          await tester.pumpAndSettle();
          expectActionsAboveNavigationBar(updatedInset, 36);
          expect(tester.state(find.byType(DrawerContent)), same(drawerState));
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }
  }
}
