import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart';
import 'package:operit2/ui/features/chat/components/style/input/classic/ClassicChatInputSection.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  setUp(ClientLogger.initialize);
  final platforms = TargetPlatformVariant(TargetPlatform.values.toSet());
  for (final classic in <bool>[false, true]) {
    final style = classic ? 'Classic' : 'Agent';
    for (final key in <LogicalKeyboardKey>[
      LogicalKeyboardKey.enter,
      LogicalKeyboardKey.numpadEnter,
    ]) {
      testWidgets(
        '$style ${key.keyLabel} confirms IME text before sending',
        (tester) async {
          final controller = TextEditingController();
          var sends = 0;
          await _pumpComposer(tester, classic, controller, () => sends++);
          const composing = TextEditingValue(
            text: 'shell',
            selection: TextSelection.collapsed(offset: 5),
            composing: TextRange(start: 0, end: 5),
          );
          tester.testTextInput.updateEditingValue(composing);
          await tester.pump();
          // A common raw event format avoids Flutter's missing Windows
          // numpad Enter mapping; the target platform still varies above.
          await tester.sendKeyEvent(key, platform: 'macos');
          expect(sends, 0, reason: 'Enter belongs to the input method');
          expect(controller.value, composing);
          await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
          await tester.sendKeyEvent(key, platform: 'macos');
          await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
          expect(controller.value, composing);
          expect(sends, 0);

          // The IME commits the original Latin text on this confirmation.
          tester.testTextInput.updateEditingValue(
            composing.copyWith(composing: TextRange.empty),
          );
          await tester.pump();
          await tester.testTextInput.receiveAction(TextInputAction.unspecified);
          expect(sends, 0, reason: 'An IME action is not a new Enter press');

          await tester.sendKeyEvent(key, platform: 'macos');
          expect(sends, 1);
          await tester.testTextInput.receiveAction(TextInputAction.done);
          expect(sends, 1, reason: 'A platform callback must not send twice');
          await tester.pumpWidget(const SizedBox.shrink());
          await tester.pump(const Duration(milliseconds: 600));
        },
        variant: platforms,
      );
    }
    testWidgets(
      '$style software keyboard edits and the send button sends',
      (tester) async {
        final controller = TextEditingController();
        var sends = 0;
        await _pumpComposer(tester, classic, controller, () => sends++);
        await tester.enterText(
          find.byKey(const ValueKey('chat.input')),
          'shell',
        );
        await tester.pump();
        await tester.testTextInput.receiveAction(TextInputAction.newline);
        expect(sends, 0);
        tester.testTextInput.updateEditingValue(
          const TextEditingValue(
            text: 'shell\n',
            selection: TextSelection.collapsed(offset: 6),
          ),
        );
        await tester.pump();
        expect(controller.text, 'shell\n');
        expect(sends, 0);
        await tester.tap(find.byKey(const ValueKey('chat.send')));
        expect(sends, 1);
        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pump(const Duration(milliseconds: 600));
      },
      variant: platforms,
    );
    testWidgets('$style space, Ctrl+Enter, and held Enter stay safe', (
      tester,
    ) async {
      final controller = TextEditingController();
      var sends = 0;
      await _pumpComposer(tester, classic, controller, () => sends++);
      await tester.enterText(find.byKey(const ValueKey('chat.input')), 'shell');
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.space);
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'shell ',
          selection: TextSelection.collapsed(offset: 6),
        ),
      );
      await tester.pump();
      expect(sends, 0);
      expect(controller.text, 'shell ');

      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      expect(controller.text, 'shell \n');
      expect(sends, 0);

      await tester.sendKeyDownEvent(LogicalKeyboardKey.enter);
      expect(sends, 1);
      await tester.sendKeyRepeatEvent(LogicalKeyboardKey.enter);
      expect(sends, 1, reason: 'Holding Enter must not send again');
      await tester.sendKeyUpEvent(LogicalKeyboardKey.enter);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump(const Duration(milliseconds: 600));
    }, variant: platforms);
  }
}

Future<void> _pumpComposer(
  WidgetTester tester,
  bool classic,
  TextEditingController controller,
  VoidCallback onSend,
) async {
  // A tablet-sized surface must use the same IME protection on mobile OSes.
  tester.view.physicalSize = const Size(1280, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final focus = FocusNode();
  final viewModel = ChatViewModel(bridge: _PendingModelBridge());
  addTearDown(controller.dispose);
  addTearDown(focus.dispose);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Scaffold(
        body: classic
            ? ClassicChatInputSection(
                controller: controller,
                focusNode: focus,
                isLoading: false,
                inputState: InputProcessingState.idle(),
                viewModel: viewModel,
                currentChatId: null,
                onSendMessage: onSend,
                onQueueMessage: () {},
                onCancelMessage: () {},
                isSpeechRecording: false,
                isSpeechTranscribing: false,
                onSpeechInput: () {},
              )
            : AgentChatInputSection(
                controller: controller,
                focusNode: focus,
                isLoading: false,
                inputState: InputProcessingState.idle(),
                viewModel: viewModel,
                currentChatId: null,
                onSendMessage: onSend,
                onQueueMessage: () {},
                onCancelMessage: () {},
                isSpeechRecording: false,
                isSpeechTranscribing: false,
                onSpeechInput: () {},
              ),
      ),
    ),
  );
  await tester.tap(find.byKey(const ValueKey('chat.input')));
  await tester.pumpAndSettle();
}

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
      const Stream<CoreEvent>.empty();
}
