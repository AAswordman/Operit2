import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/MessageEditorDialog.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies visual message editing keeps protocol separators out of text fields.
void main() {
  testWidgets('hides boundary newlines and preserves them when saving', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    const original =
        'First paragraph.\n\n'
        '<tool name="read"><param name="path">a.txt</param></tool>\n\n'
        '<tool_result name="read" status="success">'
        '<content>done</content></tool_result>\n\n'
        'Second paragraph.\n\n'
        '<status type="complete">finished</status>\n';
    String? saved;
    tester.binding.platformDispatcher.localesTestValue = const <Locale>[
      Locale('ja'),
    ];
    addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);

    await tester.pumpWidget(
      OperitTheme(
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Scaffold(
          body: MessageEditorDialog(
            initialText: original,
            showResendButton: false,
            onSave: (content) async {
              saved = content;
            },
            onResend: (_) async {},
          ),
        ),
      ),
    );

    final editableTexts = tester
        .widgetList<TextField>(find.byType(TextField))
        .map((field) => field.controller!.text)
        .toList(growable: false);
    expect(editableTexts, <String>['First paragraph.', 'Second paragraph.']);

    expect(find.text('記憶を編集'), findsOneWidget);
    expect(find.text('修改记忆'), findsNothing);
    await tester.tap(find.text('記憶を更新'));
    await tester.pumpAndSettle();
    expect(saved, original);
  });
}
