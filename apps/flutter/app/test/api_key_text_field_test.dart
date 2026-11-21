import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/settings/components/ApiKeyTextField.dart';

/// Hosts the key input and a second field for realistic focus transitions.
class _Harness extends StatelessWidget {
  /// Creates a key field with an optional provider identity and line count.
  const _Harness({required this.controller, this.fieldKey, this.maxLines = 1});

  final TextEditingController controller;
  final Key? fieldKey;
  final int maxLines;

  /// Builds two ordinary text inputs without any platform-specific behavior.
  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Column(
        children: [
          ApiKeyTextField(
            key: fieldKey,
            controller: controller,
            labelText: 'API Key',
            maxLines: maxLines,
          ),
          const TextField(key: ValueKey('other-field')),
        ],
      ),
    ),
  );
}

/// Finds the editable inside the key field rather than the focus target.
Finder _input() => find.descendant(
  of: find.byType(ApiKeyTextField),
  matching: find.byType(EditableText),
);

/// Reads the actual painted text instead of the controller's underlying value.
String _renderedText(WidgetTester tester) => tester
    .state<EditableTextState>(_input())
    .renderEditable
    .text!
    .toPlainText();

/// Checks that native input receives ordinary text flags, including on refocus.
void _expectOrdinaryKeyboard(WidgetTester tester) {
  final configuration = tester.testTextInput.setClientArgs!;
  expect(tester.testTextInput.isVisible, isTrue);
  expect(configuration['inputType'], TextInputType.text.toJson());
  expect(configuration['obscureText'], isFalse);
  expect(configuration['readOnly'], isFalse);
  expect(configuration['enableSuggestions'], isTrue);
  expect(configuration['autocorrect'], isFalse);
  expect(configuration['enableIMEPersonalizedLearning'], isFalse);
}

/// Covers visual masking, native input flags, paste, and controller lifecycle.
void main() {
  testWidgets('starts visually masked without password input mode', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'sk-test');
    addTearDown(controller.dispose);
    await tester.pumpWidget(_Harness(controller: controller));
    expect(_renderedText(tester), '*******');
    expect(tester.widget<EditableText>(_input()).obscureText, isFalse);
    expect(controller.text, 'sk-test');
  });

  testWidgets('focus reveals text and moving focus masks edited text', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'sk-test');
    addTearDown(controller.dispose);
    await tester.pumpWidget(_Harness(controller: controller));
    await tester.tap(_input());
    await tester.pump();
    expect(_renderedText(tester), 'sk-test');
    _expectOrdinaryKeyboard(tester);

    await tester.enterText(_input(), 'sk-replaced');
    await tester.tap(find.byKey(const ValueKey('other-field')));
    await tester.pump();
    expect(_renderedText(tester), '***********');
    expect(controller.text, 'sk-replaced');

    await tester.tap(_input());
    await tester.pump();
    expect(_renderedText(tester), 'sk-replaced');
    _expectOrdinaryKeyboard(tester);
  });

  testWidgets('clipboard paste replaces the selection in the original key', (
    tester,
  ) async {
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async =>
          call.method == 'Clipboard.getData' ? {'text': 'pasted'} : null,
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    final controller = TextEditingController(text: 'sk-before');
    addTearDown(controller.dispose);
    await tester.pumpWidget(_Harness(controller: controller));
    await tester.tap(_input());
    await tester.pump();
    controller.selection = const TextSelection(baseOffset: 3, extentOffset: 9);
    await tester.pump();
    await tester
        .state<EditableTextState>(_input())
        .pasteText(SelectionChangedCause.toolbar);
    await tester.pump();
    expect(controller.text, 'sk-pasted');
    expect(controller.selection, const TextSelection.collapsed(offset: 9));
    expect(_renderedText(tester), 'sk-pasted');
    _expectOrdinaryKeyboard(tester);
  });

  testWidgets('external updates and IME composing keep their original values', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'sk-test');
    addTearDown(controller.dispose);
    await tester.pumpWidget(_Harness(controller: controller));
    controller.text = 'sk-updated';
    await tester.pump();
    expect(_renderedText(tester), '**********');

    await tester.tap(_input());
    await tester.pump();
    const composing = TextEditingValue(
      text: 'sk-input',
      selection: TextSelection.collapsed(offset: 8),
      composing: TextRange(start: 3, end: 8),
    );
    tester.testTextInput.updateEditingValue(composing);
    await tester.pump();
    expect(controller.value, composing);
    expect(_renderedText(tester), 'sk-input');
  });

  testWidgets('provider changes remask and controller replacement detaches', (
    tester,
  ) async {
    final original = TextEditingController(text: 'sk-test');
    final replacement = TextEditingController(text: 'sk-next');
    addTearDown(original.dispose);
    addTearDown(replacement.dispose);
    await tester.pumpWidget(
      _Harness(controller: original, fieldKey: const ValueKey('DEEPSEEK')),
    );
    await tester.tap(_input());
    await tester.pump();
    expect(_renderedText(tester), 'sk-test');
    await tester.pumpWidget(
      _Harness(controller: original, fieldKey: const ValueKey('OPENAI')),
    );
    expect(_renderedText(tester), '*******');
    expect(original.text, 'sk-test');

    await tester.pumpWidget(
      _Harness(controller: replacement, fieldKey: const ValueKey('OPENAI')),
    );
    original.text = 'detached';
    replacement.text = 'sk-new';
    await tester.pump();
    expect(_renderedText(tester), '******');
    await tester.enterText(_input(), 'sk-edited');
    expect(replacement.text, 'sk-edited');
    expect(original.text, 'detached');
    await tester.pumpWidget(const SizedBox());
    replacement.text = 'still-owned-by-caller';
    expect(tester.takeException(), isNull);
  });

  testWidgets('batch keys retain line breaks when masked and edited', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'key-a\nkey-b');
    addTearDown(controller.dispose);
    await tester.pumpWidget(_Harness(controller: controller, maxLines: 6));
    expect(_renderedText(tester), '*****\n*****');
    await tester.enterText(_input(), 'new-a\nnew-b');
    await tester.pump();
    expect(_renderedText(tester), 'new-a\nnew-b');
    expect(
      tester.testTextInput.setClientArgs!['inputType'],
      TextInputType.multiline.toJson(),
    );
    expect(tester.testTextInput.setClientArgs!['obscureText'], isFalse);
    expect(tester.testTextInput.setClientArgs!['enableSuggestions'], isTrue);
    await tester.tap(find.byKey(const ValueKey('other-field')));
    await tester.pump();
    expect(_renderedText(tester), '*****\n*****');
    expect(controller.text, 'new-a\nnew-b');
  });
}
