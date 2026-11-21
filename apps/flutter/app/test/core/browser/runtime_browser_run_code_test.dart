import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserAutomationController.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

void main() {
  test('run code executes snippets as an async function body', () async {
    final platform = _Controller()
      ..pollStates.add(_settled({'ok': 1}));
    final browser = _browser(platform);
    expect(await browser.runCode('return { ok: 1 };'), {'ok': 1});
    final kickoff = platform.scripts[1];
    expect(kickoff, contains('async function() {\nreturn { ok: 1 };\n  })()'));
    expect(kickoff, contains('__operitRunCodeResults'));
  });

  test('run code awaits asynchronous snippets before returning', () async {
    final platform = _Controller()
      ..pollStates.add({'pending': true})
      ..pollStates.add(_settled(7));
    final browser = _browser(platform);
    expect(
      await browser.runCode('await Promise.resolve(); return 7;'),
      7,
    );
    expect(platform.evaluations, hasLength(2));
  });

  test('run code surfaces snippet errors', () async {
    final platform = _Controller()
      ..pollStates.add({'pending': false, 'error': 'boom'});
    final browser = _browser(platform);
    await expectLater(browser.runCode('throw new Error("boom");'), throwsStateError);
  });

  test('run code keeps non-serializable results as text', () async {
    final platform = _Controller()
      ..pollStates.add({
        'pending': false,
        'value': {'text': '[object HTMLDivElement]'},
      });
    final browser = _browser(platform);
    expect(await browser.runCode('return document.body;'), '[object HTMLDivElement]');
  });

  test('run code reports a lost page instead of hanging', () async {
    final platform = _Controller()..pollStates.add(null);
    final browser = _browser(platform);
    await expectLater(browser.runCode('return 1;'), throwsStateError);
  });

  test('run code keeps the file chooser capture across polling', () async {
    final platform = _Controller()
      ..pollStates.add({'pending': true})
      ..pollStates.add(_settled(null));
    final browser = _browser(platform);
    expect(await browser.runCode('return undefined;'), isNull);
    expect(platform.scripts.first, contains('beginCapture'));
    expect(platform.scripts.last, contains('endCapture'));
    expect(platform.evaluations, hasLength(2));
  });
}

RuntimeBrowserAutomationController _browser(_Controller platform) {
  return RuntimeBrowserAutomationController(
    controller: WebViewController.fromPlatform(platform),
    onSurfaceFrameRequested: () async {},
  );
}

Map<String, Object?> _settled(Object? value) {
  return {
    'pending': false,
    'value': value == null ? null : {'json': jsonEncode(value)},
  };
}

class _Controller extends PlatformWebViewController {
  _Controller()
    : super.implementation(const PlatformWebViewControllerCreationParams());

  final scripts = <String>[];
  final evaluations = <String>[];
  final pollStates = <Object?>[];
  int _polls = 0;

  @override
  Future<void> runJavaScript(String javaScript) async {
    scripts.add(javaScript);
  }

  @override
  Future<Object?> runJavaScriptReturningResult(String javaScript) async {
    evaluations.add(javaScript);
    final index = _polls < pollStates.length ? _polls++ : pollStates.length - 1;
    return jsonEncode(pollStates[index]);
  }
}
