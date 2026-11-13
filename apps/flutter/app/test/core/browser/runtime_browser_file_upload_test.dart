import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserFileUpload.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Verifies upload payload validation and the shared WebView transport contract.
void main() {
  late _RecordingController platform;
  late RuntimeBrowserFileUpload upload;

  setUp(() {
    platform = _RecordingController();
    upload = RuntimeBrowserFileUpload(WebViewController.fromPlatform(platform));
  });

  test('cancellation and an empty selection remain distinct', () async {
    await upload.upload('null');
    expect(_selectionPayload(platform.scripts.single), isNull);
    await upload.upload('[]');
    expect(_selectionPayload(platform.scripts.last), isEmpty);
  });

  test(
    'Unicode names, binary bytes and inferred media types are preserved',
    () async {
      final encoded = base64Encode([0, 255, 10]);
      await upload.upload(
        jsonEncode([
          {'name': '中文.txt', 'base64': encoded},
        ]),
      );
      expect(_selectionPayload(platform.scripts.single), [
        {'name': '中文.txt', 'base64': encoded, 'type': 'text/plain'},
      ]);
    },
  );

  test('unknown optional media types are not fabricated', () async {
    await upload.upload(
      jsonEncode([
        {'name': 'data.operitunknown', 'base64': ''},
      ]),
    );
    expect(_selectionPayload(platform.scripts.single), [
      {'name': 'data.operitunknown', 'base64': ''},
    ]);
  });

  test('invalid upload containers do not execute page code', () {
    for (final payload in ['{}', '"file"', '42', 'true']) {
      expect(() => upload.upload(payload), throwsStateError);
    }
    expect(platform.scripts, isEmpty);
  });

  test('invalid file objects do not execute page code', () {
    for (final payload in [
      '[null]',
      '["file"]',
      '[{}]',
      '[{"name":"","base64":""}]',
      '[{"name":"file","base64":42}]',
    ]) {
      expect(() => upload.upload(payload), throwsStateError);
    }
    expect(platform.scripts, isEmpty);
  });

  test('invalid base64 is reported before the chooser can be consumed', () {
    expect(
      () => upload.upload('[{"name":"file.txt","base64":"!"}]'),
      throwsFormatException,
    );
    expect(platform.scripts, isEmpty);
  });

  test('every file is validated before committing a multiple-file upload', () {
    expect(
      () => upload.upload('[{"name":"ok.txt","base64":""}, {}]'),
      throwsStateError,
    );
    expect(platform.scripts, isEmpty);
  });

  test(
    'browser failures remain failures without another upload attempt',
    () async {
      platform.failure = StateError('No active browser file chooser');
      await expectLater(upload.upload('[]'), throwsStateError);
      expect(platform.scripts, hasLength(1));
    },
  );

  test(
    'evaluation preserves the result and sends the original script',
    () async {
      const script = 'var answer = 40; answer + 2;';
      expect(await upload.evaluate(script), 42);
      expect(platform.evaluations, [script]);
      expect(platform.scripts, hasLength(2));
      expect(platform.scripts.first, contains('beginCapture(0)'));
      expect(platform.scripts.last, contains('endCapture(0)'));
    },
  );

  test(
    'actions execute directly between opening and closing capture',
    () async {
      const script = 'document.querySelector("input")?.click();';
      await upload.run(script);
      expect(platform.scripts, hasLength(3));
      expect(platform.scripts.first, contains('beginCapture(0)'));
      expect(platform.scripts[1], script);
      expect(platform.scripts.last, contains('endCapture(0)'));
      expect(platform.evaluations, isEmpty);
    },
  );

  test('capture stays open until asynchronous evaluation finishes', () async {
    final result = Completer<Object>();
    platform.pendingResult = result.future;
    final evaluation = upload.evaluate('pending');
    await Future<void>.delayed(Duration.zero);
    expect(platform.evaluations, ['pending']);
    expect(platform.scripts, hasLength(1));
    result.complete(42);
    expect(await evaluation, 42);
    expect(platform.scripts.last, contains('endCapture(0)'));
  });

  test('failed capture setup does not execute the action', () async {
    final failure = StateError('WebView unavailable');
    platform.failure = failure;
    await expectLater(upload.evaluate('42'), throwsA(same(failure)));
    expect(platform.evaluations, isEmpty);
    expect(platform.scripts, hasLength(1));
  });

  test('evaluation failures propagate and release capture', () async {
    final failure = StateError('page script failed');
    platform.evaluationFailure = failure;
    await expectLater(upload.evaluate('broken'), throwsA(same(failure)));
    expect(platform.scripts.last, contains('endCapture(0)'));
  });

  test('action failures propagate and release capture', () async {
    final failure = StateError('click failed');
    platform.failureForScript = (script) => script == 'broken' ? failure : null;
    await expectLater(upload.run('broken'), throwsA(same(failure)));
    expect(platform.scripts.last, contains('endCapture(0)'));
  });

  test('cleanup failures do not hide the original evaluation error', () async {
    final failure = StateError('original page error');
    platform.evaluationFailure = failure;
    platform.failureForScript = (script) => script.contains('?.endCapture(')
        ? StateError('WebView disposed during cleanup')
        : null;
    await expectLater(upload.evaluate('broken'), throwsA(same(failure)));
  });

  test('cleanup failures after successful actions remain failures', () async {
    final failure = StateError('WebView disposed during cleanup');
    platform.failureForScript = (script) =>
        script.contains('?.endCapture(') ? failure : null;
    await expectLater(upload.evaluate('42'), throwsA(same(failure)));
  });

  test('each native action uses a distinct capture scope', () async {
    await upload.evaluate('first');
    await upload.evaluate('second');
    expect(platform.scripts[0], contains('beginCapture(0)'));
    expect(platform.scripts[1], contains('endCapture(0)'));
    expect(platform.scripts[2], contains('beginCapture(1)'));
    expect(platform.scripts[3], contains('endCapture(1)'));
  });
}

/// Extracts the exact final upload invocation from the production transport script.
Object? _selectionPayload(String script) {
  final match = RegExp(
    r'window\.__operitBrowserFileChooser\.upload\((.*)\)\);',
  ).firstMatch(script);
  expect(match, isNotNull);
  return jsonDecode(match!.group(1)!);
}

/// Records shared controller calls without introducing a platform implementation.
class _RecordingController extends PlatformWebViewController {
  /// Creates a recorder bound to the standard WebView creation contract.
  _RecordingController()
    : super.implementation(const PlatformWebViewControllerCreationParams());

  final scripts = <String>[];
  final evaluations = <String>[];
  Object? failure;
  Object? evaluationFailure;
  Object? Function(String script)? failureForScript;
  Future<Object>? pendingResult;

  /// Records one upload attempt and propagates an explicitly configured error.
  @override
  Future<void> runJavaScript(String javaScript) async {
    scripts.add(javaScript);
    if (failure case final error?) throw error;
    if (failureForScript?.call(javaScript) case final error?) throw error;
  }

  /// Returns the decoded value provided by the shared WebView controller API.
  @override
  Future<Object> runJavaScriptReturningResult(String javaScript) async {
    evaluations.add(javaScript);
    if (evaluationFailure case final error?) throw error;
    return pendingResult ?? Future.value(42);
  }
}
