import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/chat/components/style/MessageHeaderMetadata.dart';
import 'package:operit2/ui/features/chat/components/style/ThemedChatMessage.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleStyleChatMessage.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleSurface.dart';
import 'package:operit2/ui/features/chat/components/style/cursor/CursorStyleChatMessage.dart';
import 'package:operit2/ui/features/chat/components/style/cursor/AiMessageComposable.dart';
import 'package:operit2/ui/common/ChatAvatarImage.dart';
import 'package:operit2/ui/features/chat/components/style/cursor/UserMessageComposable.dart';
import 'package:operit2/ui/features/chat/components/part/StructuredMessagePartRenderer.dart';
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart';
import 'package:operit2/ui/features/chat/components/style/input/classic/ClassicChatInputSection.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/features/settings/appearance/ChatAppearancePreview.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  setUp(ClientLogger.initialize);
  for (final style in <String>['bubble', 'cursor']) {
    for (final width in <double>[280, 380]) {
      for (final wide in <bool>[false, true]) {
        testWidgets('uses real $style widgets at $width, wide=$wide', (
          tester,
        ) async {
          final agent = width == 380;
          final snapshot = _snapshot(<String, Object?>{
            'userChatStyle': style,
            'aiChatStyle': style,
            'inputStyle': agent ? 'agent' : 'classic',
            'chatInputFloating': wide,
            'bubbleWideLayoutEnabled': wide,
            'bubbleShowAvatar': !wide,
            'bubbleUserRoundedCornersEnabled': false,
            'bubbleAiRoundedCornersEnabled': true,
            'bubbleUserContentPaddingLeft': 0.0,
            'bubbleUserContentPaddingRight': 32.0,
            'bubbleAiContentPaddingLeft': 30.0,
            'bubbleAiContentPaddingRight': 2.0,
            'bubbleUserBubbleColor': Colors.red.toARGB32(),
            'bubbleAiBubbleColor': Colors.green.toARGB32(),
            'bubbleUserTextColor': Colors.white.toARGB32(),
            'bubbleAiTextColor': Colors.black.toARGB32(),
            'showModelProvider': true,
            'showModelName': true,
            'showRoleName': true,
            'showMessageTokenStats': true,
            'showMessageTimingStats': true,
            'showMessageTimestamp': true,
          });
          await _pumpPreview(tester, snapshot, width: width, showInput: true);
          await tester.pumpAndSettle();
          expect(
            find.byType(BubbleStyleChatMessage),
            style == 'bubble' ? findsNWidgets(2) : findsNothing,
          );
          expect(
            find.byType(CursorStyleChatMessage),
            style == 'cursor' ? findsNWidgets(2) : findsNothing,
          );
          expect(
            find.byType(AgentChatInputSection),
            agent ? findsOneWidget : findsNothing,
          );
          expect(
            find.byType(ClassicChatInputSection),
            agent ? findsNothing : findsOneWidget,
          );
          if (style == 'bubble') {
            final message = tester.widget<BubbleStyleChatMessage>(
              find.byType(BubbleStyleChatMessage).first,
            );
            expect(message.userMessageColor.toARGB32(), Colors.red.toARGB32());
            expect(message.aiMessageColor.toARGB32(), Colors.green.toARGB32());
            expect(message.userTextColor.toARGB32(), Colors.white.toARGB32());
            expect(message.aiTextColor.toARGB32(), Colors.black.toARGB32());
            expect(message.bubbleUserContentPaddingLeft, 0);
            expect(message.bubbleUserContentPaddingRight, 32);
            expect(message.bubbleAiContentPaddingLeft, 30);
            expect(message.bubbleAiContentPaddingRight, 2);
            final surfaces = tester.widgetList<BubbleSurface>(
              find.byType(BubbleSurface),
            );
            expect(surfaces.first.borderRadius, BorderRadius.zero);
            expect(surfaces.last.borderRadius, BorderRadius.circular(16));
          }
          final input = tester.widget<TextField>(find.byType(TextField).first);
          await tester.tap(find.byType(TextField).first, warnIfMissed: false);
          await tester.pump();
          expect(input.focusNode!.hasFocus, isFalse);
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        });
      }
    }
  }

  for (final width in <double>[280, 760]) {
    for (final showAvatar in <bool>[false, true]) {
      for (final fontScale in <double>[1, 1.5]) {
        testWidgets(
          'cursor stays avatar-free at $width, bubble avatars=$showAvatar, font=$fontScale',
          (tester) async {
            await _pumpPreview(
              tester,
              _snapshot(<String, Object?>{
                'userChatStyle': 'cursor',
                'aiChatStyle': 'cursor',
                'bubbleShowAvatar': showAvatar,
                'bubbleAiContentPaddingLeft': 12.0,
                'bubbleAiContentPaddingRight': 12.0,
                'showRoleName': true,
                'fontScale': fontScale,
              }),
              width: width,
            );
            await tester.pumpAndSettle();
            final user = find.byType(UserMessageComposable);
            final ai = find.byType(AiMessageComposable);
            final userBody = find.descendant(
              of: user,
              matching: find.byType(BubbleSurface),
            );
            final aiBody = find.descendant(
              of: ai,
              matching: find.byType(StreamingStructuredMessageRenderer),
            );
            final rowLeft = tester.getTopLeft(user).dx;
            final contentLeft = rowLeft + 16;
            expect(tester.getTopLeft(userBody).dx, closeTo(contentLeft, 0.01));
            expect(tester.getTopLeft(aiBody).dx, closeTo(contentLeft, 0.01));
            final title = find.descendant(of: ai, matching: find.text('Operit'));
            expect(
              tester.getTopLeft(title).dx,
              closeTo(contentLeft, 0.01),
            );
            expect(find.byType(ChatAvatarImage), findsNothing);
            expect(find.byIcon(Icons.person), findsNothing);
            expect(tester.takeException(), isNull);
            await tester.pumpWidget(const SizedBox.shrink());
          },
        );
      }
    }
  }

  testWidgets('bubble avatar changes leave cursor layout and renderer intact', (
    tester,
  ) async {
    await _pumpPreview(
      tester,
      _snapshot(<String, Object?>{
        'userChatStyle': 'cursor',
        'aiChatStyle': 'cursor',
        'bubbleShowAvatar': true,
        'avatarShape': UserPreferencesManager.AVATAR_SHAPE_SQUARE,
        'avatarCornerRadius': 6.0,
      }),
    );
    await tester.pumpAndSettle();
    final renderer = find.byType(StreamingStructuredMessageRenderer);
    final state = tester.state(renderer);
    final contentLeft = tester.getTopLeft(renderer).dx;
    final controller = OperitTheme.of(
      tester.element(find.byType(ChatAppearancePreview)),
    );
    for (final showAvatar in <bool>[false, true]) {
      controller.setInitialThemePreferenceSnapshot(
        ThemePreferenceSnapshot.fromJson(<String, Object?>{
          ...controller.themePreferenceSnapshot.toJson(),
          'bubbleShowAvatar': showAvatar,
        }),
      );
      controller.previewThemeSettings();
      await tester.pumpAndSettle();
      expect(find.byType(ChatAvatarImage), findsNothing);
      expect(find.byIcon(Icons.person), findsNothing);
      expect(tester.getTopLeft(renderer).dx, closeTo(contentLeft, 0.01));
      expect(tester.state(renderer), same(state));
      expect(tester.takeException(), isNull);
    }
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('applies font scaling once and updates the live sample', (
    tester,
  ) async {
    await _pumpPreview(tester, _snapshot(<String, Object?>{'fontScale': 1.75}));
    await tester.pumpAndSettle();
    final context = tester.element(find.byType(CursorStyleChatMessage).first);
    final originalFontSize = Theme.of(context).textTheme.bodyMedium!.fontSize!;
    expect(MediaQuery.textScalerOf(context).scale(10), 10);
    OperitTheme.of(context).previewThemeSettings(fontScale: 0.75);
    await tester.pumpAndSettle();
    expect(
      Theme.of(context).textTheme.bodyMedium!.fontSize!,
      closeTo(originalFontSize * 0.75 / 1.75, 0.001),
    );
    expect(MediaQuery.textScalerOf(context).scale(10), 10);
    expect(find.byType(TextField), findsNothing);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('uses real metadata formatting instead of invented labels', (
    tester,
  ) async {
    final snapshot = _snapshot(<String, Object?>{
      'userChatStyle': 'cursor',
      'aiChatStyle': 'cursor',
      'showModelProvider': true,
      'showModelName': true,
      'showRoleName': true,
      'showMessageTokenStats': true,
      'showMessageTimingStats': true,
      'showMessageTimestamp': true,
    });
    await _pumpPreview(tester, snapshot);
    await tester.pumpAndSettle();
    final ai = tester.widget<CursorStyleChatMessage>(
      find.byType(CursorStyleChatMessage).last,
    );
    expect(ai.message.roleName, 'Operit');
    expect(ai.message.outputDurationMs, 1200);
    expect(find.text('GPT-5 · OpenAI'), findsOneWidget);
    expect(
      find.textContaining(formatCompactTimingStats(ai.message)),
      findsWidgets,
    );
    expect(find.textContaining('2610 tokens'), findsNothing);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('live style changes select the production message renderer', (
    tester,
  ) async {
    await _pumpPreview(
      tester,
      _snapshot(<String, Object?>{'userChatStyle': 'cursor', 'aiChatStyle': 'cursor'}),
    );
    await tester.pumpAndSettle();
    final context = tester.element(find.byType(ChatAppearancePreview));
    final controller = OperitTheme.of(context);
    controller.setInitialThemePreferenceSnapshot(
      _snapshot(<String, Object?>{
        'userChatStyle': 'bubble',
        'aiChatStyle': 'bubble',
        'bubbleWideLayoutEnabled': true,
        'bubbleShowAvatar': false,
      }),
    );
    controller.previewThemeSettings();
    await tester.pumpAndSettle();
    expect(find.byType(CursorStyleChatMessage), findsNothing);
    expect(find.byType(BubbleStyleChatMessage), findsNWidgets(2));
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets(
    'the shared factory preserves all bubble image slicing settings',
    (tester) async {
      await _pumpPreview(
        tester,
        _snapshot(<String, Object?>{'userChatStyle': 'cursor', 'aiChatStyle': 'cursor'}),
      );
      await tester.pumpAndSettle();
      final message = tester
          .widget<CursorStyleChatMessage>(
            find.byType(CursorStyleChatMessage).first,
          )
          .message;
      final snapshot = _snapshot(<String, Object?>{
        'userChatStyle': 'bubble',
        'aiChatStyle': 'bubble',
        'bubbleUserUseImage': true,
        'bubbleAiUseImage': true,
        'bubbleUserImageUri': 'user.png',
        'bubbleAiImageUri': 'ai.png',
        'bubbleUserImageCropLeft': 0.1,
        'bubbleUserImageCropTop': 0.2,
        'bubbleUserImageCropRight': 0.15,
        'bubbleUserImageCropBottom': 0.25,
        'bubbleUserImageRepeatStart': 0.3,
        'bubbleUserImageRepeatEnd': 0.6,
        'bubbleUserImageRepeatYStart': 0.35,
        'bubbleUserImageRepeatYEnd': 0.65,
        'bubbleUserImageScale': 1.5,
        'bubbleUserImageRenderMode':
            UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH,
        'bubbleAiImageScale': 0.75,
      });
      // Only build the configuration here: do not try to load imaginary assets.
      final bubble =
          buildThemedChatMessage(
                snapshot: snapshot,
                colorScheme: Theme.of(
                  tester.element(find.byType(ChatAppearancePreview)),
                ).colorScheme,
                message: message,
              )
              as BubbleStyleChatMessage;
      final image = bubble.userBubbleImageStyle!;
      expect(image.imagePath, 'user.png');
      expect(image.cropLeftRatio, 0.1);
      expect(image.cropTopRatio, 0.2);
      expect(image.cropRightRatio, 0.15);
      expect(image.cropBottomRatio, 0.25);
      expect(image.repeatXStartRatio, 0.3);
      expect(image.repeatXEndRatio, 0.6);
      expect(image.repeatYStartRatio, 0.35);
      expect(image.repeatYEndRatio, 0.65);
      expect(image.imageScale, 1.5);
      expect(
        image.renderMode,
        UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH,
      );
      expect(bubble.aiBubbleImageStyle!.imagePath, 'ai.png');
      expect(bubble.aiBubbleImageStyle!.imageScale, 0.75);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets('shares the application background without a synthetic overlay', (
    tester,
  ) async {
    final snapshot = _snapshot(<String, Object?>{
      'transparentSurfaceEnabled': true,
      'userChatStyle': 'bubble',
      'aiChatStyle': 'bubble',
    });
    await _pumpPreview(tester, snapshot);
    await tester.pumpAndSettle();
    final background = tester.widget<OperitThemeBackground>(
      find.descendant(
        of: find.byType(ChatAppearancePreview),
        matching: find.byType(OperitThemeBackground),
      ),
    );
    expect(background.themePreferenceSnapshot, snapshot);
    expect(background.fit, StackFit.loose);
    expect(background.muteVideo, isTrue);
    expect(
      tester
          .widgetList<BubbleSurface>(find.byType(BubbleSurface))
          .every((surface) => surface.transparentSurface),
      isTrue,
    );
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

ThemePreferenceSnapshot _snapshot(Map<String, Object?> overrides) =>
    ThemePreferenceSnapshot.fromJson(<String, Object?>{
      ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      ...overrides,
    });

Future<void> _pumpPreview(
  WidgetTester tester,
  ThemePreferenceSnapshot snapshot, {
  double width = 380,
  bool showInput = false,
}) async {
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot: snapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Builder(
        builder: (context) {
          // Bootstrap deliberately uses default typography; reproduce the
          // runtime theme's font scaling while keeping the host offline.
          final scale = OperitTheme.of(
            context,
          ).themePreferenceSnapshot.fontScale;
          final theme = Theme.of(context);
          return Theme(
            data: theme.copyWith(
              textTheme: theme.textTheme.apply(fontSizeFactor: scale),
            ),
            child: Scaffold(
              body: SingleChildScrollView(
                child: Center(
                  child: SizedBox(
                    width: width,
                    child: ChatAppearancePreview(
                      showInputPreview: showInput,
                      viewModel: _PreviewViewModel(),
                    ),
                  ),
                ),
              ),
            ),
          );
        },
      ),
    ),
  );
}

/// Leaves the model chip loading without contacting a real runtime in tests.
class _PendingModelBridge extends OperitRuntimeBridge {
  final _binding = Completer<Uint8List>();

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    if (request.methodName == 'getModelBindingForFunction') {
      return _binding.future;
    }
    throw StateError('Unexpected call: ${request.methodName}');
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}

/// Models the Core markdown boundary without requiring a running host.
class _PreviewViewModel extends ChatViewModel {
  _PreviewViewModel() : super(bridge: _PendingModelBridge());

  @override
  Future<List<MarkdownStreamEvent>> splitMarkdownContent(String content) async {
    return <MarkdownStreamEvent>[
      _event('markdownBlockStart'),
      _event('markdownBlockChunk', value: content),
      _event('completed'),
    ];
  }
}

MarkdownStreamEvent _event(String type, {String? value}) => MarkdownStreamEvent(
  chatId: 'preview',
  eventType: type,
  value: value,
  id: null,
  blockId: type == 'completed' ? null : 1,
  inlineId: null,
  parentBlockId: null,
  nodeType: null,
  headerLevel: null,
  xml: null,
);
