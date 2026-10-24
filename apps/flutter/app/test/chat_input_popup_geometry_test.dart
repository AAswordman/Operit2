import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/components/RetainedPage.dart';
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart';
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentModelSelectorPopup.dart';
import 'package:operit2/ui/features/chat/components/style/input/classic/ClassicChatInputSection.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatComposerSlotHost.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatPendingMessageQueuePanel.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies real composer popups against root and constrained page overlays.
void main() {
  setUp(ClientLogger.initialize);
  for (final agent in [false, true]) {
    for (final floating in [false, true]) {
      testWidgets(
        '${agent ? 'agent' : 'classic'} queue uses input edges above the bubble (floating: $floating)',
        (tester) async {
          final controller = TextEditingController();
          final focus = FocusNode();
          addTearDown(controller.dispose);
          addTearDown(focus.dispose);
          await tester.pumpWidget(
            OperitTheme(
              initialThemePreferenceSnapshot: ThemePreferenceSnapshot.fromJson({
                ...UserPreferencesManager.defaultThemePreferenceSnapshot
                    .toJson(),
                'chatInputFloating': floating,
                'transparentSurfaceEnabled': true,
              }),
              initialThemeIsReady: false,
              unconfiguredChildEnabled: true,
              hostInteractionHostsEnabled: false,
              child: Scaffold(
                body: _composer(
                  agent: agent,
                  controller: controller,
                  focus: focus,
                  viewModel: ChatViewModel(bridge: _PendingModelBridge()),
                  mention: null,
                  queue: const [
                    PendingQueueMessageItem(id: 1, text: 'Queue alignment'),
                  ],
                ),
              ),
            ),
          );
          final queue = find.byType(ChatPendingMessageQueuePanel);
          final bubble = find.byType(ThemeComposerCard);
          final surface = find.byKey(
            const ValueKey('chat.pending-queue.surface'),
          );
          final input = find.byKey(const ValueKey('chat.input.surface'));
          expect(find.descendant(of: bubble, matching: queue), findsNothing);
          expect(
            tester.getRect(queue).bottom,
            lessThanOrEqualTo(tester.getRect(bubble).top),
          );
          expect(
            tester.getRect(surface).left,
            closeTo(tester.getRect(input).left, 0.1),
          );
          expect(
            tester.getRect(surface).right,
            closeTo(tester.getRect(input).right, 0.1),
          );
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
      testWidgets(
        '${agent ? 'agent' : 'classic'} contributions sit above the input bubble (floating: $floating)',
        (tester) async {
          final controller = TextEditingController();
          final focus = FocusNode();
          addTearDown(controller.dispose);
          addTearDown(focus.dispose);
          await tester.pumpWidget(
            OperitTheme(
              initialThemePreferenceSnapshot: ThemePreferenceSnapshot.fromJson({
                ...UserPreferencesManager.defaultThemePreferenceSnapshot
                    .toJson(),
                'chatInputFloating': floating,
              }),
              initialThemeIsReady: false,
              unconfiguredChildEnabled: true,
              hostInteractionHostsEnabled: false,
              child: Scaffold(
                body: _composer(
                  agent: agent,
                  controller: controller,
                  focus: focus,
                  viewModel: ChatViewModel(bridge: _PendingModelBridge()),
                  mention: null,
                  chatId: 'parent',
                ),
              ),
            ),
          );
          await tester.pump();
          final slot = find.byType(ChatComposerSlotHost);
          final bubble = find.byType(ThemeComposerCard);
          expect(slot, findsOneWidget);
          expect(find.descendant(of: bubble, matching: slot), findsNothing);
          expect(
            tester.getRect(slot).bottom,
            lessThanOrEqualTo(tester.getRect(bubble).top),
          );
          final slotContent = find.descendant(
            of: slot,
            matching: find.byType(FutureBuilder<List<ToolPkgChatComposerSlot>>),
          );
          final inputSurface = find.byKey(const ValueKey('chat.input.surface'));
          expect(
            tester.getRect(slotContent).left,
            closeTo(tester.getRect(inputSurface).left, 0.1),
          );
          expect(
            tester.getRect(slotContent).right,
            closeTo(tester.getRect(inputSurface).right, 0.1),
          );
          expect(tester.getSize(slot).height, 0);
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }
  }
  for (final localOverlay in [false, true]) {
    for (final (agent, modelPopup) in [
      (true, true),
      (true, false),
      (false, false),
    ]) {
      testWidgets(
        '${agent ? 'agent' : 'classic'} ${modelPopup ? 'model' : 'mention'} '
        'popup follows its anchor (local overlay: $localOverlay)',
        (tester) async {
          tester.view.devicePixelRatio = 1;
          tester.view.physicalSize = const Size(400, 800);
          addTearDown(tester.view.resetDevicePixelRatio);
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetViewInsets);
          final controller = TextEditingController();
          final focus = FocusNode();
          addTearDown(controller.dispose);
          addTearDown(focus.dispose);
          final composer = _composer(
            agent: agent,
            controller: controller,
            focus: focus,
            viewModel: ChatViewModel(bridge: _PendingModelBridge()),
            mention: modelPopup
                ? null
                : const SizedBox(
                    key: ValueKey('mention-popup'),
                    height: 64,
                    child: Text('Mention suggestions'),
                  ),
          );
          final page = Align(
            alignment: Alignment.bottomCenter,
            child: composer,
          );
          await tester.pumpWidget(
            OperitTheme(
              initialThemePreferenceSnapshot:
                  UserPreferencesManager.defaultThemePreferenceSnapshot,
              initialThemeIsReady: false,
              unconfiguredChildEnabled: true,
              hostInteractionHostsEnabled: false,
              child: Scaffold(
                appBar: AppBar(title: const Text('Chat')),
                body: Padding(
                  padding: const EdgeInsets.fromLTRB(40, 0, 40, 24),
                  child: localOverlay
                      ? RetainedPage(active: true, child: page)
                      : page,
                ),
              ),
            ),
          );
          await tester.pump();
          final anchor = find.byType(
            modelPopup ? ThemeComposerModelSelector : ThemeComposerCard,
          );
          final popup = modelPopup
              ? find.byType(AgentModelSelectorPopup)
              : find.byKey(const ValueKey('mention-popup'));
          if (modelPopup) {
            await tester.tap(anchor);
            await tester.pump();
          }
          for (final keyboard in [0.0, 260.0, 0.0]) {
            tester.view.viewInsets = FakeViewPadding(bottom: keyboard);
            await tester.pump();
            await tester.pump(const Duration(milliseconds: 300));
            final anchorRect = tester.getRect(anchor);
            final popupRect = tester.getRect(popup);
            expect(anchorRect.top - popupRect.bottom, closeTo(8, 0.1));
            expect(
              popupRect.left,
              greaterThanOrEqualTo(localOverlay ? 52 : 12),
            );
            expect(
              popupRect.right,
              lessThanOrEqualTo(localOverlay ? 348 : 388),
            );
            expect(
              popupRect.width,
              closeTo(localOverlay ? 296 : (modelPopup ? 300 : 376), 0.1),
            );
            expect(tester.takeException(), isNull);
          }
          await tester.tapAt(const Offset(60, 120));
          await tester.pump();
          expect(popup, findsNothing);
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }
  }
}

/// Builds either composer with the same dependencies and optional mention panel.
Widget _composer({
  required bool agent,
  required TextEditingController controller,
  required FocusNode focus,
  required ChatViewModel viewModel,
  required Widget? mention,
  String? chatId,
  List<PendingQueueMessageItem> queue = const [],
}) => agent
    ? AgentChatInputSection(
        controller: controller,
        focusNode: focus,
        isLoading: false,
        inputState: InputProcessingState.idle(),
        mentionSuggestionPanel: mention,
        viewModel: viewModel,
        currentChatId: chatId,
        pendingQueueMessages: queue,
        onSendMessage: () {},
        onQueueMessage: () {},
        onCancelMessage: () {},
        isSpeechRecording: false,
        isSpeechTranscribing: false,
        onSpeechInput: () {},
      )
    : ClassicChatInputSection(
        controller: controller,
        focusNode: focus,
        isLoading: false,
        inputState: InputProcessingState.idle(),
        mentionSuggestionPanel: mention,
        viewModel: viewModel,
        currentChatId: chatId,
        pendingQueueMessages: queue,
        onSendMessage: () {},
        onQueueMessage: () {},
        onCancelMessage: () {},
        isSpeechRecording: false,
        isSpeechTranscribing: false,
        onSpeechInput: () {},
      );

class _PendingModelBridge extends OperitRuntimeBridge {
  final _pending = Completer<Uint8List>();

  /// Keeps model loading pending so geometry tests do not depend on backend data.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) => _pending.future;

  /// Rejects pushes unrelated to popup geometry.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshots unrelated to popup geometry.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Supplies no runtime events while the popup is being positioned.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream<CoreEvent>.empty();
}
