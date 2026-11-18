import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

Future<void> mountSearch(
  WidgetTester tester, {
  required Future<Object?> Function(String, Object?) submit,
  required Future<Object?> Function(String, String) edit,
  bool readOnly = false,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: buildComposeDslLayoutForTest(
          node: {
            'type': 'TextField',
            'props': {
              'key': 'search',
              'value': '',
              'singleLine': true,
              'imeAction': 'search',
              'readOnly': readOnly,
              'onValueChange': {'__actionId': 'edit'},
              'onSubmitted': {'__actionId': 'submit'},
            },
          },
          onTextInput: edit,
          hostContext: ComposeDslWebViewHostContext(
            packageName: 'test.search',
            routeInstanceId: 'search',
            executionContextKey: 'search',
            dispatchAction: (id, [payload]) => submit(id, payload),
            runtimeOptionsProvider: () => {},
          ),
          splitMarkdownContent: (_) async => [],
        ),
      ),
    ),
  );
  await tester.pump();
}

void main() {
  testWidgets('keyboard search submits current text after pending edits once', (
    tester,
  ) async {
    final editDone = Completer<void>(), searchDone = Completer<void>();
    final events = <String>[];
    await mountSearch(
      tester,
      edit: (id, value) async {
        await editDone.future;
        events.add('edit:$value');
        return null;
      },
      submit: (id, value) async {
        events.add('$id:$value');
        await searchDone.future;
        return null;
      },
    );
    expect(
      tester.widget<TextField>(find.byType(TextField)).textInputAction,
      TextInputAction.search,
    );
    await tester.enterText(find.byType(TextField), '最后输入的记忆');
    await tester.testTextInput.receiveAction(TextInputAction.search);
    await tester.testTextInput.receiveAction(TextInputAction.search);
    expect(events, isEmpty);
    editDone.complete();
    await tester.pump();
    expect(events, ['edit:最后输入的记忆', 'submit:最后输入的记忆']);
    searchDone.complete();
    await tester.pump();
    expect(tester.takeException(), isNull);
  });

  testWidgets('read-only search cannot submit while work is running', (
    tester,
  ) async {
    final submitted = <Object?>[];
    await mountSearch(
      tester,
      readOnly: true,
      edit: (_, _) async => null,
      submit: (_, value) async {
        submitted.add(value);
        return null;
      },
    );
    tester.widget<TextField>(find.byType(TextField)).onSubmitted?.call('busy');
    await tester.pump();
    expect(submitted, isEmpty);
    expect(tester.takeException(), isNull);
  });
}
