import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserExternalNavigation.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/chrome/WorkspaceBrowserExternalNavigationDialog.dart';

/// Exercises confirmation, platform failures and repeated application handoffs.
void main() {
  testWidgets('long application links keep both actions usable on a phone', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 480);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    bool? confirmed;
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (context) => Scaffold(
            body: TextButton(
              onPressed: () async {
                confirmed = await showWorkspaceBrowserExternalNavigationDialog(
                  context,
                  'intent://scan/#Intent;S.payload=${'x' * 6000};end',
                );
              },
              child: const Text('launch'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('launch'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    await tester.tap(find.text('取消'));
    await tester.pumpAndSettle();
    expect(confirmed, isFalse);
    await tester.tap(find.text('launch'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('打开'));
    await tester.pumpAndSettle();
    expect(confirmed, isTrue);
    expect(tester.takeException(), isNull);
  });
  test('an application popup does not allocate an empty browser tab', () async {
    String? external;
    final result = await openRuntimeBrowserPopup<String>(
      sourceUrl: 'https://example.test/page',
      target: 'weixin://chat',
      openExternal: (url) async {
        external = url;
      },
      openTab: (_) async =>
          throw StateError('Must not create a tab for an application'),
    );
    expect(result, isNull);
    expect(external, 'weixin://chat');
  });
  test(
    'ordinary popup pages resolve relative links against their originating page',
    () async {
      final result = await openRuntimeBrowserPopup<String>(
        sourceUrl: 'https://example.test/path/page',
        target: '../next?id=1',
        openExternal: (_) async => fail('A webpage must stay in the browser'),
        openTab: (url) async => url,
      );
      expect(result, 'https://example.test/next?id=1');
    },
  );
  test('application protocols are distinct from browser resources', () {
    for (final url in [
      'weixin://chat',
      'alipays://pay',
      'tel:123',
      'mailto:a@example.test',
      'intent://scan/#Intent;scheme=zxing;end',
    ]) {
      expect(isRuntimeBrowserExternalUrl(url), isTrue, reason: url);
    }
    for (final url in [
      'https://example.test',
      'http://example.test',
      'file:///page.html',
      'content://private/page',
      'about:blank',
      'data:text/plain,test',
      'blob:https://example.test/file',
      'javascript:void(0)',
      '/relative',
    ]) {
      expect(isRuntimeBrowserExternalUrl(url), isFalse, reason: url);
    }
  });

  test('cancelling confirmation never invokes the operating system', () async {
    final handler = RuntimeBrowserExternalNavigation(
      confirm: (_) async => false,
      failed: (_, error) async => fail('Cancellation is not an error: $error'),
    );
    await handler.open(
      'weixin://chat',
      () async => throw StateError('Must not launch'),
    );
  });

  test(
    'an approved HTTPS application link reaches the system resolver once',
    () async {
      var launched = 0;
      final handler = RuntimeBrowserExternalNavigation(
        confirm: (url) async => url == 'https://example.test/app',
        failed: (_, error) async => fail('$error'),
      );
      await handler.open('https://example.test/app', () async {
        launched++;
        return true;
      });
      expect(launched, 1);
    },
  );

  test(
    'system refusal is reported and returned to the originating command',
    () async {
      Object? failure;
      final handler = RuntimeBrowserExternalNavigation(
        confirm: (_) async => true,
        failed: (_, error) async {
          failure = error;
        },
      );
      await expectLater(
        handler.open('missing://app', () async => false),
        throwsStateError,
      );
      expect(failure, isA<StateError>());
    },
  );

  test(
    'platform errors preserve their reason and release the pending request',
    () async {
      final failure = UnsupportedError('Intent is unavailable on this owner');
      final errors = <Object>[];
      final handler = RuntimeBrowserExternalNavigation(
        confirm: (_) async => true,
        failed: (_, error) async {
          errors.add(error);
        },
      );
      await expectLater(
        handler.open('intent://app', () async => throw failure),
        throwsA(same(failure)),
      );
      await handler.open('intent://app', () async => true);
      expect(errors, [failure]);
    },
  );

  test('repeated page events share one prompt and one launch', () async {
    final confirmation = Completer<bool>();
    var prompts = 0;
    var launches = 0;
    final handler = RuntimeBrowserExternalNavigation(
      confirm: (_) {
        prompts++;
        return confirmation.future;
      },
      failed: (_, error) async => fail('$error'),
    );
    final first = handler.open('weixin://chat', () async {
      launches++;
      return true;
    });
    final second = handler.open(
      'weixin://chat',
      () async => throw StateError('Duplicate launch'),
    );
    confirmation.complete(true);
    await Future.wait([first, second]);
    expect(prompts, 1);
    expect(launches, 1);
  });
}
