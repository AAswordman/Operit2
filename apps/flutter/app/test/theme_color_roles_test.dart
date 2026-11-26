import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/ui/features/chat/components/style/ThemedChatMessage.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart';
import 'package:operit2/ui/features/chat/components/style/input/classic/ClassicChatInputSection.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'support/chat_voice_input_session.dart';

/// Preserves existing component roles while custom palette values stay distinct.
void main() {
  setUp(ClientLogger.initialize);
  for (final brightness in Brightness.values) {
    final snapshot = ThemePreferenceSnapshot.fromJson(
      ThemePreferenceDefaults.snapshot.toJson()
        ..['themeMode'] = brightness == Brightness.dark ? 'dark' : 'light'
        ..['useCustomColors'] = true
        ..['customPrimaryColor'] = 0xFF123456
        ..['customSecondaryColor'] = 0xFFFFB020,
    );
    final scheme = operitColorScheme(brightness, snapshot);
    final theme = buildOperitThemeData(
      scheme,
      snapshot,
      fontFamily: 'Aptos',
      fontFamilyFallback: const [],
    );

    for (final classic in [false, true]) {
      testWidgets(
        'composer roles stay distinct, classic=$classic in $brightness',
        (tester) async {
          final controller = TextEditingController(text: 'draft');
          final focus = FocusNode();
          final viewModel = ChatViewModel();
          addTearDown(controller.dispose);
          addTearDown(focus.dispose);
          for (final (loading, text, background, foreground) in [
            (false, 'draft', scheme.primary, scheme.onPrimary),
            (true, 'draft', scheme.tertiary, scheme.onTertiary),
            (true, '', scheme.error, scheme.onError),
          ]) {
            controller.text = text;
            final input = classic
                ? ClassicChatInputSection(
                    controller: controller,
                    focusNode: focus,
                    isLoading: loading,
                    inputState: core.InputProcessingState.idle(),
                    viewModel: viewModel,
                    currentChatId: null,
                    onSendMessage: _noOp,
                    onQueueMessage: _noOp,
                    onCancelMessage: _noOp,
                    voiceInput: createIdleVoiceInputSession(),
                  )
                : AgentChatInputSection(
                    controller: controller,
                    focusNode: focus,
                    isLoading: loading,
                    inputState: core.InputProcessingState.idle(),
                    viewModel: viewModel,
                    currentChatId: null,
                    onSendMessage: _noOp,
                    onQueueMessage: _noOp,
                    onCancelMessage: _noOp,
                    voiceInput: createIdleVoiceInputSession(),
                  );
            await tester.pumpWidget(
              OperitTheme(
                initialThemePreferenceSnapshot: snapshot,
                initialThemeIsReady: false,
                unconfiguredChildEnabled: true,
                hostInteractionHostsEnabled: false,
                child: Scaffold(body: input),
              ),
            );
            await tester.pump(const Duration(milliseconds: 300));
            final action = tester.widget<ThemeComposerActionButton>(
              find.byType(ThemeComposerActionButton),
            );
            expect(action.background, background);
            expect(action.foreground, foreground);
            expect(tester.takeException(), isNull);
          }
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }

    testWidgets('selection keeps original Material roles in $brightness', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          theme: theme,
          home: Scaffold(
            body: Column(
              children: [
                ChoiceChip(
                  label: const Text('Choice'),
                  selected: true,
                  onSelected: (_) {},
                ),
                FilterChip(
                  label: const Text('Filter'),
                  selected: true,
                  onSelected: (_) {},
                ),
                SegmentedButton<int>(
                  segments: const [
                    ButtonSegment(value: 1, label: Text('Segment')),
                  ],
                  selected: const {1},
                  onSelectionChanged: (_) {},
                ),
                FilledButton(onPressed: () {}, child: const Text('Send')),
                FilledButton.tonal(
                  onPressed: () {},
                  child: const Text('Auxiliary'),
                ),
              ],
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      for (final type in [ChoiceChip, FilterChip]) {
        final rawFinder = find.descendant(
          of: find.byType(type),
          matching: find.byType(RawChip),
        );
        final chip = tester.widget<RawChip>(rawFinder);
        final chipTheme = ChipTheme.of(tester.element(rawFinder));
        expect(chip.selected, isTrue);
        expect(chip.selectedColor, isNull);
        expect(chipTheme.selectedColor, isNull);
        expect(
          chip.defaultProperties!.color!.resolve({WidgetState.selected}),
          scheme.secondaryContainer,
        );
      }
      for (final type in [SegmentedButton<int>]) {
        final materials = tester.widgetList<Material>(
          find.descendant(
            of: find.byType(type),
            matching: find.byType(Material),
          ),
        );
        expect(
          materials.any(
            (material) => material.color == scheme.secondaryContainer,
          ),
          isTrue,
        );
      }
      for (final label in ['Choice', 'Filter', 'Segment']) {
        expect(
          DefaultTextStyle.of(tester.element(find.text(label))).style.color,
          scheme.onSecondaryContainer,
        );
      }
      for (final (label, background, foreground) in [
        ('Send', scheme.primary, scheme.onPrimary),
        ('Auxiliary', scheme.secondaryContainer, scheme.onSecondaryContainer),
      ]) {
        final button = find.ancestor(
          of: find.text(label),
          matching: find.byType(FilledButton),
        );
        final material = tester.widget<Material>(
          find.descendant(of: button, matching: find.byType(Material)),
        );
        expect(material.color, background);
        expect(
          DefaultTextStyle.of(tester.element(find.text(label))).style.color,
          foreground,
        );
      }
      final drawer = ThemeDrawerPalette.fromScheme(
        scheme,
        transparentSurface: false,
      );
      expect(
        drawer.selectedContainerColor,
        scheme.secondaryContainer.withValues(alpha: 0.78),
      );
      expect(drawer.selectedContentColor, scheme.onSecondaryContainer);
      expect(theme.navigationBarTheme, const NavigationBarThemeData());
      expect(theme.navigationRailTheme, const NavigationRailThemeData());
      expect(theme.navigationDrawerTheme, const NavigationDrawerThemeData());
      expect(tester.takeException(), isNull);
    });

    test(
      'chat bubbles keep original primary and neutral roles in $brightness',
      () {
        final colors = resolveChatMessageThemeColors(snapshot, scheme);
        expect(colors.userMessageColor, scheme.primaryContainer);
        expect(colors.userTextColor, scheme.onPrimaryContainer);
        expect(colors.aiMessageColor, scheme.surfaceContainerHighest);
        expect(colors.aiTextColor, scheme.onSurface);
        expect(colors.systemMessageColor, scheme.surfaceContainerHighest);
        final custom = ThemePreferenceSnapshot.fromJson(
          snapshot.toJson()
            ..['bubbleAiBubbleColor'] = 0xFF246813
            ..['bubbleAiTextColor'] = 0xFFABCDEF,
        );
        final overridden = resolveChatMessageThemeColors(custom, scheme);
        expect(overridden.aiMessageColor, const Color(0xFF246813));
        expect(overridden.aiTextColor, const Color(0xFFABCDEF));
      },
    );
  }
}

/// Keeps composer color checks from performing chat or speech actions.
void _noOp() {}
