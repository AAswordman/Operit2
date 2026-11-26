import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserDialogs.dart';

/// Verifies dialog inspection and responses across the shared host coordinator.
void main() {
  test('get returns null when no dialog is open', () {
    final dialogs = RuntimeBrowserDialogs();
    expect(jsonDecode(dialogs.dialog(action: 'get')), {'dialog': null});
    expect(() => dialogs.dialog(action: 'accept'), throwsStateError);
    expect(() => dialogs.dialog(action: 'dismiss'), throwsStateError);
  });

  test(
    'get inspects a prompt without resolving it and accept clears it',
    () async {
      final dialogs = RuntimeBrowserDialogs();
      var completed = false;
      final answer = dialogs
          .open(
            type: 'prompt',
            message: 'Name',
            url: 'https://example.com',
            defaultText: 'Alice',
            present: (_) => Completer<Object?>().future,
          )
          .then((value) {
            completed = true;
            return value;
          });
      expect(jsonDecode(dialogs.dialog(action: 'get')), {
        'dialog': {
          'type': 'prompt',
          'message': 'Name',
          'url': 'https://example.com',
          'defaultValue': 'Alice',
        },
      });
      await Future<void>.value();
      expect(completed, false);
      expect(jsonDecode(dialogs.dialog(action: 'accept')), {'dialog': null});
      expect(jsonDecode(dialogs.dialog(action: 'get')), {'dialog': null});
      expect(await answer, 'Alice');
    },
  );

  test('invalid actions and text do not consume a pending dialog', () async {
    final dialogs = RuntimeBrowserDialogs();
    final answer = dialogs.open(
      type: 'confirm',
      message: 'Continue?',
      url: 'about:blank',
      present: (_) => Completer<Object?>().future,
    );
    expect(() => dialogs.dialog(action: 'unknown'), throwsArgumentError);
    for (final action in ['get', 'accept', 'dismiss']) {
      expect(
        () => dialogs.dialog(action: action, promptText: ''),
        throwsArgumentError,
      );
    }
    expect(
      jsonDecode(dialogs.dialog(action: 'get'))['dialog']['type'],
      'confirm',
    );
    dialogs.dialog(action: 'accept');
    expect(await answer, true);
  });

  test(
    'responses and late manual results stay with their owning session',
    () async {
      final first = RuntimeBrowserDialogs();
      final second = RuntimeBrowserDialogs();
      final manual = Completer<Object?>();
      final one = first.open(
        type: 'confirm',
        message: 'One',
        url: 'about:blank',
        present: (_) => manual.future,
      );
      final two = second.open(
        type: 'confirm',
        message: 'Two',
        url: 'about:blank',
        present: (_) => Completer<Object?>().future,
      );
      first.dialog(action: 'dismiss');
      final next = first.open(
        type: 'alert',
        message: 'Next',
        url: 'about:blank',
        present: (_) => Completer<Object?>().future,
      );
      manual.complete(true);
      expect(await one, false);
      expect(
        jsonDecode(first.dialog(action: 'get'))['dialog']['message'],
        'Next',
      );
      expect(
        jsonDecode(second.dialog(action: 'get'))['dialog']['message'],
        'Two',
      );
      first.dialog(action: 'accept');
      second.dialog(action: 'dismiss');
      expect(await next, isNull);
      expect(await two, false);
    },
  );

  test(
    'automation receives the dialog before the triggering action finishes',
    () async {
      final dialogs = RuntimeBrowserDialogs();
      final manual = Completer<Object?>();
      final dismissed = Completer<void>();
      final result = dialogs.observe(() async {
        final answer = await dialogs.open(
          type: 'confirm',
          message: 'Continue?',
          url: 'https://example.com',
          present: (closed) {
            closed.then((_) => dismissed.complete());
            return manual.future;
          },
        );
        expect(answer, false);
        return 'action complete';
      });
      expect(jsonDecode(await result)['dialog']['message'], 'Continue?');
      dialogs.dialog(action: 'dismiss');
      await dismissed.future;
      expect(() => dialogs.dialog(action: 'accept'), throwsStateError);
      manual.complete(true);
    },
  );

  test(
    'prompt cancellation is distinct from empty and whitespace input',
    () async {
      final dialogs = RuntimeBrowserDialogs();
      final manual = Completer<Object?>();
      final result = dialogs.open(
        type: 'prompt',
        message: 'Name',
        url: 'about:blank',
        defaultText: 'default',
        present: (_) => manual.future,
      );
      dialogs.dialog(action: 'dismiss');
      expect(await result, isNull);
      manual.complete('ignored');
      for (final text in ['', '  ']) {
        final next = dialogs.open(
          type: 'prompt',
          message: 'Name',
          url: 'about:blank',
          present: (_) => Completer<Object?>().future,
        );
        dialogs.dialog(action: 'accept', promptText: text);
        expect(await next, text);
      }
    },
  );

  test(
    'manual completion and session closure resolve only their own dialog',
    () async {
      final dialogs = RuntimeBrowserDialogs();
      final manual = Completer<Object?>();
      final result = dialogs.open(
        type: 'confirm',
        message: 'Manual',
        url: 'about:blank',
        present: (_) => manual.future,
      );
      manual.complete(true);
      expect(await result, true);
      final next = dialogs.open(
        type: 'alert',
        message: 'Closing',
        url: 'about:blank',
        present: (_) => Completer<Object?>().future,
      );
      final failure = expectLater(next, throwsStateError);
      dialogs.close();
      await failure;
    },
  );
}
