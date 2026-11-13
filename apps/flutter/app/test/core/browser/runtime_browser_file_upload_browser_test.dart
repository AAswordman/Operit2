@TestOn('browser')
library;

import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserFileUpload.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

@JS('JSON.stringify')
external JSString? _stringify(JSAny? value);

/// Exercises production scripts with native injection independent of page eval.
void main() {
  late _NativeScriptController platform;
  late RuntimeBrowserFileUpload upload;
  late JSFunction originalEval;

  setUp(() {
    originalEval = globalContext.getProperty<JSFunction>('eval'.toJS);
    platform = _NativeScriptController(originalEval);
    upload = RuntimeBrowserFileUpload(WebViewController.fromPlatform(platform));
    platform.inject('''
window.eval = function() { throw new EvalError('unsafe-eval is blocked'); };
''');
  });

  tearDown(() {
    globalContext.setProperty('eval'.toJS, originalEval);
  });

  test(
    'expressions and statement completion work when page eval is blocked',
    () async {
      expect(await upload.evaluate('1 + 1'), 2);
      expect(await upload.evaluate('({sum: 2, ok: true})'), {
        'sum': 2,
        'ok': true,
      });
      expect(await upload.evaluate('null'), isNull);
      expect(await upload.evaluate('undefined'), isNull);
      expect(
        await upload.evaluate('var __cspAnswer = 40; __cspAnswer + 2;'),
        42,
      );
      expect(await upload.evaluate('window.__cspAnswer'), 40);
      expect(
        await upload.evaluate('window.__operitBrowserFileChooser.captureDepth'),
        1,
      );
      expect(
        platform
            .inject('window.__operitBrowserFileChooser.captureDepth')
            ?.dartify(),
        0,
      );
    },
  );

  test(
    'direct actions still capture file inputs and complete uploads',
    () async {
      platform.inject('''
var __cspInput = document.createElement('input');
__cspInput.type = 'file';
window.__cspChanges = 0;
__cspInput.addEventListener('change', () => window.__cspChanges++);
''');
      await upload.run('window.__cspInput.click();');
      expect(
        await upload.evaluate(
          'window.__operitBrowserFileChooser.pendingInput === window.__cspInput',
        ),
        true,
      );
      await upload.upload(
        jsonEncode([
          {
            'name': 'csp.txt',
            'base64': base64Encode(utf8.encode('CSP upload')),
          },
        ]),
      );
      expect(
        await upload.evaluate('window.__cspInput.files[0].name'),
        'csp.txt',
      );
      expect(await upload.evaluate('window.__cspChanges'), 1);
      expect(
        await upload.evaluate('window.__operitBrowserFileChooser.pendingInput'),
        isNull,
      );
    },
  );

  test('syntax and runtime errors propagate and release capture', () async {
    for (final script in [
      'const = invalid',
      'throw new Error("page failure")',
    ]) {
      await expectLater(upload.evaluate(script), throwsStateError);
      expect(
        platform
            .inject('window.__operitBrowserFileChooser.captureDepth')
            ?.dartify(),
        0,
      );
    }
    await expectLater(
      upload.run('throw new Error("click failure")'),
      throwsStateError,
    );
    expect(
      platform
          .inject('window.__operitBrowserFileChooser.captureDepth')
          ?.dartify(),
      0,
    );
  });

  test(
    'independent native scopes tolerate overlap and stale cleanup',
    () async {
      await upload.evaluate('42');
      platform.inject('''
window.__operitBrowserFileChooser.beginCapture('first');
window.__operitBrowserFileChooser.beginCapture('second');
window.__operitBrowserFileChooser.endCapture('first');
window.__operitBrowserFileChooser.endCapture('old-document');
''');
      expect(
        platform
            .inject('window.__operitBrowserFileChooser.captureDepth')
            ?.dartify(),
        1,
      );
      platform.inject(
        "window.__operitBrowserFileChooser.endCapture('second');",
      );
      expect(
        platform
            .inject('window.__operitBrowserFileChooser.captureDepth')
            ?.dartify(),
        0,
      );
    },
  );
}

/// Uses a captured native evaluator, as WebViews do, even if page eval is blocked.
class _NativeScriptController extends PlatformWebViewController {
  _NativeScriptController(this._nativeEvaluate)
    : super.implementation(const PlatformWebViewControllerCreationParams());

  final JSFunction _nativeEvaluate;

  JSAny? inject(String script) {
    try {
      return _nativeEvaluate.callAsFunction(null, script.toJS);
    } catch (error) {
      throw StateError(error.toString());
    }
  }

  @override
  Future<void> runJavaScript(String javaScript) async {
    inject(javaScript);
  }

  @override
  Future<Object?> runJavaScriptReturningResult(String javaScript) async {
    final result = inject(javaScript);
    if (result == null) return null;
    final encoded = _stringify(result);
    if (encoded == null) throw StateError('Unsupported JavaScript result');
    return jsonDecode(encoded.toDart);
  }
}
