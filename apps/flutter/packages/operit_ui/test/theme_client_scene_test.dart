import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

/// Verifies the shared preview shell and its workspace navigation.
void main() {
  testWidgets('Inspector hit target selects the corresponding chat component', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(1280, 800);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);
    String? selected;
    await tester.pumpWidget(
      MaterialApp(
        home: ThemeClientScene(
          snapshot: ThemePreferenceDefaults.snapshot,
          inspectEnabled: true,
          onInspectTargetSelected: (target) => selected = target,
        ),
      ),
    );
    final target = find.byWidgetPredicate(
      (widget) =>
          widget is ThemeInspectTarget &&
          widget.targetId == ThemeInspectableTargetId.userMessage,
    );
    expect(target, findsOneWidget);
    await tester.tap(target);
    expect(selected, ThemeInspectableTargetId.userMessage);
  });

  testWidgets(
    'bubble conversation scene paints both declared message surfaces',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1280, 800);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
        ...ThemePreferenceDefaults.snapshot.toJson(),
        'chatStyle': ThemePreferenceDefaults.CHAT_STYLE_BUBBLE,
      });

      await tester.pumpWidget(
        MaterialApp(
          home: ThemeClientScene(
            snapshot: snapshot,
            mode: ThemeClientSceneMode.conversation,
          ),
        ),
      );
      await tester.pump();

      expect(find.text('帮我把主题工作台调整得更简洁一些。'), findsOneWidget);
      expect(find.text('可以。选择预览里的消息气泡，我会显示它可编辑的全部主题参数。'), findsOneWidget);
      expect(
        find.byWidgetPredicate(
          (widget) =>
              widget is ThemeInspectTarget &&
              (widget.targetId == ThemeInspectableTargetId.userMessage ||
                  widget.targetId == ThemeInspectableTargetId.aiMessage),
        ),
        findsNWidgets(2),
      );
      expect(find.byType(BubbleSurface), findsNWidgets(2));
      final surfaces = tester.widgetList<BubbleSurface>(
        find.byType(BubbleSurface),
      );
      expect(surfaces.first.borderRadius, BorderRadius.circular(16));
      expect(surfaces.last.borderRadius, BorderRadius.circular(16));
      expect(find.byIcon(Icons.person), findsOneWidget);
      expect(find.byIcon(Icons.assistant), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('conversation and empty modes remain valid at phone dimensions', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(390, 844);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);
    final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
      ...ThemePreferenceDefaults.snapshot.toJson(),
      'chatStyle': ThemePreferenceDefaults.CHAT_STYLE_BUBBLE,
    });

    for (final mode in <ThemeClientSceneMode>[
      ThemeClientSceneMode.empty,
      ThemeClientSceneMode.conversation,
    ]) {
      await tester.pumpWidget(
        MaterialApp(
          home: ThemeClientScene(snapshot: snapshot, mode: mode),
        ),
      );
      await tester.pump();
      expect(find.byKey(const ValueKey<String>('chat.input')), findsOneWidget);
      expect(
        find.text('帮我把主题工作台调整得更简洁一些。'),
        mode == ThemeClientSceneMode.conversation
            ? findsOneWidget
            : findsNothing,
      );
      expect(
        find.byType(BubbleSurface),
        mode == ThemeClientSceneMode.conversation
            ? findsNWidgets(2)
            : findsNothing,
      );
      expect(tester.takeException(), isNull);
    }
  });

  testWidgets(
    'empty scene hides messages but keeps the chat shell and composer',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1280, 800);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      await tester.pumpWidget(
        MaterialApp(
          home: ThemeClientScene(
            snapshot: ThemePreferenceDefaults.snapshot,
            mode: ThemeClientSceneMode.empty,
            inspectEnabled: true,
          ),
        ),
      );
      await tester.pump();

      expect(find.text('帮我把主题工作台调整得更简洁一些。'), findsNothing);
      expect(find.text('可以。选择预览里的消息气泡，我会显示它可编辑的全部主题参数。'), findsNothing);
      expect(find.byKey(const ValueKey<String>('chat.input')), findsOneWidget);
      expect(
        find.byWidgetPredicate(
          (widget) =>
              widget is ThemeInspectTarget &&
              (widget.targetId == ThemeInspectableTargetId.userMessage ||
                  widget.targetId == ThemeInspectableTargetId.aiMessage),
        ),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  for (final style in <String>[
    ThemePreferenceDefaults.INPUT_STYLE_AGENT,
    ThemePreferenceDefaults.INPUT_STYLE_CLASSIC,
  ]) {
    testWidgets('client shell paints the $style composer at 1280x800', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1280, 800);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
        ...ThemePreferenceDefaults.snapshot.toJson(),
        'inputStyle': style,
      });
      await tester.pumpWidget(
        MaterialApp(home: ThemeClientScene(snapshot: snapshot)),
      );
      await tester.pump();

      expect(find.byType(ThemeTopBarChrome), findsOneWidget);
      expect(find.byType(ThemeWorkspaceTopBarButton), findsOneWidget);
      expect(find.text('角色卡'), findsNothing);
      expect(find.byIcon(Icons.smart_toy_outlined), findsNothing);
      expect(find.text('未绑定工作区'), findsOneWidget);
      expect(find.text('主题工作台'), findsOneWidget);
      expect(find.byIcon(Icons.work_outline), findsNWidgets(2));
      expect(find.byType(ThemeUnifiedCreateBar), findsOneWidget);
      final sections = tester.widgetList<ThemeHistorySectionHeader>(
        find.byType(ThemeHistorySectionHeader),
      );
      expect(sections.map((section) => section.count), <int>[2, 6]);
      expect(sections.every((section) => section.workspaceStyle), isTrue);
      expect(
        tester
            .widgetList<ThemeHistoryGroupHeader>(
              find.byType(ThemeHistoryGroupHeader),
            )
            .every((group) => group.workspaceStyle),
        isTrue,
      );
      expect(
        tester
            .widgetList<ThemeConversationRow>(find.byType(ThemeConversationRow))
            .every((row) => row.workspaceStyle),
        isTrue,
      );
      expect(
        tester
            .widget<ThemeTopBarTitle>(find.byType(ThemeTopBarTitle))
            .primaryText,
        'Operit',
      );
      expect(find.byType(ThemeHistorySectionHeader), findsNWidgets(2));
      expect(find.byType(ThemeHistoryGroupHeader), findsNWidgets(4));
      expect(find.byType(ThemeConversationRow), findsNWidgets(8));
      expect(find.byKey(const ValueKey<String>('chat.input')), findsOneWidget);
      expect(find.byType(ThemeComposerCard), findsOneWidget);
      expect(
        find.byType(AgentInputSurface),
        style == ThemePreferenceDefaults.INPUT_STYLE_AGENT
            ? findsOneWidget
            : findsNothing,
      );
      expect(
        find.byType(ClassicInputSurface),
        style == ThemePreferenceDefaults.INPUT_STYLE_CLASSIC
            ? findsOneWidget
            : findsNothing,
      );
      expect(
        tester.getSize(find.byType(ThemeClientScene)),
        const Size(1280, 800),
      );
      expect(tester.takeException(), isNull);
    });
    testWidgets('client shell paints the $style composer at 390x844', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(390, 844);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
        ...ThemePreferenceDefaults.snapshot.toJson(),
        'inputStyle': style,
      });
      await tester.pumpWidget(
        MaterialApp(home: ThemeClientScene(snapshot: snapshot)),
      );
      await tester.pump();

      expect(find.byType(ThemeTopBarChrome), findsOneWidget);
      expect(find.byType(ThemeDesktopSidebarSurface), findsNothing);
      expect(find.byType(ThemeComposerCard), findsOneWidget);
      expect(find.byKey(const ValueKey<String>('chat.input')), findsOneWidget);
      expect(
        tester.getSize(find.byType(ThemeClientScene)),
        const Size(390, 844),
      );
      expect(tester.takeException(), isNull);
    });
  }
}
