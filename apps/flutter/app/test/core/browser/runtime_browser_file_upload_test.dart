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

  test('evaluation preserves the decoded WebView return value', () async {
    expect(await upload.evaluate('42'), 42);
    expect(platform.evaluations, hasLength(1));
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

  /// Records one upload attempt and propagates an explicitly configured error.
  @override
  Future<void> runJavaScript(String javaScript) async {
    scripts.add(javaScript);
    if (failure case final error?) throw error;
  }

  /// Returns the decoded value provided by the shared WebView controller API.
  @override
  Future<Object> runJavaScriptReturningResult(String javaScript) async {
    evaluations.add(javaScript);
    return 42;
  }
}
