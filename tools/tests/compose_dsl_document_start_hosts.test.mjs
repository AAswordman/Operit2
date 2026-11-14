import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
const root = new URL('../../', import.meta.url);
/** Loads a workspace source file for checking the native registration contract. */
function source(path) { return readFileSync(new URL(path, root), 'utf8'); }

test('Android selects native document-start by provider capability and owns removable scripts', () => {
  const native = source('apps/flutter/thirdparty/webview_flutter_android/android/src/main/java/io/flutter/plugins/webviewflutter/WebViewUserScripts.java');
  assert.match(native, /WebViewCompat\.addDocumentStartJavaScript\(view, source, Collections\.singleton\("\*"\)\)/);
  assert.match(native, /ScriptHandler handler/);
  assert.match(native, /handler\.remove\(\)/);
  assert.match(native, /if \(WebViewFeature\.isFeatureSupported\(WebViewFeature\.DOCUMENT_START_SCRIPT\)\)/);
  assert.match(native, /if \(script\.handler == null\) view\.evaluateJavascript\(script\.source, null\)/);
  assert.doesNotMatch(native, /throw new UnsupportedOperationException|loadDataWithBaseURL/);
});

test('application awaits document-start registration and never rewrites URL resource HTML', () => {
  const app = source('apps/flutter/app/lib/ui/features/packages/screens/ToolPkgComposeDslWebView.dart');
  const loader = source('apps/flutter/app/lib/ui/features/packages/screens/ToolPkgComposeDslWebViewResourceLoader.dart');
  const load = app.slice(app.indexOf('Future<void> _load()'), app.indexOf('void _applyControllerSettingsIfNeeded'));
  assert.ok(load.indexOf('await _prepareDocumentStartBridge()') < load.indexOf('_controller.loadRequest'));
  assert.match(app, /_controller\s*\.addDocumentStartJavaScript\(source\)/);
  assert.match(app, /_controller\.removeDocumentStartJavaScript\(/);
  assert.doesNotMatch(loader, /prepareMainFrameHtml|injectComposeDslWebViewBridgeRuntime/);
  assert.doesNotMatch(app, /isUserScriptInjectionSupported|_installComposeDslWebViewBridgeRuntime/);
});

test('Apple and Windows use native lifecycle scripts rather than resource body rewriting', () => {
  const apple = source('apps/flutter/thirdparty/webview_flutter_wkwebview/lib/src/webkit_webview_controller.dart');
  const windows = source('apps/flutter/thirdparty/webview_all_windows/lib/src/windows_webview_controller.dart');
  assert.match(apple, /injectionTime: UserScriptInjectionTime\.atDocumentStart/);
  assert.match(apple, /for \(final script in _applicationUserScripts\.values\)/);
  assert.match(windows, /addScriptToExecuteOnDocumentCreated/);
  assert.match(windows, /removeScriptToExecuteOnDocumentCreated\(identifier\)/);
});

test('ArkWeb commits its native document-start list before acknowledging registration', () => {
  const component = source('apps/flutter/thirdparty/webview_all_ohos/ohos/src/main/ets/com.abandoft.webview_all_ohos/OhosWebView.ets');
  const view = source('apps/flutter/thirdparty/webview_all_ohos/ohos/src/main/ets/com.abandoft.webview_all_ohos/WebViewPlatformView.ets');
  assert.match(component, /\.runJavaScriptOnDocumentStart\(this\.documentStartScripts\)/);
  assert.match(component, /postFrameCallback\(commit\)/);
  assert.match(component, /onIdle\(_timeLeftInNano: number\)/);
  assert.match(view, /await host\(this\.getDocumentStartScripts\(\)\)/);
});

test('the common interface has no default false capability or late-execution substitution', () => {
  const common = source('apps/flutter/thirdparty/webview_flutter_platform_interface/lib/src/platform_webview_controller.dart');
  const facade = source('apps/flutter/thirdparty/webview_all/lib/src/webview_controller.dart');
  assert.doesNotMatch(common, /isUserScriptInjectionSupported/);
  assert.match(facade, /addDocumentStartJavaScript\(String source\)/);
  assert.match(facade, /platform\.addUserScript\(/);
});
