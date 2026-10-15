@TestOn('browser')
library;

import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

@JS('JSON.stringify')
external JSString? _stringify(JSAny? value);

void main() {
  late WebViewController controller;

  setUp(() {
    controller = WebViewController.fromPlatform(_ScriptPlatform());
  });

  test('null, undefined and void functions resolve to null', () async {
    for (final script in ['null', 'undefined', '(function() {})()', '']) {
      expect(await controller.runJavaScriptReturningResult(script), isNull);
    }
  });

  test('preserves JSON-compatible result types and string contents', () async {
    final cases = <String, Object?>{
      '42': 42,
      'false': false,
      '"null"': 'null',
      '"42"': '42',
      '""': '',
      '"{\\"ok\\":true}"': '{"ok":true}',
      '[1, null, "two"]': [1, null, 'two'],
      '({ok: true, value: null})': {'ok': true, 'value': null},
    };
    for (final entry in cases.entries) {
      expect(
        await controller.runJavaScriptReturningResult(entry.key),
        entry.value,
        reason: entry.key,
      );
    }
  });

  test('executes once and preserves global declarations', () async {
    await controller.runJavaScriptReturningResult(
      'var __resultTestCount = 0; '
      'function __resultTestIncrement() { __resultTestCount++; }',
    );
    expect(
      await controller.runJavaScriptReturningResult('__resultTestIncrement();'),
      isNull,
    );
    expect(
      await controller.runJavaScriptReturningResult('__resultTestCount'),
      1,
    );
  });

  test('save completion executes when page eval is blocked', () async {
    final originalEval = globalContext.getProperty<JSFunction>('eval'.toJS);
    void blockEval(JSAny? script) {
      throw StateError('Content Security Policy blocks eval');
    }

    final blockedEval = blockEval.toJS;
    await controller.runJavaScriptReturningResult(
      "window.__entityClosed = null; "
      "window.addEventListener('character-entity-closed', function(event) { "
      "window.__entityClosed = event.detail; }, {once: true});",
    );
    globalContext.setProperty('eval'.toJS, blockedEval);
    try {
      expect(
        await controller.runJavaScriptReturningResult(
          "window.dispatchEvent(new CustomEvent('character-entity-closed', "
          "{detail: {operation: 'saved', id: 'group-1'}}));",
        ),
        true,
      );
      expect(
        await controller.runJavaScriptReturningResult('window.__entityClosed'),
        {'operation': 'saved', 'id': 'group-1'},
      );
    } finally {
      globalContext.setProperty('eval'.toJS, originalEval);
    }
  });

  test('execution and serialization errors remain failures', () async {
    for (final script in [
      'throw new Error("test failure")',
      'const = invalid',
      '({ value: 1n })',
      '(() => { const a = {}; a.self = a; return a; })()',
      '(function() {})',
    ]) {
      await expectLater(
        controller.runJavaScriptReturningResult(script),
        throwsStateError,
        reason: script,
      );
    }
  });
}

/// Captures the native evaluator independently of page-owned eval and returns
/// decoded results as the native WebView adapters do.
class _ScriptPlatform extends PlatformWebViewController {
  _ScriptPlatform()
    : _nativeEvaluate = globalContext.getProperty<JSFunction>('eval'.toJS),
      super.implementation(const PlatformWebViewControllerCreationParams());

  final JSFunction _nativeEvaluate;

  @override
  Future<Object?> runJavaScriptReturningResult(String javaScript) async {
    try {
      final result = _nativeEvaluate.callAsFunction(null, javaScript.toJS);
      if (result == null) return null;
      final encoded = _stringify(result);
      if (encoded == null) throw StateError('Unsupported JavaScript result');
      return jsonDecode(encoded.toDart);
    } catch (error) {
      throw StateError(error.toString());
    }
  }
}
