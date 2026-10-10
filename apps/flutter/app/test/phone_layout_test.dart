import 'support/native_sidebar_bridge.dart';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/interactions/DrawerGestureExclusion.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/DrawerMotionScope.dart';
import 'package:operit2/ui/main/layout/PhoneLayout.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies drawer transitions preserve the mounted conversation list and input.
void main() {
  for (final enableNavigationAnimation in <bool>[true, false]) {
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
      'preserves drawer state across transitions (effects: $enableNavigationAnimation)',
      (tester) async {
        final drawerOpen = ValueNotifier<bool>(false);
        final conversations = ValueNotifier<DrawerConversationState>(
          const DrawerConversationState(loading: false),
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
        final drawerState = tester.state(find.byType(DrawerContent));

        drawerOpen.value = true;
        await tester.pumpAndSettle();
        expect(tester.state(find.byType(DrawerContent)), same(drawerState));

        await tester.tap(find.byTooltip('搜索对话'));
        await tester.pumpAndSettle();
        await tester.enterText(find.byType(TextField), 'retained query');
        await tester.pumpAndSettle();

        for (var cycle = 0; cycle < 3; cycle++) {
          drawerOpen.value = false;
          await tester.pumpAndSettle();
          expect(tester.state(find.byType(DrawerContent)), same(drawerState));

          drawerOpen.value = true;
          await tester.pumpAndSettle();
          expect(tester.state(find.byType(DrawerContent)), same(drawerState));
          expect(find.text('retained query'), findsOneWidget);
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }
}

Future<void> _pumpSwipeTestLayout(
  WidgetTester tester, {
  required ValueNotifier<bool> drawerOpen,
  required bool enableNavigationAnimation,
  required Widget content,
}) async {
  tester.view.physicalSize = const Size(400, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final conversations = ValueNotifier<DrawerConversationState>(
    const DrawerConversationState(loading: false),
  );
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
          content: content,
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
