import 'support/native_sidebar_bridge.dart';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/code_editor/CodeEditor.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorZoomTransform.dart';
import 'package:operit2/ui/common/components/AdaptiveSidePanel.dart';
import 'package:operit2/ui/common/interactions/DrawerGestureExclusion.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspacePanelSurface.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/DrawerMotionScope.dart';
import 'package:operit2/ui/main/layout/PhoneLayout.dart';
import 'package:operit2/ui/main/layout/SidebarDockController.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies drawer transitions preserve the mounted conversation list and input.
void main() {
  testWidgets('phone drawer docks an eligible plugin on a workspace drop', (
    tester,
  ) async {
    const plugin = NavigationEntrySpec(
      entryId: 'plugin.entry',
      routeId: 'plugin.route',
      surface: NavigationSurface.mainSidebarPlugins,
      title: 'Plugin View',
      icon: Icons.extension_outlined,
      kind: NavigationEntryKind.plugin,
    );
    const action = NavigationEntrySpec(
      entryId: 'action.entry',
      routeId: 'action.route',
      surface: NavigationSurface.mainSidebarPlugins,
      title: 'Plugin Action',
      icon: Icons.bolt_outlined,
      kind: NavigationEntryKind.plugin,
      action: NavigationEntryActionSpec(functionName: 'run'),
    );
    final dock = SidebarDockController();
    addTearDown(dock.dispose);
    dock.synchronize(
      pluginEntries: [plugin, action],
      routesById: {
        plugin.routeId: const RouteSpec(
          routeId: 'plugin.route',
          runtime: RouteRuntime.toolPkgComposeDsl,
          ownerPackageName: 'plugin',
          toolPkgUiModuleId: 'module',
        ),
      },
    );
    final layout = MainLayoutController();
    final owner = Object();
    final drawerOpen = ValueNotifier<bool>(true);
    addTearDown(drawerOpen.dispose);
    addTearDown(layout.dispose);
    NavigationEntrySpec? selected;
    var workspaceOpens = 0;
    await _pumpSwipeTestLayout(
      tester,
      drawerOpen: drawerOpen,
      enableNavigationAnimation: true,
      content: const SizedBox.expand(),
      dock: dock,
      mainLayout: layout,
      onNavigationEntrySelected: (entry) => selected = entry,
    );

    final pluginRow = find.text(plugin.title);
    final actionRow = find.text(action.title);
    final dropZone = find.byIcon(Icons.drive_folder_upload_outlined);
    await tester.tap(pluginRow);
    expect(selected, same(plugin));

    final noWorkspace = await tester.startGesture(tester.getCenter(pluginRow));
    await tester.pump(kLongPressTimeout);
    expect(dropZone, findsNothing);
    await noWorkspace.cancel();
    await tester.pumpAndSettle();

    layout.setWorkspaceDockDestination(() => workspaceOpens++, owner: owner);
    await tester.longPress(actionRow);
    expect(dropZone, findsNothing);
    expect(dock.secondaryViews, isEmpty);

    final canceled = await tester.startGesture(tester.getCenter(pluginRow));
    await tester.pump(kLongPressTimeout);
    expect(dropZone, findsOneWidget);
    await canceled.moveTo(tester.getCenter(dropZone));
    await tester.pump();
    await canceled.cancel();
    await tester.pumpAndSettle();
    expect(dropZone, findsNothing);
    expect(drawerOpen.value, isTrue);
    expect(workspaceOpens, 0);
    expect(dock.secondaryViews, isEmpty);

    final accepted = await tester.startGesture(tester.getCenter(pluginRow));
    await tester.pump(kLongPressTimeout);
    await accepted.moveTo(tester.getCenter(dropZone));
    await tester.pump();
    await accepted.up();
    await tester.pumpAndSettle();
    expect(drawerOpen.value, isFalse);
    expect(workspaceOpens, 1);
    expect(dock.primaryEntries, [action]);
    expect(dock.secondaryViews.single.entry, same(plugin));
    expect(dock.selectedSecondaryViewId, plugin.entryId);
    expect(dropZone, findsNothing);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  for (final enableNavigationAnimation in <bool>[true, false]) {
    testWidgets(
      'hidden drawer skips keyboard builds and detaches layouts while retaining state (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(true);
        final conversations = ValueNotifier(
          const DrawerConversationState(loading: false),
        );
        addTearDown(drawerOpen.dispose);
        addTearDown(conversations.dispose);
        addTearDown(tester.view.resetViewInsets);
        final counts = _ContentCounts();
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: _ContentProbe(counts: counts),
          conversationState: conversations,
        );
        await tester.tap(find.byTooltip('搜索对话'));
        await tester.pumpAndSettle();
        await tester.enterText(find.byType(TextField), 'retained search');
        await tester.pumpAndSettle();
        final drawer = find.byType(DrawerContent);
        final drawerState = tester.state(drawer);
        final elements = <Element>{};
        final renders = <RenderObject>[];

        /// Records the real sidebar subtree so hidden work is observed directly.
        void collect(Element element) {
          elements.add(element);
          if (element is RenderObjectElement) renders.add(element.renderObject);
          element.visitChildren(collect);
        }

        collect(tester.element(drawer));
        drawerOpen.value = false;
        await tester.pumpAndSettle();
        expect(renders.every((render) => !render.attached), isTrue);
        final initialLayouts = counts.layouts;
        var hiddenBuilds = 0;
        final previous = debugOnRebuildDirtyWidget;
        debugOnRebuildDirtyWidget = (element, builtOnce) {
          previous?.call(element, builtOnce);
          if (elements.contains(element)) hiddenBuilds++;
        };
        addTearDown(() => debugOnRebuildDirtyWidget = previous);

        for (var cycle = 0; cycle < 3; cycle++) {
          conversations.value = DrawerConversationState(
            loading: false,
            currentChatId: 'latest-$cycle',
            histories: [
              core.ChatHistoryListItem(
                id: 'latest-$cycle',
                title: 'Latest $cycle',
                updatedAt: '0',
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
          for (final inset in <double>[80, 160, 240, 320, 240, 160, 80, 0]) {
            tester.view.viewInsets = FakeViewPadding(bottom: inset);
            await tester.pump(const Duration(milliseconds: 16));
            expect(hiddenBuilds, 0);
            expect(renders.every((render) => !render.attached), isTrue);
            expect(
              tester.getSize(find.byType(_ContentProbe)).height,
              800 - inset,
            );
          }
        }
        expect(counts.layouts, greaterThan(initialLayouts));
        drawerOpen.value = true;
        await tester.pumpAndSettle();
        expect(tester.state(drawer), same(drawerState));
        expect(find.text('retained search'), findsOneWidget);
        expect(tester.widget<DrawerContent>(drawer).currentChatId, 'latest-2');
        expect(hiddenBuilds, greaterThan(0));
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );

    testWidgets(
      'edge swipe opens drawer while eager page keeps pointer events (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        addTearDown(drawerOpen.dispose);
        var moves = 0;
        var releases = 0;
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: _eagerPage(
            onPointerMove: (_) => moves++,
            onPointerUp: (_) => releases++,
          ),
        );
        final gesture = await tester.startGesture(const Offset(12, 400));
        await gesture.moveBy(const Offset(20, 0));
        expect(drawerOpen.value, isFalse);
        await gesture.moveBy(const Offset(35, 0));
        expect(drawerOpen.value, isTrue);
        await gesture.up();
        expect(moves, 2);
        expect(releases, 1);
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'eager page retains interior and vertical drags and edge taps (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        addTearDown(drawerOpen.dispose);
        var releases = 0;
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: _eagerPage(onPointerUp: (_) => releases++),
        );
        await tester.dragFrom(const Offset(80, 400), const Offset(120, 0));
        expect(drawerOpen.value, isFalse);
        final vertical = await tester.startGesture(const Offset(12, 200));
        await vertical.moveBy(const Offset(0, 35));
        await vertical.moveBy(const Offset(120, 0));
        await vertical.up();
        expect(drawerOpen.value, isFalse);
        final leftward = await tester.startGesture(const Offset(12, 200));
        await leftward.moveBy(const Offset(-25, 0));
        await leftward.moveBy(const Offset(150, 0));
        await leftward.up();
        expect(drawerOpen.value, isFalse);
        await tester.tapAt(const Offset(12, 400));
        expect(releases, 4);
        expect(drawerOpen.value, isFalse);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'multi-touch and cancellation abandon edge swipes (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        addTearDown(drawerOpen.dispose);
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: _eagerPage(),
        );
        final first = await tester.startGesture(const Offset(12, 200));
        final second = await tester.startGesture(
          const Offset(120, 400),
          pointer: 2,
        );
        await first.moveBy(const Offset(100, 0));
        await second.up();
        await first.moveBy(const Offset(100, 0));
        await first.up();
        expect(drawerOpen.value, isFalse);
        final cancelled = await tester.startGesture(const Offset(12, 200));
        await cancelled.moveBy(const Offset(20, 0));
        await cancelled.cancel();
        await tester.dragFrom(const Offset(80, 400), const Offset(120, 0));
        expect(drawerOpen.value, isFalse);
        await tester.dragFrom(const Offset(12, 400), const Offset(100, 0));
        expect(drawerOpen.value, isTrue);
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'edge observation respects excluded eager content (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        addTearDown(drawerOpen.dispose);
        var moves = 0;
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: DrawerGestureExclusion(
            child: _eagerPage(onPointerMove: (_) => moves++),
          ),
        );
        await tester.dragFrom(const Offset(12, 400), const Offset(100, 0));
        expect(moves, greaterThan(0));
        expect(drawerOpen.value, isFalse);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'workspace keeps direct single-finger code pans (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        final scroll = CodeEditorScrollController();
        addTearDown(drawerOpen.dispose);
        addTearDown(scroll.dispose);
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: WorkspacePanelSurface(
            transparentSurface: false,
            child: CodeEditor(
              text: List.generate(
                200,
                (index) => 'const line$index = true;',
              ).join('\n'),
              readOnly: true,
              wrapToViewport: false,
              contentWidth: 1200,
              scrollController: scroll,
            ),
          ),
        );
        final transform = find.byType(CodeEditorZoomTransform);

        /// Reads the actual content translation after each single-finger drag.
        double horizontalOffset() =>
            tester.widget<CodeEditorZoomTransform>(transform).horizontalOffset;

        await tester.dragFrom(const Offset(260, 400), const Offset(-120, 0));
        await tester.pumpAndSettle();
        final left = horizontalOffset();
        expect(left, lessThan(0));
        expect(drawerOpen.value, isFalse);

        await tester.dragFrom(const Offset(150, 400), const Offset(80, 0));
        await tester.pumpAndSettle();
        expect(horizontalOffset(), greaterThan(left));
        expect(drawerOpen.value, isFalse);

        await tester.dragFrom(const Offset(12, 400), const Offset(120, 0));
        await tester.pumpAndSettle();
        expect(drawerOpen.value, isFalse);

        await tester.dragFrom(const Offset(200, 500), const Offset(0, -120));
        await tester.pumpAndSettle();
        expect(scroll.offset, greaterThan(0));
        expect(drawerOpen.value, isFalse);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'workspace excludes blank and eager content until closed (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        final workspaceOpen = ValueNotifier<bool>(true);
        addTearDown(drawerOpen.dispose);
        addTearDown(workspaceOpen.dispose);
        var moves = 0;
        var releases = 0;
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: ValueListenableBuilder<bool>(
            valueListenable: workspaceOpen,
            builder: (context, open, _) => AdaptiveSidePanel(
              layout: SidePanelLayout.overlay,
              animate: false,
              open: open,
              onOpenChanged: (value) => workspaceOpen.value = value,
              panel: WorkspacePanelSurface(
                transparentSurface: false,
                child: Column(
                  children: [
                    const SizedBox(height: 64),
                    IconButton(
                      tooltip: 'Close workspace',
                      onPressed: () => workspaceOpen.value = false,
                      icon: const Icon(Icons.close),
                    ),
                    Expanded(
                      child: _eagerPage(
                        onPointerMove: (_) => moves++,
                        onPointerUp: (_) => releases++,
                      ),
                    ),
                  ],
                ),
              ),
              child: const SizedBox.expand(),
            ),
          ),
        );
        await tester.dragFrom(const Offset(150, 32), const Offset(120, 0));
        expect(drawerOpen.value, isFalse);
        await tester.dragFrom(const Offset(12, 32), const Offset(120, 0));
        expect(drawerOpen.value, isFalse);

        await tester.dragFrom(const Offset(12, 400), const Offset(120, 0));
        expect(drawerOpen.value, isFalse);
        expect(moves, greaterThan(0));
        expect(releases, 1);

        await tester.tap(find.byTooltip('Close workspace'));
        await tester.pumpAndSettle();
        expect(workspaceOpen.value, isFalse);
        await tester.dragFrom(const Offset(150, 400), const Offset(120, 0));
        expect(drawerOpen.value, isTrue);
        await tester.pumpAndSettle();
        drawerOpen.value = false;
        await tester.pumpAndSettle();
        await tester.dragFrom(const Offset(12, 400), const Offset(120, 0));
        expect(drawerOpen.value, isTrue);
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'input cursor drag does not open drawer (effects: $enableNavigationAnimation)',
      (tester) async {
        final controller = TextEditingController(
          text: 'Drag the cursor to edit this message',
        );
        final focus = FocusNode();
        final drawerOpen = ValueNotifier<bool>(false);
        addTearDown(controller.dispose);
        addTearDown(focus.dispose);
        addTearDown(drawerOpen.dispose);
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: Align(
            alignment: Alignment.bottomCenter,
            child: TextField(controller: controller, focusNode: focus),
          ),
        );
        await tester.tap(find.byType(TextField));
        await tester.pumpAndSettle();
        final start =
            tester.getTopLeft(find.byType(EditableText)) + const Offset(24, 12);
        final gesture = await tester.startGesture(start);
        await gesture.moveBy(const Offset(24, 0));
        await gesture.moveBy(const Offset(80, 0));
        await tester.pump();
        expect(drawerOpen.value, isFalse);
        await gesture.up();
        expect(focus.hasFocus, isTrue);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
      variant: TargetPlatformVariant.only(TargetPlatform.iOS),
    );
    testWidgets(
      'long press selection drag remains usable (effects: $enableNavigationAnimation)',
      (tester) async {
        final controller = TextEditingController(
          text: 'Select these words without opening the sidebar',
        );
        final drawerOpen = ValueNotifier<bool>(false);
        addTearDown(controller.dispose);
        addTearDown(drawerOpen.dispose);
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: Align(
            alignment: Alignment.bottomCenter,
            child: DrawerGestureExclusion(
              child: TextField(controller: controller, maxLines: 3),
            ),
          ),
        );
        final start =
            tester.getTopLeft(find.byType(EditableText)) + const Offset(24, 12);
        final gesture = await tester.startGesture(start);
        await tester.pump(const Duration(milliseconds: 600));
        final initialSelection = controller.selection;
        expect(initialSelection.isCollapsed, isFalse);
        await gesture.moveBy(const Offset(24, 0));
        await gesture.moveBy(const Offset(100, 0));
        await tester.pump();
        expect(drawerOpen.value, isFalse);
        expect(
          controller.selection.extentOffset,
          greaterThan(initialSelection.extentOffset),
        );
        await gesture.up();
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
      variant: TargetPlatformVariant.only(TargetPlatform.iOS),
    );
    testWidgets(
      'composer padding and cancelled drags do not steal child gestures (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        addTearDown(drawerOpen.dispose);
        var childDragUpdates = 0;
        var taps = 0;
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: Column(
            children: <Widget>[
              const Expanded(child: SizedBox.expand()),
              DrawerGestureExclusion(
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onPanUpdate: (_) => childDragUpdates++,
                    onTap: () => taps++,
                    child: const SizedBox(height: 80, width: double.infinity),
                  ),
                ),
              ),
            ],
          ),
        );
        // The empty padding is protected, and the exclusion follows the
        // pointer even after it moves outside the original input bounds.
        final paddingDrag = await tester.startGesture(const Offset(10, 700));
        await paddingDrag.moveBy(const Offset(24, 0));
        await paddingDrag.moveBy(const Offset(100, -120));
        expect(drawerOpen.value, isFalse);
        await paddingDrag.cancel();
        await tester.dragFrom(const Offset(100, 720), const Offset(100, 0));
        expect(childDragUpdates, greaterThan(0));
        expect(drawerOpen.value, isFalse);
        await tester.tapAt(const Offset(100, 720));
        expect(taps, 1);
        // Cancellation must not suppress the next ordinary drawer gesture.
        await tester.dragFrom(const Offset(20, 400), const Offset(100, 0));
        expect(drawerOpen.value, isTrue);
        await tester.pumpAndSettle();
        await tester.dragFrom(const Offset(200, 400), const Offset(-100, 0));
        expect(drawerOpen.value, isFalse);
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'retains content build layout and paint during drawer frames (effects: $enableNavigationAnimation)',
      (tester) async {
        tester.view.physicalSize = const Size(400, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final drawerOpen = ValueNotifier<bool>(false);
        final conversations = ValueNotifier<DrawerConversationState>(
          const DrawerConversationState(loading: false),
        );
        addTearDown(drawerOpen.dispose);
        addTearDown(conversations.dispose);
        final counts = _ContentCounts();
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
                content: _ContentProbe(counts: counts),
                navigationEntries: const [],
                pluginSidebarEntries: const [],
                selectedRouteId: 'ai_chat',
                drawerConversationState: conversations,
                drawerWidth: 300,
                drawerOpenState: drawerOpen,
                enableNavigationAnimation: enableNavigationAnimation,
                onOpenDrawer: () => drawerOpen.value = true,
                onCloseDrawer: () => drawerOpen.value = false,
                onNavigationEntrySelected: (_) {},
                onConversationActivated: () {},
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final baseline = (counts.builds, counts.layouts, counts.paints);
        expect(baseline.$1, greaterThan(0));
        expect(baseline.$2, greaterThan(0));
        expect(baseline.$3, greaterThan(0));
        final contentElement = tester.element(find.byType(_ContentProbe));
        for (var cycle = 0; cycle < 3; cycle++) {
          await tester.dragFrom(const Offset(20, 400), const Offset(100, 0));
          expect(drawerOpen.value, isTrue);
          for (var frame = 0; frame < 40; frame++) {
            await tester.pump(const Duration(milliseconds: 16));
            expect((counts.builds, counts.layouts, counts.paints), baseline);
          }
          await tester.tapAt(const Offset(350, 400));
          expect(drawerOpen.value, isFalse);
          for (var frame = 0; frame < 40; frame++) {
            await tester.pump(const Duration(milliseconds: 16));
            expect((counts.builds, counts.layouts, counts.paints), baseline);
          }
          expect(
            tester.element(find.byType(_ContentProbe)),
            same(contentElement),
          );
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'keeps non-page content live during drawer motion (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        final revision = ValueNotifier<int>(0);
        final contentKey = GlobalKey();
        addTearDown(drawerOpen.dispose);
        addTearDown(revision.dispose);
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: RepaintBoundary(
            key: contentKey,
            child: ValueListenableBuilder<int>(
              valueListenable: revision,
              builder: (_, value, _) => ColoredBox(
                color: value.isEven ? Colors.red : Colors.blue,
                child: Text('Message revision $value'),
              ),
            ),
          ),
        );
        final motion = tester
            .widget<DrawerMotionScope>(find.byType(DrawerMotionScope))
            .isAnimating;
        final liveContent = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(contentKey),
        );
        final contentElement = tester.element(find.byKey(contentKey));
        expect(find.byType(SnapshotWidget), findsNothing);
        expect(motion.value, isFalse);
        expect(liveContent.debugLayer!.parent, isNotNull);

        for (final open in <bool>[true, false, true, false]) {
          drawerOpen.value = open;
          await tester.pump();
          expect(motion.value, isTrue);
          for (var frame = 0; frame < 5; frame++) {
            revision.value++;
            await tester.pump(const Duration(milliseconds: 16));
            expect(motion.value, isTrue);
            expect(liveContent.debugLayer!.parent, isNotNull);
            expect(
              find.text('Message revision ${revision.value}'),
              findsOneWidget,
            );
          }
          await tester.pumpAndSettle();
          expect(motion.value, isFalse);
          expect(liveContent.debugLayer!.parent, isNotNull);
          expect(tester.element(find.byKey(contentKey)), same(contentElement));
          revision.value++;
          await tester.pump();
          expect(tester.binding.hasScheduledFrame, isFalse);
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'drawer motion handles reversal resize and disposal (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(true);
        addTearDown(drawerOpen.dispose);
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: const ColoredBox(color: Colors.green),
        );
        final motion = tester
            .widget<DrawerMotionScope>(find.byType(DrawerMotionScope))
            .isAnimating;
        expect(motion.value, isFalse);
        drawerOpen.value = false;
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 32));
        expect(motion.value, isTrue);
        drawerOpen.value = true;
        await tester.pump(const Duration(milliseconds: 16));
        expect(motion.value, isTrue);
        tester.view.physicalSize = const Size(400, 600);
        await tester.pump(const Duration(milliseconds: 16));
        expect(tester.getSize(find.byType(PhoneLayout)), const Size(400, 600));
        await tester.pumpAndSettle();
        expect(motion.value, isFalse);
        expect(tester.binding.hasScheduledFrame, isFalse);
        drawerOpen.value = false;
        await tester.pump();
        expect(motion.value, isTrue);
        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
      },
    );
    testWidgets(
      'non-rasterizable host content stays live during drawer motion (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        final contentKey = GlobalKey();
        addTearDown(drawerOpen.dispose);
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: RepaintBoundary(
            key: contentKey,
            child: const _PlatformLayerProbe(),
          ),
        );
        final motion = tester
            .widget<DrawerMotionScope>(find.byType(DrawerMotionScope))
            .isAnimating;
        final liveContent = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(contentKey),
        );
        expect(find.byType(SnapshotWidget), findsNothing);
        for (final open in <bool>[true, false]) {
          drawerOpen.value = open;
          await tester.pump();
          for (var frame = 0; frame < 5; frame++) {
            await tester.pump(const Duration(milliseconds: 16));
            expect(motion.value, isTrue);
            expect(liveContent.debugLayer!.parent, isNotNull);
          }
          await tester.pumpAndSettle();
          expect(motion.value, isFalse);
          expect(tester.takeException(), isNull);
        }
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'hidden running conversation stops scheduling frames (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        addTearDown(drawerOpen.dispose);
        await _pumpSwipeTestLayout(
          tester,
          drawerOpen: drawerOpen,
          enableNavigationAnimation: enableNavigationAnimation,
          content: const SizedBox.expand(),
          drawerState: const DrawerConversationState(
            loading: false,
            activeStreamingChatIds: {'running-chat'},
            histories: [
              core.ChatHistoryListItem(
                id: 'running-chat',
                title: 'Running chat',
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
          ),
        );
        final indicator = find.byKey(
          const ValueKey<String>('conversation-running-indicator'),
        );
        expect(find.byType(DrawerContent, skipOffstage: false), findsNothing);
        expect(tester.binding.transientCallbackCount, 0);

        drawerOpen.value = true;
        await tester.pump();
        await tester.pump(const Duration(seconds: 1));
        final element = tester.element(indicator);
        expect(TickerMode.valuesOf(element).enabled, isTrue);
        expect(tester.binding.transientCallbackCount, greaterThan(0));

        drawerOpen.value = false;
        await tester.pumpAndSettle();
        expect(
          tester.element(
            find.byKey(
              const ValueKey<String>('conversation-running-indicator'),
              skipOffstage: false,
            ),
          ),
          same(element),
        );
        expect(TickerMode.valuesOf(element).enabled, isFalse);
        expect(tester.binding.transientCallbackCount, 0);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
    testWidgets(
      'preserves drawer state across transitions (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        final conversations = ValueNotifier<DrawerConversationState>(
          DrawerConversationState(
            loading: false,
            histories: [
              for (var index = 0; index < 100; index++)
                core.ChatHistoryListItem(
                  id: 'retained-$index',
                  title: 'retained query $index',
                  updatedAt: '0',
                  displayOrder: index,
                  parentChatId: null,
                  group: null,
                  workspaceId: null,
                  workspaceName: null,
                  locked: false,
                  pinned: true,
                ),
            ],
          ),
        );
        addTearDown(drawerOpen.dispose);
        addTearDown(conversations.dispose);

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
                navigationEntries: const [],
                pluginSidebarEntries: const [],
                selectedRouteId: 'ai_chat',
                drawerConversationState: conversations,
                drawerWidth: 300,
                drawerOpenState: drawerOpen,
                enableNavigationAnimation: enableNavigationAnimation,
                onOpenDrawer: () => drawerOpen.value = true,
                onCloseDrawer: () => drawerOpen.value = false,
                onNavigationEntrySelected: (_) {},
                onConversationActivated: () {},
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.byType(DrawerContent, skipOffstage: false), findsNothing);

        drawerOpen.value = true;
        await tester.pumpAndSettle();
        final drawerState = tester.state(find.byType(DrawerContent));

        await tester.tap(find.byTooltip('搜索对话'));
        await tester.pumpAndSettle();
        await tester.enterText(find.byType(TextField), 'retained query');
        await tester.pumpAndSettle();
        final scroll = tester
            .widget<CustomScrollView>(find.byType(CustomScrollView))
            .controller!;
        scroll.jumpTo(500);
        await tester.pumpAndSettle();

        for (var cycle = 0; cycle < 3; cycle++) {
          drawerOpen.value = false;
          await tester.pumpAndSettle();
          expect(
            tester.state(find.byType(DrawerContent, skipOffstage: false)),
            same(drawerState),
          );

          drawerOpen.value = true;
          await tester.pumpAndSettle();
          expect(tester.state(find.byType(DrawerContent)), same(drawerState));
          expect(scroll.position.pixels, 500);
          expect(
            tester
                .widget<TextField>(find.byType(TextField, skipOffstage: false))
                .controller!
                .text,
            'retained query',
          );
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }
}

/// Models a WebView's eager gesture ownership while exposing raw page events.
Widget _eagerPage({
  PointerMoveEventListener? onPointerMove,
  PointerUpEventListener? onPointerUp,
}) => RawGestureDetector(
  gestures: <Type, GestureRecognizerFactory>{
    EagerGestureRecognizer:
        GestureRecognizerFactoryWithHandlers<EagerGestureRecognizer>(
          EagerGestureRecognizer.new,
          (_) {},
        ),
  },
  child: Listener(
    behavior: HitTestBehavior.opaque,
    onPointerMove: onPointerMove,
    onPointerUp: onPointerUp,
    child: const SizedBox.expand(),
  ),
);

/// Mounts the phone layout with isolated preferences and drawer state.
Future<void> _pumpSwipeTestLayout(
  WidgetTester tester, {
  required ValueNotifier<bool> drawerOpen,
  required bool enableNavigationAnimation,
  required Widget content,
  DrawerConversationState drawerState = const DrawerConversationState(
    loading: false,
  ),
  ValueNotifier<DrawerConversationState>? conversationState,
  SidebarDockController? dock,
  MainLayoutController? mainLayout,
  ValueChanged<NavigationEntrySpec>? onNavigationEntrySelected,
}) async {
  tester.view.physicalSize = const Size(400, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final conversations =
      conversationState ?? ValueNotifier<DrawerConversationState>(drawerState);
  if (conversationState == null) addTearDown(conversations.dispose);
  Widget buildLayout(List<NavigationEntrySpec> entries) => PhoneLayout(
    bridge: NativeSidebarBridge(),
    content: content,
    navigationEntries: const [],
    pluginSidebarEntries: entries,
    selectedRouteId: 'ai_chat',
    drawerConversationState: conversations,
    drawerWidth: 300,
    drawerOpenState: drawerOpen,
    enableNavigationAnimation: enableNavigationAnimation,
    onOpenDrawer: () => drawerOpen.value = true,
    onCloseDrawer: () => drawerOpen.value = false,
    onNavigationEntrySelected: onNavigationEntrySelected ?? (_) {},
    onConversationActivated: () {},
  );
  Widget body = dock == null
      ? buildLayout(const [])
      : SidebarDockScope(
          controller: dock,
          child: ListenableBuilder(
            listenable: dock,
            builder: (_, _) => buildLayout(dock.primaryEntries),
          ),
        );
  if (mainLayout != null) {
    body = MainLayoutScope(controller: mainLayout, child: body);
  }
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Scaffold(body: body),
    ),
  );
  await tester.pumpAndSettle();
}

class _ContentCounts {
  int builds = 0;
  int layouts = 0;
  int paints = 0;
}

class _ContentProbe extends StatelessWidget {
  /// Creates a content probe with independently counted rendering phases.
  const _ContentProbe({required this.counts});

  final _ContentCounts counts;

  /// Counts content builds independently of the drawer animation builder.
  @override
  Widget build(BuildContext context) {
    counts.builds++;
    return _ContentRenderProbe(counts: counts);
  }
}

class _ContentRenderProbe extends LeafRenderObjectWidget {
  /// Creates a render probe for the retained content layer.
  const _ContentRenderProbe({required this.counts});

  final _ContentCounts counts;

  /// Creates the render box that records layout and paint calls.
  @override
  RenderObject createRenderObject(BuildContext context) => _ContentBox(counts);
}

class _ContentBox extends RenderBox {
  /// Creates a static content surface backed by shared counters.
  _ContentBox(this.counts);

  final _ContentCounts counts;

  /// Records each content layout under the phone viewport constraints.
  @override
  void performLayout() {
    counts.layouts++;
    size = constraints.biggest;
  }

  /// Records each content repaint and draws an opaque surface.
  @override
  void paint(PaintingContext context, Offset offset) {
    counts.paints++;
    context.canvas.drawRect(offset & size, Paint()..color = Colors.white);
  }
}

/// Emulates a native platform view's rasterization restriction without a plugin.
class _PlatformLayerProbe extends LeafRenderObjectWidget {
  /// Creates a non-rasterizable host content probe.
  const _PlatformLayerProbe();

  /// Creates the render box that contributes a non-rasterizable layer.
  @override
  RenderObject createRenderObject(BuildContext context) => _PlatformLayerBox();
}

class _PlatformLayerBox extends RenderBox {
  /// Requires a composited layer for the host surface.
  @override
  bool get alwaysNeedsCompositing => true;

  /// Sizes the host surface to the available content viewport.
  @override
  void performLayout() => size = constraints.biggest;

  /// Adds the host surface layer without drawing a raster image.
  @override
  void paint(PaintingContext context, Offset offset) {
    context.addLayer(_NonRasterizableLayer());
  }
}

class _NonRasterizableLayer extends ContainerLayer {
  /// Models a host surface that is not represented by a raster capture.
  @override
  bool supportsRasterization() => false;
}
