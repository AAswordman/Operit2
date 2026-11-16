import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

/// Verifies the shared preview shell and its workspace navigation.
void main() {
  testWidgets(
    'theme cards use localized samples without the full navigation bar',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(560, 360);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      await tester.pumpWidget(
        MaterialApp(
          home: ThemeClientScene(
            snapshot: ThemePreferenceDefaults.snapshot,
            showTopBar: false,
            userSample: 'Plan my day',
            aiSample: 'Here is your task list.',
          ),
        ),
      );
      await tester.pump();
      expect(find.text('Plan my day'), findsOneWidget);
      expect(find.text('Here is your task list.'), findsOneWidget);
      expect(find.byType(ThemeTopBarChrome), findsNothing);
      expect(find.byKey(const ValueKey<String>('chat.input')), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  const bubbleGeometry = [
    (
      sceneWidth: 320.0,
      normalColumn: 288.0,
      normalOn: 210.8,
      normalOff: 244.8,
      wideColumn: 288.0,
    ),
    (
      sceneWidth: 360.0,
      normalColumn: 328.0,
      normalOn: 244.8,
      normalOff: 278.8,
      wideColumn: 328.0,
    ),
    (
      sceneWidth: 464.0,
      normalColumn: 432.0,
      normalOn: 333.2,
      normalOff: 367.2,
      wideColumn: 432.0,
    ),
    // The fixed Studio sidebar consumes 280 of the local scene width.
    (
      sceneWidth: 793.7,
      normalColumn: 481.7,
      normalOn: 375.445,
      normalOff: 409.445,
      wideColumn: 481.7,
    ),
    (
      sceneWidth: 1214.0,
      normalColumn: 700.0,
      normalOn: 561.0,
      normalOff: 595.0,
      wideColumn: 902.0,
    ),
    (
      sceneWidth: 1854.0,
      normalColumn: 700.0,
      normalOn: 561.0,
      normalOff: 595.0,
      wideColumn: 1040.0,
    ),
  ];
  for (final geometry in bubbleGeometry) {
    for (final wide in [false, true]) {
      for (final showAvatar in [false, true]) {
        for (final padding in [8.0, 12.0]) {
          testWidgets(
            'local scene ${geometry.sceneWidth}, wide=$wide, '
            'avatar=$showAvatar, padding=$padding matches bubble geometry',
            (tester) async {
              await _pumpLocalBubbleScene(
                tester,
                sceneWidth: geometry.sceneWidth,
                wide: wide,
                showAvatar: showAvatar,
                left: padding,
                right: padding,
              );
              final expectedColumn = wide
                  ? geometry.wideColumn
                  : geometry.normalColumn;
              final expectedBubble = wide
                  ? geometry.wideColumn
                  : showAvatar
                  ? geometry.normalOn
                  : geometry.normalOff;
              for (final user in [false, true]) {
                _expectSceneBubbleGeometry(
                  tester,
                  user: user,
                  wide: wide,
                  showAvatar: showAvatar,
                  columnWidth: expectedColumn,
                  bubbleWidth: expectedBubble,
                  left: padding,
                  right: padding,
                );
              }
              expect(tester.takeException(), isNull);
            },
          );
        }
      }
    }
  }

  for (final (left, right) in [(0.0, 32.0), (30.0, 2.0)]) {
    for (final wide in [false, true]) {
      for (final showAvatar in [false, true]) {
        testWidgets(
          'local bubble retains asymmetric $left/$right, '
          'wide=$wide, avatar=$showAvatar',
          (tester) async {
            await _pumpLocalBubbleScene(
              tester,
              sceneWidth: 320,
              wide: wide,
              showAvatar: showAvatar,
              left: left,
              right: right,
            );
            for (final user in [false, true]) {
              _expectSceneBubbleGeometry(
                tester,
                user: user,
                wide: wide,
                showAvatar: showAvatar,
                columnWidth: 288,
                bubbleWidth: wide
                    ? 288
                    : showAvatar
                    ? 210.8
                    : 244.8,
                left: left,
                right: right,
              );
            }
            expect(tester.takeException(), isNull);
          },
        );
      }
    }
  }

  testWidgets('wide short AI fills the column while user remains compact', (
    tester,
  ) async {
    await _pumpLocalBubbleScene(
      tester,
      sceneWidth: 360,
      wide: true,
      showAvatar: true,
      sample: 'OK',
    );
    final ai = _sceneBubble(false);
    final user = _sceneBubble(true);
    expect(tester.getSize(ai).width, closeTo(328, 0.01));
    final userSurface = tester.widget<BubbleSurface>(user);
    final userPadding = userSurface.child as Padding;
    final textWidth = tester.getSize(find.byWidget(userPadding.child!)).width;
    expect(tester.getSize(user).width, closeTo(textWidth + 16, 0.01));
    final column = tester.getRect(
      find
          .ancestor(of: _sceneMessageTarget(true), matching: find.byType(Column))
          .first,
    );
    expect(tester.getRect(user).right, closeTo(column.right, 0.01));
    expect(tester.takeException(), isNull);
  });

  for (final padding in [8.0, 12.0]) {
    testWidgets('cursor sample retains fixed insets at density $padding', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(2400, 1200);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
        ...ThemePreferenceDefaults.snapshot.toJson(),
        'userChatStyle': ThemePreferenceDefaults.CHAT_STYLE_CURSOR,
        'aiChatStyle': ThemePreferenceDefaults.CHAT_STYLE_CURSOR,
        'bubbleShowAvatar': true,
        'bubbleUserContentPaddingLeft': padding,
        'bubbleUserContentPaddingRight': padding,
        'bubbleAiContentPaddingLeft': padding,
        'bubbleAiContentPaddingRight': padding,
      });
      final message = List.filled(200, 'W').join();
      await tester.pumpWidget(
        MaterialApp(
          home: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 320,
              height: 1000,
              child: ThemeClientScene(
                snapshot: snapshot,
                showTopBar: false,
                userSample: message,
                aiSample: message,
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      final user = _sceneBubble(true);
      expect(tester.getSize(user).width, closeTo(210.8, 0.01));
      expect(
        (tester.widget<BubbleSurface>(user).child as Padding).padding,
        const EdgeInsets.all(16),
      );
      expect(_sceneBubble(false), findsNothing);
      expect(find.byIcon(Icons.person), findsNothing);
      expect(find.byIcon(Icons.assistant), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }

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
        'userChatStyle': ThemePreferenceDefaults.CHAT_STYLE_BUBBLE,
        'aiChatStyle': ThemePreferenceDefaults.CHAT_STYLE_BUBBLE,
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
      'userChatStyle': ThemePreferenceDefaults.CHAT_STYLE_BUBBLE,
      'aiChatStyle': ThemePreferenceDefaults.CHAT_STYLE_BUBBLE,
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

Finder _sceneMessageTarget(bool user) => find.byWidgetPredicate(
  (widget) =>
      widget is ThemeInspectTarget &&
      widget.targetId ==
          (user
              ? ThemeInspectableTargetId.userMessage
              : ThemeInspectableTargetId.aiMessage),
);

Finder _sceneBubble(bool user) => find.descendant(
  of: _sceneMessageTarget(user),
  matching: find.byType(BubbleSurface),
);

Future<void> _pumpLocalBubbleScene(
  WidgetTester tester, {
  required double sceneWidth,
  required bool wide,
  required bool showAvatar,
  double left = 8,
  double right = 8,
  String? sample,
}) async {
  // A large MediaQuery deliberately differs from the scene's local constraints.
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = const Size(2400, 1200);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
  final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
    ...ThemePreferenceDefaults.snapshot.toJson(),
    'userChatStyle': ThemePreferenceDefaults.CHAT_STYLE_BUBBLE,
    'aiChatStyle': ThemePreferenceDefaults.CHAT_STYLE_BUBBLE,
    'bubbleShowAvatar': showAvatar,
    'bubbleWideLayoutEnabled': wide,
    'bubbleUserContentPaddingLeft': left,
    'bubbleUserContentPaddingRight': right,
    'bubbleAiContentPaddingLeft': left,
    'bubbleAiContentPaddingRight': right,
  });
  final message = sample ?? List.filled(200, 'W').join();
  await tester.pumpWidget(
    MaterialApp(
      home: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: sceneWidth,
          height: 1000,
          child: ThemeClientScene(
            snapshot: snapshot,
            showTopBar: false,
            userSample: message,
            aiSample: message,
          ),
        ),
      ),
    ),
  );
  await tester.pump();
}

void _expectSceneBubbleGeometry(
  WidgetTester tester, {
  required bool user,
  required bool wide,
  required bool showAvatar,
  required double columnWidth,
  required double bubbleWidth,
  required double left,
  required double right,
}) {
  final target = _sceneMessageTarget(user);
  final bubble = _sceneBubble(user);
  expect(target, findsOneWidget);
  expect(bubble, findsOneWidget);
  final column = tester.getRect(
    find.ancestor(of: target, matching: find.byType(Column)).first,
  );
  final bubbleRect = tester.getRect(bubble);
  expect(column.width, closeTo(columnWidth, 0.01));
  expect(bubbleRect.width, closeTo(bubbleWidth, 0.01));
  final contentPadding = tester.widget<BubbleSurface>(bubble).child as Padding;
  expect(contentPadding.padding, EdgeInsets.fromLTRB(left, 12, right, 12));
  final contentRect = tester.getRect(find.byWidget(contentPadding.child!));
  expect(contentRect.left - bubbleRect.left, closeTo(left, 0.01));
  expect(bubbleRect.right - contentRect.right, closeTo(right, 0.01));

  final avatar = find.descendant(
    of: target,
    matching: find.byWidgetPredicate(
      (widget) =>
          widget is SizedBox && widget.width == 32 && widget.height == 32,
    ),
  );
  expect(avatar, showAvatar ? findsOneWidget : findsNothing);
  if (showAvatar) {
    final avatarRect = tester.getRect(avatar);
    expect(avatarRect.size, const Size(32, 32));
    if (wide) {
      expect(bubbleRect.top - avatarRect.bottom, closeTo(6, 0.01));
    } else {
      expect(bubbleRect.top, closeTo(avatarRect.top, 0.01));
      final gap = user
          ? avatarRect.left - bubbleRect.right
          : bubbleRect.left - avatarRect.right;
      expect(gap, closeTo(8, 0.01));
    }
  }
  final edgeGap = user
      ? column.right - bubbleRect.right
      : bubbleRect.left - column.left;
  expect(edgeGap, closeTo(showAvatar && !wide ? 40 : 0, 0.01));
}
