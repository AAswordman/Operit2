import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {stripTypeScriptTypes} from 'node:module';
import vm from 'node:vm';
import test from 'node:test';

const root = new URL('../../apps/flutter/thirdparty/webview_all_ohos/ohos/src/main/ets/com.abandoft.webview_all_ohos/', import.meta.url);

/** Loads the production ArkTS control flow with only the Flutter channel replaced. */
function load(name, globals, exported) {
  const source = readFileSync(new URL(name, root), 'utf8')
    .replace(/^import .*;\r?$/gm, '').replace(/^export /gm, '');
  const context = vm.createContext(globals);
  vm.runInContext(stripTypeScriptTypes(source) + `\nthis.exports = {${exported}};`, context);
  return context.exports;
}

/** Records replies separately from the pending operating-system handoff. */
function resultRecorder() {
  const replies = [];
  return {replies, result: {
    success: value => replies.push(['success', value]),
    error: (code, message) => replies.push(['error', code, message]),
    notImplemented: () => replies.push(['notImplemented']),
  }};
}

/** Attaches the real native handler to a controllable Ability lifecycle. */
function handler(getContext) {
  const channels = [];
  class MethodChannel {
    constructor(_, name) { this.name = name; channels.push(this); }
    setMethodCallHandler(value) { this.handler = value; }
  }
  const api = load('WebViewExternalNavigation.ets', {MethodChannel}, 'WebViewExternalNavigation, isExternalApplicationUrl');
  const bridge = new api.WebViewExternalNavigation({}, getContext);
  return {bridge, channels, api};
}

/** Supplies the same named argument envelope as the Dart method channel. */
function request(url) {
  return {method: 'open', argument: name => name === 'url' ? url : undefined};
}

test('ArkWeb does not report success before the Ability has accepted the URL', async () => {
  let resolve;
  let want;
  const pending = new Promise(done => { resolve = done; });
  const {bridge, channels} = handler(() => ({startAbility: target => { want = target; return pending; }}));
  const {replies, result} = resultRecorder();
  bridge.onMethodCall(request('weixin://chat?id=1'), result);
  assert.deepEqual(replies, []);
  assert.equal(want.uri, 'weixin://chat?id=1');
  assert.equal(want.action, 'ohos.want.action.viewData');
  assert.deepEqual(Array.from(want.entities), ['entity.system.browsable']);
  resolve();
  await pending;
  assert.deepEqual(replies, [['success', true]]);
  bridge.dispose();
  assert.equal(channels[0].handler, null);
});

test('ArkWeb preserves asynchronous system refusal instead of swallowing it', async () => {
  const pending = Promise.reject(new Error('No handler installed'));
  const {bridge} = handler(() => ({startAbility: () => pending}));
  const {replies, result} = resultRecorder();
  bridge.onMethodCall(request('missing://app'), result);
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(replies, [['error', 'external_navigation_failed', 'No handler installed']]);
});

test('Ability detachment and unsupported targets never masquerade as success', () => {
  const {bridge} = handler(() => null);
  const unavailable = resultRecorder();
  bridge.onMethodCall(request('weixin://chat'), unavailable.result);
  assert.equal(unavailable.replies[0][1], 'external_navigation_unavailable');
  for (const url of ['file:///private', 'content://private', 'javascript:alert(1)', 'intent://app', '/relative']) {
    const {replies, result} = resultRecorder();
    bridge.onMethodCall(request(url), result);
    assert.equal(replies[0][1], 'invalid_external_url', url);
  }
});

test('ArkWeb leaves webpage subframes in the engine and captures application protocols', () => {
  const {api} = handler(() => null);
  const {OhosWebViewClient} = load('WebViewClientHostApiImpl.ets', {isExternalApplicationUrl: api.isExternalApplicationUrl}, 'OhosWebViewClient');
  const requests = [];
  const client = new OhosWebViewClient({requestLoadingImpl: (...args) => requests.push(args)});
  client.setReturnValueForShouldOverrideUrlLoading(true);
  for (const [url, mainFrame, prevented] of [
    ['https://example.test/frame', false, false],
    ['about:blank', false, false],
    ['https://example.test/page', true, true],
    ['WEIXIN://chat', false, true],
    ['intent://app', false, true],
  ]) {
    assert.equal(client.shouldOverrideUrlLoading({}, {
      isMainFrame: () => mainFrame, getRequestUrl: () => url,
    }), prevented, url);
  }
  assert.equal(requests.length, 5);
});
