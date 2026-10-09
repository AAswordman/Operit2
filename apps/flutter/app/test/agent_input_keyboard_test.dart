import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Checks the real Agent composer's paint work during keyboard inset changes.
void main() {
  setUp(ClientLogger.initialize);
  testWidgets('keyboard motion reuses the unchanged Agent surface paint', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(390, 844);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetViewInsets);
    final controller = TextEditingController();
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: RepaintBoundary(
          key: const ValueKey<String>('keyboard-scene'),
          child: Scaffold(
            body: Column(
              children: <Widget>[
                const Expanded(child: Center(child: Text('Operit'))),
                AgentChatInputSection(
                  controller: controller,
                  focusNode: focus,
                  isLoading: false,
                  inputState: InputProcessingState.idle(),
                  viewModel: ChatViewModel(bridge: _PendingModelBridge()),
                  currentChatId: 'empty-chat',

                  onSendMessage: () {},
                  onQueueMessage: () {},
                  onCancelMessage: () {},
                  isSpeechRecording: false,
                  isSpeechTranscribing: false,
                  onSpeechInput: () {},
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey<String>('chat.input')));
    await tester.pumpAndSettle();
    final surfaceFinder = find.descendant(
      of: find.byType(AgentChatInputSection),
      matching: find.byWidgetPredicate((widget) {
        if (widget is! DecoratedBox) {
          return false;
        }
        final decoration = widget.decoration;
        return decoration is ShapeDecoration && decoration.shadows?.length == 2;
      }),
    );
    expect(surfaceFinder, findsOneWidget);
    final surface = tester.renderObject<RenderDecoratedBox>(surfaceFinder);
    final originalRect = tester.getRect(surfaceFinder);
    final originalDecoration = surface.decoration;
    final field = find.byKey(const ValueKey<String>('chat.input'));
    final originalFieldSize = tester.getSize(field);
    var paints = 0;
    final oldCallback = debugOnProfilePaint;
    debugOnProfilePaint = (renderObject) {
      oldCallback?.call(renderObject);
      if (identical(renderObject, surface)) {
        paints++;
      }
    };
    try {
      for (final bottom in <double>[
        for (var step = 1; step <= 12; step++) step * 25,
        for (var step = 11; step >= 0; step--) step * 25,
      ]) {
        tester.view.viewInsets = FakeViewPadding(bottom: bottom);
        await tester.pump(const Duration(milliseconds: 16));
        expect(
          tester.getRect(surfaceFinder),
          originalRect.shift(Offset(0, -bottom)),
        );
        expect(tester.getSize(field), originalFieldSize);
        expect(surface.decoration, originalDecoration);
        expect(focus.hasFocus, isTrue);
      }
    } finally {
      debugOnProfilePaint = oldCallback;
    }
    expect(paints, 0);
    tester.view.viewInsets = const FakeViewPadding(bottom: 300);
    await tester.pump(const Duration(milliseconds: 16));
    final cachedPixels = await _scenePixels(tester);
    surface.markNeedsPaint();
    await tester.pump();
    final repaintedPixels = await _scenePixels(tester);
    expect(repaintedPixels, orderedEquals(cachedPixels));

    await tester.enterText(field, 'draft stays editable');
    await tester.pump();
    expect(controller.text, 'draft stays editable');
    expect(
      tester.getRect(surfaceFinder),
      originalRect.shift(const Offset(0, -300)),
    );
    await tester.tap(find.byIcon(Icons.fullscreen));
    await tester.pumpAndSettle();
    expect(tester.getSize(field).height, greaterThan(originalFieldSize.height));
    expect(focus.hasFocus, isTrue);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

/// Captures the complete scene to compare retained and freshly painted pixels.
Future<Uint8List> _scenePixels(WidgetTester tester) async {
  final boundary = tester.renderObject<RenderRepaintBoundary>(
    find.byKey(const ValueKey<String>('keyboard-scene')),
  );
  final pixels = await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final data = (await image.toByteData())!;
      return Uint8List.fromList(data.buffer.asUint8List());
    } finally {
      image.dispose();
    }
  });
  return pixels!;
}

class _PendingModelBridge extends OperitRuntimeBridge {
  final _binding = Completer<Uint8List>();

  /// Keeps model metadata loading while the test exercises the empty composer.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    if (request.methodName == 'getModelBindingForFunction') {
      return _binding.future;
    }
    throw StateError('Unexpected call: ${request.methodName}');
  }

  /// Rejects streams that are unrelated to rendering the input.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshots that are unrelated to rendering the input.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects watches until model metadata has completed.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream<CoreEvent>.empty();
}
