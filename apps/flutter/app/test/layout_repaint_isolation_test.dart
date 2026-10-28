import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitGlassSurface.dart';
import 'package:operit2/ui/main/components/CollapsedDrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/PhoneLayout.dart';
import 'package:operit2/ui/main/layout/TabletLayout.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';
import 'package:operit2/l10n/generated/app_localizations_en.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit_ui/operit_ui.dart';

import 'support/native_sidebar_bridge.dart';

void main() {
  for (final layout in [
    'phone-effects',
    'phone',
    'tablet',
    'tablet-collapsed',
  ]) {
    for (final transparent in [false, true]) {
      testWidgets(
        '$layout isolates repaint in both directions (glass: $transparent)',
        (tester) async {
          tester.view.physicalSize = layout.startsWith('phone')
              ? const Size(400, 800)
              : const Size(1200, 800);
          tester.view.devicePixelRatio = 1;
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);
          final conversations = ValueNotifier(_drawerState());
          final drawerOpen = ValueNotifier(true);
          final revision = ValueNotifier(0);
          addTearDown(conversations.dispose);
          addTearDown(drawerOpen.dispose);
          addTearDown(revision.dispose);
          final counts = _Counts();
          final content = ValueListenableBuilder<int>(
            valueListenable: revision,
            builder: (_, value, _) => _ContentProbe(counts, value),
          );
          final bridge = NativeSidebarBridge();
          final snapshot = ThemePreferenceSnapshot.fromJson({
            ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
            'transparentSurfaceEnabled': transparent,
          });
          await tester.pumpWidget(
            OperitTheme(
              initialThemePreferenceSnapshot: snapshot,
              initialThemeIsReady: false,
              unconfiguredChildEnabled: true,
              hostInteractionHostsEnabled: false,
              child: Scaffold(
                body: layout.startsWith('phone')
                    ? PhoneLayout(
                        bridge: bridge,
                        content: content,
                        navigationEntries:
                            ScreenRouteRegistry.mainSidebarEntries(
                              AppLocalizationsEn(),
                            ),
                        pluginSidebarEntries: const [],
                        selectedRouteId: 'ai_chat',
                        drawerConversationState: conversations,
                        drawerWidth: 300,
                        drawerOpenState: drawerOpen,
                        enableNavigationAnimation: layout == 'phone-effects',
                        onOpenDrawer: () => drawerOpen.value = true,
                        onCloseDrawer: () => drawerOpen.value = false,
                        onNavigationEntrySelected: (_) {},
                        onConversationActivated: () {},
                      )
                    : TabletLayout(
                        bridge: bridge,
                        content: content,
                        navigationEntries:
                            ScreenRouteRegistry.mainSidebarEntries(
                              AppLocalizationsEn(),
                            ),
                        pluginSidebarEntries: const [],
                        selectedRouteId: 'ai_chat',
                        drawerConversationState: conversations,
                        isTabletSidebarExpanded: layout == 'tablet',
                        tabletSidebarWidth: 300,
                        collapsedTabletSidebarWidth: 72,
                        onNavigationEntrySelected: (_) {},
                        onConversationActivated: () {},
                      ),
              ),
            ),
          );
          await tester.pumpAndSettle();
          final main = tester.renderObject<_ContentBox>(
            find.byType(_ContentRenderProbe),
          );
          final surface = layout.startsWith('phone')
              ? find.descendant(
                  of: find.byKey(const ValueKey('phoneDrawerLayer')),
                  matching: find.byType(OperitGlassSurface),
                )
              : find.byType(ThemeDesktopSidebarSurface);
          final sidebar = _nearestBoundary(tester.element(surface));
          final mainBoundary = _nearestBoundary(
            tester.element(find.byType(_ContentProbe)),
          );
          expect(sidebar, isNot(same(mainBoundary)));
          // Neither boundary may be the shared ancestor of the other pane.
          expect(_hasAncestor(mainBoundary, sidebar), isFalse);
          expect(_hasAncestor(sidebar, mainBoundary), isFalse);
          final baseline = counts.snapshot;

          // Real running status ticks must not rebuild, lay out or paint content.
          if (layout != 'tablet-collapsed') {
            conversations.value = _drawerState(running: true);
            await tester.pump();
            await tester.pump(const Duration(seconds: 1));
            final indicator = find.byKey(
              const ValueKey('conversation-running-indicator'),
            );
            final statusBoundary = _nearestBoundary(tester.element(indicator));
            statusBoundary.debugResetMetrics();
            for (var frame = 0; frame < 5; frame++) {
              await tester.pump(const Duration(milliseconds: 16));
              expect(counts.snapshot, baseline);
            }
            expect(statusBoundary.debugAsymmetricPaintCount, greaterThan(0));
            expect(statusBoundary.debugSymmetricPaintCount, 0);
            conversations.value = _drawerState();
            await tester.pumpAndSettle();
          }

          // Sidebar data changes exercise layout/paint beyond the status dot.
          conversations.value = _drawerState(
            title: 'Updated conversation title',
          );
          await tester.pumpAndSettle();
          expect(counts.snapshot, baseline);
          if (layout != 'tablet-collapsed') {
            expect(find.text('Updated conversation title'), findsOneWidget);
          }

          sidebar.debugResetMetrics();
          mainBoundary.debugResetMetrics();
          sidebar.markNeedsPaint();
          await tester.pump();
          expect(sidebar.debugAsymmetricPaintCount, 1);
          expect(sidebar.debugSymmetricPaintCount, 0);
          expect(counts.snapshot, baseline);
          expect(mainBoundary.debugAsymmetricPaintCount, 0);
          expect(mainBoundary.debugSymmetricPaintCount, 0);

          // Main content rebuilds and repaints while the entire sidebar stays put.
          final sidebarElement = tester.element(
            layout == 'tablet-collapsed'
                ? find.byType(CollapsedDrawerContent)
                : find.byType(DrawerContent),
          );
          final sidebarWidgets = <Element, Widget>{};
          void capture(Element element) {
            sidebarWidgets[element] = element.widget;
            element.visitChildren(capture);
          }

          capture(sidebarElement);
          sidebar.debugResetMetrics();
          for (var frame = 0; frame < 5; frame++) {
            revision.value++;
            await tester.pump();
            expect(counts.builds, baseline.$1 + frame + 1);
            expect(counts.layouts, baseline.$2);
            expect(counts.paints, baseline.$3 + frame + 1);
            expect(sidebar.debugAsymmetricPaintCount, 0);
            expect(sidebar.debugSymmetricPaintCount, 0);
            for (final entry in sidebarWidgets.entries) {
              expect(entry.key.widget, same(entry.value));
            }
          }
          expect(main.attached, isTrue);
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }
  }
}

bool _hasAncestor(RenderObject child, RenderObject ancestor) {
  for (RenderObject? node = child.parent; node != null; node = node.parent) {
    if (identical(node, ancestor)) return true;
  }
  return false;
}

RenderRepaintBoundary _nearestBoundary(Element element) {
  RenderRepaintBoundary? result;
  element.visitAncestorElements((ancestor) {
    if (ancestor is RenderObjectElement &&
        ancestor.renderObject is RenderRepaintBoundary) {
      result = ancestor.renderObject as RenderRepaintBoundary;
      return false;
    }
    return true;
  });
  return result!;
}

DrawerConversationState _drawerState({
  bool running = false,
  String title = 'Running conversation',
}) => DrawerConversationState(
  loading: false,
  activeStreamingChatIds: running ? const {'running-chat'} : const {},
  histories: [
    core.ChatHistoryListItem(
      id: 'running-chat',
      title: title,
      updatedAt: '2026-10-28T00:00:00Z',
      displayOrder: 0,
      parentChatId: null,
      group: null,
      workspaceId: null,
      workspaceName: null,
      locked: false,
      pinned: false,
    ),
  ],
);

class _Counts {
  int builds = 0;
  int layouts = 0;
  int paints = 0;
  (int, int, int) get snapshot => (builds, layouts, paints);
}

class _ContentProbe extends StatelessWidget {
  const _ContentProbe(this.counts, this.revision);
  final _Counts counts;
  final int revision;

  @override
  Widget build(BuildContext context) {
    counts.builds++;
    return _ContentRenderProbe(counts, revision);
  }
}

class _ContentRenderProbe extends LeafRenderObjectWidget {
  const _ContentRenderProbe(this.counts, this.revision);
  final _Counts counts;
  final int revision;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _ContentBox(counts, revision);

  @override
  void updateRenderObject(BuildContext context, _ContentBox renderObject) {
    if (renderObject.revision != revision) {
      renderObject.revision = revision;
      renderObject.markNeedsPaint();
    }
  }
}

class _ContentBox extends RenderBox {
  _ContentBox(this.counts, this.revision);
  final _Counts counts;
  int revision;

  @override
  void performLayout() {
    counts.layouts++;
    size = constraints.biggest;
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    counts.paints++;
    context.canvas.drawRect(
      offset & size,
      Paint()..color = revision.isEven ? Colors.red : Colors.blue,
    );
  }
}
